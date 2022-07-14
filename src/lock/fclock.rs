/*
 * This code is refered to https://github.com/khizmax/libcds/blob/master/cds/algo/flat_combining/kernel.h
 */

use std::{
    cell::SyncUnsafeCell,
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering},
};

use crossbeam_epoch::{pin, Atomic, Guard, Owned, Shared};
use crossbeam_utils::Backoff;
use thread_local::ThreadLocal;

use super::spinlock::RawSpinLock;

pub trait FlatCombining<T> {
    fn apply(&mut self, operation: &mut T);
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
            .store(Owned::new(operation), Ordering::Relaxed);
    }

    pub fn release(&self) {
        self.state
            .store(Owned::new(State::Inactive), Ordering::Release);
    }

    #[inline]
    pub fn get_state<'a>(&self, guard: &'a Guard) -> &'a State {
        unsafe { self.state.load(Ordering::Acquire, guard).deref() }
    }

    pub fn get_operation(&self, guard: &Guard) -> T {
        unsafe {
            *self
                .operation
                .load(Ordering::Acquire, guard)
                .into_owned()
                .into_box()
        }
    }
}

pub struct FCLock<T: Send + Sync> {
    publications: Atomic<Record<T>>,
    lock: RawSpinLock,
    target: SyncUnsafeCell<Box<dyn FlatCombining<T>>>,
    thread_local: ThreadLocal<Atomic<Record<T>>>,
}

impl<T: Send + Sync + Debug> FCLock<T> {
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

            let parent = self.publications.load(Ordering::Acquire, guard);
            let mut node = parent.deref().next.load(Ordering::Acquire, guard);

            // println!("request");
            // self.print_publications(guard);

            while !node.is_null() {
                let node_ref = node.deref();

                match node_ref.get_state(guard) {
                    State::Active => {
                        let operation = node_ref
                            .operation
                            .load(Ordering::Acquire, guard)
                            .deref_mut();

                        target.apply(operation);

                        node_ref
                            .state
                            .store(Owned::new(State::Inactive), Ordering::Release);
                    }
                    State::Inactive => {
                        node_ref.age.fetch_add(1, Ordering::Relaxed);
                    }
                    State::Removed => unreachable!(),
                }

                node = node_ref.next.load(Ordering::Acquire, guard);
            }

            // println!("finish");
            // self.print_publications(guard);
        }
    }

    pub fn new(target: Box<dyn FlatCombining<T>>) -> Self {
        let dummy = Record {
            operation: Atomic::null(),
            state: Atomic::null(),
            age: AtomicUsize::new(99991),
            next: Atomic::null(),
        };

        Self {
            publications: Atomic::new(dummy),
            lock: RawSpinLock::new(),
            target: SyncUnsafeCell::new(target),
            thread_local: ThreadLocal::new(),
        }
    }

    pub fn acquire_record<'a>(&self, guard: &'a Guard) -> Shared<'a, Record<T>> {
        let node = self.thread_local.get_or(|| Atomic::null());

        if node.load(Ordering::Relaxed, guard).is_null() {
            let new = Owned::new(Record {
                operation: Atomic::null(),
                state: Atomic::new(State::Removed),
                age: AtomicUsize::new(0),
                next: Atomic::null(),
            });

            node.store(new, Ordering::Relaxed);
        }

        node.load(Ordering::Relaxed, guard)
    }

    pub fn push_record(&self, record: Shared<Record<T>>, guard: &Guard) {
        let backoff = Backoff::new();

        let mut node = record;
        let node_ref = unsafe { node.deref() };

        let node_state = unsafe { node_ref.state.load(Ordering::Relaxed, guard).deref() };

        debug_assert_ne!(*node_state, State::Active);

        node_ref.age.store(0, Ordering::Relaxed);

        if *node_state == State::Inactive {
            // already pushed on publications
            node_ref
                .state
                .store(Owned::new(State::Active), Ordering::Release);
            return;
        }

        debug_assert_eq!(*node_state, State::Removed);

        node_ref
            .state
            .store(Owned::new(State::Active), Ordering::Release);

        loop {
            let dummy = unsafe { self.publications.load(Ordering::Acquire, guard).deref() };
            let head = dummy.next.load(Ordering::Relaxed, guard);

            unsafe { node.deref_mut().next.store(head, Ordering::Relaxed) };

            match dummy.next.compare_exchange(
                head,
                node,
                Ordering::Release,
                Ordering::Relaxed,
                guard,
            ) {
                Ok(_) => break,
                Err(err) => node = err.new,
            }

            backoff.spin();
        }
    }

    pub fn release_local_record(&self, guard: &Guard) {
        unsafe {
            if let Some(node) = self.thread_local.get() {
                let node = node.load(Ordering::Relaxed, guard);

                if !node.is_null() {
                    guard.defer_destroy(node);
                }
            }
        }
    }

    pub fn try_combine(&self, record: Shared<Record<T>>, guard: &Guard) {
        unsafe {
            let record_ref = record.deref();

            if self.lock.try_lock().is_ok() {
                // now the thread is combiner
                // println!("I'm combiner! {:?}", unsafe { record.deref() });
                // self.print_publications(guard);

                if *record_ref.get_state(guard) == State::Inactive {
                    // already finished
                    self.lock.unlock();
                    return;
                }

                self.combine(guard);
                debug_assert_eq!(*record_ref.get_state(guard), State::Inactive);

                self.lock.unlock();
            } else {
                // wait and the thread may be combiner if its operation is not finished and it gets lock
                let backoff = Backoff::new();

                while *record_ref.get_state(guard) != State::Inactive {
                    backoff.snooze();

                    if self.lock.try_lock().is_ok() {
                        // Another combiner is finished. So, it can receive response

                        if *record_ref.get_state(guard) != State::Inactive {
                            // println!("waiting, and I'm combiner! {:?}", record.deref());
                            // self.print_publications(guard);

                            // It does not receive response. So, the thread becomes combiner
                            self.combine(guard);
                        }

                        debug_assert_eq!(*record_ref.get_state(guard), State::Inactive);

                        self.lock.unlock();
                        break;
                    }
                }
            }
        }
    }
}
