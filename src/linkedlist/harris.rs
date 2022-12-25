use std::{mem::MaybeUninit, ptr, sync::atomic::Ordering};

use crossbeam_epoch::{pin, Atomic, Guard, Owned, Shared};

use crate::map::ConcurrentMap;

struct Node<K, V> {
    key: MaybeUninit<K>,
    value: MaybeUninit<V>,
    next: Atomic<Node<K, V>>,
}

impl<K, V> Node<K, V> {
    fn uninit() -> Self {
        Self {
            key: MaybeUninit::uninit(),
            value: MaybeUninit::uninit(),
            next: Atomic::null(),
        }
    }

    fn new(key: K, value: V) -> Self {
        Self {
            key: MaybeUninit::new(key),
            value: MaybeUninit::new(value),
            next: Atomic::null(),
        }
    }
}

struct HarrisList<K, V> {
    head: Atomic<Node<K, V>>,
}

impl<K, V> HarrisList<K, V>
where
    K: Eq + PartialOrd,
{
    /// search left.key <= key < right.key and return (left ptr, right)
    fn search<'g>(
        &'g self,
        key: &K,
        guard: &'g Guard,
    ) -> (&'g Atomic<Node<K, V>>, Shared<Node<K, V>>) {
        unsafe {
            let mut left = &self.head;

            loop {
                let left_ref = left.load(Ordering::Acquire, guard).deref();
                let right_ref = left_ref.next.load(Ordering::Acquire, guard);

                if left_ref.key.assume_init_ref() <= key {
                    if right_ref.is_null() || (key < right_ref.deref().key.assume_init_ref()) {
                        return (left, right_ref);
                    }
                }

                left = &left_ref.next;
            }
        }
    }
}

impl<K, V> ConcurrentMap<K, V> for HarrisList<K, V>
where
    K: Eq + PartialOrd + Clone,
{
    fn new() -> Self {
        Self {
            head: Atomic::new(Node::uninit()), // dummy node
        }
    }

    fn insert(&self, key: &K, value: V) -> Result<(), V> {
        let guard = pin();

        let mut node = Owned::new(Node::new(key.clone(), value));

        unsafe {
            loop {
                let (left, right) = self.search(key, &guard);

                let left_ref = left.load(Ordering::Relaxed, &guard);

                if left_ref.deref().key.assume_init_ref() == key {
                    return Err(ptr::read(&node.value).assume_init());
                }

                node.next.store(right, Ordering::Relaxed);

                match left.compare_exchange(
                    left_ref,
                    node,
                    Ordering::Release,
                    Ordering::Relaxed,
                    &guard,
                ) {
                    Ok(_) => return Ok(()),
                    Err(e) => node = e.new,
                }
            }
        }
    }

    fn lookup<F, R>(&self, key: &K, f: F) -> R
    where
        F: FnOnce(Option<&V>) -> R,
    {
        let guard = pin();

        unsafe {
            let (left, _) = self.search(key, &guard);
            let left_ref = left.load(Ordering::Relaxed, &guard).deref();

            let value = if key == left_ref.key.assume_init_ref() {
                Some(left_ref.value.assume_init_ref())
            } else {
                None
            };

            f(value)
        }
    }

    fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        let guard = pin();

        unsafe {
            let (left, _) = self.search(key, &guard);
            let left_ref = left.load(Ordering::Relaxed, &guard).deref();

            if key == left_ref.key.assume_init_ref() {
                Some(left_ref.value.assume_init_ref().clone())
            } else {
                None
            }
        }
    }

    fn remove(&self, key: &K) -> Result<V, ()> {
        let guard = pin();

        unsafe {
            loop {
                let (left, right) = self.search(key, &guard);

                let left_ref = left.load(Ordering::Relaxed, &guard);

                if left_ref.deref().key.assume_init_ref() != key {
                    return Err(());
                }

                if left
                    .compare_exchange(
                        left_ref,
                        right,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                        &guard,
                    )
                    .is_ok()
                {
                    let value = ptr::read(&left_ref.deref().value).assume_init();
                    guard.defer_destroy(left_ref);

                    return Ok(value);
                }
            }
        }
    }
}
