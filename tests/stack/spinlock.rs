use cds::stack::{ConcurrentStack, SpinLockStack};
use crossbeam_utils::thread::scope;

#[test]
fn test_spinlock_stack() {
    let stack = SpinLockStack::new();

    scope(|scope| {
        for _ in 0..10 {
            scope.spawn(|_| {
                for i in 0..10_000 {
                    stack.push(i);
                    assert!(stack.try_pop().is_some());
                }
            });
        }
    })
    .unwrap();

    assert!(stack.try_pop().is_none());
}
