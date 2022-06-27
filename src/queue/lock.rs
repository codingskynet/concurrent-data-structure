use std::{
    mem::{self, MaybeUninit},
    ops::DerefMut,
    ptr::NonNull,
};

use crossbeam_utils::Backoff;

use super::ConcurrentQueue;

use crate::lock::spinlock::SpinLock;

pub struct TwoSpinLockQueue<V> {
    head: SpinLock<NonNull<Node<V>>>,
    tail: SpinLock<NonNull<Node<V>>>,
}

unsafe impl<T: Send> Send for TwoSpinLockQueue<T> {}
unsafe impl<T: Send> Sync for TwoSpinLockQueue<T> {}

struct Node<V> {
    value: MaybeUninit<V>,
    next: Option<NonNull<Node<V>>>,
}

impl<V> Node<V> {
    fn new(value: MaybeUninit<V>) -> Self {
        Self { value, next: None }
    }

    fn new_non_null(value: MaybeUninit<V>) -> NonNull<Self> {
        let node = Box::new(Self::new(value));
        NonNull::new(Box::leak(node)).unwrap()
    }
}

impl<V> ConcurrentQueue<V> for TwoSpinLockQueue<V> {
    fn new() -> Self {
        let dummy = Node::new_non_null(MaybeUninit::uninit());

        Self {
            head: SpinLock::new(dummy),
            tail: SpinLock::new(dummy),
        }
    }

    fn push(&self, value: V) {
        let node = Node::new_non_null(MaybeUninit::new(value));

        let mut lock_guard = self.tail.lock();

        unsafe {
            lock_guard.as_mut().next = Some(node);
            *lock_guard.deref_mut() = node;
        }
    }

    fn try_pop(&self) -> Option<V> {
        unsafe {
            let mut lock_guard = self.head.lock();

            let head_ref = lock_guard.as_mut();

            if let Some(mut next) = head_ref.next {
                let value = mem::replace(&mut next.as_mut().value, MaybeUninit::uninit());
                *lock_guard.deref_mut() = next;

                Some(value.assume_init())
            } else {
                None
            }
        }
    }

    fn pop(&self) -> V {
        let backoff = Backoff::new();

        loop {
            match self.try_pop() {
                Some(value) => return value,
                None => {}
            }

            backoff.spin();
        }
    }
}

impl<V> Drop for TwoSpinLockQueue<V> {
    fn drop(&mut self) {
        while let Some(_) = self.try_pop() {}
    }
}
