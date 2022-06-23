use cds::stack::{ConcurrentStack, EBStack};
use crossbeam_utils::thread::scope;

#[test]
fn test_ebstack() {
    let stack = EBStack::new();

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
