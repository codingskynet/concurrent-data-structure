/*
 * This code is refered to https://github.com/khizmax/libcds/blob/master/cds/algo/flat_combining/kernel.h
 */

use std::{
    cell::UnsafeCell,
    fmt::Debug,
    ptr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use crossbeam_epoch::{pin, unprotected, Atomic, Guard, Owned, Shared};
use crossbeam_utils::Backoff;
use thread_local::ThreadLocal;

use super::spinlock::RawSpinLock;

pub trait FlatCombining<T> {
    fn apply(&mut self, operation: T) -> T;
}

const COMPACT_FACTOR: usize = 16 - 1;
const MAX_AGE: usize = 1000;

pub struct Record<T> {
    operation: Atomic<T>, // The tag 0/1 means response/request.
    state: AtomicBool,    // false: inactive, true: active
    age: AtomicUsize,
    next: Atomic<Record<T>>,
}

impl<T: Debug> Debug for Record<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let guard = &pin();

        unsafe {
            let mut debug = f.debug_struct("Record");

            if let Some(operation) = self.operation.load(Ordering::SeqCst, guard).as_ref() {
                debug.field("operation", operation);
            } else {
                debug.field("operation", &"null");
            }

            debug.field("state", &self.state.load(Ordering::SeqCst));

            debug.field("age", &self.age.load(Ordering::SeqCst));

            if let Some(next) = self.next.load(Ordering::SeqCst, guard).as_ref() {
                debug.field("next", next).finish()
            } else {
                debug.field("next", &"null").finish()
            }
        }
    }
}

impl<T: Send> Record<T> {
    #[inline]
    pub fn set(&self, operation: T) {
        self.operation
            .store(Owned::new(operation).with_tag(1), Ordering::Release);
    }

    #[inline]
    fn is_response(&self, guard: &Guard) -> bool {
        self.operation.load(Ordering::Acquire, guard).tag() == 0
    }

    #[inline]
    pub fn get_operation(&self, guard: &Guard) -> T {
        unsafe { ptr::read(self.operation.load(Ordering::Relaxed, guard).deref()) }
    }
}

pub struct FCLock<T: Send + Sync> {
    publications: Atomic<Record<T>>,
    lock: RawSpinLock,
    target: UnsafeCell<Box<dyn FlatCombining<T>>>,
    thread_local: ThreadLocal<Atomic<Record<T>>>,
    age: AtomicUsize,
}

impl<T: Send + Sync> Drop for FCLock<T> {
    fn drop(&mut self) {
        unsafe {
            let guard = unprotected();

            for local_record in self.thread_local.iter() {
                let dummy = local_record.load(Ordering::Relaxed, guard);
                drop(dummy.into_owned());
            }
        }
    }
}

impl<T: Send + Sync> FCLock<T> {
    fn combine(&self, guard: &Guard) {
        let current_age = self.age.fetch_add(1, Ordering::Relaxed) + 1;

        // TODO: this way is useful?
        let mut useful_pass = 0;
        let mut empty_pass = 0;
        for _ in 0..100 {
            if self.combine_pass(current_age, guard) {
                useful_pass += 1;
            } else {
                empty_pass += 1;

                if empty_pass > useful_pass {
                    break;
                }
            }
        }

        if current_age & COMPACT_FACTOR == 0 {
            self.clean(guard);
        }
    }

    fn combine_pass(&self, current_age: usize, guard: &Guard) -> bool {
        let mut is_done = false;

        unsafe {
            let target = &mut *self.target.get();

            let mut node = self.publications.load(Ordering::Acquire, guard);

            while !node.is_null() {
                let node_ref = node.deref();

                if node_ref.state.load(Ordering::Acquire) {
                    // active record
                    let operation = node_ref.operation.load(Ordering::Acquire, guard);

                    if operation.tag() == 1 {
                        let operation_ref = operation.deref();

                        let op = ptr::read(operation_ref);

                        node_ref.age.store(current_age, Ordering::Relaxed);

                        let result_op = target.apply(op);

                        node_ref
                            .operation
                            .store(Owned::new(result_op).with_tag(0), Ordering::Release);

                        is_done = true;
                    }
                }

                node = node_ref.next.load(Ordering::Acquire, guard);
            }
        }

        is_done
    }

    fn clean(&self, guard: &Guard) {
        unsafe {
            let current_age = self.age.load(Ordering::Relaxed);

            let mut parent = self.publications.load(Ordering::Acquire, guard);
            let mut node = parent.deref().next.load(Ordering::Acquire, guard);

            while !node.is_null() {
                let node_ref = node.deref();

                if !node_ref.state.load(Ordering::Acquire)
                    && current_age.wrapping_sub(node_ref.age.load(Ordering::Relaxed)) >= MAX_AGE
                {
                    // remove old inactive node
                    let parent_ref = parent.deref();
                    let new = node_ref.next.load(Ordering::Acquire, guard);

                    if parent_ref
                        .next
                        .compare_exchange(node, new, Ordering::Acquire, Ordering::Relaxed, guard)
                        .is_ok()
                    {
                        node_ref.state.store(false, Ordering::Relaxed);

                        node = new;
                    }

                    continue; // retry
                }

                // just move next
                parent = node;
                node = node_ref.next.load(Ordering::Acquire, guard);
            }
        }
    }

    pub fn new(target: impl FlatCombining<T> + 'static) -> Self {
        Self {
            publications: Atomic::null(),
            lock: RawSpinLock::new(),
            target: UnsafeCell::new(Box::new(target)),
            thread_local: ThreadLocal::new(),
            age: AtomicUsize::new(0),
        }
    }

    pub fn acquire_record<'a>(&self, guard: &'a Guard) -> Shared<'a, Record<T>> {
        let node = self.thread_local.get_or(|| {
            Atomic::new(Record {
                operation: Atomic::null(),
                state: AtomicBool::new(false),
                age: AtomicUsize::new(0),
                next: Atomic::null(),
            })
        });

        let node = node.load(Ordering::Relaxed, guard);

        if unsafe { !node.deref().state.load(Ordering::Acquire) } {
            self.push_record(node, guard);
        }

        node
    }

    pub fn push_record(&self, record: Shared<Record<T>>, guard: &Guard) {
        unsafe {
            let record_ref = record.deref();

            debug_assert!(!record_ref.state.load(Ordering::Relaxed));

            record_ref.state.store(true, Ordering::Relaxed);

            let backoff = Backoff::new();

            loop {
                let head = self.publications.load(Ordering::Relaxed, guard);

                record_ref.next.store(head, Ordering::Release);

                if self
                    .publications
                    .compare_exchange(head, record, Ordering::Release, Ordering::Relaxed, guard)
                    .is_ok()
                {
                    return;
                }

                backoff.spin();
            }
        }
    }

    #[inline]
    fn repush_record(&self, record: Shared<Record<T>>, guard: &Guard) {
        unsafe {
            if !record.deref().state.load(Ordering::Acquire) {
                self.push_record(record, guard);
            }
        }
    }

    pub fn try_combine(&self, record: Shared<Record<T>>, guard: &Guard) {
        unsafe {
            let record_ref = record.deref();

            if self.lock.try_lock().is_ok() {
                // now the thread is combiner
                self.repush_record(record, guard);

                self.combine(guard);

                self.lock.unlock();
            } else {
                // wait and the thread may be combiner if its operation is not finished and it gets lock
                let backoff = Backoff::new();

                while !record_ref.is_response(guard) {
                    self.repush_record(record, guard);

                    if self.lock.try_lock().is_ok() {
                        // Another combiner is finished. So, it can receive response

                        if !record_ref.is_response(guard) {
                            // It does not receive response. So, the thread becomes combiner
                            self.repush_record(record, guard);

                            self.combine(guard);
                        }

                        self.lock.unlock();
                        break;
                    }

                    backoff.snooze();
                }
            }
        }
    }
}
