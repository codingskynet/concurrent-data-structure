use std::time::{Duration, Instant};

use cds::map::SequentialMap;
use criterion::{black_box, Criterion};
use rand::{prelude::SliceRandom, thread_rng, Rng};

pub fn bench_sequential<M>(name: &str, already_inserted: u64, c: &mut Criterion)
where
    M: SequentialMap<u64, u64>,
{
    c.bench_function(
        &format!("{} Inserted {} Insert", already_inserted, name),
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
        &format!("{} Inserted {} Remove", already_inserted, name),
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
