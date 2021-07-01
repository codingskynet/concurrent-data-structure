use std::{mem, ptr::null};

pub struct Queue<T> {
    head: Option<Box<Node<T>>>,
    tail: *mut Box<Node<T>>,
}

struct Node<T> {
    data: T,
    next: Option<Box<Node<T>>>,
}

impl<T> Node<T> {
    fn new(data: T) -> Node<T> {
        Node {
            data, 
            next: None
        }
    }
}

impl<T> Queue<T> {
    pub fn new() -> Queue<T> {
        Queue {
            head: None,
            tail: null::<T>() as *mut _,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    pub fn top(&self) -> Option<&T> {
        match &self.head {
            Some(node) => Some(&node.as_ref().data),
            None => None,
        }
    }

    pub fn push(&mut self, data: T) {
        let node = Box::new(Node::new(data));

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

    pub fn pop(&mut self) -> Option<T> {
        if self.head.is_some() {
            let mut top = mem::replace(&mut self.head, None);
            self.head = mem::replace(&mut top.as_mut().unwrap().next, None);

            return Some(top.unwrap().data)
        }

        None
    }
}
