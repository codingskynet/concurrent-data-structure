use cds::queue::{SpinLockQueue, TwoSpinLockQueue};

use super::*;

#[test]
fn test_spin_lock_queue_sequential() {
    test_sequential_concurrent_queue::<SpinLockQueue<_>>();
}

#[test]
fn test_spin_lock_queue_simple() {
    test_simple_concurrent_queue::<SpinLockQueue<_>>();
}

#[test]
fn test_spin_lock_queue_spsc() {
    test_spsc_concurrent_queue::<SpinLockQueue<_>>();
}

#[test]
fn test_spin_lock_queue_spmc() {
    test_spmc_concurrent_queue::<SpinLockQueue<_>>();
}

#[test]
fn test_spin_lock_queue_mpsc() {
    test_mpsc_concurrent_queue::<SpinLockQueue<_>>();
}

#[test]
fn test_spin_lock_queue_mpmc() {
    test_mpmc_concurrent_queue::<SpinLockQueue<_>>();
}

#[test]
fn test_two_spin_lock_queue_sequential() {
    test_sequential_concurrent_queue::<TwoSpinLockQueue<_>>();
}

#[test]
fn test_two_spin_lock_queue_simple() {
    test_simple_concurrent_queue::<TwoSpinLockQueue<_>>();
}

#[test]
fn test_two_spin_lock_queue_spsc() {
    test_spsc_concurrent_queue::<TwoSpinLockQueue<_>>();
}

#[test]
fn test_two_spin_lock_queue_spmc() {
    test_spmc_concurrent_queue::<TwoSpinLockQueue<_>>();
}

#[test]
fn test_two_spin_lock_queue_mpsc() {
    test_mpsc_concurrent_queue::<TwoSpinLockQueue<_>>();
}

#[test]
fn test_two_spin_lock_queue_mpmc() {
    test_mpmc_concurrent_queue::<TwoSpinLockQueue<_>>();
}
