// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shared JSON extraction utilities for test runners.
//!
//! Test runner output often contains JSON embedded in other text (logs,
//! warnings, etc.). These utilities extract valid JSON from mixed output.

/// Find the first complete JSON object in a string.
///
/// Handles nested braces correctly. Returns None if no valid object found.
pub fn find_json_object(s: &str) -> Option<&str> {
    find_json_delimited(s, '{', '}')
}

/// Find the first complete JSON array in a string.
///
/// Handles nested brackets correctly. Returns None if no valid array found.
pub fn find_json_array(s: &str) -> Option<&str> {
    find_json_delimited(s, '[', ']')
}

fn find_json_delimited(s: &str, open: char, close: char) -> Option<&str> {
    let start = s.find(open)?;
    let mut depth = 0;
    let mut end = start;

    for (i, c) in s[start..].char_indices() {
        match c {
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    if depth == 0 && end > start { Some(&s[start..end]) } else { None }
}

#[cfg(test)]
#[path = "json_utils_tests.rs"]
mod tests;
