use std::{fmt::Debug, hint::unreachable_unchecked};

use crossbeam_epoch::pin;
use crossbeam_utils::Backoff;

use crate::lock::fclock::{FCLock, FlatCombining};

use super::{ConcurrentQueue, Queue};

#[derive(Debug, PartialEq)]
enum QueueOp<V> {
    EnqRequest(V),
    EnqResponse,
    DeqRequest,
    DeqResponse(Option<V>),
}

unsafe impl<T> Send for QueueOp<T> {}
unsafe impl<T> Sync for QueueOp<T> {}

impl<V> FlatCombining<QueueOp<V>> for Queue<V> {
    fn apply(&mut self, operation: QueueOp<V>) -> QueueOp<V> {
        match operation {
            QueueOp::EnqRequest(value) => {
                self.push(value);
                QueueOp::EnqResponse
            }
            QueueOp::DeqRequest => QueueOp::DeqResponse(self.pop()),
            _ => unreachable!("The response cannot be applied."),
        }
    }
}

pub struct FCQueue<V> {
    queue: FCLock<QueueOp<V>>,
}

unsafe impl<T> Send for FCQueue<T> {}
unsafe impl<T> Sync for FCQueue<T> {}

impl<V: Debug + 'static + PartialEq + Clone> ConcurrentQueue<V> for FCQueue<V> {
    fn new() -> Self {
        let queue = Queue::new();

        Self {
            queue: FCLock::new(queue),
        }
    }

    fn push(&self, value: V) {
        let guard = pin();

        let record = self.queue.acquire_record(&guard);
        let record_ref = unsafe { record.deref() };

        record_ref.set(QueueOp::EnqRequest(value.clone()));

        self.queue.try_combine(record, &guard);
    }

    fn try_pop(&self) -> Option<V> {
        let guard = pin();

        let record = self.queue.acquire_record(&guard);
        let record_ref = unsafe { record.deref() };

        record_ref.set(QueueOp::DeqRequest);

        self.queue.try_combine(record, &guard);

        let operation = record_ref.get_operation(&guard);

        if let QueueOp::DeqResponse(value) = operation {
            value
        } else {
            unsafe { unreachable_unchecked() }
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
