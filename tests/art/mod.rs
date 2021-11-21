use cds::art::ART;
use cds::map::SequentialMap;

#[test]
fn test_art() {
    let mut art: ART<String, usize> = ART::new();

    assert_eq!(art.insert(&"a".to_string(), 1), Ok(()));
    assert_eq!(art.insert(&"aa".to_string(), 1), Ok(()));

    println!("{:?}", art);
}
