use cds::linkedlist::LinkedList;
use cds::sequential::SequentialMap;

#[test]
fn test_linkedlist() {
    let mut list: LinkedList<i32, i32> = LinkedList::new();

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

    assert_eq!(list.delete(&1), Ok(1));
    assert_eq!(list.delete(&3), Ok(3));
    assert_eq!(list.delete(&5), Ok(5));

    assert_eq!(list.lookup(&1), None);
    assert_eq!(list.lookup(&2), Some(&2));
    assert_eq!(list.lookup(&3), None);
    assert_eq!(list.lookup(&4), Some(&4));
    assert_eq!(list.lookup(&5), None);

    assert_eq!(list.delete(&4), Ok(4));
    assert_eq!(list.delete(&2), Ok(2));

    assert_eq!(list.insert(&0, 0), Ok(()));
    assert_eq!(list.lookup(&0), Some(&0));
    assert_eq!(list.delete(&0), Ok(0));
    assert_eq!(list.lookup(&0), None);
}
