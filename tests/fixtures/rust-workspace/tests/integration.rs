#![allow(clippy::unwrap_used)]

use workspace_core::process;

#[test]
fn test_integration() {
    assert_eq!(process("test"), "TEST");
}
