use std::{
    mem::{self, MaybeUninit},
    ops::DerefMut,
    ptr::NonNull,
    sync::Arc,
};

use crossbeam_utils::{Backoff, CachePadded};

use super::{ConcurrentQueue, Node, Queue, SequentialQueue};

use crate::lock::spinlock::SpinLock;

pub struct SpinLockQueue<V> {
    queue: Arc<SpinLock<Queue<V>>>,
}

unsafe impl<T: Send> Send for SpinLockQueue<T> {}
unsafe impl<T: Send> Sync for SpinLockQueue<T> {}

impl<V> ConcurrentQueue<V> for SpinLockQueue<V> {
    fn new() -> Self {
        Self {
            queue: Arc::new(SpinLock::new(Queue::new())),
        }
    }

    fn push(&self, value: V) {
        let queue = self.queue.clone();
        let mut lock_guard = queue.lock();
        lock_guard.push(value);
    }

    fn try_pop(&self) -> Option<V> {
        let queue = self.queue.clone();
        let mut lock_guard = queue.lock();
        lock_guard.pop()
    }

    fn pop(&self) -> V {
        let backoff = Backoff::new();

        loop {
            match self.try_pop() {
                Some(value) => return value,
                None => {}
            }

            backoff.snooze();
        }
    }
}

pub struct TwoSpinLockQueue<V> {
    head: CachePadded<SpinLock<NonNull<Node<V>>>>,
    tail: CachePadded<SpinLock<NonNull<Node<V>>>>,
}

unsafe impl<T: Send> Send for TwoSpinLockQueue<T> {}
unsafe impl<T: Send> Sync for TwoSpinLockQueue<T> {}

impl<V> ConcurrentQueue<V> for TwoSpinLockQueue<V> {
    fn new() -> Self {
        let dummy = Node::new_non_null(MaybeUninit::uninit());

        Self {
            head: CachePadded::new(SpinLock::new(dummy)),
            tail: CachePadded::new(SpinLock::new(dummy)),
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

            backoff.snooze();
        }
    }
}

impl<V> Drop for TwoSpinLockQueue<V> {
    fn drop(&mut self) {
        while let Some(_) = self.try_pop() {}

        unsafe {
            drop(Box::from_raw(self.head.lock().as_ptr()));
        }
    }
}
