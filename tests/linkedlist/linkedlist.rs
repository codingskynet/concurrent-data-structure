use crate::util::map::stress_sequential;
use cds::linkedlist::SortedList;
use cds::map::SequentialMap;
use rand::{thread_rng, Rng};

#[test]
fn test_sorted_list() {
    let mut list: SortedList<i32, i32> = SortedList::new();

    assert_eq!(list.lookup(&1), None);

    assert_eq!(list.insert(&1, 1), Ok(()));
    assert_eq!(list.insert(&2, 2), Ok(()));
    assert_eq!(list.insert(&3, 3), Ok(()));
    assert_eq!(list.insert(&4, 4), Ok(()));
    assert_eq!(list.insert(&5, 5), Ok(()));

    assert_eq!(list.lookup(&1), Some(&1));
    assert_eq!(list.lookup(&2), Some(&2));
    assert_eq!(list.lookup(&3), Some(&3));
    assert_eq!(list.lookup(&4), Some(&4));
    assert_eq!(list.lookup(&5), Some(&5));

    assert_eq!(list.remove(&1), Ok(1));
    assert_eq!(list.remove(&3), Ok(3));
    assert_eq!(list.remove(&5), Ok(5));

    assert_eq!(list.lookup(&1), None);
    assert_eq!(list.lookup(&2), Some(&2));
    assert_eq!(list.lookup(&3), None);
    assert_eq!(list.lookup(&4), Some(&4));
    assert_eq!(list.lookup(&5), None);

    assert_eq!(list.remove(&4), Ok(4));
    assert_eq!(list.remove(&2), Ok(2));

    assert_eq!(list.insert(&0, 0), Ok(()));
    assert_eq!(list.lookup(&0), Some(&0));
    assert_eq!(list.remove(&0), Ok(0));
    assert_eq!(list.lookup(&0), None);
}

#[test]
fn test_sorted_list_is_sorted() {
    let mut list: SortedList<i32, i32> = SortedList::new();

    let mut rng = thread_rng();

    for _ in 0..100 {
        let val = rng.gen_range(0..39393939);
        assert_eq!(list.insert(&val, val), Ok(()));
    }

    list.keys()
        .windows(2)
        .enumerate()
        .map(|(i, n)| assert!(n[0] < n[1], "{} < {} on {}", n[0], n[1], i))
        .nth(98);
}

#[test]
fn stress_sorted_list() {
    stress_sequential::<String, SortedList<_, _>>(100_000);
}
