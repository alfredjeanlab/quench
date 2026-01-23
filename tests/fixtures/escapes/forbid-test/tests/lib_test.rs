#[test]
fn test_something() {
    let opt: Option<i32> = Some(42);
    assert_eq!(opt.unwrap(), 42);  // Allowed in tests
}
