use std::time::Duration;

use cds::avltree::SeqLockAVLTree;
use criterion::{criterion_group, Criterion};
use criterion::{criterion_main, SamplingMode, Throughput};

mod util;

use util::concurrent::{
    criterion_flat_bench_mixed_concurrent_map, criterion_linear_bench_mixed_concurrent_map,
    get_test_thread_nums,
};

const MAP_ALREADY_INSERTED: u64 = 500_000;
const MAP_TOTAL_OPS: u64 = 192_000;

const OPS_RATE: [(u64, u64, u64); 7] = [
    (100, 0, 0),
    (0, 100, 0),
    (0, 0, 100),
    (5, 90, 5),
    (30, 50, 20),
    (40, 20, 40),
    (50, 0, 50),
];

// use for micro benchmark
fn bench_mixed_per_seqlockavltree(c: &mut Criterion) {
    for (insert, lookup, remove) in OPS_RATE {
        let mut group = c.benchmark_group(format!(
            "SeqLockAVLTree/{:+e} pre-inserted, Ops(I: {}%, L: {}%, R: {}%, per: scaled by iters)",
            MAP_ALREADY_INSERTED, insert, lookup, remove
        ));
        group.sample_size(20);
        group.measurement_time(Duration::from_secs(10));
        group.sampling_mode(SamplingMode::Linear);

        for num in get_test_thread_nums() {
            group.throughput(Throughput::Elements((100 * num) as u64));
            criterion_linear_bench_mixed_concurrent_map::<SeqLockAVLTree<_, _>>(
                MAP_ALREADY_INSERTED,
                insert,
                lookup,
                remove,
                num,
                &mut group,
            );
        }
        group.finish();
    }
}

// use for macro benchmark
fn bench_mixed_total_seqlockavltree(c: &mut Criterion) {
    for (insert, lookup, remove) in OPS_RATE {
        let mut group = c.benchmark_group(format!(
            "SeqLockAVLTree/{:+e} pre-inserted, Ops(I: {}%, L: {}%, R: {}%, total: {:+e})",
            MAP_ALREADY_INSERTED, insert, lookup, remove, MAP_TOTAL_OPS
        ));
        group.sample_size(20);
        group.measurement_time(Duration::from_secs(15));
        group.sampling_mode(SamplingMode::Flat);

        for num in get_test_thread_nums() {
            group.throughput(Throughput::Elements(MAP_TOTAL_OPS as u64));
            let per_op = MAP_TOTAL_OPS / num as u64;
            criterion_flat_bench_mixed_concurrent_map::<SeqLockAVLTree<_, _>>(
                MAP_ALREADY_INSERTED,
                per_op * insert / 100,
                per_op * lookup / 100,
                per_op * remove / 100,
                num,
                &mut group,
            );
        }

        group.finish();
    }
}

criterion_group!(
    bench,
    bench_mixed_per_seqlockavltree,
    bench_mixed_total_seqlockavltree,
);
criterion_main! {
    bench,
}
