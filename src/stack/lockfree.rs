use std::{mem::ManuallyDrop, ptr, sync::atomic::Ordering, thread, time::Duration};

use crossbeam_epoch::{pin, Atomic, Guard, Owned, Shared};
use crossbeam_utils::Backoff;
use rand::{thread_rng, Rng};

use super::ConcurrentStack;

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
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Relaxed, &pin()).is_null()
    }

    pub fn top(&self) -> Option<V>
    where
        V: Clone,
    {
        if let Some(node) = unsafe { self.head.load(Ordering::Acquire, &pin()).as_ref() } {
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

    fn push(&self, value: V) {
        let guard = pin();

        let mut node = Owned::new(Node::new(value));
        let backoff = Backoff::new();

        while let Err(e) = self.try_push(node, &guard) {
            node = e;
            backoff.spin();
        }
    }

    fn pop(&self) -> Option<V> {
        let guard = pin();

        let backoff = Backoff::new();

        loop {
            if let Ok(value) = self.try_pop(&guard) {
                return value;
            }

            backoff.spin();
        }
    }
}

const ELIM_SIZE: usize = 4;
const ELIM_DELAY: Duration = Duration::from_millis(1);

/// Elimination-Backoff Stack
///
/// the tag of slot
/// 0: empty slot
/// 1: push slot
/// 2: pop slot
/// 3: paired slot
pub struct EBStack<V> {
    stack: TreiberStack<V>,
    slots: [Atomic<Node<V>>; ELIM_SIZE],
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
            Err(node) => node.into_shared(guard),
        };

        let slot = unsafe { self.slots.get_unchecked(rand_idx()) };
        let s = slot.load(Ordering::Relaxed, guard);
        let tag = s.tag();

        let result = match tag {
            0 => slot.compare_exchange(
                s,
                node.with_tag(1),
                Ordering::Release,
                Ordering::Relaxed,
                guard,
            ),
            2 => slot.compare_exchange(
                s,
                node.with_tag(3),
                Ordering::Release,
                Ordering::Relaxed,
                guard,
            ),
            _ => return unsafe { Err(node.into_owned()) },
        };

        if let Err(e) = result {
            return unsafe { Err(e.new.into_owned()) };
        }

        thread::sleep(ELIM_DELAY);

        let s = slot.load(Ordering::Relaxed, guard);

        if tag == 0 && s.tag() == 1 {
            return match slot.compare_exchange(
                node.with_tag(1),
                Shared::null(),
                Ordering::Relaxed,
                Ordering::Relaxed,
                guard,
            ) {
                Ok(_) => unsafe { Err(s.into_owned()) },
                Err(_) => Ok(()),
            };
        }

        Ok(())
    }

    fn try_pop(&self, guard: &Guard) -> Result<Option<V>, ()> {
        if let Ok(value) = self.stack.try_pop(guard) {
            return Ok(value);
        }

        let slot = unsafe { self.slots.get_unchecked(rand_idx()) };
        let s = slot.load(Ordering::Relaxed, guard);

        let result = match s.tag() {
            0 => slot.compare_exchange(
                s,
                s.with_tag(2),
                Ordering::Relaxed,
                Ordering::Relaxed,
                guard,
            ),
            1 => slot.compare_exchange(
                s,
                s.with_tag(3),
                Ordering::Relaxed,
                Ordering::Relaxed,
                guard,
            ),
            _ => return Err(()),
        };

        if result.is_err() {
            return Err(());
        }

        thread::sleep(ELIM_DELAY);

        let s = slot.load(Ordering::Acquire, guard);

        if s.tag() == 3 {
            slot.store(Shared::null(), Ordering::Relaxed);
            let node = unsafe { s.into_owned() };
            let value = ManuallyDrop::into_inner(node.into_box().value);
            Ok(Some(value))
        } else {
            slot.store(Shared::null(), Ordering::Relaxed);
            Err(())
        }
    }
}

impl<V> ConcurrentStack<V> for EBStack<V> {
    fn new() -> Self {
        Self {
            stack: TreiberStack::new(),
            slots: Default::default(),
        }
    }

    fn push(&self, value: V) {
        let guard = pin();

        let mut node = Owned::new(Node::new(value));

        while let Err(e) = self.try_push(node, &guard) {
            node = e;
        }
    }

    fn pop(&self) -> Option<V> {
        let guard = pin();

        loop {
            if let Ok(value) = self.try_pop(&guard) {
                return value;
            }
        }
    }
}
