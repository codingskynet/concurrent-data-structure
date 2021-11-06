use std::time::{Duration, Instant};

use cds::{map::ConcurrentMap, stack::ConcurrentStack};
use criterion::{black_box, measurement::WallTime, BenchmarkGroup};
use crossbeam_epoch::pin;
use crossbeam_utils::thread;
use rand::{prelude::SliceRandom, thread_rng, Rng};

pub fn get_test_thread_nums() -> Vec<usize> {
    let mut nums = Vec::new();
    let logical_cores = num_cpus::get();

    let mut num = 1;

    while num <= logical_cores {
        nums.push(num);

        if num <= 16 {
            num *= 2;
        } else {
            num += 16;
        }
    }

    if *nums.last().unwrap() != logical_cores {
        nums.push(logical_cores);
    }

    nums
}

pub fn bench_mixed_concurrent_stack<S>(
    name: &str,
    push: usize,
    pop: usize,
    thread_num: usize,
    c: &mut BenchmarkGroup<WallTime>,
) where
    S: Sync + ConcurrentStack<u64>,
{
    let per_ops = push + pop;

    c.bench_function(
        &format!(
            "{} Ops (push: {}%, pop: {}%, per: {:+e}) by {} threads",
            name,
            push * 100 / per_ops,
            pop * 100 / per_ops,
            per_ops,
            thread_num,
        ),
        |b| {
            b.iter_custom(|iters| {
                let stack = S::new();

                let mut duration = Duration::ZERO;
                for _ in 0..iters {
                    let batched_time = thread::scope(|s| {
                        let mut threads = Vec::new();

                        for _ in 0..thread_num {
                            let t = s.spawn(|_| {
                                let mut rng = thread_rng();
                                let mut duration = Duration::ZERO;

                                for _ in 0..per_ops {
                                    let op_idx = rng.gen_range(0..per_ops);

                                    if op_idx < push {
                                        let value: u64 = rng.gen();

                                        let start = Instant::now();
                                        let _ = black_box(stack.push(value, &pin()));
                                        duration += start.elapsed();
                                    } else {
                                        let start = Instant::now();
                                        let _ = black_box(stack.pop(&pin()));
                                        duration += start.elapsed();
                                    }
                                }

                                duration
                            });

                            threads.push(t);
                        }

                        threads
                            .into_iter()
                            .map(|h| h.join().unwrap())
                            .collect::<Vec<_>>()
                            .iter()
                            .sum::<Duration>()
                    })
                    .unwrap();

                    duration += batched_time
                }

                // avg thread time
                duration / (thread_num as u32)
            });
        },
    );
}

pub fn bench_mixed_concurrent_map<M>(
    name: &str,
    already_inserted: u64,
    insert: usize,
    lookup: usize,
    remove: usize,
    thread_num: usize,
    c: &mut BenchmarkGroup<WallTime>,
) where
    M: Sync + ConcurrentMap<u64, u64>,
{
    let per_ops = insert + lookup + remove;

    c.bench_function(
        &format!(
            "Inserted {:+e} {} Ops (I: {}%, L: {}%, R: {}%, per: {:+e}) by {} threads",
            already_inserted,
            name,
            insert * 100 / per_ops,
            lookup * 100 / per_ops,
            remove * 100 / per_ops,
            per_ops,
            thread_num,
        ),
        |b| {
            b.iter_custom(|iters| {
                let map = M::new();
                let mut rng = thread_rng();

                let mut range: Vec<u64> = (0..already_inserted).collect();
                range.shuffle(&mut rng);

                // pre-insert
                for i in range {
                    let _ = map.insert(&i, i, &pin());
                }

                let mut duration = Duration::ZERO;
                for _ in 0..iters {
                    let batched_time = thread::scope(|s| {
                        let mut threads = Vec::new();

                        for _ in 0..thread_num {
                            let t = s.spawn(|_| {
                                let mut rng = thread_rng();
                                let mut duration = Duration::ZERO;

                                for _ in 0..per_ops {
                                    let op_idx = rng.gen_range(0..per_ops);

                                    if op_idx < insert {
                                        let key: u64 = rng.gen_range(already_inserted..u64::MAX);

                                        let start = Instant::now();
                                        let _ = black_box(map.insert(&key, key, &pin()));
                                        duration += start.elapsed();
                                    } else if op_idx < insert + lookup {
                                        let key: u64 = rng.gen_range(0..already_inserted);

                                        let start = Instant::now();
                                        let _ = black_box(map.get(&key, &pin()));
                                        duration += start.elapsed();
                                    } else {
                                        let key: u64 = rng.gen_range(0..already_inserted);

                                        let start = Instant::now();
                                        let _ = black_box(map.remove(&key, &pin()));
                                        duration += start.elapsed();
                                    }
                                }

                                duration
                            });

                            threads.push(t);
                        }

                        threads
                            .into_iter()
                            .map(|h| h.join().unwrap())
                            .collect::<Vec<_>>()
                            .iter()
                            .sum::<Duration>()
                    })
                    .unwrap();

                    duration += batched_time
                }

                // avg thread time
                duration / (thread_num as u32)
            });
        },
    );
}
