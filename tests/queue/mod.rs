mod lockfree;
mod mutex;
mod spinlock;

use cds::queue::Queue;

#[test]
fn test_queue() {
    let mut queue = Queue::new();
    assert_eq!(queue.is_empty(), true);

    queue.push(1);
    queue.push(2);
    queue.push(3);
    queue.push(4);
    queue.push(5);

    assert_eq!(queue.is_empty(), false);
    assert_eq!(queue.top(), Some(&1));

    assert_eq!(queue.pop(), Some(1));
    assert_eq!(queue.pop(), Some(2));
    assert_eq!(queue.pop(), Some(3));
    assert_eq!(queue.pop(), Some(4));
    assert_eq!(queue.pop(), Some(5));

    assert_eq!(queue.is_empty(), true);
    assert_eq!(queue.pop(), None);
}

#[test]
fn test_deep_queue() {
    let mut queue = Queue::new();

    for n in 1..100_000 {
        queue.push(n);
    }

    for n in 1..100_000 {
        assert_eq!(queue.pop(), Some(n));
    }

    assert_eq!(queue.is_empty(), true);
}
