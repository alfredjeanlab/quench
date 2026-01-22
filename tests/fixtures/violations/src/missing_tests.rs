//! Source file with no corresponding test file.
//!
//! This triggers the tests check because there's no
//! tests/missing_tests_tests.rs or similar.

/// A function that should have tests but doesn't.
pub fn untested_logic(x: i32) -> i32 {
    if x > 0 { x * 2 } else { x * -1 }
}

/// Another untested function.
pub fn more_untested_code(s: &str) -> String {
    s.chars().rev().collect()
}
