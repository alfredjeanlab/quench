//! Unit tests for the cloc check.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;

use tempfile::NamedTempFile;

use super::*;

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
// NON-BLANK LINE COUNTING TESTS
// =============================================================================

#[test]
fn count_nonblank_lines_empty_file() {
    let mut file = NamedTempFile::new().unwrap();
    // Write nothing
    file.flush().unwrap();

    let count = count_nonblank_lines(file.path()).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn count_nonblank_lines_whitespace_only() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "   ").unwrap();
    writeln!(file, "\t\t").unwrap();
    writeln!(file).unwrap();
    writeln!(file, "    \t  ").unwrap();
    file.flush().unwrap();

    let count = count_nonblank_lines(file.path()).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn count_nonblank_lines_mixed_content() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "fn main() {{").unwrap();
    writeln!(file).unwrap();
    writeln!(file, "    let x = 1;").unwrap();
    writeln!(file).unwrap();
    writeln!(file, "}}").unwrap();
    file.flush().unwrap();

    let count = count_nonblank_lines(file.path()).unwrap();
    assert_eq!(count, 3); // fn main, let x, closing brace
}

#[test]
fn count_nonblank_lines_no_trailing_newline() {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "line1\nline2\nline3").unwrap();
    file.flush().unwrap();

    let count = count_nonblank_lines(file.path()).unwrap();
    assert_eq!(count, 3);
}

#[test]
fn count_nonblank_lines_with_trailing_newline() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "line1").unwrap();
    writeln!(file, "line2").unwrap();
    writeln!(file, "line3").unwrap();
    file.flush().unwrap();

    let count = count_nonblank_lines(file.path()).unwrap();
    assert_eq!(count, 3);
}

#[test]
fn count_nonblank_lines_crlf_endings() {
    // Windows-style line endings
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "line1\r\nline2\r\n\r\nline3").unwrap();
    file.flush().unwrap();

    let count = count_nonblank_lines(file.path()).unwrap();
    assert_eq!(count, 3); // Should handle CRLF correctly
}

#[test]
fn count_nonblank_lines_mixed_endings() {
    // Mixed LF and CRLF
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "line1\nline2\r\nline3\n").unwrap();
    file.flush().unwrap();

    let count = count_nonblank_lines(file.path()).unwrap();
    assert_eq!(count, 3);
}

#[test]
fn count_nonblank_lines_unicode_whitespace() {
    // Non-breaking space (U+00A0) should still be whitespace
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "content").unwrap();
    writeln!(file, "\u{00A0}").unwrap(); // non-breaking space only
    writeln!(file, "more").unwrap();
    file.flush().unwrap();

    let count = count_nonblank_lines(file.path()).unwrap();
    // Note: Rust's trim() handles unicode whitespace
    assert_eq!(count, 2);
}

// =============================================================================
// PATTERN MATCHER TESTS
// =============================================================================

#[test]
fn pattern_matcher_identifies_test_directories() {
    let matcher = PatternMatcher::new(&["**/tests/**".to_string(), "**/test/**".to_string()], &[]);

    let root = Path::new("/project");

    // Files in tests/ directory should match
    assert!(matcher.is_test_file(Path::new("/project/tests/foo.rs"), root));
    assert!(matcher.is_test_file(Path::new("/project/tests/sub/bar.rs"), root));
    assert!(matcher.is_test_file(Path::new("/project/crate/tests/test.rs"), root));

    // Files in test/ directory should match
    assert!(matcher.is_test_file(Path::new("/project/test/foo.rs"), root));

    // Regular source files should not match
    assert!(!matcher.is_test_file(Path::new("/project/src/lib.rs"), root));
    assert!(!matcher.is_test_file(Path::new("/project/src/main.rs"), root));
}

#[test]
fn pattern_matcher_identifies_test_suffixes() {
    let matcher = PatternMatcher::new(
        &[
            "**/*_test.*".to_string(),
            "**/*_tests.*".to_string(),
            "**/*.test.*".to_string(),
            "**/*.spec.*".to_string(),
        ],
        &[],
    );

    let root = Path::new("/project");

    // Files with test suffixes should match
    assert!(matcher.is_test_file(Path::new("/project/src/foo_test.rs"), root));
    assert!(matcher.is_test_file(Path::new("/project/src/foo_tests.rs"), root));
    assert!(matcher.is_test_file(Path::new("/project/src/foo.test.js"), root));
    assert!(matcher.is_test_file(Path::new("/project/src/foo.spec.ts"), root));

    // Regular source files should not match
    assert!(!matcher.is_test_file(Path::new("/project/src/lib.rs"), root));
    assert!(!matcher.is_test_file(Path::new("/project/src/testing.rs"), root));
}

#[test]
fn pattern_matcher_excludes_patterns() {
    let matcher = PatternMatcher::new(&[], &["**/generated/**".to_string()]);

    let root = Path::new("/project");

    // Files in generated/ should be excluded
    assert!(matcher.is_excluded(Path::new("/project/generated/foo.rs"), root));
    assert!(matcher.is_excluded(Path::new("/project/src/generated/bar.rs"), root));

    // Regular files should not be excluded
    assert!(!matcher.is_excluded(Path::new("/project/src/lib.rs"), root));
}

#[test]
fn pattern_matcher_identifies_test_prefix() {
    let matcher = PatternMatcher::new(&["**/test_*.*".to_string()], &[]);

    let root = Path::new("/project");

    // Files with test_ prefix should match
    assert!(matcher.is_test_file(Path::new("/project/src/test_utils.rs"), root));
    assert!(matcher.is_test_file(Path::new("/project/test_helpers.py"), root));

    // Regular source files should not match
    assert!(!matcher.is_test_file(Path::new("/project/src/testing.rs"), root));
    assert!(!matcher.is_test_file(Path::new("/project/src/contest.rs"), root));
}

// =============================================================================
// TOKEN COUNTING TESTS
// =============================================================================

#[test]
fn count_tokens_empty_file() {
    let mut file = NamedTempFile::new().unwrap();
    file.flush().unwrap();

    let count = count_tokens(file.path()).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn count_tokens_short_content() {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "abc").unwrap(); // 3 chars < 4
    file.flush().unwrap();

    let count = count_tokens(file.path()).unwrap();
    assert_eq!(count, 0); // 3 / 4 = 0
}

#[test]
fn count_tokens_exact_math() {
    let mut file = NamedTempFile::new().unwrap();
    // Write exactly 100 characters
    write!(file, "{}", "a".repeat(100)).unwrap();
    file.flush().unwrap();

    let count = count_tokens(file.path()).unwrap();
    assert_eq!(count, 25); // 100 / 4 = 25
}

#[test]
fn count_tokens_unicode() {
    let mut file = NamedTempFile::new().unwrap();
    // Unicode chars: 4 chars (not 4 bytes)
    write!(file, "日本語の").unwrap(); // 4 Unicode chars
    file.flush().unwrap();

    let count = count_tokens(file.path()).unwrap();
    assert_eq!(count, 1); // 4 chars / 4 = 1 token
}
