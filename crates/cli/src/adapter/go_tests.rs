#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use super::*;
use crate::adapter::{Adapter, FileKind};

#[test]
fn classifies_go_files_as_source() {
    let adapter = GoAdapter::new();
    assert_eq!(adapter.classify(Path::new("main.go")), FileKind::Source);
    assert_eq!(adapter.classify(Path::new("pkg/lib.go")), FileKind::Source);
    assert_eq!(
        adapter.classify(Path::new("internal/config/config.go")),
        FileKind::Source
    );
}

#[test]
fn classifies_test_files_as_test() {
    let adapter = GoAdapter::new();
    assert_eq!(adapter.classify(Path::new("main_test.go")), FileKind::Test);
    assert_eq!(
        adapter.classify(Path::new("pkg/lib_test.go")),
        FileKind::Test
    );
}

#[test]
fn ignores_vendor_directory() {
    let adapter = GoAdapter::new();
    assert_eq!(
        adapter.classify(Path::new("vendor/dep/dep.go")),
        FileKind::Other
    );
    assert!(adapter.should_ignore(Path::new("vendor/foo/bar.go")));
}

#[test]
fn non_go_files_are_other() {
    let adapter = GoAdapter::new();
    assert_eq!(adapter.classify(Path::new("README.md")), FileKind::Other);
    assert_eq!(adapter.classify(Path::new("Makefile")), FileKind::Other);
}

#[test]
fn has_correct_name_and_extensions() {
    use super::Adapter;
    let adapter = GoAdapter::new();
    assert_eq!(adapter.name(), "go");
    assert_eq!(adapter.extensions(), &["go"]);
}

#[test]
fn provides_default_escape_patterns() {
    use super::Adapter;
    let adapter = GoAdapter::new();
    let escapes = adapter.default_escapes();

    // Should have 3 default escape patterns
    assert_eq!(escapes.len(), 3);

    // Check unsafe.Pointer pattern
    let unsafe_ptr = escapes.iter().find(|e| e.name == "unsafe_pointer").unwrap();
    assert_eq!(unsafe_ptr.pattern, r"unsafe\.Pointer");
    assert_eq!(unsafe_ptr.comment, Some("// SAFETY:"));

    // Check go:linkname pattern
    let linkname = escapes.iter().find(|e| e.name == "go_linkname").unwrap();
    assert_eq!(linkname.pattern, r"//go:linkname");
    assert_eq!(linkname.comment, Some("// LINKNAME:"));

    // Check go:noescape pattern
    let noescape = escapes.iter().find(|e| e.name == "go_noescape").unwrap();
    assert_eq!(noescape.pattern, r"//go:noescape");
    assert_eq!(noescape.comment, Some("// NOESCAPE:"));
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
