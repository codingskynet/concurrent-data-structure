use std::{mem::MaybeUninit, ptr::NonNull, sync::atomic::Ordering};

use crossbeam_epoch::{unprotected, Atomic, Owned};
use crossbeam_utils::Backoff;

use super::ConcurrentQueue;

use crate::lock::spinlock::SpinLock;

pub struct TwoSpinLockQueue<V> {
    head: SpinLock<Atomic<Node<V>>>,
    tail: SpinLock<Atomic<Node<V>>>,
}

unsafe impl<T: Send> Send for TwoSpinLockQueue<T> {}
unsafe impl<T: Send> Sync for TwoSpinLockQueue<T> {}

struct Node<V> {
    value: MaybeUninit<V>,
    next: Atomic<Node<V>>,
}

impl<V> Node<V> {
    fn new(value: MaybeUninit<V>) -> Self {
        Self {
            value,
            next: Atomic::null(),
        }
    }

    fn new_non_null(value: MaybeUninit<V>) -> NonNull<Self> {
        let node = Box::new(Node::new(value));
        NonNull::new(Box::leak(node)).unwrap()
    }
}

impl<V> ConcurrentQueue<V> for TwoSpinLockQueue<V> {
    fn new() -> Self {
        unsafe {
            let guard = unprotected();

            let queue = Self {
                head: SpinLock::new(Atomic::null()),
                tail: SpinLock::new(Atomic::null()),
            };

            let dummy = Owned::new(Node::new(MaybeUninit::uninit())).into_shared(guard);

            queue.head.lock().store(dummy, Ordering::Relaxed);
            queue.tail.lock().store(dummy, Ordering::Relaxed);

            queue
        }
    }

    fn push(&self, value: V) {
        let guard = unsafe { unprotected() };

        let node = Owned::new(Node::new(MaybeUninit::new(value))).into_shared(guard);

        let lock_guard = self.tail.lock();

        let tail_ref = unsafe { lock_guard.load(Ordering::Relaxed, guard).deref_mut() };
        tail_ref.next.store(node, Ordering::Relaxed);

        lock_guard.store(node, Ordering::Relaxed);
    }

    fn try_pop(&self) -> Option<V> {
        let guard = unsafe { unprotected() };

        let lock_guard = self.head.lock();

        let head_ref = unsafe { lock_guard.load(Ordering::Relaxed, guard).deref_mut() };
        let mut next = head_ref.next.load(Ordering::Relaxed, guard);

        if next.is_null() {
            return None;
        }

        unsafe {
            let node = head_ref.next.swap(
                next.deref_mut().next.load(Ordering::Relaxed, guard),
                Ordering::Relaxed,
                guard,
            );
            let node = node.into_owned().into_box();

            Some(node.value.assume_init())
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
