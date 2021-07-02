use std::mem;
use crate::sequential::SequentialMap;

// simple sequential linked list
pub struct LinkedList<K: Eq + Copy, V> {
    head: Option<Box<Node<K, V>>>,
}

impl<K: Eq + Copy, V> LinkedList<K, V> {
    pub fn new() -> LinkedList<K, V> {
        LinkedList {
            head: None
        }
    }
}

struct Node<K: Eq, V> {
    key: K,
    value: V,
    next: Option<Box<Node<K, V>>>,
}

impl<K: Eq, V> Node<K, V> {
    fn new(key: K, value: V) -> Node<K, V> {
        Node {
            key,
            value, 
            next: None
        }
    }
}

impl<K: Eq + Copy, V> SequentialMap<K, V> for LinkedList<K, V> {
    fn insert(&mut self, key: &K, value: V) -> Result<(), V> {
        let node = Box::new(Node::new(key.clone(), value));

        if self.head.is_none() {
            self.head = Some(node);
            return Ok(())
        }

        let mut current = self.head.as_mut().unwrap();

        loop {
            if current.key == *key {
                return Err(node.value)
            }

            if current.next.is_none() {
                current.next = Some(node);
                return Ok(());
            }

            current = current.next.as_mut().unwrap();
        }
    }

    fn lookup(&self, key: &K) -> Option<&V> {
        if self.head.is_none() {
            return None
        }

        let mut current = self.head.as_ref().unwrap();

        loop {
            if current.key == *key {
                return Some(&current.value)
            }

            if current.next.is_some() {
                current = current.next.as_ref().unwrap();
            } else {
                return None
            }
        }
    }

    fn delete(&mut self, key: &K) -> Result<V, ()> {
        if self.head.is_some() {
            if self.head.as_ref().unwrap().key == *key {
                let mut node = mem::replace(&mut self.head, None);
                self.head = mem::replace(&mut node.as_mut().unwrap().next, None);

                return Ok(node.unwrap().value)
            }
        } else {
            return Err(())
        }

        if self.head.is_none() {
            return Err(())
        }

        let mut prev = self.head.as_mut().unwrap();

        loop {
            if prev.next.is_none() {
                return Err(())
            }

            if prev.next.as_ref().unwrap().key == *key {
                let mut node = mem::replace(&mut prev.next, None);
                prev.next = mem::replace(&mut node.as_mut().unwrap().next, None);

                return Ok(node.unwrap().value)
            }

            prev = prev.next.as_mut().unwrap();
        }
    }
}
