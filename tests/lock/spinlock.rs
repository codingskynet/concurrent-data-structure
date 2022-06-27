use std::ops::{Deref, DerefMut};

use cds::lock::SpinLock;
use crossbeam_utils::thread::scope;

#[test]
fn test_spin_lock() {
    let counter = SpinLock::new(0);

    scope(|scope| {
        for _ in 0..50 {
            scope.spawn(|_| {
                for _ in 0..1_000 {
                    let mut lock_guard = counter.lock();
                    *lock_guard.deref_mut() += 1;
                }
            });
        }
    })
    .unwrap();

    assert_eq!(*counter.lock(), 50_000);
}
