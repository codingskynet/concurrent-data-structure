use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use cds::{map::SequentialMap, queue::SequentialQueue, util::random::Random};
use criterion::{black_box, measurement::WallTime, BenchmarkGroup};
use rand::{prelude::SliceRandom, thread_rng, Rng};

#[derive(Clone, Copy)]
pub enum Op<K, V> {
    Insert(K, V),
    Lookup(K),
    Remove(K),
}

type Logs<K, V> = Vec<(Vec<(K, V)>, Vec<Op<K, V>>)>;

pub fn fuzz_sequential_logs<K: Ord + Clone + Random, V: Clone + Random>(
    iters: u64,
    already_inserted: u64,
    insert: usize,
    lookup: usize,
    remove: usize,
) -> Logs<K, V> {
    let mut rng = thread_rng();
    let mut result = Vec::new();

    for _ in 0..iters {
        let mut logs = Vec::new();

        let mut pre_inserted = Vec::new();

        for _ in 0..already_inserted {
            pre_inserted.push((K::gen(&mut rng), V::gen(&mut rng)));
        }

        for _ in 0..insert {
            logs.push(Op::Insert(K::gen(&mut rng), V::gen(&mut rng)));
        }

        for i in 0..lookup {
            if i % 2 == 0 {
                logs.push(Op::Lookup(K::gen(&mut rng)));
            } else {
                logs.push(Op::Lookup(
                    pre_inserted.choose(&mut rng).cloned().unwrap().0,
                ));
            }
        }

        for i in 0..remove {
            if i % 2 == 0 {
                logs.push(Op::Remove(K::gen(&mut rng)));
            } else {
                logs.push(Op::Remove(
                    pre_inserted.choose(&mut rng).cloned().unwrap().0,
                ));
            }
        }

        logs.shuffle(&mut rng);
        result.push((pre_inserted, logs));
    }

    result
}

pub fn bench_mixed_sequential_queue<S>(push: usize, pop: usize, c: &mut BenchmarkGroup<WallTime>)
where
    S: SequentialQueue<u64>,
{
    let per_ops = push + pop;

    c.bench_function("sequential", |b| {
        b.iter_custom(|iters| {
            let mut queue = S::new();

            let mut duration = Duration::ZERO;

            for _ in 0..iters {
                let mut rng = thread_rng();

                let op_idx = rng.gen_range(0..per_ops);

                if op_idx < per_ops {
                    let value: u64 = rng.gen();

                    let start = Instant::now();
                    let _ = black_box(queue.push(value));
                    duration += start.elapsed();
                } else {
                    let start = Instant::now();
                    let _ = black_box(queue.pop());
                    duration += start.elapsed();
                }
            }

            duration
        });
    });
}

pub fn bench_logs_btreemap<K: Ord, V>(mut logs: Logs<K, V>, c: &mut BenchmarkGroup<WallTime>) {
    c.bench_function("std::BTreeMap", |b| {
        b.iter_custom(|iters| {
            let mut duration = Duration::ZERO;

            for _ in 0..iters {
                let (pre_inserted, logs) = logs.pop().unwrap();
                let mut map = BTreeMap::new();

                // pre-insert
                for (key, value) in pre_inserted {
                    let _ = map.insert(key, value);
                }

                let start = Instant::now();
                for op in logs {
                    match op {
                        Op::Insert(key, value) => {
                            let _ = black_box(map.insert(key, value));
                        }
                        Op::Lookup(key) => {
                            let _ = black_box(map.get(&key));
                        }
                        Op::Remove(key) => {
                            let _ = black_box(map.remove(&key));
                        }
                    }
                }
                duration += start.elapsed();
            }

            duration
        });
    });
}

pub fn bench_logs_sequential_map<K, V, M>(
    name: &str,
    mut logs: Logs<K, V>,
    c: &mut BenchmarkGroup<WallTime>,
) where
    K: Eq + Random,
    M: SequentialMap<K, V>,
{
    c.bench_function(name, |b| {
        b.iter_custom(|iters| {
            let mut duration = Duration::ZERO;

            for _ in 0..iters {
                let (pre_inserted, logs) = logs.pop().unwrap();
                let mut map = M::new();

                // pre-insert
                for (key, value) in pre_inserted {
                    let _ = map.insert(&key, value);
                }

                let start = Instant::now();
                for op in logs {
                    match op {
                        Op::Insert(key, value) => {
                            let _ = black_box(map.insert(&key, value));
                        }
                        Op::Lookup(key) => {
                            let _ = black_box(map.lookup(&key));
                        }
                        Op::Remove(key) => {
                            let _ = black_box(map.remove(&key));
                        }
                    }
                }
                duration += start.elapsed();
            }

            duration
        });
    });
}
