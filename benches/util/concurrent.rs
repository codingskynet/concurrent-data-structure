use std::time::{Duration, Instant};

use cds::{map::ConcurrentMap, stack::ConcurrentStack};
use criterion::{black_box, measurement::WallTime, BenchmarkGroup};
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
    push: usize,
    pop: usize,
    thread_num: usize,
    c: &mut BenchmarkGroup<WallTime>,
) where
    S: Sync + ConcurrentStack<u64>,
{
    let per_ops = push + pop;

    c.bench_function(&format!("{} threads", thread_num,), |b| {
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
                                    let _ = black_box(stack.push(value));
                                    duration += start.elapsed();
                                } else {
                                    let start = Instant::now();
                                    let _ = black_box(stack.pop());
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
    });
}

pub fn criterion_flat_bench_mixed_concurrent_map<M>(
    already_inserted: u64,
    insert: u64,
    lookup: u64,
    remove: u64,
    thread_num: usize,
    c: &mut BenchmarkGroup<WallTime>,
) where
    M: Sync + ConcurrentMap<u64, u64>,
{
    c.bench_function(&format!("{} threads", thread_num,), |b| {
        b.iter_custom(|iters| {
            let mut duraction = Duration::ZERO;

            for _ in 0..iters {
                duraction += bench_mixed_concurrent_map::<M>(
                    already_inserted,
                    insert,
                    lookup,
                    remove,
                    thread_num,
                );
            }

            duraction
        })
    });
}

pub fn criterion_linear_bench_mixed_concurrent_map<M>(
    already_inserted: u64,
    insert: u64,
    lookup: u64,
    remove: u64,
    thread_num: usize,
    c: &mut BenchmarkGroup<WallTime>,
) where
    M: Sync + ConcurrentMap<u64, u64>,
{
    c.bench_function(&format!("{} threads", thread_num,), |b| {
        b.iter_custom(|iters| {
            bench_mixed_concurrent_map::<M>(
                already_inserted,
                insert * iters,
                lookup * iters,
                remove * iters,
                thread_num,
            )
        })
    });
}

pub fn bench_mixed_concurrent_map<M>(
    already_inserted: u64,
    insert: u64,
    lookup: u64,
    remove: u64,
    thread_num: usize,
) -> Duration
where
    M: Sync + ConcurrentMap<u64, u64>,
{
    let per_ops = insert + lookup + remove;

    let map = M::new();
    let mut rng = thread_rng();

    let mut range: Vec<u64> = (0..already_inserted).collect();
    range.shuffle(&mut rng);

    // pre-insert
    for i in range {
        let _ = map.insert(&i, i);
    }

    let duration = thread::scope(|s| {
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
                        let _ = black_box(map.insert(&key, key));
                        duration += start.elapsed();
                    } else if op_idx < insert + lookup {
                        let key: u64 = rng.gen_range(0..already_inserted);

                        let start = Instant::now();
                        let _ = black_box(map.get(&key));
                        duration += start.elapsed();
                    } else {
                        let key: u64 = rng.gen_range(0..already_inserted);

                        let start = Instant::now();
                        let _ = black_box(map.remove(&key));
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

    // avg thread time
    duration / (thread_num as u32)
}
