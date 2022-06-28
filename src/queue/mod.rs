mod lockfree;
mod mutex;
mod spinlock;

pub use lockfree::MSQueue;
pub use mutex::MutexQueue;
pub use mutex::TwoMutexQueue;
pub use spinlock::SpinLockQueue;
pub use spinlock::TwoSpinLockQueue;

use std::{mem, mem::MaybeUninit, ptr::NonNull};

pub trait ConcurrentQueue<V> {
    fn new() -> Self;
    fn push(&self, value: V);
    // non-blocking pop that can return `None` when the queue is observed as Empty.
    fn try_pop(&self) -> Option<V>;
    // blocking pop that can wait for returing value.
    fn pop(&self) -> V;
}

// simple sequential queue
pub struct Queue<V> {
    head: NonNull<Node<V>>,
    tail: NonNull<Node<V>>,
}

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

impl<V> Queue<V> {
    pub fn new() -> Queue<V> {
        let dummy = Node::new_non_null(MaybeUninit::uninit());

        Self {
            head: dummy,
            tail: dummy,
        }
    }

    pub fn is_empty(&self) -> bool {
        unsafe { self.head.as_ref().next.is_none() }
    }

    pub fn top(&self) -> Option<&V> {
        unsafe {
            match self.head.as_ref().next.as_ref() {
                Some(node) => Some(node.as_ref().value.assume_init_ref()),
                None => None,
            }
        }
    }

    pub fn push(&mut self, value: V) {
        let node = Node::new_non_null(MaybeUninit::new(value));

        let tail = unsafe { self.tail.as_mut() };

        tail.next = Some(node);
        self.tail = node;
    }

    pub fn pop(&mut self) -> Option<V> {
        unsafe {
            let head = self.head.as_mut();

            if let Some(mut next) = head.next {
                let value = mem::replace(&mut next.as_mut().value, MaybeUninit::uninit());
                self.head = next;

                Some(value.assume_init())
            } else {
                None
            }
        }
    }
}

impl<V> Drop for Queue<V> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}
