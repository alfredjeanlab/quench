//! Core library functionality.

pub fn process(input: &str) -> String {
    input.to_uppercase()
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
