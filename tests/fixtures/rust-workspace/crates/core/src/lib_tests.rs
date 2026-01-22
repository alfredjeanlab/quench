#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn test_process() {
    assert_eq!(process("hello"), "HELLO");
}
