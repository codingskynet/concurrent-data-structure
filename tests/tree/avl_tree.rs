use crate::util::map::stress_sequential;
use cds::{map::SequentialMap, tree::avl_tree::AVLTree};

#[test]
fn test_avl_tree() {
    let mut avl: AVLTree<i32, i32> = AVLTree::new();

    // assert_eq!(avl.insert(&1, 1), Ok(()));
    // assert_eq!(avl.insert(&2, 2), Ok(()));
    // assert_eq!(avl.insert(&3, 3), Ok(()));
    // assert_eq!(avl.insert(&4, 4), Ok(()));
    // assert_eq!(avl.insert(&5, 5), Ok(()));

    for i in 0..65536 {
        assert_eq!(avl.insert(&i, i), Ok(()));
        println!("[{}] height: {}", i, avl.get_height());
    }

    for i in 0..65536 {
        assert_eq!(avl.lookup(&i), Some(&i));
    }

    println!("height: {}", avl.get_height());
}

#[test]
fn stress_avl_tree() {
    stress_sequential::<String, AVLTree<_, _>>(100_000);
}
