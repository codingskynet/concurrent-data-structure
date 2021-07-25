use cds::{avl_tree::lock::RwLockAVLTree, map::ConcurrentMap};
use crossbeam_epoch::pin;

use crate::util::map::{stress_concurrent, stress_concurrent_as_sequential};

#[test]
fn test_rwlock_avl_tree_insert_lookup() {
    let pin = pin();
    let avl: RwLockAVLTree<i32, i32> = RwLockAVLTree::new();

    for i in 0..100 {
        assert_eq!(avl.insert(&i, i, &pin), Ok(()));
    }

    for i in 0..100 {
        assert_eq!(avl.insert(&i, i, &pin), Err(i));
    }

    for i in 0..100 {
        assert_eq!(avl.lookup(&i, &pin), Some(i));
    }
}

#[test]
fn stress_rwlock_avl_tree_sequential() {
    stress_concurrent_as_sequential::<u8, RwLockAVLTree<_, _>>(1_000_000);
}

#[test]
fn stress_rwlock_avl_tree_concurrent() {
    stress_concurrent::<u8, RwLockAVLTree<_, _>>(100_000, 10);
}
