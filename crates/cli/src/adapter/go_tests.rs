#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use super::*;
use crate::adapter::{Adapter, FileKind};
use yare::parameterized;

#[parameterized(
    src_root = { "main.go", FileKind::Source },
    src_pkg = { "pkg/lib.go", FileKind::Source },
    src_nested = { "internal/config/config.go", FileKind::Source },
    test_root = { "main_test.go", FileKind::Test },
    test_pkg = { "pkg/lib_test.go", FileKind::Test },
    vendor = { "vendor/dep/dep.go", FileKind::Other },
    readme = { "README.md", FileKind::Other },
    makefile = { "Makefile", FileKind::Other },
)]
fn classify_path(path: &str, expected: FileKind) {
    let adapter = GoAdapter::new();
    assert_eq!(
        adapter.classify(Path::new(path)),
        expected,
        "path {:?} should be {:?}",
        path,
        expected
    );
}

#[parameterized(
    vendor_root = { "vendor/foo/bar.go", true },
    src_main = { "main.go", false },
    pkg = { "pkg/lib.go", false },
)]
fn should_ignore_path(path: &str, expected: bool) {
    let adapter = GoAdapter::new();
    assert_eq!(
        adapter.should_ignore(Path::new(path)),
        expected,
        "path {:?} should_ignore = {}",
        path,
        expected
    );
}

#[test]
fn has_correct_name_and_extensions() {
    let adapter = GoAdapter::new();
    assert_eq!(adapter.name(), "go");
    assert_eq!(adapter.extensions(), &["go"]);
}

#[test]
fn returns_three_default_escape_patterns() {
    let adapter = GoAdapter::new();
    assert_eq!(adapter.default_escapes().len(), 3);
}

#[parameterized(
    unsafe_pointer = { "unsafe_pointer", r"unsafe\.Pointer", Some("// SAFETY:") },
    go_linkname = { "go_linkname", r"//go:linkname", Some("// LINKNAME:") },
    go_noescape = { "go_noescape", r"//go:noescape", Some("// NOESCAPE:") },
)]
fn default_escape_pattern(name: &str, pattern: &str, expected_comment: Option<&str>) {
    let adapter = GoAdapter::new();
    let patterns = adapter.default_escapes();
    let found = patterns
        .iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("pattern {:?} not found", name));
    assert_eq!(found.pattern, pattern, "pattern {:?}", name);
    assert_eq!(found.comment, expected_comment, "comment for {:?}", name);
}

#[test]
fn parses_module_name_from_go_mod() {
    let content = r#"module github.com/example/project

go 1.21

require (
    github.com/foo/bar v1.0.0
)
"#;
    let module = parse_go_mod(content);
    assert_eq!(module, Some("github.com/example/project".to_string()));
}

#[test]
fn parses_simple_module_name() {
    let content = "module myproject\n\ngo 1.21\n";
    let module = parse_go_mod(content);
    assert_eq!(module, Some("myproject".to_string()));
}

#[test]
fn returns_none_for_invalid_go_mod() {
    let content = "go 1.21\n";
    let module = parse_go_mod(content);
    assert!(module.is_none());
}
