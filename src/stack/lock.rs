use std::sync::{Arc, Mutex};

use crate::lock::spinlock::SpinLock;

use super::{ConcurrentStack, Stack};

pub struct MutexStack<V> {
    stack: Arc<Mutex<Stack<V>>>,
}

impl<V> ConcurrentStack<V> for MutexStack<V> {
    fn new() -> Self {
        Self {
            stack: Arc::new(Mutex::new(Stack::new())),
        }
    }

    fn push(&self, value: V) {
        let guard = self.stack.clone();

        guard.lock().unwrap().push(value);
    }

    fn pop(&self) -> Option<V> {
        let guard = self.stack.clone();

        let value = match guard.lock() {
            Ok(mut guard) => guard.pop(),
            Err(_) => unreachable!(),
        };

        value
    }
}

pub struct SpinLockStack<V> {
    stack: SpinLock<Stack<V>>,
}

impl<V> ConcurrentStack<V> for SpinLockStack<V> {
    fn new() -> Self {
        Self {
            stack: SpinLock::new(Stack::new()),
        }
    }

    fn push(&self, value: V) {
        let mut guard = self.stack.lock();

        guard.push(value);
    }

    fn pop(&self) -> Option<V> {
        let mut guard = self.stack.lock();

        guard.pop()
    }
}