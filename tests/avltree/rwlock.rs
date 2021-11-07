use cds::{avltree::RwLockAVLTree, map::ConcurrentMap};
use crossbeam_epoch::pin;

use crate::util::map::{stress_concurrent, stress_concurrent_as_sequential};

#[test]
fn test_rwlock_avl_tree() {
    let num = 64;
    let pin = pin();
    let avl: RwLockAVLTree<i32, i32> = RwLockAVLTree::new();

    for i in 0..num {
        assert_eq!(avl.insert(&i, i, &pin), Ok(()));
    }

    for i in 0..num {
        assert_eq!(avl.insert(&i, i, &pin), Err(i));
    }

    assert_eq!(avl.get_height(&pin), f32::log2(num as f32) as usize + 1);

    for i in 0..num {
        assert_eq!(avl.get(&i, &pin), Some(i));
    }

    for i in 0..num {
        assert_eq!(avl.remove(&i, &pin), Ok(i));
    }

    for i in 0..num {
        assert_eq!(avl.remove(&i, &pin), Err(()));
    }
}

#[test]
fn stress_rwlock_avl_tree_sequential() {
    stress_concurrent_as_sequential::<u8, RwLockAVLTree<_, _>>(100_000);
}

#[test]
fn stress_rwlock_avl_tree_concurrent() {
    stress_concurrent::<u32, RwLockAVLTree<_, _>>(200_000, 16, false);
}

#[test]
fn assert_rwlock_avl_tree_concurrent() {
    stress_concurrent::<u8, RwLockAVLTree<_, _>>(100_000, 32, true);
}
