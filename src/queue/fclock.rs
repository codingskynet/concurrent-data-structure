use std::{fmt::Debug, ptr};

use crossbeam_epoch::{pin, unprotected};
use crossbeam_utils::Backoff;

use crate::lock::fclock::{FCLock, FlatCombining, State};

use super::{ConcurrentQueue, Queue};

#[derive(Debug)]
enum QueueOp<V> {
    EnqRequest(V),
    Deq,
    DeqResponse(Option<V>),
}

unsafe impl<T> Send for QueueOp<T> {}
unsafe impl<T> Sync for QueueOp<T> {}

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

unsafe impl<T> Send for FCQueue<T> {}
unsafe impl<T> Sync for FCQueue<T> {}

impl<V: 'static + Debug> ConcurrentQueue<V> for FCQueue<V> {
    fn new() -> Self {
        let queue = Box::new(Queue::new());

        Self {
            queue: FCLock::new(queue),
        }
    }

    fn push(&self, value: V) {
        let guard = pin();

        let record = self.queue.acquire_record(&guard);
        let record_ref = unsafe { record.deref() };

        debug_assert_ne!(*record_ref.get_state(&guard), State::Active);

        record_ref.set(QueueOp::EnqRequest(value));

        self.queue.push_record(record, &guard);
        self.queue.try_combine(record, &guard);
    }

    fn try_pop(&self) -> Option<V> {
        let guard = pin();

        let record = self.queue.acquire_record(&guard);
        let record_ref = unsafe { record.deref() };

        debug_assert_ne!(*record_ref.get_state(&guard), State::Active);

        record_ref.set(QueueOp::Deq);

        self.queue.push_record(record, &guard);
        self.queue.try_combine(record, &guard);

        let operation = record_ref.get_operation(&guard);

        debug_assert_ne!(*record_ref.get_state(&guard), State::Active);

        if let QueueOp::DeqResponse(value) = operation {
            value
        } else {
            println!("{:?}", operation);
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

// impl<V> Drop for FCQueue<V> {
//     fn drop(&mut self) {
//         unsafe {
//             self.queue.release_local_record(unprotected());
//         }
//     }
// }
