use crate::map::SequentialMap;

// simple sequential linked list
pub struct LinkedList<K: Default + Eq + Clone, V: Default> {
    head: Node<K, V>, // dummy node with key = Default, but the key is not considered on algorithm
}

struct Node<K: Eq, V> {
    key: K,
    value: V,
    next: Option<Box<Node<K, V>>>,
}

impl<K: Default + Eq, V: Default> Default for Node<K, V> {
    fn default() -> Self {
        Node {
            key: K::default(),
            value: V::default(),
            next: None,
        }
    }
}

impl<K: Eq, V> Node<K, V> {
    fn new(key: K, value: V) -> Node<K, V> {
        Node {
            key,
            value,
            next: None,
        }
    }
}

impl<K: Default + Ord + Clone, V: Default> SequentialMap<K, V> for LinkedList<K, V> {
    fn new() -> LinkedList<K, V> {
        LinkedList {
            head: Node::default(),
        }
    }

    fn insert(&mut self, key: &K, value: V) -> Result<(), V> {
        let new = Box::new(Node::new(key.clone(), value));

        let mut current = &mut self.head.next;

        loop {
            match current {
                Some(node) => {
                    if node.key == *key {
                        return Err(new.value);
                    }

                    current = &mut node.next;
                }
                None => {
                    *current = Some(new);
                    return Ok(());
                }
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

impl<K: Default + Eq + Clone, V: Default> Drop for LinkedList<K, V> {
    fn drop(&mut self) {
        let mut node = self.head.next.take();

        while let Some(mut inside) = node {
            node = inside.next.take();
        }
    }
}
