/*
 * This code is refered to https://github.com/khizmax/libcds/blob/master/cds/algo/flat_combining/kernel.h
 */

use std::{
    cell::UnsafeCell,
    fmt::Debug,
    ops::Deref,
    ptr,
    sync::atomic::{fence, AtomicBool, AtomicUsize, Ordering},
    thread::{self, ThreadId},
};

use crossbeam_epoch::{pin, Atomic, Guard, Owned, Shared};
use crossbeam_utils::{atomic::AtomicConsume, Backoff};
use thread_local::ThreadLocal;

use super::spinlock::RawSpinLock;

pub trait FlatCombining<T: Operation> {
    fn apply(&mut self, operation: T) -> T;
}

pub trait Operation {
    fn is_request(&self) -> bool;
}

#[derive(Clone, PartialEq, Debug)]
pub enum State {
    Inactive,
    Active,
    Removed,
}

const MAX_AGE: usize = 5;

pub struct Record<T> {
    operation: Atomic<T>,
    state: Atomic<State>,
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

            if let Some(state) = self.state.load(Ordering::SeqCst, guard).as_ref() {
                debug.field("state", state);
            } else {
                debug.field("state", &"null");
            }

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
    pub fn set(&self, operation: T) {
        self.operation
            .store(Owned::new(operation), Ordering::Release);
    }

    pub fn release(&self) {
        self.state
            .store(Owned::new(State::Inactive), Ordering::Release);
    }

    #[inline]
    pub fn get_state<'a>(&self, guard: &'a Guard) -> &'a State {
        unsafe { self.state.load(Ordering::Acquire, guard).deref() }
    }

    pub fn get_operation_ref<'a>(&self, guard: &'a Guard) -> &'a T {
        unsafe { self.operation.load(Ordering::Acquire, guard).deref() }
    }

    pub fn operation_null(&self, guard: &Guard) -> bool {
        self.operation.load(Ordering::Acquire, guard).is_null()
    }

    pub fn get_operation(&self, guard: &Guard) -> T {
        unsafe { ptr::read(self.operation.load(Ordering::Acquire, guard).deref()) }
    }
}

pub struct FCLock<T: Operation + Send + Sync> {
    publications: Atomic<Record<T>>,
    lock: RawSpinLock,
    target: UnsafeCell<Box<dyn FlatCombining<T>>>,
    thread_local: ThreadLocal<Atomic<Record<T>>>,
    age: AtomicUsize,
}

impl<T: Operation + Send + Sync + Debug> FCLock<T> {
    fn print_publications(&self, guard: &Guard) {
        unsafe {
            println!(
                "{:?}",
                self.publications.load(Ordering::SeqCst, guard).deref()
            );
        }
    }

    fn combine(&self, guard: &Guard) {
        unsafe {
            let target = &mut *self.target.get();

            let current_age = self.age.fetch_add(1, Ordering::Relaxed) + 1;

            let mut node = self.publications.load(Ordering::Acquire, guard);

            // if !node.is_null() {
            // println!("B: {:?}, {:?}", node.deref(), thread::current().id());
            // }

            while !node.is_null() {
                let node_ref = node.deref();

                match node_ref.get_state(guard) {
                    State::Active => {
                        if !node_ref.operation_null(guard)
                            && node_ref.get_operation_ref(guard).is_request()
                        {
                            let op = ptr::read(
                                node_ref.operation.load(Ordering::Acquire, guard).deref(),
                            );

                            node_ref.age.store(current_age, Ordering::Relaxed);

                            let result_op = target.apply(op);

                            node_ref
                                .operation
                                .store(Owned::new(result_op), Ordering::Release);
                        }
                    }
                    State::Inactive => {
                        node_ref.age.fetch_add(1, Ordering::Relaxed);
                    }
                    State::Removed => unreachable!(),
                }

                node = node_ref.next.load(Ordering::Acquire, guard);
            }

            // if !self.publications.load(Ordering::Acquire, guard).is_null() {
            //     println!(
            //         "A: {:?}, {:?}",
            //         self.publications.load(Ordering::Acquire, guard).deref(),
            //         thread::current().id()
            //     );
            // }
        }

        self.clean(guard);
    }

    fn clean(&self, guard: &Guard) {
        unsafe {
            let current_age = self.age.load(Ordering::Relaxed);

            let mut parent = self.publications.load(Ordering::Acquire, guard);
            let mut node = parent.deref().next.load(Ordering::Acquire, guard);

            while !node.is_null() {
                let node_ref = node.deref();

                if *node_ref.state.load(Ordering::Acquire, guard).deref() == State::Inactive
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
                        node_ref
                            .state
                            .store(Owned::new(State::Inactive), Ordering::Relaxed);

                        // node_ref.next.store(Shared::null(), Ordering::Release);

                        node = new;
                        continue;
                    } else {
                        continue; // retry
                    }
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
                state: Atomic::new(State::Inactive),
                age: AtomicUsize::new(0),
                next: Atomic::null(),
            })
        });

        let node = node.load(Ordering::Relaxed, guard);

        if unsafe { *node.deref().get_state(guard) != State::Active } {
            self.push_record(node, guard);
        }

        node
    }

    pub fn push_record(&self, record: Shared<Record<T>>, guard: &Guard) {
        unsafe {
            let record_ref = record.deref();

            debug_assert_eq!(*record_ref.get_state(guard), State::Inactive);

            record_ref
                .age
                .store(self.age.load(Ordering::Relaxed), Ordering::Relaxed);

            record_ref
                .state
                .store(Owned::new(State::Active), Ordering::Relaxed);

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
            }
        }
    }

    #[inline]
    fn repush_record(&self, record: Shared<Record<T>>, guard: &Guard) {
        unsafe {
            if *record.deref().get_state(guard) != State::Active {
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
                while record_ref.get_operation_ref(guard).is_request() {
                    self.repush_record(record, guard);

                    if self.lock.try_lock().is_ok() {
                        // Another combiner is finished. So, it can receive response

                        if record_ref.get_operation_ref(guard).is_request() {
                            // It does not receive response. So, the thread becomes combiner
                            self.repush_record(record, guard);

                            self.combine(guard);
                        }

                        self.lock.unlock();
                        break;
                    }
                }
            }

            debug_assert!(!record_ref.get_operation_ref(guard).is_request());
        }
    }
}
