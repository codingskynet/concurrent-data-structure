use criterion::criterion_main;

mod util;

use cds::{avltree::AVLTree, btree::BTree};
use criterion::{criterion_group, Criterion};
use util::sequential::{bench_mixed_sequential_map, bench_sequential_map};

const MAP_ALREADY_INSERTED: u64 = 500_000;
const MAP_TOTAL_OPS: usize = 500_000;
const MAP_INSERT_RATE: usize = 30;
const MAP_LOOKUP_RATE: usize = 50;
const MAP_REMOVE_RATE: usize = 20;

fn bench_btreemap(c: &mut Criterion) {
    util::sequential::bench_btreemap(1_000_000, c);
}

fn bench_mixed_btreemap(c: &mut Criterion) {
    assert_eq!(MAP_INSERT_RATE + MAP_LOOKUP_RATE + MAP_REMOVE_RATE, 100);

    util::sequential::bench_mixed_btreemap(
        MAP_ALREADY_INSERTED,
        MAP_TOTAL_OPS * MAP_INSERT_RATE / 100,
        MAP_TOTAL_OPS * MAP_LOOKUP_RATE / 100,
        MAP_TOTAL_OPS * MAP_REMOVE_RATE / 100,
        c,
    );
}

fn bench_avltree(c: &mut Criterion) {
    bench_sequential_map::<AVLTree<_, _>>("AVLTree", 1_000_000, c);
}

fn bench_mixed_avltree(c: &mut Criterion) {
    bench_mixed_sequential_map::<AVLTree<_, _>>(
        "AVLTree",
        MAP_ALREADY_INSERTED,
        MAP_TOTAL_OPS * MAP_INSERT_RATE / 100,
        MAP_TOTAL_OPS * MAP_LOOKUP_RATE / 100,
        MAP_TOTAL_OPS * MAP_REMOVE_RATE / 100,
        c,
    );
}

fn bench_btree(c: &mut Criterion) {
    bench_sequential_map::<BTree<_, _>>("BTree", 1_000_000, c);
}

fn bench_mixed_btree(c: &mut Criterion) {
    bench_mixed_sequential_map::<BTree<_, _>>(
        "BTree",
        MAP_ALREADY_INSERTED,
        MAP_TOTAL_OPS * MAP_INSERT_RATE / 100,
        MAP_TOTAL_OPS * MAP_LOOKUP_RATE / 100,
        MAP_TOTAL_OPS * MAP_REMOVE_RATE / 100,
        c,
    );
}

criterion_group!(
    bench,
    bench_btreemap,
    bench_mixed_btreemap,
    bench_avltree,
    bench_mixed_avltree,
    bench_btree,
    bench_mixed_btree,
);
criterion_main! {
    bench,
}
