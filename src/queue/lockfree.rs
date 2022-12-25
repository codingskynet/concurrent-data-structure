/*
 Refer to
 https://github.com/kaist-cp/cs431/blob/main/lockfree/src/queue.rs and
 https://www.cs.rochester.edu/~scott/papers/1996_PODC_queues.pdf
*/

use std::{mem::MaybeUninit, ptr, sync::atomic::Ordering};

use crossbeam_epoch::{pin, unprotected, Atomic, Owned, Shared};
use crossbeam_utils::{Backoff, CachePadded};

use super::ConcurrentQueue;

pub struct MSQueue<V> {
    head: CachePadded<Atomic<Node<V>>>,
    tail: CachePadded<Atomic<Node<V>>>,
}

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
}

impl<V> ConcurrentQueue<V> for MSQueue<V> {
    fn new() -> Self {
        let queue = Self {
            head: CachePadded::new(Atomic::null()),
            tail: CachePadded::new(Atomic::null()),
        };

        // store dummy node into both head and tail
        unsafe {
            let dummy =
                Owned::new(Node::new(MaybeUninit::<V>::uninit())).into_shared(unprotected());

            queue.head.store(dummy, Ordering::Relaxed);
            queue.tail.store(dummy, Ordering::Relaxed);
        }

        queue
    }

    fn push(&self, value: V) {
        let guard = pin();

        let node = Owned::new(Node::new(MaybeUninit::new(value))).into_shared(&guard);

        loop {
            let tail = self.tail.load(Ordering::Acquire, &guard);
            let tail_ref = unsafe { tail.deref() };
            let tail_next = tail_ref.next.load(Ordering::Acquire, &guard);

            if tail_next.is_null() {
                // If null, The tail pointer is real tail at that time. Try CAS
                if tail_ref
                    .next
                    .compare_exchange(
                        Shared::null(),
                        node,
                        Ordering::Release,
                        Ordering::Relaxed,
                        &guard,
                    )
                    .is_ok()
                {
                    // just try move tail pointer to next(node)
                    let _ = self.tail.compare_exchange(
                        tail,
                        node,
                        Ordering::Release,
                        Ordering::Relaxed,
                        &guard,
                    );
                    break;
                }
            } else {
                // The tail pointer is not real tail. Move to next and try again.
                let _ = self.tail.compare_exchange(
                    tail,
                    tail_next,
                    Ordering::Release,
                    Ordering::Relaxed,
                    &guard,
                );
            }
        }
    }

    fn try_pop(&self) -> Option<V> {
        let guard = pin();

        loop {
            let head = self.head.load(Ordering::Acquire, &guard); // the dummy node
            let head_next = unsafe { head.deref().next.load(Ordering::Acquire, &guard) }; // the real head

            let tail = self.tail.load(Ordering::Relaxed, &guard);

            if head_next.is_null() {
                // if the head's next pointer is null, the queue is observed as empty.
                return None;
            }

            if head == tail {
                // the head's next pointer is not null, but head == tail means that the tail pointer is STALE!
                // So, set the tail pointer to head's next.
                let _ = self.tail.compare_exchange(
                    tail,
                    head_next,
                    Ordering::Release,
                    Ordering::Relaxed,
                    &guard,
                );
            }

            // the queue may not be empty. Try CAS for moving head to next
            if self
                .head
                .compare_exchange(
                    head,
                    head_next,
                    Ordering::Release,
                    Ordering::Relaxed,
                    &guard,
                )
                .is_ok()
            {
                // free head and get head_next's value
                unsafe {
                    guard.defer_destroy(head);
                    return Some(ptr::read(&head_next.deref().value).assume_init());
                }
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

impl<V> Drop for MSQueue<V> {
    fn drop(&mut self) {
        unsafe {
            let guard = unprotected();

            while self.try_pop().is_some() {}

            let dummy = self.head.load(Ordering::Relaxed, guard);
            drop(dummy.into_owned());
        }
    }
}
