use std::{mem, ptr::null};

// simple sequential queue
pub struct Queue<V> {
    head: Option<Box<Node<V>>>,
    tail: *mut Box<Node<V>>,
}

struct Node<V> {
    value: V,
    next: Option<Box<Node<V>>>,
}

impl<V> Node<V> {
    fn new(value: V) -> Node<V> {
        Node {
            value, 
            next: None
        }
    }
}

impl<V> Queue<V> {
    pub fn new() -> Queue<V> {
        Queue {
            head: None,
            tail: null::<V>() as *mut _,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    pub fn top(&self) -> Option<&V> {
        match &self.head {
            Some(node) => Some(&node.as_ref().value),
            None => None,
        }
    }

    pub fn push(&mut self, value: V) {
        let node = Box::new(Node::new(value));

        if self.head.is_none() {
            self.head = Some(node);
            self.tail = self.head.as_mut().unwrap() as *mut _;
        } else {
            unsafe {
                (*self.tail).next = Some(node);
                self.tail = (*self.tail).next.as_mut().unwrap() as *mut _;
            }
        }
    }

    pub fn pop(&mut self) -> Option<V> {
        if self.head.is_some() {
            let mut top = mem::replace(&mut self.head, None);
            self.head = mem::replace(&mut top.as_mut().unwrap().next, None);

            return Some(top.unwrap().value)
        }

        None
    }
}
