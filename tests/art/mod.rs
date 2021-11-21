use cds::art::ART;
use cds::map::SequentialMap;

#[test]
fn test_art() {
    let mut art: ART<String, usize> = ART::new();

    assert_eq!(art.insert(&"a".to_string(), 1), Ok(()));
    assert_eq!(art.insert(&"ab".to_string(), 2), Ok(()));
    assert_eq!(art.insert(&"ac".to_string(), 3), Ok(()));
    assert_eq!(art.insert(&"ad".to_string(), 4), Ok(()));
    assert_eq!(art.insert(&"acb".to_string(), 5), Ok(()));

    assert_eq!(art.lookup(&"a".to_string()), Some(&1));
    assert_eq!(art.lookup(&"ab".to_string()),Some(&2));
    assert_eq!(art.lookup(&"ac".to_string()),Some(&3));
    assert_eq!(art.lookup(&"ad".to_string()),Some(&4));
    assert_eq!(art.lookup(&"acb".to_string()),Some(&5));
}
