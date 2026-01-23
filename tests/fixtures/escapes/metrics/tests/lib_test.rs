#[test]
fn test_safe() {
    // TODO: More tests
    let x: Option<i32> = Some(1);
    assert_eq!(x.unwrap(), 1);  // unwrap in test
}
