use std::{mem::MaybeUninit, ptr::NonNull};

use arr_macro::arr;

use crate::util::get_thread_id;

use super::spinlock::RawSpinLock;

pub trait FlatCombining {
    fn apply<T>(&self, request: T);
}

enum State {
    Inactive,
    Active,
    Empty,
}

pub struct Record<T> {
    request: MaybeUninit<T>,
    state: State,
}

impl<T> Default for Record<T> {
    fn default() -> Self {
        Self {
            request: MaybeUninit::uninit(),
            state: State::Empty,
        }
    }
}

const MAX_THREAD_NUM: usize = 128;

pub struct FCLock<T> {
    publications: [Record<T>; MAX_THREAD_NUM + 1],
    lock: RawSpinLock,
}

impl<T> FCLock<T> {
    pub fn new() -> Self {
        Self {
            publications: arr![Record::default(); 129], // TODO: how to improve?
            lock: RawSpinLock::new(),
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

        unsafe { NonNull::new_unchecked(self.publications.get_unchecked(id) as *const _ as *mut _) }
    }

    pub fn combine(&self, target: impl FlatCombining) {}
}
