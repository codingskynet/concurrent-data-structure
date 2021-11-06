use criterion::criterion_main;

mod util;

use cds::{avltree::AVLTree, btree::BTree};
use criterion::{criterion_group, Criterion};
use util::sequential::{bench_mixed_sequential, bench_sequential, bench_mixed_sequential_reference, bench_sequential_reference};

const ALREADY_INSERTED: u64 = 1_000_000;
const TOTAL_OPS: usize = 1_000_000;
const INSERT_RATE: usize = 30;
const LOOKUP_RATE: usize = 50;
const REMOVE_RATE: usize = 20;

fn bench_reference_tree(c: &mut Criterion) {
    bench_sequential_reference(1_000_000, c);
}

fn bench_mixed_reference_tree(c: &mut Criterion) {
    assert_eq!(INSERT_RATE + LOOKUP_RATE + REMOVE_RATE, 100);

    bench_mixed_sequential_reference(
        ALREADY_INSERTED,
        TOTAL_OPS * INSERT_RATE / 100,
        TOTAL_OPS * LOOKUP_RATE / 100,
        TOTAL_OPS * REMOVE_RATE / 100,
        c,
    );
}

fn bench_avltree(c: &mut Criterion) {
    bench_sequential::<AVLTree<_, _>>("AVLTree", 1_000_000, c);
}

fn bench_mixed_avltree(c: &mut Criterion) {
    bench_mixed_sequential::<AVLTree<_, _>>(
        "AVLTree",
        ALREADY_INSERTED,
        TOTAL_OPS * INSERT_RATE / 100,
        TOTAL_OPS * LOOKUP_RATE / 100,
        TOTAL_OPS * REMOVE_RATE / 100,
        c,
    );
}

fn bench_btree(c: &mut Criterion) {
    bench_sequential::<BTree<_, _>>("BTree", 1_000_000, c);
}

fn bench_mixed_btree(c: &mut Criterion) {
    bench_mixed_sequential::<BTree<_, _>>(
        "BTree",
        ALREADY_INSERTED,
        TOTAL_OPS * INSERT_RATE / 100,
        TOTAL_OPS * LOOKUP_RATE / 100,
        TOTAL_OPS * REMOVE_RATE / 100,
        c,
    );
}

criterion_group!(
    bench,
    bench_reference_tree,
    bench_mixed_reference_tree,
    bench_avltree,
    bench_mixed_avltree,
    bench_btree,
    bench_mixed_btree,
);
criterion_main! {
    bench,
}
