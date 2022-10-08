mod fclock;
mod lockfree;
mod mutex;
mod spinlock;

use std::thread;

use cds::queue::{ConcurrentQueue, FatNodeQueue, Queue, SequentialQueue};

#[test]
fn test_simple_queue() {
    test_simple_sequential_queue::<Queue<_>>();
}

#[test]
fn test_deep_queue() {
    test_deep_sequential_queue::<Queue<_>>();
}

#[test]
fn test_fat_node_queue() {
    test_simple_sequential_queue::<FatNodeQueue<_>>();
}

#[test]
fn test_deep_fat_node_queue() {
    test_deep_sequential_queue::<FatNodeQueue<_>>();
}

fn test_simple_sequential_queue<Q: SequentialQueue<u64>>() {
    let mut queue = Q::new();

    queue.push(1);
    queue.push(2);
    queue.push(3);
    queue.push(4);
    queue.push(5);

    assert_eq!(queue.pop(), Some(1));
    assert_eq!(queue.pop(), Some(2));
    assert_eq!(queue.pop(), Some(3));
    assert_eq!(queue.pop(), Some(4));
    assert_eq!(queue.pop(), Some(5));

    assert_eq!(queue.pop(), None);
}

fn test_deep_sequential_queue<Q: SequentialQueue<u64>>() {
    let mut queue = Q::new();

    for n in 1..100_000 {
        queue.push(n);
    }

    for n in 1..100_000 {
        assert_eq!(queue.pop(), Some(n));
    }

    assert_eq!(queue.pop(), None);
}

fn test_sequential_concurrent_queue<Q: ConcurrentQueue<u64>>() {
    let queue = Q::new();

    for i in 0..1_000 {
        queue.push(i);
        queue.pop();
    }

    assert!(queue.try_pop().is_none());
}

fn test_simple_concurrent_queue<Q: Sync + ConcurrentQueue<u64>>() {
    let queue = Q::new();

    thread::scope(|scope| {
        for _ in 0..10 {
            scope.spawn(|| {
                for i in 0..1_000 {
                    queue.push(i);
                    queue.pop();
                }
            });
        }
    });

    assert!(queue.try_pop().is_none());
}

fn test_spsc_concurrent_queue<Q: Sync + ConcurrentQueue<u64>>() {
    let queue = Q::new();

    thread::scope(|scope| {
        scope.spawn(|| {
            for i in 0..1_000_000 {
                queue.push(i);
            }
        });

        scope.spawn(|| {
            let mut result = Vec::new();

            for _ in 0..1_000_000 {
                result.push(queue.pop());
            }

            let mut expected = result.clone();
            expected.sort();

            assert_eq!(expected, result);
        });
    });

    assert!(queue.try_pop().is_none());
}

fn test_spmc_concurrent_queue<Q: Sync + ConcurrentQueue<u64>>() {
    let queue = Q::new();

    thread::scope(|scope| {
        scope.spawn(|| {
            for i in 0..1_000_000 {
                queue.push(i);
            }
        });

        for _ in 0..10 {
            scope.spawn(|| {
                for _ in 0..100_000 {
                    queue.pop();
                }
            });
        }
    });

    assert!(queue.try_pop().is_none());
}

fn test_mpsc_concurrent_queue<Q: Sync + ConcurrentQueue<u64>>() {
    let queue = Q::new();

    thread::scope(|scope| {
        for _ in 0..10 {
            scope.spawn(|| {
                for i in 0..100_000 {
                    queue.push(i);
                }
            });
        }

        scope.spawn(|| {
            for _ in 0..1_000_000 {
                queue.pop();
            }
        });
    });

    assert!(queue.try_pop().is_none());
}

fn test_mpmc_concurrent_queue<Q: Sync + ConcurrentQueue<u64>>() {
    let queue = Q::new();

    thread::scope(|scope| {
        for _ in 0..10 {
            scope.spawn(|| {
                for i in 0..100_000 {
                    queue.push(i);
                }
            });

            scope.spawn(|| {
                for _ in 0..100_000 {
                    queue.pop();
                }
            });
        }
    });

    assert!(queue.try_pop().is_none());
}
