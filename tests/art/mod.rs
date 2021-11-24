use cds::art::ART;
use cds::map::SequentialMap;

use crate::util::map::stress_sequential;

#[test]
fn test_art() {
    let mut art: ART<String, usize> = ART::new();

    assert_eq!(art.insert(&"a".to_string(), 1), Ok(()));
    assert_eq!(art.insert(&"ab".to_string(), 2), Ok(()));
    assert_eq!(art.insert(&"ac".to_string(), 3), Ok(()));
    assert_eq!(art.insert(&"ad".to_string(), 4), Ok(()));
    assert_eq!(art.insert(&"acb".to_string(), 5), Ok(()));

    assert_eq!(art.lookup(&"a".to_string()), Some(&1));
    assert_eq!(art.lookup(&"ab".to_string()), Some(&2));
    assert_eq!(art.lookup(&"ac".to_string()), Some(&3));
    assert_eq!(art.lookup(&"ad".to_string()), Some(&4));
    assert_eq!(art.lookup(&"acb".to_string()), Some(&5));

    assert_eq!(art.remove(&"a".to_string()), Ok(1));
    assert_eq!(art.remove(&"ab".to_string()), Ok(2));
    assert_eq!(art.remove(&"ac".to_string()), Ok(3));
    assert_eq!(art.remove(&"ad".to_string()), Ok(4));
    assert_eq!(art.remove(&"acb".to_string()), Ok(5));
}

#[test]
fn test_large_key_art() {
    let mut art: ART<String, usize> = ART::new();
    assert_eq!(art.insert(&"1234567890".to_string(), 1), Ok(()));
    assert_eq!(art.insert(&"12345678901234567890".to_string(), 2), Ok(()));
    assert_eq!(art.insert(&"123456789012345678901234567890".to_string(), 3), Ok(()));
    assert_eq!(art.insert(&"1234567890123456789012345678901234567890".to_string(), 4), Ok(()));
    assert_eq!(art.insert(&"12345678901234567890123456789012345678901234567890".to_string(), 5), Ok(()));
    assert_eq!(art.insert(&"123456789012345678901234567890123456789012345678901234567890".to_string(), 6), Ok(()));
    assert_eq!(art.lookup(&"1234567890".to_string()), Some(&1));
    assert_eq!(art.lookup(&"12345678901234567890".to_string()), Some(&2));
    assert_eq!(art.lookup(&"123456789012345678901234567890".to_string()), Some(&3));
    assert_eq!(art.lookup(&"1234567890123456789012345678901234567890".to_string()), Some(&4));
    assert_eq!(art.lookup(&"12345678901234567890123456789012345678901234567890".to_string()), Some(&5));
    assert_eq!(art.lookup(&"123456789012345678901234567890123456789012345678901234567890".to_string()), Some(&6));
    assert_eq!(art.remove(&"1234567890".to_string()), Ok(1));
    assert_eq!(art.remove(&"12345678901234567890".to_string()), Ok(2));
    assert_eq!(art.remove(&"123456789012345678901234567890".to_string()), Ok(3));
    assert_eq!(art.remove(&"1234567890123456789012345678901234567890".to_string()), Ok(4));
    assert_eq!(art.remove(&"12345678901234567890123456789012345678901234567890".to_string()), Ok(5));
    assert_eq!(art.remove(&"123456789012345678901234567890123456789012345678901234567890".to_string()), Ok(6));
}

#[test]
fn test_split_key_insert_art() {
    let mut art: ART<String, usize> = ART::new();
    assert_eq!(art.insert(&"123456789012345678901234567890123456789012345678901234567890".to_string(), 6), Ok(()));
    assert_eq!(art.insert(&"12345678901234567890123456789012345678901234567890".to_string(), 5), Ok(()));
    assert_eq!(art.lookup(&"12345678901234567890123456789012345678901234567890".to_string()), Some(&5));
    assert_eq!(art.lookup(&"123456789012345678901234567890123456789012345678901234567890".to_string()), Some(&6));
    assert_eq!(art.insert(&"1234567890123456789012345678901234567890".to_string(), 4), Ok(()));
    assert_eq!(art.insert(&"123456789012345678901234567890".to_string(), 3), Ok(()));
    assert_eq!(art.insert(&"12345678901234567890".to_string(), 2), Ok(()));
    assert_eq!(art.insert(&"1234567890".to_string(), 1), Ok(()));
    assert_eq!(art.lookup(&"1234567890".to_string()), Some(&1));
    assert_eq!(art.lookup(&"12345678901234567890".to_string()), Some(&2));
    assert_eq!(art.lookup(&"123456789012345678901234567890".to_string()), Some(&3));
    assert_eq!(art.lookup(&"1234567890123456789012345678901234567890".to_string()), Some(&4));
    assert_eq!(art.lookup(&"12345678901234567890123456789012345678901234567890".to_string()), Some(&5));
    assert_eq!(art.lookup(&"123456789012345678901234567890123456789012345678901234567890".to_string()), Some(&6));
    assert_eq!(art.remove(&"123456789012345678901234567890123456789012345678901234567890".to_string()), Ok(6));
    assert_eq!(art.remove(&"12345678901234567890123456789012345678901234567890".to_string()), Ok(5));
    assert_eq!(art.remove(&"1234567890123456789012345678901234567890".to_string()), Ok(4));
    assert_eq!(art.remove(&"123456789012345678901234567890".to_string()), Ok(3));
    assert_eq!(art.remove(&"12345678901234567890".to_string()), Ok(2));
    assert_eq!(art.remove(&"1234567890".to_string()), Ok(1));
}

#[test]
fn stress_art() {
    stress_sequential::<String, ART<_, _>>(1_000_000);
}
