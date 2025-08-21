use mca_rs::region::Region;

#[test]
fn test_parse_empty_region() {
    let bytes = vec![0u8; 8192];
    let region = Region::parse_bytes(&bytes).unwrap();

    assert_eq!(region.count_chunks(), 0);
}

#[test]
fn test_parse_real_region() {
    let region = Region::parse_bytes(include_bytes!("data/r.0.0.mca")).unwrap();
    assert_eq!(region.count_chunks(), 975);
}
