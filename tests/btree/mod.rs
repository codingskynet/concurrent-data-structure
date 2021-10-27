use cds::{btree::BTree, map::SequentialMap};

use crate::util::map::{stress_sequential};

#[test]
fn test_insert_lookup_btree() {
    let num = 4095;
    let mut tree: BTree<i32, i32> = BTree::new();

    for i in 0..num {
        assert_eq!(tree.insert(&i, i), Ok(()));
        tree.assert();
        // tree.print();
    }

    for i in 0..num {
        assert_eq!(tree.lookup(&i), Some(&i));
    }
}

#[test]
fn test_remove_btree() {
    // CASE 0-1: remove on leaf root
    {
        let mut tree: BTree<i32, i32> = BTree::new();
        assert_eq!(tree.insert(&1, 1), Ok(()));
        // tree.print();
        assert_eq!(tree.remove(&1), Ok(1));
        // tree.print();
        tree.assert();
    }

    // CASE 0-2: remove on non-leaf root
    {
        let target = 2;

        let mut tree: BTree<i32, i32> = BTree::new();

        assert_eq!(tree.insert(&0, 0), Ok(()));
        assert_eq!(tree.insert(&2, 2), Ok(()));
        assert_eq!(tree.insert(&3, 3), Ok(()));
        assert_eq!(tree.insert(&4, 4), Ok(()));
        assert_eq!(tree.insert(&5, 5), Ok(()));
        assert_eq!(tree.insert(&1, 1), Ok(()));

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..5 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // (parent_size, sibiling_size)
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

    // CASE 2-1: remove on (1, 2) with left leaf node
    {
        let target = 0;

        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 0..4 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..4 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // (CASE 5-1 ->) CASE 2-2: remove on (1, 2) with left non-leaf node
    {
        let target = 1;

        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 0..9 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..9 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // CASE 6-1: remove on (1, 2) with right leaf node
    {
        let target = 3;

        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 1..4 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }
        assert_eq!(tree.insert(&0, 0), Ok(()));

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..4 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // (CASE 5-1 ->) CASE 6-2: remove on (1, 2) with right non-leaf node
    {
        let target = 7;

        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 2..9 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }
        assert_eq!(tree.insert(&1, 1), Ok(()));
        assert_eq!(tree.insert(&0, 0), Ok(()));

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..9 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // CASE 3-1: remove on (2, 1) with left leaf node
    {
        let target = 0;

        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 0..5 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..5 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // (CASE 5-1 ->) CASE 3-2: remove on (2, 1) with left non-leaf node
    {
        let target = 1;

        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 0..11 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..11 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // CASE 7-1: remove on (2, 1) with right leaf node
    {
        let target = 4;

        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 0..5 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..5 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // (CASE 5-1 ->) CASE 7-2: remove on (2, 1) with right non-leaf node
    {
        let target = 9;

        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 0..11 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..11 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // CASE 4-1: remove on (2, 2) with left leaf node
    {
        let target = 0;

        let mut tree: BTree<i32, i32> = BTree::new();

        assert_eq!(tree.insert(&0, 0), Ok(()));
        assert_eq!(tree.insert(&1, 1), Ok(()));
        assert_eq!(tree.insert(&2, 2), Ok(()));
        assert_eq!(tree.insert(&4, 4), Ok(()));
        assert_eq!(tree.insert(&5, 5), Ok(()));
        assert_eq!(tree.insert(&3, 3), Ok(()));

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..6 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // (CASE 5-1 ->) CASE 4-2: remove on (2, 2) with left non-leaf node
    {
        let target = 1;

        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 0..7 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        for i in 9..13 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        assert_eq!(tree.insert(&7, 7), Ok(()));
        assert_eq!(tree.insert(&8, 8), Ok(()));

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..13 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // CASE 8-1: remove on (2, 2) with right leaf node
    {
        let target = 5;

        let mut tree: BTree<i32, i32> = BTree::new();

        assert_eq!(tree.insert(&0, 0), Ok(()));
        assert_eq!(tree.insert(&1, 1), Ok(()));
        assert_eq!(tree.insert(&2, 2), Ok(()));
        assert_eq!(tree.insert(&4, 4), Ok(()));
        assert_eq!(tree.insert(&5, 5), Ok(()));
        assert_eq!(tree.insert(&3, 3), Ok(()));

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..6 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }

    // CASE 8-2: remove on (2, 2) with right non-leaf node
    {
        let target = 11;

        let mut tree: BTree<i32, i32> = BTree::new();

        for i in 0..7 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        for i in 9..13 {
            assert_eq!(tree.insert(&i, i), Ok(()));
        }

        assert_eq!(tree.insert(&7, 7), Ok(()));
        assert_eq!(tree.insert(&8, 8), Ok(()));

        // tree.print();
        assert_eq!(tree.remove(&target), Ok(target));
        // tree.print();

        for i in 0..13 {
            if i == target {
                assert_eq!(tree.lookup(&i), None);
            } else {
                assert_eq!(tree.lookup(&i), Some(&i));
            }
        }
        tree.assert();
    }
}

#[test]
fn stress_btree() {
    stress_sequential::<String, BTree<_, _>>(100_000);
}
