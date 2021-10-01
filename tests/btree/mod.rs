use cds::{btree::BTree, map::SequentialMap};

#[test]
fn test() {
    let mut tree: BTree<i32, i32> = BTree::new();

    assert_eq!(tree.insert(&1, 1), Ok(()));
    // tree.print();
    assert_eq!(tree.insert(&2, 2), Ok(()));
    // tree.print();
    assert_eq!(tree.insert(&3, 3), Ok(()));
    // tree.print();
    assert_eq!(tree.insert(&4, 4), Ok(()));
    // tree.print();
    assert_eq!(tree.insert(&5, 5), Ok(()));
    // tree.print();
    // assert_eq!(tree.insert(&6, 6), Ok(()));
    // tree.print();
    // assert_eq!(tree.remove(&4), Ok(4));
    tree.print();
    assert_eq!(tree.insert(&7, 7), Ok(()));
    assert_eq!(tree.insert(&8, 8), Ok(()));
    assert_eq!(tree.insert(&6, 6), Ok(()));
    assert_eq!(tree.remove(&4), Ok(4));
    tree.print();


    // assert_eq!(tree.insert(&9, 9), Ok(()));
    // assert_eq!(tree.insert(&10, 10), Ok(()));
    // assert_eq!(tree.insert(&11, 11), Ok(()));
    // assert_eq!(tree.insert(&12, 12), Ok(()));
    // assert_eq!(tree.insert(&13, 13), Ok(()));
    // assert_eq!(tree.insert(&14, 14), Ok(()));
    // assert_eq!(tree.insert(&15, 15), Ok(()));
    // assert_eq!(tree.insert(&8, 8), Ok(()));
    // tree.print();
}
