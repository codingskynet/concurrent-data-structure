use std::{mem::ManuallyDrop, ptr, sync::atomic::Ordering};

use crossbeam_epoch::{Atomic, Guard, Owned};
use crossbeam_utils::Backoff;

pub struct TreiberStack<V> {
    head: Atomic<Node<V>>,
}

struct Node<V> {
    value: ManuallyDrop<V>,
    next: Atomic<Node<V>>,
}

impl<V> Node<V> {
    fn new(value: V) -> Self {
        Self {
            value: ManuallyDrop::new(value),
            next: Atomic::null(),
        }
    }
}

impl<V> TreiberStack<V> {
    pub fn new() -> Self {
        Self {
            head: Atomic::null(),
        }
    }

    pub fn is_empty(&self, guard: &Guard) -> bool {
        self.head.load(Ordering::Relaxed, guard).is_null()
    }

    pub fn top(&self, guard: &Guard) -> Option<V>
    where
        V: Clone,
    {
        if let Some(node) = unsafe { self.head.load(Ordering::Acquire, guard).as_ref() } {
            Some(ManuallyDrop::into_inner(node.value.clone()))
        } else {
            None
        }
    }

    pub fn push(&self, value: V, guard: &Guard) {
        let mut node = Owned::new(Node::new(value));
        let backoff = Backoff::new();

        loop {
            let head = self.head.load(Ordering::Relaxed, guard);
            node.next.store(head, Ordering::Relaxed);

            match self.head.compare_exchange(
                head,
                node,
                Ordering::Release,
                Ordering::Relaxed,
                guard,
            ) {
                Ok(_) => break,
                Err(e) => node = e.new,
            }

            backoff.spin();
        }
    }

    pub fn pop(&self, guard: &Guard) -> Option<V> {
        let backoff = Backoff::new();

        loop {
            let head = self.head.load(Ordering::Acquire, guard);

            if let Some(h) = unsafe { head.as_ref() } {
                let next = h.next.load(Ordering::Relaxed, guard);

                if self
                    .head
                    .compare_exchange(head, next, Ordering::Relaxed, Ordering::Relaxed, guard)
                    .is_ok()
                {
                    unsafe { guard.defer_destroy(head) };
                    return unsafe { Some(ManuallyDrop::into_inner(ptr::read(&(*h).value))) };
                }

                backoff.spin();
            } else {
                return None;
            }
        }
    }
}

// pub struct EBStack {}
