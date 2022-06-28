use std::sync::Mutex;

use crossbeam_utils::Backoff;

use crate::lock::spinlock::SpinLock;

use super::{ConcurrentStack, Stack};

pub struct MutexStack<V> {
    stack: Mutex<Stack<V>>,
}

impl<V> ConcurrentStack<V> for MutexStack<V> {
    fn new() -> Self {
        Self {
            stack: Mutex::new(Stack::new()),
        }
    }

    fn push(&self, value: V) {
        self.stack.lock().unwrap().push(value);
    }

    fn try_pop(&self) -> Option<V> {
        let value = match self.stack.lock() {
            Ok(mut guard) => guard.pop(),
            Err(_) => unreachable!(),
        };

        value
    }

    fn pop(&self) -> V {
        let backoff = Backoff::new();

        loop {
            match self.try_pop() {
                Some(value) => return value,
                None => {}
            }

            backoff.snooze();
        }
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

    fn try_pop(&self) -> Option<V> {
        let mut guard = self.stack.lock();

        guard.pop()
    }

    fn pop(&self) -> V {
        let backoff = Backoff::new();

        loop {
            match self.try_pop() {
                Some(value) => return value,
                None => {}
            }

            backoff.snooze();
        }
    }
}
