#[test]
fn test_covered() {
    assert_eq!(rust_coverage::covered_function(), 42);
}
