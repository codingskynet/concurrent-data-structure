use cds::{avltree::RwLockAVLTree, map::ConcurrentMap};

use crate::util::map::stress_concurrent_as_sequential;

#[test]
fn test_rwlock_avl_tree() {
    let num = 64;
    let avl: RwLockAVLTree<i32, i32> = RwLockAVLTree::new();

    for i in 0..num {
        assert_eq!(avl.insert(&i, i), Ok(()));
    }

    for i in 0..num {
        assert_eq!(avl.insert(&i, i), Err(i));
    }

    assert_eq!(avl.get_height(), f32::log2(num as f32) as usize + 1);

    for i in 0..num {
        assert_eq!(avl.get(&i), Some(i));
    }

    for i in 0..num {
        assert_eq!(avl.remove(&i), Ok(i));
    }

    for i in 0..num {
        assert_eq!(avl.remove(&i), Err(()));
    }
}

#[test]
fn stress_rwlock_avl_tree_sequential() {
    stress_concurrent_as_sequential::<u8, RwLockAVLTree<_, _>>(100_000);
}

// TODO: this DS may have deadlock
// #[test]
// fn stress_rwlock_avl_tree_concurrent() {
//     stress_concurrent::<u32, RwLockAVLTree<_, _>>(200_000, 16, false);
// }

// #[test]
// fn assert_rwlock_avl_tree_concurrent() {
//     stress_concurrent::<u8, RwLockAVLTree<_, _>>(100_000, 32, true);
// }
