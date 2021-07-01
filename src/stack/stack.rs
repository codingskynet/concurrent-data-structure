use std::mem;

pub struct Stack<T> {
    head: Option<Box<Node<T>>>
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

impl<T> Stack<T> {
    pub fn new() -> Stack<T> {
        Stack {
            head: None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    pub fn top(&self) -> Option<&T> { // Option<Box<Node<T>>> -> Option<&T> ?
        match &self.head {
            Some(node) => Some(&node.as_ref().data),
            None => None,
        }
    }

    pub fn push(&mut self, data: T) {
        let node = Box::new(Node::new(data));
        
        let prev = mem::replace(&mut self.head, Some(node));
        self.head.as_mut().unwrap().next = prev;
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
