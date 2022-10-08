use cds::{
    lock::{spinlock::RawSpinLock, RawMutex},
    queue::{FCQueue, FatNodeQueue, Queue},
};

use super::*;

// Maybe general macro for generating all cases of DS is very useful, but it is too complicated.

#[test]
fn test_fc_queue_sequential() {
    test_sequential_concurrent_queue::<FCQueue<_, RawSpinLock, Queue<_>>>();
    test_sequential_concurrent_queue::<FCQueue<_, RawSpinLock, FatNodeQueue<_>>>();
    test_sequential_concurrent_queue::<FCQueue<_, RawMutex, Queue<_>>>();
    test_sequential_concurrent_queue::<FCQueue<_, RawMutex, FatNodeQueue<_>>>();
}

#[test]
fn test_fc_queue_simple() {
    test_simple_concurrent_queue::<FCQueue<_, RawSpinLock, Queue<_>>>();
    test_simple_concurrent_queue::<FCQueue<_, RawSpinLock, FatNodeQueue<_>>>();
    test_simple_concurrent_queue::<FCQueue<_, RawMutex, Queue<_>>>();
    test_simple_concurrent_queue::<FCQueue<_, RawMutex, FatNodeQueue<_>>>();
}

#[test]
fn test_fc_queue_spsc() {
    test_spsc_concurrent_queue::<FCQueue<_, RawSpinLock, Queue<_>>>();
    test_spsc_concurrent_queue::<FCQueue<_, RawSpinLock, FatNodeQueue<_>>>();
    test_spsc_concurrent_queue::<FCQueue<_, RawMutex, Queue<_>>>();
    test_spsc_concurrent_queue::<FCQueue<_, RawMutex, FatNodeQueue<_>>>();
}

#[test]
fn test_fc_queue_spmc() {
    test_spmc_concurrent_queue::<FCQueue<_, RawSpinLock, Queue<_>>>();
    test_spmc_concurrent_queue::<FCQueue<_, RawSpinLock, FatNodeQueue<_>>>();
    test_spmc_concurrent_queue::<FCQueue<_, RawMutex, Queue<_>>>();
    test_spmc_concurrent_queue::<FCQueue<_, RawMutex, FatNodeQueue<_>>>();
}

#[test]
fn test_fc_queue_mpsc() {
    test_mpsc_concurrent_queue::<FCQueue<_, RawSpinLock, Queue<_>>>();
    test_mpsc_concurrent_queue::<FCQueue<_, RawSpinLock, FatNodeQueue<_>>>();
    test_mpsc_concurrent_queue::<FCQueue<_, RawMutex, Queue<_>>>();
    test_mpsc_concurrent_queue::<FCQueue<_, RawMutex, FatNodeQueue<_>>>();
}

#[test]
fn test_fc_queue_mpmc() {
    test_mpmc_concurrent_queue::<FCQueue<_, RawSpinLock, Queue<_>>>();
    test_mpmc_concurrent_queue::<FCQueue<_, RawSpinLock, FatNodeQueue<_>>>();
    test_mpmc_concurrent_queue::<FCQueue<_, RawMutex, Queue<_>>>();
    test_mpmc_concurrent_queue::<FCQueue<_, RawMutex, FatNodeQueue<_>>>();
}
