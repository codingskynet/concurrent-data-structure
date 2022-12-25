use std::{
    mem::{self, MaybeUninit},
    ops::DerefMut,
    ptr::NonNull,
    sync::Mutex,
};

use crossbeam_utils::{Backoff, CachePadded};

use super::{ConcurrentQueue, Node, Queue, SequentialQueue};

pub struct MutexQueue<V> {
    queue: Mutex<Queue<V>>,
}

unsafe impl<T: Send> Send for MutexQueue<T> {}
unsafe impl<T: Send> Sync for MutexQueue<T> {}

impl<V> ConcurrentQueue<V> for MutexQueue<V> {
    fn new() -> Self {
        Self {
            queue: Mutex::new(Queue::new()),
        }
    }

    fn push(&self, value: V) {
        self.queue.lock().unwrap().push(value);
    }

    fn try_pop(&self) -> Option<V> {
        self.queue.lock().unwrap().pop()
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

pub struct TwoMutexQueue<V> {
    head: CachePadded<Mutex<NonNull<Node<V>>>>,
    tail: CachePadded<Mutex<NonNull<Node<V>>>>,
}

unsafe impl<T: Send> Send for TwoMutexQueue<T> {}
unsafe impl<T: Send> Sync for TwoMutexQueue<T> {}

impl<V> ConcurrentQueue<V> for TwoMutexQueue<V> {
    fn new() -> Self {
        let dummy = Node::new_non_null(MaybeUninit::uninit());

        Self {
            head: CachePadded::new(Mutex::new(dummy)),
            tail: CachePadded::new(Mutex::new(dummy)),
        }
    }

    fn push(&self, value: V) {
        let node = Node::new_non_null(MaybeUninit::new(value));

        let mut lock_guard = self.tail.lock().unwrap();

        unsafe {
            lock_guard.as_mut().next = Some(node);
            *lock_guard.deref_mut() = node;
        }
    }

    fn try_pop(&self) -> Option<V> {
        unsafe {
            let mut lock_guard = self.head.lock().unwrap();

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

impl<V> Drop for TwoMutexQueue<V> {
    fn drop(&mut self) {
        while let Some(_) = self.try_pop() {}

        unsafe {
            drop(Box::from_raw(self.head.lock().unwrap().as_ptr()));
        }
    }
}
