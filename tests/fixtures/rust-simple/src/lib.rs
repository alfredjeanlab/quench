//! A simple library for testing quench.

/// Adds two numbers together.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
