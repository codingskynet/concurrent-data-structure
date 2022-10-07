mod fclock;
mod lockfree;
mod mutex;
mod spinlock;

pub use fclock::FCQueue;
pub use lockfree::MSQueue;
pub use mutex::MutexQueue;
pub use mutex::TwoMutexQueue;
pub use spinlock::SpinLockQueue;
pub use spinlock::TwoSpinLockQueue;

use std::{fmt::Debug, mem, mem::MaybeUninit, ptr, ptr::NonNull};

pub trait SequentialQueue<V> {
    fn new() -> Self;
    fn push(&mut self, value: V);
    fn pop(&mut self) -> Option<V>;
}

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
}

impl<V> SequentialQueue<V> for Queue<V> {
    fn new() -> Self {
        let dummy = Node::new_non_null(MaybeUninit::uninit());

        Self {
            head: dummy,
            tail: dummy,
        }
    }

    fn push(&mut self, value: V) {
        let node = Node::new_non_null(MaybeUninit::new(value));

        let tail = unsafe { self.tail.as_mut() };

        tail.next = Some(node);
        self.tail = node;
    }

    fn pop(&mut self) -> Option<V> {
        unsafe {
            let head = self.head.as_mut();

            if let Some(mut next) = head.next {
                let value = mem::replace(&mut next.as_mut().value, MaybeUninit::uninit());
                self.head = next;
                drop(Box::from_raw(head));

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

        unsafe {
            drop(Box::from_raw(self.head.as_ptr()));
        }
    }
}

// fat node sequential queue
const FAT_SIZE: u8 = 16;

pub struct FatNodeQueue<V> {
    head: NonNull<FatNode<V>>,
    tail: NonNull<FatNode<V>>,
}

struct FatNode<V> {
    head: u8,
    tail: u8,
    values: [V; FAT_SIZE as usize], // very unsafe since init_array is not stable...
    next: Option<NonNull<FatNode<V>>>,
}

impl<V: Debug> Debug for FatNode<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FatNode")
            .field(
                "values",
                &self.values.get((self.head as usize)..(self.tail as usize)),
            )
            .field("head", &self.head)
            .field("tail", &self.tail)
            .field("next", &self.next.map(|next| unsafe { next.as_ref() }))
            .finish()
    }
}

impl<V> Drop for FatNode<V> {
    fn drop(&mut self) {
        for i in self.head..self.tail {
            unsafe { drop(ptr::read(self.values.get_unchecked(i as usize))) };
        }

        mem::forget(self);
    }
}

impl<V> FatNode<V> {
    #[allow(deprecated, invalid_value)]
    fn new() -> Self {
        Self {
            head: 0,
            tail: 0,
            values: unsafe { mem::uninitialized() },
            next: None,
        }
    }
}

impl<V: Debug> Debug for FatNodeQueue<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            f.debug_struct("FatNodeQueue")
                .field("inner", &self.head.as_ref())
                .finish()
        }
    }
}

impl<V> FatNodeQueue<V> {
    pub fn is_empty(&self) -> bool {
        unsafe {
            if let Some(next) = self.head.as_ref().next {
                let next_ref = next.as_ref();

                if next_ref.head != next_ref.tail {
                    return false;
                }
            }
        }

        true
    }

    pub fn top(&self) -> Option<&V> {
        unsafe {
            match self.head.as_ref().next.as_ref() {
                Some(node) => {
                    let node_ref = node.as_ref();

                    Some(&node_ref.values.get_unchecked(node_ref.head as usize))
                }
                None => None,
            }
        }
    }
}

impl<V> SequentialQueue<V> for FatNodeQueue<V> {
    fn new() -> Self {
        let dummy = unsafe { NonNull::new_unchecked(Box::leak(Box::new(FatNode::new()))) };

        Self {
            head: dummy,
            tail: dummy,
        }
    }

    fn push(&mut self, value: V) {
        unsafe {
            let tail = self.tail.as_mut();

            if self.head != self.tail && tail.tail < FAT_SIZE {
                *tail.values.get_unchecked_mut(tail.tail as usize) = value;
                tail.tail += 1;
                return;
            }

            let mut node = FatNode::new();
            node.values[0] = value;
            node.tail = 1;

            let node = NonNull::new_unchecked(Box::leak(Box::new(node)));
            tail.next = Some(node);
            self.tail = node;
        }
    }

    fn pop(&mut self) -> Option<V> {
        unsafe {
            let head = self.head.as_mut();

            if let Some(mut next) = head.next {
                let next_ref = next.as_mut();

                if next_ref.head == next_ref.tail {
                    return None;
                }

                let value = ptr::read(next_ref.values.get_unchecked(next_ref.head as usize));
                next_ref.head += 1;

                if next_ref.head == FAT_SIZE {
                    self.head = next;
                    drop(Box::from(head));
                }

                Some(value)
            } else {
                None
            }
        }
    }
}

impl<V> Drop for FatNodeQueue<V> {
    fn drop(&mut self) {
        while self.pop().is_some() {}

        unsafe {
            drop(Box::from_raw(self.head.as_ptr()));
        }
    }
}
