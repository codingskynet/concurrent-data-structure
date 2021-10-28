use cds::{avltree::AVLTree, btree::BTree};
use criterion::{Criterion, criterion_group};

use crate::util::map::{bench_sequential, bench_sequential_reference};

fn bench_reference_tree(c: &mut Criterion) {
    bench_sequential_reference(100_000, c);
}

fn bench_avltree(c: &mut Criterion) {
    bench_sequential::<AVLTree<_, _>>("AVLTree", 100_000, c);
}

fn bench_btree(c: &mut Criterion) {
    bench_sequential::<BTree<_, _>>("BTree", 100_000, c);
}

criterion_group!(bench, bench_reference_tree, bench_avltree, bench_btree);
