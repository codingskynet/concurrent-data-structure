use cds::{btree::BTree, map::SequentialMap};

#[test]
fn test_insert_lookup_btree() {
    let num = 4095;
    let mut tree: BTree<i32, i32> = BTree::new();

    for i in 0..num {
        assert_eq!(tree.insert(&i, i), Ok(()));
        tree.assert();
    }

    for i in 0..num {
        assert_eq!(tree.lookup(&i), Some(&i));
    }
}

#[test]
fn test_remove_btree() {
    // CASE 0: remove on leaf root
    {
        let mut tree: BTree<i32, i32> = BTree::new();
        assert_eq!(tree.insert(&1, 1), Ok(()));
        // tree.print();
        assert_eq!(tree.remove(&1), Ok(1));
        // tree.print();
        tree.assert();
    }

    // CASE 1-1: remove on (1, 1) with left leaf node
    {
        let target = 0;

        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 0..3 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..3 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // CASE 5-1: remove on (1, 1) with right leaf node
    {
        let target = 2;

        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 0..3 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..3 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // (CASE 5-1 ->) CASE 5-2: remove on (1, 1) with right non-leaf node
    {
        let target = 5;
        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 0..7 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        tree.print();

        for i in 0..3 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // (CASE 5-1 ->) CASE 1-2: remove on (1, 1) with left non-leaf node
    {
        let target = 1;
        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 0..7 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..3 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }
}
