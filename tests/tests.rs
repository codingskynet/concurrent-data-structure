use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use rand::{prelude::SliceRandom, thread_rng, Rng};

mod avltree;
mod btree;
mod linkedlist;
mod queue;
mod stack;
mod util;

#[test]
fn bench_reference_tree() {
    let already_inserted = 1_000_000;
    let iter = 100; // should iter << already_inserted

    let mut rng = thread_rng();

    let mut reference: BTreeMap<u64, u64> = BTreeMap::new();

    // pre-insertion
    let mut range: Vec<u64> = (0..already_inserted).collect();
    range.shuffle(&mut rng);

    for i in range {
        reference.insert(i, i);
    }

    let mut time = Duration::ZERO;
    for _ in 0..iter {
        let key: u64 = rng.gen_range(already_inserted..u64::MAX);

        let start = Instant::now();
        let _ = reference.insert(key, key);
        time += start.elapsed();
    }
    println!(
        "{} Inserted std::BTreemap {} Insert: avg {:>7.3} ns/op",
        already_inserted,
        iter,
        time.as_nanos() as f64 / iter as f64
    );

    let mut time = Duration::ZERO;
    for _ in 0..iter {
        let key: u64 = rng.gen_range(0..already_inserted);
        let start = Instant::now();
        let _ = reference.get(&key);
        time += start.elapsed();
    }
    println!(
        "{} Inserted std::BTreemap {} Lookup: avg {:>7.3} ns/op",
        already_inserted,
        iter,
        time.as_nanos() as f64 / iter as f64
    );

    let mut time = Duration::ZERO;
    for _ in 0..iter {
        let key: u64 = rng.gen_range(0..already_inserted);
        let start = Instant::now();
        let _ = reference.remove(&key);
        time += start.elapsed();
    }
    println!(
        "{} Inserted std::BTreemap {} Remove: avg {:>7.3} ns/op",
        already_inserted,
        iter,
        time.as_nanos() as f64 / iter as f64
    );
}
