use std::{fmt::Debug, hint::unreachable_unchecked, marker::PhantomData};

use crossbeam_epoch::pin;
use crossbeam_utils::Backoff;

use crate::lock::fclock::{FCLock, FlatCombining};

use super::{ConcurrentQueue, SequentialQueue};

#[derive(Debug, PartialEq)]
enum QueueOp<V> {
    EnqRequest(V),
    EnqResponse,
    DeqRequest,
    DeqResponse(Option<V>),
}

unsafe impl<T> Send for QueueOp<T> {}
unsafe impl<T> Sync for QueueOp<T> {}

impl<V, Q: SequentialQueue<V>> FlatCombining<QueueOp<V>> for Q {
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

pub struct FCQueue<V, Q: SequentialQueue<V>> {
    queue: FCLock<QueueOp<V>>,
    _marker: PhantomData<Q>,
}

unsafe impl<V, Q: SequentialQueue<V>> Send for FCQueue<V, Q> {}
unsafe impl<V, Q: SequentialQueue<V>> Sync for FCQueue<V, Q> {}

impl<V, Q: SequentialQueue<V>> FCQueue<V, Q> {
    #[cfg(feature = "concurrent_stat")]
    pub fn print_stat(&self) {
        self.queue.print_stat();
    }
}

impl<V: 'static, Q: 'static + SequentialQueue<V> + FlatCombining<QueueOp<V>>> ConcurrentQueue<V>
    for FCQueue<V, Q>
{
    fn new() -> Self {
        let queue = Q::new();

        Self {
            queue: FCLock::new(queue),
            _marker: PhantomData,
        }
    }

    fn push(&self, value: V) {
        let guard = pin();

        let record = self.queue.acquire_record(&guard);
        let record_ref = unsafe { record.deref() };

        record_ref.set(QueueOp::EnqRequest(value));

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
                None => backoff.snooze(),
            }
        }
    }
}
