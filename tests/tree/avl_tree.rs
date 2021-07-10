use crate::util::map::stress_sequential;
use cds::{map::SequentialMap, tree::avl_tree::AVLTree};

#[test]
fn test_avl_tree() {
    let mut avl: AVLTree<i32, i32> = AVLTree::new();

    for i in 0..65535 { // 65535 = 2^16 - 1
        assert_eq!(avl.insert(&i, i), Ok(()));
    }

    assert_eq!(avl.get_height(), 16);

    for i in 0..65535 {
        assert_eq!(avl.lookup(&i), Some(&i));
    }
}

#[test]
fn stress_avl_tree() {
    stress_sequential::<String, AVLTree<_, _>>(100_000);
}
