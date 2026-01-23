// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for the cloc check.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use yare::parameterized;

use super::*;
use crate::test_utils::temp_file_with_content;

#[test]
fn is_text_file_recognizes_rust() {
    assert!(is_text_file(Path::new("foo.rs")));
    assert!(is_text_file(Path::new("path/to/file.rs")));
}

#[test]
fn is_text_file_recognizes_common_extensions() {
    assert!(is_text_file(Path::new("foo.py")));
    assert!(is_text_file(Path::new("foo.js")));
    assert!(is_text_file(Path::new("foo.ts")));
    assert!(is_text_file(Path::new("foo.go")));
    assert!(is_text_file(Path::new("foo.java")));
    // Config/data files are not counted as source code
    assert!(!is_text_file(Path::new("foo.md")));
    assert!(!is_text_file(Path::new("foo.toml")));
    assert!(!is_text_file(Path::new("foo.json")));
}

#[test]
fn is_text_file_rejects_binary() {
    assert!(!is_text_file(Path::new("foo.exe")));
    assert!(!is_text_file(Path::new("foo.bin")));
    assert!(!is_text_file(Path::new("foo.png")));
    assert!(!is_text_file(Path::new("foo.jpg")));
    assert!(!is_text_file(Path::new("no_extension")));
}

#[test]
fn cloc_check_name() {
    let check = ClocCheck;
    assert_eq!(check.name(), "cloc");
}

#[test]
fn cloc_check_description() {
    let check = ClocCheck;
    assert_eq!(check.description(), "Lines of code and file size limits");
}

#[test]
fn cloc_check_default_enabled() {
    let check = ClocCheck;
    assert!(check.default_enabled());
}

// =============================================================================
// FILE METRICS TESTS (NON-BLANK LINE COUNTING)
// =============================================================================

#[parameterized(
    empty_file = { "", 0 },
    whitespace_only = { "   \n\t\t\n\n    \t  \n", 0 },
    mixed_content = { "fn main() {\n\n    let x = 1;\n\n}\n", 3 },
    no_trailing_newline = { "line1\nline2\nline3", 3 },
    with_trailing_newline = { "line1\nline2\nline3\n", 3 },
    crlf_endings = { "line1\r\nline2\r\n\r\nline3", 3 },
    mixed_endings = { "line1\nline2\r\nline3\n", 3 },
    unicode_whitespace = { "content\n\u{00A0}\nmore\n", 2 },
)]
fn file_metrics_nonblank_lines(content: &str, expected: usize) {
    let file = temp_file_with_content(content);
    let metrics = count_file_metrics(file.path()).unwrap();
    assert_eq!(
        metrics.nonblank_lines, expected,
        "content {:?} should have {} nonblank lines",
        content, expected
    );
}

#[test]
fn file_metrics_empty_file_tokens() {
    // Separate test for empty file also having 0 tokens
    let file = temp_file_with_content("");
    let metrics = count_file_metrics(file.path()).unwrap();
    assert_eq!(metrics.tokens, 0);
}

// =============================================================================
// ADAPTER/PATTERN TESTS
// =============================================================================

use crate::adapter::{Adapter, FileKind, GenericAdapter};

#[parameterized(
    test_dirs_matches_tests = { &["**/tests/**", "**/test/**"], "tests/foo.rs", true },
    test_dirs_matches_nested = { &["**/tests/**", "**/test/**"], "tests/sub/bar.rs", true },
    test_dirs_matches_crate = { &["**/tests/**", "**/test/**"], "crate/tests/test.rs", true },
    test_dirs_matches_test = { &["**/tests/**", "**/test/**"], "test/foo.rs", true },
    test_dirs_excludes_src_lib = { &["**/tests/**", "**/test/**"], "src/lib.rs", false },
    test_dirs_excludes_src_main = { &["**/tests/**", "**/test/**"], "src/main.rs", false },
    suffix_matches_test_rs = { &["**/*_test.*", "**/*_tests.*", "**/*.test.*", "**/*.spec.*"], "src/foo_test.rs", true },
    suffix_matches_tests_rs = { &["**/*_test.*", "**/*_tests.*", "**/*.test.*", "**/*.spec.*"], "src/foo_tests.rs", true },
    suffix_matches_test_js = { &["**/*_test.*", "**/*_tests.*", "**/*.test.*", "**/*.spec.*"], "src/foo.test.js", true },
    suffix_matches_spec_ts = { &["**/*_test.*", "**/*_tests.*", "**/*.test.*", "**/*.spec.*"], "src/foo.spec.ts", true },
    suffix_excludes_lib = { &["**/*_test.*", "**/*_tests.*", "**/*.test.*", "**/*.spec.*"], "src/lib.rs", false },
    suffix_excludes_testing = { &["**/*_test.*", "**/*_tests.*", "**/*.test.*", "**/*.spec.*"], "src/testing.rs", false },
    prefix_matches_test_utils = { &["**/test_*.*"], "src/test_utils.rs", true },
    prefix_matches_test_helpers = { &["**/test_*.*"], "test_helpers.py", true },
    prefix_excludes_testing = { &["**/test_*.*"], "src/testing.rs", false },
    prefix_excludes_contest = { &["**/test_*.*"], "src/contest.rs", false },
)]
fn adapter_test_file_classification(patterns: &[&str], path: &str, expected: bool) {
    let owned: Vec<String> = patterns.iter().map(|s| (*s).to_string()).collect();
    let adapter = GenericAdapter::new(&[], &owned);
    let file_kind = adapter.classify(Path::new(path));
    let is_test = file_kind == FileKind::Test;
    assert_eq!(
        is_test,
        expected,
        "path {} with patterns {:?} should be {}",
        path,
        patterns,
        if expected { "test" } else { "non-test" }
    );
}

#[parameterized(
    excludes_generated = { "**/generated/**", "/project/generated/foo.rs", true },
    excludes_nested = { "**/generated/**", "/project/src/generated/bar.rs", true },
    allows_regular = { "**/generated/**", "/project/src/lib.rs", false },
)]
fn exclude_matcher_exclusion(pattern: &str, path: &str, expected: bool) {
    let matcher = ExcludeMatcher::new(&[pattern.to_string()]);
    let root = Path::new("/project");
    assert_eq!(
        matcher.is_excluded(Path::new(path), root),
        expected,
        "path {} with exclude pattern {} should be {}",
        path,
        pattern,
        if expected { "excluded" } else { "included" }
    );
}

// =============================================================================
// FILE METRICS TESTS (TOKEN COUNTING)
// =============================================================================

#[parameterized(
    short_content = { "abc", 0 },     // 3 chars < 4
    unicode_chars = { "日本語の", 1 }, // 4 Unicode chars / 4 = 1
)]
fn file_metrics_tokens(content: &str, expected: usize) {
    let file = temp_file_with_content(content);
    let metrics = count_file_metrics(file.path()).unwrap();
    assert_eq!(
        metrics.tokens, expected,
        "content {:?} should have {} tokens",
        content, expected
    );
}

#[test]
fn file_metrics_tokens_exact_math() {
    // Keep separate: requires String::repeat which can't be a &str literal
    let file = temp_file_with_content(&"a".repeat(100));
    let metrics = count_file_metrics(file.path()).unwrap();
    assert_eq!(metrics.tokens, 25); // 100 / 4 = 25
}

// =============================================================================
// PATTERN MATCHING BENCHMARK
// =============================================================================

#[test]
#[ignore = "benchmark only"]
fn bench_pattern_matching() {
    use std::path::PathBuf;

    let adapter = GenericAdapter::new(
        &[],
        &[
            "**/tests/**".into(),
            "**/test/**".into(),
            "**/*_test.*".into(),
            "**/*_tests.*".into(),
            "**/*.test.*".into(),
            "**/*.spec.*".into(),
            "**/test_*.*".into(),
        ],
    );

    let paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("src/module_{}.rs", i)))
        .collect();

    let start = std::time::Instant::now();
    for _ in 0..100 {
        for path in &paths {
            let _ = adapter.classify(path);
        }
    }
    let elapsed = start.elapsed();
    println!("100K pattern matches: {:?}", elapsed);
    // Target: < 100ms for 100K matches
    assert!(
        elapsed.as_millis() < 100,
        "Pattern matching too slow: {:?}",
        elapsed
    );
}
