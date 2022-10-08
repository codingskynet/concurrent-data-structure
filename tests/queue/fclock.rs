use cds::queue::{FCQueue, Queue};

use super::*;

#[test]
fn test_spin_lock_fc_queue_sequential() {
    test_sequential_concurrent_queue::<FCQueue<_, Queue<_>>>();
}

#[test]
fn test_spin_lock_fc_queue_simple() {
    test_simple_concurrent_queue::<FCQueue<_, Queue<_>>>();
}

#[test]
fn test_spin_lock_fc_queue_spsc() {
    test_spsc_concurrent_queue::<FCQueue<_, Queue<_>>>();
}

#[test]
fn test_spin_lock_fc_queue_spmc() {
    test_spmc_concurrent_queue::<FCQueue<_, Queue<_>>>();
}

#[test]
fn test_spin_lock_fc_queue_mpsc() {
    test_mpsc_concurrent_queue::<FCQueue<_, Queue<_>>>();
}

#[test]
fn test_spin_lock_fc_queue_mpmc() {
    test_mpmc_concurrent_queue::<FCQueue<_, Queue<_>>>();
}

#[test]
fn test_spin_lock_fc_fat_node_queue_sequential() {
    test_sequential_concurrent_queue::<FCQueue<_, FatNodeQueue<_>>>();
}

#[test]
fn test_spin_lock_fc_fat_node_queue_simple() {
    test_simple_concurrent_queue::<FCQueue<_, FatNodeQueue<_>>>();
}

#[test]
fn test_spin_lock_fc_fat_node_queue_spsc() {
    test_spsc_concurrent_queue::<FCQueue<_, FatNodeQueue<_>>>();
}

#[test]
fn test_spin_lock_fc_fat_node_queue_spmc() {
    test_spmc_concurrent_queue::<FCQueue<_, FatNodeQueue<_>>>();
}

#[test]
fn test_spin_lock_fc_fat_node_queue_mpsc() {
    test_mpsc_concurrent_queue::<FCQueue<_, FatNodeQueue<_>>>();
}

#[test]
fn test_spin_lock_fc_fat_node_queue_mpmc() {
    test_mpmc_concurrent_queue::<FCQueue<_, FatNodeQueue<_>>>();
}
