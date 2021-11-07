mod rwlock;
mod seqlock;

use crate::util::map::stress_sequential;
use cds::{avltree::AVLTree, map::SequentialMap};

#[test]
fn test_insert_lookup_avl_tree() {
    let mut avl: AVLTree<i32, i32> = AVLTree::new();

    for i in 0..65535 {
        // 65535 = 2^16 - 1
        assert_eq!(avl.insert(&i, i), Ok(()));
    }

    assert_eq!(avl.get_height(), 16);
    assert_eq!(avl.insert(&65536, 65536), Ok(()));
    assert_eq!(avl.get_height(), 17);

    for i in 0..65535 {
        assert_eq!(avl.lookup(&i), Some(&i));
    }
}

#[test]
fn test_remove_avl_tree() {
    let mut avl: AVLTree<i32, i32> = AVLTree::new();

    /* make tree like this
     *
     *          3
     *       2     4
     *    1           5
     */

    assert_eq!(avl.insert(&3, 3), Ok(()));
    assert_eq!(avl.insert(&2, 2), Ok(()));
    assert_eq!(avl.insert(&4, 4), Ok(()));
    assert_eq!(avl.insert(&1, 1), Ok(()));
    assert_eq!(avl.insert(&5, 5), Ok(()));

    assert_eq!(avl.remove(&1), Ok(1)); // remove when the node is leaf
    assert_eq!(avl.insert(&1, 1), Ok(()));

    assert_eq!(avl.remove(&2), Ok(2)); // remove when the node has only left node
    assert_eq!(avl.remove(&4), Ok(4)); // remove when the node has only right node
    assert_eq!(avl.remove(&3), Ok(3)); // remove when the node has two nodes

    assert_eq!(avl.lookup(&1), Some(&1));
    assert_eq!(avl.lookup(&5), Some(&5));

    // side case of remove when the node has two nodes
    let mut avl: AVLTree<i32, i32> = AVLTree::new();
    assert_eq!(avl.insert(&4, 4), Ok(()));
    assert_eq!(avl.insert(&0, 0), Ok(()));
    assert_eq!(avl.insert(&-1, -1), Ok(()));
    assert_eq!(avl.insert(&5, 5), Ok(()));
    assert_eq!(avl.insert(&6, 6), Ok(()));
    assert_eq!(avl.insert(&2, 2), Ok(()));
    assert_eq!(avl.insert(&1, 1), Ok(()));

    assert_eq!(avl.remove(&4), Ok(4));

    assert_eq!(avl.lookup(&-1), Some(&-1));
    assert_eq!(avl.lookup(&0), Some(&0));
    assert_eq!(avl.lookup(&1), Some(&1));
    assert_eq!(avl.lookup(&2), Some(&2));
    assert_eq!(avl.lookup(&5), Some(&5));
    assert_eq!(avl.lookup(&6), Some(&6));
}

#[test]
fn stress_avl_tree() {
    stress_sequential::<String, AVLTree<_, _>>(100_000);
}
