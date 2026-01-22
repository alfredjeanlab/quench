//! Unit tests for the cloc check.

#![allow(clippy::unwrap_used, clippy::expect_used)]

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
    assert!(is_text_file(Path::new("foo.md")));
    assert!(is_text_file(Path::new("foo.toml")));
    assert!(is_text_file(Path::new("foo.json")));
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
fn is_test_file_recognizes_rust_tests() {
    assert!(is_test_file(Path::new("foo_test.rs")));
    assert!(is_test_file(Path::new("foo_tests.rs")));
    assert!(is_test_file(Path::new("path/to/bar_test.rs")));
}

#[test]
fn is_test_file_recognizes_js_tests() {
    assert!(is_test_file(Path::new("foo.test.js")));
    assert!(is_test_file(Path::new("foo.spec.js")));
    assert!(is_test_file(Path::new("foo.test.ts")));
    assert!(is_test_file(Path::new("foo.spec.tsx")));
}

#[test]
fn is_test_file_rejects_non_tests() {
    assert!(!is_test_file(Path::new("foo.rs")));
    assert!(!is_test_file(Path::new("foo.js")));
    assert!(!is_test_file(Path::new("tests/helper.rs")));
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
