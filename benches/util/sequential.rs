use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use cds::map::SequentialMap;
use criterion::{black_box, measurement::WallTime, BenchmarkGroup, Criterion};
use rand::{prelude::SliceRandom, thread_rng, Rng};

#[derive(Clone, Copy)]
pub enum Op {
    Insert(u64),
    Lookup(u64),
    Remove(u64),
}

pub fn fuzz_sequential_logs(
    iters: u64,
    already_inserted: u64,
    insert: usize,
    lookup: usize,
    remove: usize,
) -> Vec<(Vec<u64>, Vec<Op>)> {
    let mut rng = thread_rng();
    let mut result = Vec::new();

    for _ in 0..iters {
        let mut logs = Vec::new();

        let mut pre_inserted: Vec<u64> = (0..already_inserted).collect();
        pre_inserted.shuffle(&mut rng);

        for _ in 0..insert {
            logs.push(Op::Insert(rng.gen_range(already_inserted..u64::MAX)));
        }

        for _ in 0..lookup {
            logs.push(Op::Lookup(rng.gen_range(0..already_inserted)));
        }

        for _ in 0..remove {
            logs.push(Op::Remove(rng.gen_range(0..already_inserted)));
        }

        logs.shuffle(&mut rng);
        result.push((pre_inserted, logs));
    }

    result
}

pub fn bench_btreemap(already_inserted: u64, c: &mut Criterion) {
    c.bench_function(
        &format!(
            "Inserted {:+e} std::BTreeMap Insert (batch: 100)",
            already_inserted
        ),
        |b| {
            b.iter_custom(|iters| {
                let mut map = BTreeMap::new();
                let mut rng = thread_rng();

                let mut range: Vec<u64> = (0..already_inserted).collect();
                range.shuffle(&mut rng);

                for i in range.clone() {
                    let _ = map.insert(i, i.clone());
                }

                let mut duration: Duration = Duration::ZERO;

                for _ in 0..iters {
                    let mut keys = Vec::new();

                    for _ in 0..100 {
                        let mut key: u64 = rng.gen();

                        loop {
                            if !range.contains(&key) {
                                break;
                            }

                            key = rng.gen();
                        }

                        keys.push(key.clone());

                        let start = Instant::now();
                        let _ = black_box(map.insert(key, key));
                        duration += start.elapsed();
                    }

                    for key in &keys {
                        map.remove(&key).expect("Error on removing inserted keys");
                    }
                }

                duration / 100
            });
        },
    );

    c.bench_function(
        &format!("Inserted {:+e} std::BTreeMap Lookup", already_inserted),
        |b| {
            b.iter_custom(|iters| {
                let mut map = BTreeMap::new();
                let mut rng = thread_rng();

                let mut range: Vec<u64> = (0..already_inserted).collect();
                range.shuffle(&mut rng);

                for i in range {
                    let _ = map.insert(i, i);
                }

                let mut duration = Duration::ZERO;
                for _ in 0..iters {
                    let key: u64 = rng.gen_range(0..already_inserted);

                    let start = Instant::now();
                    let _ = black_box(map.get(&key));
                    duration += start.elapsed();
                }
                duration
            });
        },
    );

    c.bench_function(
        &format!(
            "Inserted {:+e} std::BTreeMap Remove (batch: 100)",
            already_inserted
        ),
        |b| {
            b.iter_custom(|iters| {
                let mut map = BTreeMap::new();
                let mut rng = thread_rng();

                let mut range: Vec<u64> = (0..already_inserted).collect();
                range.shuffle(&mut rng);

                for i in range.clone() {
                    let _ = map.insert(i, i.clone());
                }

                let mut duration = Duration::ZERO;
                for _ in 0..iters {
                    let keys: Vec<&u64> = range.choose_multiple(&mut rng, 100).collect();

                    for key in &keys {
                        let start = Instant::now();
                        let _ = black_box(map.remove(key));
                        duration += start.elapsed();
                    }

                    for key in keys {
                        let key = key.clone();
                        assert_eq!(map.insert(key, key.clone()), None);
                    }
                }
                duration / 100
            });
        },
    );
}

pub fn bench_logs_btreemap(mut logs: Vec<(Vec<u64>, Vec<Op>)>, c: &mut BenchmarkGroup<WallTime>) {
    c.bench_function("std::BTreeMap", |b| {
        b.iter_custom(|iters| {
            let mut duration = Duration::ZERO;

            for _ in 0..iters {
                let (pre_inserted, logs) = logs.pop().unwrap();
                let mut map = BTreeMap::new();

                // pre-insert
                for key in pre_inserted {
                    let _ = map.insert(key, key);
                }

                let start = Instant::now();
                for op in logs {
                    match op {
                        Op::Insert(key) => {
                            let _ = black_box(map.insert(key, key));
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

pub fn bench_sequential_map<M>(name: &str, already_inserted: u64, c: &mut Criterion)
where
    M: SequentialMap<u64, u64>,
{
    c.bench_function(
        &format!(
            "Inserted {:+e} {} Insert (batch: 100)",
            already_inserted, name
        ),
        |b| {
            b.iter_custom(|iters| {
                let mut map = M::new();
                let mut rng = thread_rng();

                let mut range: Vec<u64> = (0..already_inserted).collect();
                range.shuffle(&mut rng);

                for i in &range {
                    let _ = map.insert(&i, i.clone());
                }

                let mut duration: Duration = Duration::ZERO;

                for _ in 0..iters {
                    let mut keys = Vec::new();

                    for _ in 0..100 {
                        let mut key: u64 = rng.gen();

                        loop {
                            if !range.contains(&key) {
                                break;
                            }

                            key = rng.gen();
                        }

                        keys.push(key);

                        let start = Instant::now();
                        let _ = black_box(map.insert(&key, key));
                        duration += start.elapsed();
                    }

                    for key in &keys {
                        map.remove(key).expect("Error on removing inserted keys");
                    }
                }

                duration / 100
            });
        },
    );

    c.bench_function(
        &format!("Inserted {:+e} {} Lookup", already_inserted, name),
        |b| {
            b.iter_custom(|iters| {
                let mut map = M::new();
                let mut rng = thread_rng();

                let mut range: Vec<u64> = (0..already_inserted).collect();
                range.shuffle(&mut rng);

                for i in range {
                    let _ = map.insert(&i, i);
                }

                let mut duration = Duration::ZERO;
                for _ in 0..iters {
                    let key: u64 = rng.gen_range(0..already_inserted);

                    let start = Instant::now();
                    let _ = black_box(map.lookup(&key));
                    duration += start.elapsed();
                }
                duration
            });
        },
    );

    c.bench_function(
        &format!(
            "Inserted {:+e} {} Remove (batch: 100)",
            already_inserted, name
        ),
        |b| {
            b.iter_custom(|iters| {
                let mut map = M::new();
                let mut rng = thread_rng();

                let mut range: Vec<u64> = (0..already_inserted).collect();
                range.shuffle(&mut rng);

                for i in &range {
                    let _ = map.insert(&i, i.clone());
                }

                let mut duration = Duration::ZERO;
                for _ in 0..iters {
                    let keys: Vec<&u64> = range.choose_multiple(&mut rng, 100).collect();

                    for key in &keys {
                        let start = Instant::now();
                        let _ = black_box(map.remove(key));
                        duration += start.elapsed();
                    }

                    for key in keys {
                        assert_eq!(map.insert(key, key.clone()), Ok(()));
                    }
                }
                duration / 100
            });
        },
    );
}

pub fn bench_logs_sequential_map<M>(
    name: &str,
    mut logs: Vec<(Vec<u64>, Vec<Op>)>,
    c: &mut BenchmarkGroup<WallTime>,
) where
    M: SequentialMap<u64, u64>,
{
    c.bench_function(name, |b| {
        b.iter_custom(|iters| {
            let mut duration = Duration::ZERO;

            for _ in 0..iters {
                let (pre_inserted, logs) = logs.pop().unwrap();
                let mut map = M::new();

                // pre-insert
                for key in pre_inserted {
                    let _ = map.insert(&key, key);
                }

                let start = Instant::now();
                for op in logs {
                    match op {
                        Op::Insert(key) => {
                            let _ = black_box(map.insert(&key, key));
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
