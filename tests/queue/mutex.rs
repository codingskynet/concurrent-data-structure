use cds::queue::{MutexQueue, TwoMutexQueue};

use super::*;

#[test]
fn test_mutex_queue_sequential() {
    test_sequential_concurrent_queue::<MutexQueue<_>>();
}

#[test]
fn test_mutex_queue_simple() {
    test_simple_concurrent_queue::<MutexQueue<_>>();
}

#[test]
fn test_mutex_queue_spsc() {
    test_spsc_concurrent_queue::<MutexQueue<_>>();
}

#[test]
fn test_mutex_queue_spmc() {
    test_spmc_concurrent_queue::<MutexQueue<_>>();
}

#[test]
fn test_mutex_queue_mpsc() {
    test_mpsc_concurrent_queue::<MutexQueue<_>>();
}

#[test]
fn test_mutex_queue_mpmc() {
    test_mpmc_concurrent_queue::<MutexQueue<_>>();
}

#[test]
fn test_two_mutex_queue_sequential() {
    test_sequential_concurrent_queue::<TwoMutexQueue<_>>();
}

#[test]
fn test_two_mutex_queue_simple() {
    test_simple_concurrent_queue::<TwoMutexQueue<_>>();
}

#[test]
fn test_two_mutex_queue_spsc() {
    test_spsc_concurrent_queue::<TwoMutexQueue<_>>();
}

#[test]
fn test_two_mutex_queue_spmc() {
    test_spmc_concurrent_queue::<TwoMutexQueue<_>>();
}

#[test]
fn test_two_mutex_queue_mpsc() {
    test_mpsc_concurrent_queue::<TwoMutexQueue<_>>();
}

#[test]
fn test_two_mutex_queue_mpmc() {
    test_mpmc_concurrent_queue::<TwoMutexQueue<_>>();
}
