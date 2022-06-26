use cds::queue::{ConcurrentQueue, MSQueue};
use crossbeam_utils::thread::scope;

#[test]
fn test_ms_queue_spsc() {
    let queue = MSQueue::new();

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
fn test_ms_queue_spmc() {
    let queue = MSQueue::new();

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
fn test_ms_queue_mpmc() {
    let queue = MSQueue::new();

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
