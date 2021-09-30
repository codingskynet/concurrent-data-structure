use cds::stack::{ConcurrentStack, TreiberStack};
use crossbeam_epoch::pin;

#[test]
fn test_treiber_stack() {
    let stack = TreiberStack::new();
    let pin = pin();

    assert_eq!(stack.is_empty(&pin), true);

    stack.push(1, &pin);
    stack.push(2, &pin);
    stack.push(3, &pin);
    stack.push(4, &pin);
    stack.push(5, &pin);

    assert_eq!(stack.is_empty(&pin), false);
    assert_eq!(stack.top(&pin), Some(5));

    assert_eq!(stack.pop(&pin), Some(5));
    assert_eq!(stack.pop(&pin), Some(4));
    assert_eq!(stack.pop(&pin), Some(3));
    assert_eq!(stack.pop(&pin), Some(2));
    assert_eq!(stack.pop(&pin), Some(1));

    assert_eq!(stack.is_empty(&pin), true);
    assert_eq!(stack.pop(&pin), None);
}
