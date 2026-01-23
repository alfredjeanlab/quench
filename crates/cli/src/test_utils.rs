// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shared unit test utilities.
//!
//! Provides common helpers for unit tests in the cli crate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

/// Creates a temp directory with a minimal quench.toml.
pub fn temp_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();
    dir
}

/// Creates a temp directory with custom config content.
pub fn temp_project_with_config(config: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("quench.toml"), config).unwrap();
    dir
}

/// Creates a directory tree from a list of (path, content) pairs.
///
/// Parent directories are created automatically.
///
/// # Example
///
/// ```ignore
/// let tmp = temp_project();
/// create_tree(tmp.path(), &[
///     ("src/lib.rs", "fn main() {}"),
///     ("src/test.rs", "fn test() {}"),
/// ]);
/// ```
pub fn create_tree(root: &Path, files: &[(&str, &str)]) {
    for (path, content) in files {
        let full_path = root.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full_path, content).unwrap();
    }
}

/// Creates a temp file with the given content for testing.
///
/// Returns the NamedTempFile which keeps the file alive.
pub fn temp_file_with_content(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{}", content).unwrap();
    file.flush().unwrap();
    file
}

/// Creates a temp file with content using writeln! for each line.
///
/// Useful for tests that need explicit newlines.
pub fn temp_file_with_lines(lines: &[&str]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    for line in lines {
        writeln!(file, "{}", line).unwrap();
    }
    file.flush().unwrap();
    file
}

// Pattern matching test utilities

use crate::pattern::CompiledPattern;

/// Compiles a pattern and asserts it finds expected number of matches.
///
/// Useful for parameterized tests that verify pattern matching behavior.
pub fn assert_pattern_matches(pattern: &str, content: &str, expected_count: usize) {
    let compiled = CompiledPattern::compile(pattern)
        .unwrap_or_else(|e| panic!("failed to compile pattern {:?}: {}", pattern, e));
    let matches = compiled.find_all(content);
    assert_eq!(
        matches.len(),
        expected_count,
        "pattern {:?} in {:?} should have {} matches, found {}",
        pattern,
        content,
        expected_count,
        matches.len()
    );
}

/// Compiles a pattern and asserts it matches at specific line numbers.
///
/// Line numbers are 1-indexed.
pub fn assert_pattern_at_lines(pattern: &str, content: &str, expected_lines: &[u32]) {
    let compiled = CompiledPattern::compile(pattern)
        .unwrap_or_else(|e| panic!("failed to compile pattern {:?}: {}", pattern, e));
    let matches = compiled.find_all_with_lines(content);
    let actual_lines: Vec<u32> = matches.iter().map(|m| m.line).collect();
    assert_eq!(
        actual_lines, expected_lines,
        "pattern {:?} should match at lines {:?}, found {:?}",
        pattern, expected_lines, actual_lines
    );
}
