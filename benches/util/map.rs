use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use cds::map::SequentialMap;
use criterion::{black_box, Criterion};
use rand::{prelude::SliceRandom, thread_rng, Rng};

pub fn bench_sequential_reference(already_inserted: u64, c: &mut Criterion) {
    c.bench_function(
        &format!(
            "{} Inserted std::BTreeMap Insert (batch: 100)",
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
        &format!("{} Inserted std::BTreeMap Lookup", already_inserted),
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
            "{} Inserted std::BTreeMap Remove (batch: 100)",
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

pub fn bench_mixed_sequential_reference(
    already_inserted: u64,
    insert: u64,
    lookup: u64,
    remove: u64,
    c: &mut Criterion,
) {
    c.bench_function(
        &format!(
            "{} Inserted std::BTreeMap Mixed Operations (I: {}, L: {}, R: {})",
            already_inserted, insert, lookup, remove
        ),
        |b| {
            b.iter_custom(|iters| {
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
                    let key: u64 = rng.gen_range(0..already_inserted);

                    let start = Instant::now();
                    let _ = black_box(map.get(&key));
                    duration += start.elapsed();
                }
                duration
            });
        },
    );
}

pub fn bench_sequential<M>(name: &str, already_inserted: u64, c: &mut Criterion)
where
    M: SequentialMap<u64, u64>,
{
    c.bench_function(
        &format!("{} Inserted {} Insert (batch: 100)", already_inserted, name),
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
        &format!("{} Inserted {} Lookup", already_inserted, name),
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
        &format!("{} Inserted {} Remove (batch: 100)", already_inserted, name),
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

/*
// the benchmark function is not suitable since Criterion cannot run efficiently on multithreads
pub fn bench_mixed_concurrent<M>(
    name: &str,
    already_inserted: u64,
    insert: u32,
    lookup: u32,
    remove: u32,
    thread_num: u32,
    c: &mut Criterion,
) where
    M: Sync + ConcurrentMap<u64, u64>,
{
    let total_ops = insert + lookup + remove;

    c.bench_function(
    &format!(
            "{} Inserted {} Mixed Operations (I: {} + L: {} + R: {} = total: {}) splitted by {} threads",
            name, already_inserted, insert, lookup, remove, total_ops, thread_num,
        ),
        |b| {
            b.iter_custom(|iters| {
                let total_ops = insert + lookup + remove;

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

                                for _ in 0..(total_ops / thread_num) {
                                    let op_idx: u32 = rng.gen_range(0..total_ops);

                                    if op_idx < insert {
                                        // insert
                                        let key: u64 = rng.gen_range(already_inserted..u64::MAX);

                                        let start = Instant::now();
                                        let _ = map.insert(&key, key, &pin());
                                        duration += start.elapsed();
                                    } else if op_idx < insert + lookup {
                                        // lookup
                                        let key: u64 = rng.gen_range(0..already_inserted);

                                        let start = Instant::now();
                                        let _ = map.lookup(&key, &pin());
                                        duration += start.elapsed();
                                    } else {
                                        // remove
                                        let key: u64 = rng.gen_range(0..already_inserted);

                                        let start = Instant::now();
                                        let _ = map.remove(&key, &pin());
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
                duration / thread_num
            });
        },
    );
}
*/
