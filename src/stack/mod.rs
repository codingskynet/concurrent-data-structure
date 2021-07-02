use std::mem;

// simple sequential stack
pub struct Stack<V> {
    head: Option<Box<Node<V>>>
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

impl<V> Stack<V> {
    pub fn new() -> Stack<V> {
        Stack {
            head: None
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
        
        let prev = mem::replace(&mut self.head, Some(node));
        self.head.as_mut().unwrap().next = prev;
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
