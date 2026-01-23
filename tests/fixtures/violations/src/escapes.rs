//! File with escape hatch violations.

/// Function using unwrap (forbidden in production code).
pub fn risky_parse(input: &str) -> i32 {
    input.parse().unwrap()  // VIOLATION: unwrap forbidden in source
}

/// Function using expect (also forbidden).
pub fn risky_get(map: &std::collections::HashMap<String, i32>, key: &str) -> i32 {
    *map.get(key).expect("key must exist")  // VIOLATION: expect forbidden in source
}

/// Unsafe block without SAFETY comment.
pub fn unsafe_op(ptr: *const i32) -> i32 {
    unsafe { *ptr }  // VIOLATION: unsafe block requires SAFETY comment
}

/// Proper unsafe with SAFETY comment (should pass).
pub fn safe_unsafe_op(ptr: *const i32) -> i32 {
    // SAFETY: Caller guarantees ptr is valid and aligned.
    unsafe { *ptr }
}

/// Suppressed lint without justification.
#[allow(dead_code)]  // VIOLATION: #[allow] without comment
fn unused_function() {}

/// Properly justified lint suppression (should pass).
// JUSTIFIED: This function is used via FFI, not detected by Rust analysis.
#[allow(dead_code)]
fn ffi_callback() {}
