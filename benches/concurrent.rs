use criterion::{Throughput, criterion_main};

mod util;

use std::time::Duration;

use cds::{
    avltree::SeqLockAVLTree,
    stack::{EBStack, TreiberStack},
};
use criterion::{criterion_group, Criterion};

use crate::util::concurrent::{
    bench_mixed_concurrent_map, bench_mixed_concurrent_stack, get_test_thread_nums,
};

const STACK_PER_OPS: usize = 1_000_000;
const STACK_PUSH_RATE: usize = 50;
const STACK_POP_RATE: usize = 50;

fn bench_mixed_treiberstack(c: &mut Criterion) {
    assert_eq!(STACK_PUSH_RATE + STACK_POP_RATE, 100);

    let mut group = c.benchmark_group("TreiberStack");
    group.measurement_time(Duration::from_secs(20));

    for num in get_test_thread_nums() {
        group.throughput(Throughput::Elements((STACK_PER_OPS * num) as u64));
        bench_mixed_concurrent_stack::<TreiberStack<_>>(
            "TreiberStack",
            STACK_PER_OPS * STACK_PUSH_RATE / 100,
            STACK_PER_OPS * STACK_POP_RATE / 100,
            num,
            &mut group,
        );
    }
}

fn bench_mixed_ebstack(c: &mut Criterion) {
    let mut group = c.benchmark_group("EBStack");
    group.measurement_time(Duration::from_secs(20));

    for num in get_test_thread_nums() {
        group.throughput(Throughput::Elements((STACK_PER_OPS * num) as u64));
        bench_mixed_concurrent_stack::<EBStack<_>>(
            "EBStack",
            STACK_PER_OPS * STACK_PUSH_RATE / 100,
            STACK_PER_OPS * STACK_POP_RATE / 100,
            num,
            &mut group,
        );
    }
}

const MAP_ALREADY_INSERTED: u64 = 500_000;
const MAP_PER_OPS: usize = 500_000;
const MAP_INSERT_RATE: usize = 30;
const MAP_LOOKUP_RATE: usize = 50;
const MAP_REMOVE_RATE: usize = 20;

fn bench_mixed_seqlockavltree(c: &mut Criterion) {
    assert_eq!(MAP_INSERT_RATE + MAP_LOOKUP_RATE + MAP_REMOVE_RATE, 100);

    let mut group = c.benchmark_group("SeqLockAVLTree");

    for num in get_test_thread_nums() {
        group.throughput(Throughput::Elements((MAP_PER_OPS * num) as u64));
        bench_mixed_concurrent_map::<SeqLockAVLTree<_, _>>(
            "SeqlockAVLTree",
            MAP_ALREADY_INSERTED,
            MAP_PER_OPS * MAP_INSERT_RATE / 100,
            MAP_PER_OPS * MAP_LOOKUP_RATE / 100,
            MAP_PER_OPS * MAP_REMOVE_RATE / 100,
            num,
            &mut group,
        )
    }

    group.finish();
}

criterion_group!(
    name = bench;
    config = Criterion::default().measurement_time(Duration::from_secs(60)).sample_size(10);
    targets = bench_mixed_treiberstack, bench_mixed_ebstack, bench_mixed_seqlockavltree
);
criterion_main! {
    bench,
}
