use cds::queue::{ConcurrentQueue, SpinLockQueue, TwoSpinLockQueue};
use crossbeam_utils::thread::scope;

#[test]
fn test_spin_lock_queue_simple() {
    let queue = SpinLockQueue::new();

    scope(|scope| {
        for _ in 0..10 {
            scope.spawn(|_| {
                for i in 0..1_000 {
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
fn test_spin_lock_queue_spsc() {
    let queue = SpinLockQueue::new();

    scope(|scope| {
        scope.spawn(|_| {
            for i in 0..1_000_000 {
                queue.push(i);
            }
        });

        scope.spawn(|_| {
            let mut result = Vec::new();

            for _ in 0..1_000_000 {
                let n = queue.pop();
                result.push(n);
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
fn test_spin_lock_queue_spmc() {
    let queue = SpinLockQueue::new();

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
fn test_spin_lock_queue_mpsc() {
    let queue = SpinLockQueue::new();

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
fn test_spin_lock_queue_mpmc() {
    let queue = SpinLockQueue::new();

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

#[test]
fn test_two_spin_lock_queue_simple() {
    let queue = TwoSpinLockQueue::new();

    scope(|scope| {
        for _ in 0..10 {
            scope.spawn(|_| {
                for i in 0..1_000 {
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
fn test_two_spin_lock_queue_spsc() {
    let queue = TwoSpinLockQueue::new();

    scope(|scope| {
        scope.spawn(|_| {
            for i in 0..1_000_000 {
                queue.push(i);
            }
        });

        scope.spawn(|_| {
            let mut result = Vec::new();

            for _ in 0..1_000_000 {
                let n = queue.pop();
                result.push(n);
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
fn test_two_spin_lock_queue_spmc() {
    let queue = TwoSpinLockQueue::new();

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
fn test_two_spin_lock_queue_mpsc() {
    let queue = TwoSpinLockQueue::new();

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
fn test_two_spin_lock_queue_mpmc() {
    let queue = TwoSpinLockQueue::new();

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
