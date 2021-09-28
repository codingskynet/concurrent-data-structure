use std::{mem::ManuallyDrop, ptr, sync::atomic::Ordering, time::Duration};

use crossbeam_epoch::{Atomic, Guard, Owned};
use crossbeam_utils::Backoff;
use rand::{Rng, thread_rng};

pub trait ConcurrentStack<V> {
    fn new() -> Self;
    fn push(&self, value: V, guard: &Guard);
    fn pop(&self, guard: &Guard) -> Option<V>;
}

pub struct TreiberStack<V> {
    head: Atomic<Node<V>>,
}

impl<V> Default for TreiberStack<V> {
    fn default() -> Self {
        Self::new()
    }
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

    fn try_push(&self, node: Owned<Node<V>>, guard: &Guard) -> Result<(), Owned<Node<V>>> {
        let head = self.head.load(Ordering::Relaxed, guard);
        node.next.store(head, Ordering::Relaxed);

        match self
            .head
            .compare_exchange(head, node, Ordering::Release, Ordering::Relaxed, guard)
        {
            Ok(_) => Ok(()),
            Err(e) => Err(e.new),
        }
    }

    fn try_pop(&self, guard: &Guard) -> Result<Option<V>, ()> {
        let head = self.head.load(Ordering::Acquire, guard);

        if let Some(h) = unsafe { head.as_ref() } {
            let next = h.next.load(Ordering::Relaxed, guard);

            if self
                .head
                .compare_exchange(head, next, Ordering::Relaxed, Ordering::Relaxed, guard)
                .is_ok()
            {
                unsafe { guard.defer_destroy(head) };
                return unsafe { Ok(Some(ManuallyDrop::into_inner(ptr::read(&(*h).value)))) };
            }

            return Err(());
        } else {
            return Ok(None);
        }
    }
}

impl<V> ConcurrentStack<V> for TreiberStack<V> {
    fn new() -> Self {
        Self {
            head: Atomic::null(),
        }
    }

    fn push(&self, value: V, guard: &Guard) {
        let mut node = Owned::new(Node::new(value));
        let backoff = Backoff::new();

        while let Err(e) = self.try_push(node, guard) {
            node = e;
            backoff.spin();
        }
    }

    fn pop(&self, guard: &Guard) -> Option<V> {
        let backoff = Backoff::new();

        loop {
            if let Ok(value) = self.try_pop(guard) {
                return value;
            }

            backoff.spin();
        }
    }
}

const ELIM_SIZE: usize = 10;
const ELIM_DELAY: Duration = Duration::from_millis(10);
pub struct EBStack<V> {
    stack: TreiberStack<V>,
    slots: [Atomic<V>; ELIM_SIZE],
}

#[inline]
fn rand_idx() -> usize {
    thread_rng().gen_range(0..ELIM_SIZE)
}

impl<V> Default for EBStack<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> EBStack<V> {
    fn try_push(&self, node: Owned<Node<V>>, guard: &Guard) -> Result<(), Owned<Node<V>>> {
        let node = match self.stack.try_push(node, guard) {
            Ok(_) => return Ok(()),
            Err(node) => node,
        };

        Err(node)
    }

    fn try_pop(&self, guard: &Guard) -> Result<Option<V>, ()> {
        if let Ok(value) = self.stack.try_pop(guard) {
            return Ok(value);
        }

        Err(())
    }
}

impl<V> ConcurrentStack<V> for EBStack<V> {
    fn new() -> Self {
        Self {
            stack: TreiberStack::new(),
            slots: Default::default(),
        }
    }

    fn push(&self, value: V, guard: &Guard) {
        let mut node = Owned::new(Node::new(value));

        while let Err(e) = self.try_push(node, guard) {
            node = e;
        }
    }

    fn pop(&self, guard: &Guard) -> Option<V> {
        loop {
            if let Ok(value) = self.try_pop(guard) {
                return value;
            }
        }
    }
}
