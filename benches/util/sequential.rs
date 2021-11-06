use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use cds::map::SequentialMap;
use criterion::{black_box, Criterion};
use rand::{prelude::SliceRandom, thread_rng, Rng};

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

pub fn bench_mixed_btreemap(
    already_inserted: u64,
    insert: usize,
    lookup: usize,
    remove: usize,
    c: &mut Criterion,
) {
    let total_ops = insert + lookup + remove;

    c.bench_function(
        &format!(
            "Inserted {:+e} std::BTreeMap Ops (I: {}%, L: {}%, R: {}%, total: {:+e})",
            already_inserted,
            insert * 100 / total_ops,
            lookup * 100 / total_ops,
            remove * 100 / total_ops,
            total_ops,
        ),
        |b| {
            b.iter_custom(|iters| {
                let total_ops = insert + lookup + remove;

                let mut map = BTreeMap::new();
                let mut rng = thread_rng();

                let mut range: Vec<u64> = (0..already_inserted).collect();
                range.shuffle(&mut rng);

                // pre-insert
                for i in range {
                    let _ = map.insert(i, i);
                }

                let mut duration = Duration::ZERO;
                for _ in 0..iters {
                    let mut rng = thread_rng();

                    for _ in 0..total_ops {
                        let op_idx = rng.gen_range(0..total_ops);

                        if op_idx < insert {
                            let key: u64 = rng.gen_range(already_inserted..u64::MAX);

                            let start = Instant::now();
                            let _ = black_box(map.insert(key, key));
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
                }

                duration
            });
        },
    );
}

pub fn bench_sequential_map<M>(name: &str, already_inserted: u64, c: &mut Criterion)
where
    M: SequentialMap<u64, u64>,
{
    c.bench_function(
        &format!("Inserted {:+e} {} Insert (batch: 100)", already_inserted, name),
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
        &format!("Inserted {:+e} {} Remove (batch: 100)", already_inserted, name),
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

pub fn bench_mixed_sequential_map<M>(
    name: &str,
    already_inserted: u64,
    insert: usize,
    lookup: usize,
    remove: usize,
    c: &mut Criterion,
) where
    M: SequentialMap<u64, u64>,
{
    let total_ops = insert + lookup + remove;

    c.bench_function(
        &format!(
            "Inserted {:+e} {} Ops (I: {}%, L: {}%, R: {}%, total: {:+e})",
            already_inserted,
            name,
            insert * 100 / total_ops,
            lookup * 100 / total_ops,
            remove * 100 / total_ops,
            total_ops,
        ),
        |b| {
            b.iter_custom(|iters| {
                let total_ops = insert + lookup + remove;

                let mut map = M::new();
                let mut rng = thread_rng();

                let mut range: Vec<u64> = (0..already_inserted).collect();
                range.shuffle(&mut rng);

                // pre-insert
                for i in range {
                    let _ = map.insert(&i, i);
                }

                let mut duration = Duration::ZERO;
                for _ in 0..iters {
                    let mut rng = thread_rng();

                    for _ in 0..total_ops {
                        let op_idx = rng.gen_range(0..total_ops);

                        if op_idx < insert {
                            let key: u64 = rng.gen_range(already_inserted..u64::MAX);

                            let start = Instant::now();
                            let _ = black_box(map.insert(&key, key));
                            duration += start.elapsed();
                        } else if op_idx < insert + lookup {
                            let key: u64 = rng.gen_range(0..already_inserted);

                            let start = Instant::now();
                            let _ = black_box(map.lookup(&key));
                            duration += start.elapsed();
                        } else {
                            let key: u64 = rng.gen_range(0..already_inserted);

                            let start = Instant::now();
                            let _ = black_box(map.remove(&key));
                            duration += start.elapsed();
                        }
                    }
                }

                duration
            });
        },
    );
}
