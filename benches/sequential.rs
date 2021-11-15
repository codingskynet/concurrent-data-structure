use std::time::Duration;

use criterion::{criterion_main, SamplingMode, Throughput};

mod util;

use cds::{avltree::AVLTree, btree::BTree};
use criterion::{criterion_group, Criterion};
use util::sequential::fuzz_sequential_logs;

use crate::util::sequential::{bench_logs_btreemap, bench_logs_sequential_map};

const MAP_ALREADY_INSERTED: u64 = 500_000;
const MAP_TOTAL_OPS: usize = 192_000;

const OPS_RATE: [(usize, usize, usize); 7] = [
    (100, 0, 0),
    (0, 100, 0),
    (0, 0, 100),
    (5, 90, 5),
    (30, 50, 20),
    (40, 20, 40),
    (50, 0, 50),
];

fn bench_vs_btreemap(c: &mut Criterion) {
    for (insert, lookup, remove) in OPS_RATE {
        let logs = fuzz_sequential_logs(
            200,
            MAP_ALREADY_INSERTED,
            MAP_TOTAL_OPS * insert / 100,
            MAP_TOTAL_OPS * lookup / 100,
            MAP_TOTAL_OPS * remove / 100,
        );

        let mut group = c.benchmark_group(format!(
            "Inserted {:+e}, Ops (I: {}%, L: {}%, R: {}%, total: {:+e})",
            MAP_ALREADY_INSERTED, insert, lookup, remove, MAP_TOTAL_OPS
        ));
        group.measurement_time(Duration::from_secs(15)); // Note: make almost same the measurement_time to iters * avg_op_time
        group.sampling_mode(SamplingMode::Flat);
        group.sample_size(20);
        group.throughput(Throughput::Elements(MAP_TOTAL_OPS as u64));

        bench_logs_btreemap(logs.clone(), &mut group);
        bench_logs_sequential_map::<BTree<_, _>>("BTree", logs.clone(), &mut group);
        bench_logs_sequential_map::<AVLTree<_, _>>("AVLTree", logs, &mut group);
    }
}

criterion_group!(bench, bench_vs_btreemap);
criterion_main! {
    bench,
}
