use cds::queue::MSQueue;

use super::*;

#[test]
fn test_ms_queue_sequential() {
    test_sequential_concurrent_queue::<MSQueue<_>>();
}

#[test]
fn test_ms_queue_simple() {
    test_simple_concurrent_queue::<MSQueue<_>>();
}

#[test]
fn test_ms_queue_spsc() {
    test_spsc_concurrent_queue::<MSQueue<_>>();
}

#[test]
fn test_ms_queue_spmc() {
    test_spmc_concurrent_queue::<MSQueue<_>>();
}

#[test]
fn test_ms_queue_mpsc() {
    test_mpsc_concurrent_queue::<MSQueue<_>>();
}

#[test]
fn test_ms_queue_mpmc() {
    test_mpmc_concurrent_queue::<MSQueue<_>>();
}
