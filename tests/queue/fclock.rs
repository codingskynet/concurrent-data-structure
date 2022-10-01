use cds::queue::{ConcurrentQueue, FCQueue};
use crossbeam_utils::thread::scope;

#[test]
fn test_fc_queue_sequential() {
    let queue = FCQueue::new();

    for i in 0..1_000 {
        queue.push(i);
        queue.pop();
    }

    assert!(queue.try_pop().is_none());
}

#[test]
fn test_fc_queue_simple() {
    let queue = FCQueue::new();

    scope(|scope| {
        for _ in 0..10 {
            scope.spawn(|_| {
                for i in 0..100 {
                    queue.push(i);
                    queue.pop();
                }
            });
        }
    })
    .unwrap();

    assert!(queue.try_pop().is_none());
}

#[test]
fn test_fc_queue_spsc() {
    let queue = FCQueue::new();

    scope(|scope| {
        scope.spawn(|_| {
            for i in 0..1_000_000 {
                queue.push(i);
            }
        });

        scope.spawn(|_| {
            let mut result = Vec::new();

            for _ in 0..1_000_000 {
                result.push(queue.pop());
            }

            let mut expected = result.clone();
            expected.sort();

            assert_eq!(expected, result);
        });
    })
    .unwrap();

    assert!(queue.try_pop().is_none());
}

#[test]
fn test_fc_queue_spmc() {
    let queue = FCQueue::new();

    scope(|scope| {
        scope.spawn(|_| {
            for i in 0..1_000_000 {
                queue.push(i);
            }
        });

        for _ in 0..10 {
            scope.spawn(|_| {
                for _ in 0..100_000 {
                    queue.pop();
                }
            });
        }
    })
    .unwrap();

    assert!(queue.try_pop().is_none());
}

#[test]
fn test_fc_queue_mpsc() {
    let queue = FCQueue::new();

    scope(|scope| {
        for _ in 0..10 {
            scope.spawn(|_| {
                for i in 0..100_000 {
                    queue.push(i);
                }
            });
        }

        scope.spawn(|_| {
            for _ in 0..1_000_000 {
                queue.pop();
            }
        });
    })
    .unwrap();

    assert!(queue.try_pop().is_none());
}

#[test]
fn test_fc_queue_mpmc() {
    let queue = FCQueue::new();

    scope(|scope| {
        for _ in 0..10 {
            scope.spawn(|_| {
                for i in 0..100_000 {
                    queue.push(i);
                }
            });

            scope.spawn(|_| {
                for _ in 0..100_000 {
                    queue.pop();
                }
            });
        }
    })
    .unwrap();

    assert!(queue.try_pop().is_none());
}
