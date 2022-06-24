use cds::stack::{ConcurrentStack, MutexStack};
use crossbeam_utils::thread::scope;

#[test]
fn test_mutex_stack() {
    let stack = MutexStack::new();

    scope(|scope| {
        for _ in 0..10 {
            scope.spawn(|_| {
                for i in 0..10_000 {
                    stack.push(i);
                    assert!(stack.pop().is_some());
                }
            });
        }
    })
    .unwrap();

    assert!(stack.pop().is_none());
}
