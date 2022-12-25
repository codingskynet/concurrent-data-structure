mod harris;

use std::{fmt::Debug, mem};

use crate::map::SequentialMap;

struct Node<K, V> {
    key: K,
    value: V,
    next: Option<Box<Node<K, V>>>,
}

impl<K: Debug, V: Debug> Debug for Node<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("key", &self.key)
            .field("value", &self.value)
            .field("next", &self.next)
            .finish()
    }
}

impl<K: Default, V: Default> Default for Node<K, V> {
    fn default() -> Self {
        Self::new(K::default(), V::default())
    }
}

impl<K, V> Node<K, V> {
    fn new(key: K, value: V) -> Node<K, V> {
        Node {
            key,
            value,
            next: None,
        }
    }
}

// simple sequential sorted linked list
pub struct SortedList<K, V> {
    head: Node<K, V>, // dummy node with key = Default, but the key is not considered on algorithm
}

impl<K: Debug, V: Debug> Debug for SortedList<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SortedList")
            .field("head", &self.head)
            .finish()
    }
}

impl<K, V> SortedList<K, V> {
    pub fn keys(&self) -> Vec<&K> {
        let mut result = Vec::new();

        let mut node = &self.head.next;

        while let Some(inner) = node {
            result.push(&inner.key);
            node = &inner.next;
        }

        result
    }
}

impl<K, V> SequentialMap<K, V> for SortedList<K, V>
where
    K: Default + Eq + PartialOrd + Clone,
    V: Default,
{
    fn new() -> SortedList<K, V> {
        SortedList {
            head: Node::default(),
        }
    }

    fn insert(&mut self, key: &K, value: V) -> Result<(), V> {
        let mut new = Box::new(Node::new(key.clone(), value));

        let mut current = &mut self.head;

        loop {
            let next = &mut current.next;

            if next.is_some() {
                let next_key = &next.as_ref().unwrap().key;

                if next_key == key {
                    return Err(new.value);
                } else if next_key > key {
                    // insert for sorted keys
                    mem::swap(next, &mut new.next);
                    let _ = mem::replace(next, Some(new));

                    return Ok(());
                }

                current = next.as_mut().unwrap();
            } else {
                *next = Some(new);
                return Ok(());
            }
        }
    }

    fn lookup(&self, key: &K) -> Option<&V> {
        let mut current = &self.head.next;

        loop {
            match current {
                Some(node) => {
                    let value = &node.value;

                    if node.key == *key {
                        return Some(value);
                    }

                    current = &node.next;
                }
                None => return None,
            }
        }
    }

    fn remove(&mut self, key: &K) -> Result<V, ()> {
        let mut prev = &mut self.head;

        loop {
            match prev.next.is_some() {
                true => {
                    if prev.next.as_ref().unwrap().key == *key {
                        let mut node = prev.next.take();
                        prev.next = node.as_mut().unwrap().next.take();

                        return Ok(node.unwrap().value);
                    }

                    prev = prev.next.as_mut().unwrap();
                }
                false => return Err(()),
            }
        }
    }
}

impl<K, V> Drop for SortedList<K, V> {
    fn drop(&mut self) {
        let mut node = self.head.next.take();

        while let Some(mut inside) = node {
            node = inside.next.take();
        }
    }
}
