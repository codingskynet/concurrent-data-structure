use std::time::Instant;

use cds::{avl_tree::rwlock::RwLockAVLTree, map::ConcurrentMap};
use crossbeam_epoch::pin;
use crossbeam_utils::thread;
use rand::{thread_rng, Rng};

use crate::util::map::{stress_concurrent, stress_concurrent_as_sequential};

#[test]
fn test_rwlock_avl_tree() {
    let num = 64;
    let pin = pin();
    let avl: RwLockAVLTree<i32, i32> = RwLockAVLTree::new();

    for i in 0..num {
        assert_eq!(avl.insert(&i, i, &pin), Ok(()));
    }

    for i in 0..num {
        assert_eq!(avl.insert(&i, i, &pin), Err(i));
    }

    assert_eq!(avl.get_height(&pin), f32::log2(num as f32) as usize + 1);

    for i in 0..num {
        assert_eq!(avl.lookup(&i, &pin), Some(i));
    }

    for i in 0..num {
        assert_eq!(avl.remove(&i, &pin), Ok(i));
    }

    for i in 0..num {
        assert_eq!(avl.remove(&i, &pin), Err(()));
    }
}

#[test]
fn stress_rwlock_avl_tree_sequential() {
    stress_concurrent_as_sequential::<u8, RwLockAVLTree<_, _>>(100_000);
}

#[test]
fn stress_rwlock_avl_tree_concurrent() {
    stress_concurrent::<u32, RwLockAVLTree<_, _>>(200_000, 16, false);
}

#[test]
fn assert_rwlock_avl_tree_concurrent() {
    stress_concurrent::<u8, RwLockAVLTree<_, _>>(100_000, 32, true);
}

#[test]
fn bench_large_rwlock_avl_tree() {
    let thread_num = 16;
    let iter = 1_000_000 / thread_num;

    let avl: RwLockAVLTree<u64, u64> = RwLockAVLTree::new();

    let start = Instant::now();
    let _ = thread::scope(|s| {
        let mut threads = Vec::new();

        for _ in 0..thread_num {
            let t = s.spawn(|_| {
                let mut rng = thread_rng();

                for _ in 0..iter {
                    let key = rng.gen_range(0..(thread_num * iter * 2));
                    let _ = avl.insert(&key, key, &pin());
                }
            });

            threads.push(t);
        }

        threads
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    println!(
        "RwLockAVL {} Insert: {} ms",
        thread_num * iter,
        start.elapsed().as_millis()
    );
    println!("RwLockAVL height: {}", avl.get_height(&pin()));

    let start = Instant::now();
    let _ = thread::scope(|s| {
        let mut threads = Vec::new();

        for _ in 0..thread_num {
            let t = s.spawn(|_| {
                let mut rng = thread_rng();

                for _ in 0..iter {
                    let key = rng.gen_range(0..(thread_num * iter * 2));
                    let _ = avl.lookup(&key, &pin());
                }
            });

            threads.push(t);
        }

        threads
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    println!(
        "RwLockAVL {} Lookup(50% success): {} ms",
        thread_num * iter,
        start.elapsed().as_millis()
    );

    let start = Instant::now();
    let _ = thread::scope(|s| {
        let mut threads = Vec::new();

        for _ in 0..thread_num {
            let t = s.spawn(|_| {
                let mut rng = thread_rng();

                for _ in 0..iter {
                    let key = rng.gen_range(0..(thread_num * iter * 2));
                    let _ = avl.remove(&key, &pin());
                }
            });

            threads.push(t);
        }

        threads
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    println!(
        "RwLockAVL {} Remove(50% success): {} ms",
        thread_num * iter,
        start.elapsed().as_millis()
    );
}
