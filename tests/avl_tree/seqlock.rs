use std::time::Instant;

use cds::{avl_tree::seqlock::SeqLockAVLTree, map::ConcurrentMap};
use crossbeam_epoch::pin;
use crossbeam_utils::thread;
use rand::{thread_rng, Rng};

use crate::util::map::{bench_concurrent_stat, stress_concurrent, stress_concurrent_as_sequential};

#[test]
fn test_seqlock_avl_tree() {
    let num = 64;
    let pin = pin();
    let avl: SeqLockAVLTree<i32, i32> = SeqLockAVLTree::new();

    for i in 0..num {
        assert_eq!(avl.insert(&i, i, &pin), Ok(()));
    }

    for i in 0..num {
        assert_eq!(avl.insert(&i, i, &pin), Err(i));
    }

    assert_eq!(avl.get_height(&pin), f32::log2(num as f32) as usize + 1);

    for i in 0..num {
        assert_eq!(avl.get(&i, &pin), Some(i));
    }

    for i in 0..num {
        assert_eq!(avl.remove(&i, &pin), Ok(i));
    }

    for i in 0..num {
        assert_eq!(avl.remove(&i, &pin), Err(()));
    }
}

#[test]
fn stress_seqlock_avl_tree_sequential() {
    stress_concurrent_as_sequential::<u8, SeqLockAVLTree<_, _>>(100_000);
}

#[test]
fn stress_seqlock_avl_tree_concurrent() {
    stress_concurrent::<u32, SeqLockAVLTree<_, _>>(200_000, 16, false);
}

#[test]
fn assert_seqlock_avl_tree_concurrent() {
    stress_concurrent::<u8, SeqLockAVLTree<_, _>>(100_000, 32, true);
    stress_concurrent::<u64, SeqLockAVLTree<_, _>>(100_000, 32, true);
}

#[test]
fn bench_worst_large_seqlock_avl_tree() {
    let thread_num = 4;
    let iter = 1_000_000 / thread_num;

    let avl: SeqLockAVLTree<u64, u64> = SeqLockAVLTree::new();

    let start = Instant::now();
    let _ = thread::scope(|s| {
        let mut threads = Vec::new();

        for _ in 0..thread_num {
            let t = s.spawn(|_| {
                let mut rng = thread_rng();
                let id = rng.gen_range(0..100);

                for i in 0..iter {
                    let key = id * iter + i;
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
        "SeqLockAVL {} Worst Insert: {} ms",
        thread_num * iter,
        start.elapsed().as_millis()
    );
    println!("SeqLockAVL height: {}", avl.get_height(&pin()));
}

#[test]
fn bench_large_seqlock_avl_tree() {
    let thread_num = 4;
    let iter = 1_000_000 / thread_num;

    let avl: SeqLockAVLTree<u64, u64> = SeqLockAVLTree::new();

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
        "SeqLockAVL {} Insert: {} ms",
        thread_num * iter,
        start.elapsed().as_millis()
    );
    println!("SeqLockAVL height: {}", avl.get_height(&pin()));

    let start = Instant::now();
    let _ = thread::scope(|s| {
        let mut threads = Vec::new();

        for _ in 0..thread_num {
            let t = s.spawn(|_| {
                let mut rng = thread_rng();

                for _ in 0..iter {
                    let key = rng.gen_range(0..(thread_num * iter * 2));
                    let _ = avl.get(&key, &pin());
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
        "SeqLockAVL {} Lookup(50% success): {} ms",
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
        "SeqLockAVL {} Remove(50% success): {} ms",
        thread_num * iter,
        start.elapsed().as_millis()
    );
}

#[test]
fn bench_mixed_seqlock_avl_tree() {
    let already_inserted = 1_000_000;
    let total_ops = 1_000_000;
    let insert_rate = 30;
    let lookup_rate = 50;
    let remove_rate = 20;
    let thread_num = 4;
    let max_time = 60; // the max time for checking repeating benches (second)

    assert_eq!(insert_rate + lookup_rate + remove_rate, 100);

    bench_concurrent_stat::<SeqLockAVLTree<_, _>>(
        "SeqLockAVLTree",
        already_inserted,
        total_ops * insert_rate / 100,
        total_ops * lookup_rate / 100,
        total_ops * remove_rate / 100,
        thread_num,
        max_time,
    )
}
