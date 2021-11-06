use criterion::criterion_main;

mod util;

use std::time::Duration;

use cds::avltree::SeqLockAVLTree;
use criterion::{criterion_group, Criterion};

use crate::util::concurrent::{bench_mixed_concurrent, get_test_thread_nums};

const ALREADY_INSERTED: u64 = 1_000_000;
const TOTAL_OPS: usize = 1_000_000;
const INSERT_RATE: usize = 30;
const LOOKUP_RATE: usize = 50;
const REMOVE_RATE: usize = 20;

fn bench_mixed_seqlockavltree(c: &mut Criterion) {
    assert_eq!(INSERT_RATE + LOOKUP_RATE + REMOVE_RATE, 100);

    let mut group = c.benchmark_group("SeqLockAVLTree");

    for num in get_test_thread_nums() {
        bench_mixed_concurrent::<SeqLockAVLTree<_, _>>(
            "SeqlockAVLTree",
            ALREADY_INSERTED,
            TOTAL_OPS * INSERT_RATE / 100,
            TOTAL_OPS * LOOKUP_RATE / 100,
            TOTAL_OPS * REMOVE_RATE / 100,
            num,
            &mut group,
        )
    }

    group.finish();
}

criterion_group!(
    name = bench;
    config = Criterion::default().measurement_time(Duration::from_secs(60)).sample_size(10);
    targets = bench_mixed_seqlockavltree
);
criterion_main! {
    bench,
}
