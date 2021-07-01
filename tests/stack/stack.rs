use cds::stack::stack::*;

#[test]
fn test_stack() {
    let mut stack = Stack::new();
    assert_eq!(stack.is_empty(), true);

    stack.push(1);
    stack.push(2);
    stack.push(3);
    stack.push(4);
    stack.push(5);

    assert_eq!(stack.is_empty(), false);
    assert_eq!(stack.top(), Some(&5));

    assert_eq!(stack.pop(), Some(5));
    assert_eq!(stack.pop(), Some(4));
    assert_eq!(stack.pop(), Some(3));
    assert_eq!(stack.pop(), Some(2));
    assert_eq!(stack.pop(), Some(1));

    assert_eq!(stack.is_empty(), true);
    assert_eq!(stack.pop(), None);
}
