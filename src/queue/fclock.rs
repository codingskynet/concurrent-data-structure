use std::ptr;

use crossbeam_epoch::pin;
use crossbeam_utils::Backoff;

use crate::lock::fclock::{FCLock, FlatCombining, State};

use super::{ConcurrentQueue, Queue};

enum QueueOp<V> {
    EnqRequest(V),
    Deq,
    DeqResponse(Option<V>),
}

impl<V> FlatCombining<QueueOp<V>> for Queue<V> {
    fn apply(&mut self, operation: &mut QueueOp<V>) {
        match operation {
            QueueOp::EnqRequest(value) => unsafe { self.push(ptr::read(value)) },
            QueueOp::Deq => *operation = QueueOp::DeqResponse(self.pop()),
            QueueOp::DeqResponse(_) => unreachable!(),
        }
    }
}

pub struct FCQueue<V> {
    queue: FCLock<QueueOp<V>>,
}

unsafe impl<T: Send> Send for FCQueue<T> {}
unsafe impl<T: Send> Sync for FCQueue<T> {}

impl<V: 'static> ConcurrentQueue<V> for FCQueue<V> {
    fn new() -> Self {
        let queue = Box::new(Queue::new());

        Self {
            queue: FCLock::new(queue),
        }
    }

    fn push(&self, value: V) {
        let guard = pin();

        let record = unsafe { self.queue.acquire_record().as_mut() };
        record.set(QueueOp::EnqRequest(value), State::Request, &guard);

        self.queue.try_combine(&guard);
    }

    fn try_pop(&self) -> Option<V> {
        let guard = pin();

        let record = unsafe { self.queue.acquire_record().as_mut() };
        record.set(QueueOp::Deq, State::Request, &guard);

        self.queue.try_combine(&guard);

        let operation = record.get_operation(&guard);

        if let QueueOp::DeqResponse(value) = operation {
            value
        } else {
            unreachable!()
        }
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
