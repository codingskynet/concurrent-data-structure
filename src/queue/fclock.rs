use std::{fmt::Debug, hint::unreachable_unchecked};

use crossbeam_epoch::{pin, unprotected};
use crossbeam_utils::Backoff;

use crate::lock::fclock::{FCLock, FlatCombining, Operation};

use super::{ConcurrentQueue, Queue};

#[derive(Debug, PartialEq)]
enum QueueOp<V> {
    EnqRequest(V),
    EnqResponse,
    Deq,
    DeqResponse(Option<V>),
}

impl<V> Operation for QueueOp<V> {
    fn is_request(&self) -> bool {
        match self {
            QueueOp::EnqRequest(_) => true,
            QueueOp::EnqResponse => false,
            QueueOp::Deq => true,
            QueueOp::DeqResponse(_) => false,
        }
    }
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
            QueueOp::Deq => QueueOp::DeqResponse(self.pop()),
            op => op, //unreachable!("enq response should be removed."), //unreachable!("deq response should be removed."),
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

        record_ref.set(QueueOp::Deq);

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

// impl<V> Drop for FCQueue<V> {
//     fn drop(&mut self) {
//         unsafe {
//             self.queue.release_local_record(unprotected());
//         }
//     }
// }
