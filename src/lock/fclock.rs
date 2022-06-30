/*
 * This code is refered to https://github.com/khizmax/libcds/blob/master/cds/algo/flat_combining/kernel.h
 */

use std::{cell::SyncUnsafeCell, mem::MaybeUninit, ptr::NonNull, sync::atomic::Ordering};

use arr_macro::arr;
use crossbeam_epoch::{Atomic, Guard, Owned};
use crossbeam_utils::Backoff;

use crate::util::get_thread_id;

use super::spinlock::RawSpinLock;

pub trait FlatCombining<T> {
    fn apply(&mut self, operation: &mut T);
}

#[derive(Clone, PartialEq)]
pub enum State {
    Inactive,
    Request,
    Response,
}

pub struct Record<T> {
    operation: Atomic<MaybeUninit<T>>, // this is not atomicT, but the atomic pointer of T. TODO: optimize using AtomicU64
    state: Atomic<State>,
}

impl<T> Default for Record<T> {
    fn default() -> Self {
        Self {
            operation: Atomic::new(MaybeUninit::uninit()),
            state: Atomic::new(State::Inactive),
        }
    }
}

impl<T> Record<T> {
    pub fn set(&self, operation: T, state: State, guard: &Guard) {
        self.operation.store(
            Owned::new(MaybeUninit::new(operation)).into_shared(guard),
            Ordering::Relaxed,
        );

        self.state
            .store(Owned::new(state).into_shared(guard), Ordering::Relaxed);
    }

    pub fn get_state(&self, guard: &Guard) -> State {
        unsafe { self.state.load(Ordering::Relaxed, guard).deref().clone() }
    }

    pub fn get_operation(&self, guard: &Guard) -> T {
        unsafe {
            self.operation
                .swap(
                    Owned::new(MaybeUninit::uninit()).into_shared(guard),
                    Ordering::Relaxed,
                    guard,
                )
                .into_owned()
                .assume_init_read()
        }
    }
}

const MAX_THREAD_NUM: usize = 14;

pub struct FCLock<T> {
    publications: SyncUnsafeCell<[Record<T>; MAX_THREAD_NUM + 1]>,
    lock: RawSpinLock,
    target: SyncUnsafeCell<Box<dyn FlatCombining<T>>>,
}

impl<T> FCLock<T> {
    fn get_publications(&self) -> &mut [Record<T>; MAX_THREAD_NUM + 1] {
        unsafe { &mut *self.publications.get() }
    }

    fn get_record(&self, id: usize) -> &Record<T> {
        unsafe { self.get_publications().get_unchecked(id) }
    }

    fn get_record_mut(&self, id: usize) -> &mut Record<T> {
        unsafe { self.get_publications().get_unchecked_mut(id) }
    }

    fn combine(&self, guard: &Guard) {
        unsafe {
            let target = &mut *self.target.get();

            for record in self.get_publications().iter_mut() {
                match record.state.load(Ordering::Acquire, guard).deref() {
                    State::Request => {
                        let operation = record
                            .operation
                            .load(Ordering::Relaxed, guard)
                            .deref_mut()
                            .assume_init_mut();

                        target.apply(operation);

                        record.state.store(
                            Owned::new(State::Response).into_shared(guard),
                            Ordering::Release,
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn new(target: Box<dyn FlatCombining<T>>) -> Self {
        Self {
            publications: SyncUnsafeCell::new(arr![Record::default(); 15]), // TODO: how to improve?
            lock: RawSpinLock::new(),
            target: SyncUnsafeCell::new(target),
        }
    }

    pub fn acquire_record(&self) -> NonNull<Record<T>> {
        let id = get_thread_id() as usize;

        if id >= MAX_THREAD_NUM {
            panic!(
                "the thread num for using FCLock is limited to {}",
                MAX_THREAD_NUM
            );
        }

        unsafe { NonNull::new_unchecked(self.get_record_mut(id)) }
    }

    pub fn try_combine(&self, guard: &Guard) {
        let id = get_thread_id() as usize;

        if self.lock.try_lock().is_ok() {
            // now the thread is combiner
            self.combine(guard);

            self.lock.unlock();
        } else {
            // wait and the thread may be combiner if its operation is not finished and it gets lock
            let backoff = Backoff::new();

            unsafe {
                while *self
                    .get_record(id)
                    .state
                    .load(Ordering::Acquire, guard)
                    .deref()
                    != State::Response
                {
                    backoff.snooze();

                    if self.lock.try_lock().is_ok() {
                        // Another combiner is finished. So, it can receive response

                        if *self
                            .get_record(id)
                            .state
                            .load(Ordering::Acquire, guard)
                            .deref()
                            != State::Response
                        {
                            // It does not receive response. So, the thread becomes combiner
                            self.combine(guard);
                        }

                        self.lock.unlock();
                        break;
                    }
                }
            }
        }
    }
}
