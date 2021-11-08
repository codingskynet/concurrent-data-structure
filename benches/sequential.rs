use std::time::Duration;

use criterion::{criterion_main, SamplingMode};

mod util;

use cds::{avltree::AVLTree, btree::BTree};
use criterion::{criterion_group, Criterion};
use util::sequential::{bench_sequential_map, fuzz_sequential_logs};

use crate::util::sequential::{bench_logs_btreemap, bench_logs_sequential_map};

const MAP_ALREADY_INSERTED: u64 = 500_000;
const MAP_TOTAL_OPS: usize = 500_000;

fn bench_btreemap(c: &mut Criterion) {
    util::sequential::bench_btreemap(1_000_000, c);
}

fn bench_avltree(c: &mut Criterion) {
    bench_sequential_map::<AVLTree<_, _>>("AVLTree", 1_000_000, c);
}

fn bench_btree(c: &mut Criterion) {
    bench_sequential_map::<BTree<_, _>>("BTree", 1_000_000, c);
}

fn bench_btree_vs_btreemap(c: &mut Criterion) {
    let ops_rate = [(10, 80, 10), (20, 40, 20), (30, 50, 20), (40, 20, 40)];

    for (insert, lookup, remove) in ops_rate {
        println!("Creating logs...");
        let logs = fuzz_sequential_logs(
            200,
            MAP_ALREADY_INSERTED,
            MAP_TOTAL_OPS * insert / 100,
            MAP_TOTAL_OPS * lookup / 100,
            MAP_TOTAL_OPS * remove / 100,
        );

        let mut group = c.benchmark_group(format!(
            "std::BTreeMap vs BTree: Inserted {:+e}, Ops (I: {}%, L: {}%, R: {}%, total: {:+e})",
            MAP_ALREADY_INSERTED, insert, lookup, remove, MAP_TOTAL_OPS
        ));
        group.measurement_time(Duration::from_secs(15)); // Note: make almost same the measurement_time to iters * avg_op_time
        group.sampling_mode(SamplingMode::Flat);
        group.sample_size(20);

        bench_logs_btreemap(logs.clone(), &mut group);
        bench_logs_sequential_map::<BTree<_, _>>("BTree", logs, &mut group);
    }
}

criterion_group!(
    bench,
    bench_btreemap,
    bench_avltree,
    bench_btree,
    bench_btree_vs_btreemap
);
criterion_main! {
    bench,
}
