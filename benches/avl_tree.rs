mod util;

use crate::util::map::bench_sequential;
use cds::tree::avl_tree::AVLTree;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_avl_tree(c: &mut Criterion) {
    bench_sequential::<AVLTree<_, _>>("AVL Tree", 65536, c);
}

criterion_group!(avl_tree, bench_avl_tree);
criterion_main!(avl_tree);
