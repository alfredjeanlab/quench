// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn detects_allow_attribute() {
    let content = "#[allow(dead_code)]\nfn unused() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].kind, "allow");
    assert_eq!(attrs[0].codes, vec!["dead_code"]);
}

#[test]
fn detects_expect_attribute() {
    let content = "#[expect(unused)]\nlet _x = 42;";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].kind, "expect");
    assert_eq!(attrs[0].codes, vec!["unused"]);
}

#[test]
fn detects_multiple_codes() {
    let content = "#[allow(dead_code, unused_variables)]\nfn f() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].codes, vec!["dead_code", "unused_variables"]);
}

#[test]
fn detects_comment_justification() {
    let content = "// This is needed for FFI compatibility\n#[allow(unsafe_code)]\nfn ffi() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert!(attrs[0].has_comment);
    assert_eq!(
        attrs[0].comment_text,
        Some("This is needed for FFI compatibility".to_string())
    );
}

#[test]
fn no_comment_when_none_present() {
    let content = "#[allow(dead_code)]\nfn unused() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert!(!attrs[0].has_comment);
    assert!(attrs[0].comment_text.is_none());
}

#[test]
fn requires_specific_comment_pattern() {
    let content = "// Regular comment\n#[allow(dead_code)]\nfn f() {}";
    let attrs = parse_suppress_attrs(content, Some("// JUSTIFIED:"));

    // Regular comment doesn't match pattern
    assert!(!attrs[0].has_comment);
}

#[test]
fn matches_specific_comment_pattern() {
    let content = "// JUSTIFIED: Reserved for plugin system\n#[allow(dead_code)]\nfn f() {}";
    let attrs = parse_suppress_attrs(content, Some("// JUSTIFIED:"));

    assert!(attrs[0].has_comment);
}

#[test]
fn handles_multiple_attributes_on_item() {
    let content = "// Documented reason\n#[derive(Debug)]\n#[allow(dead_code)]\nstruct S;";
    let attrs = parse_suppress_attrs(content, None);

    // Should find the allow attribute and its comment (skipping #[derive])
    assert_eq!(attrs.len(), 1);
    assert!(attrs[0].has_comment);
}

#[test]
fn clippy_lint_codes() {
    let content = "#[allow(clippy::unwrap_used, clippy::expect_used)]\nfn f() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(
        attrs[0].codes,
        vec!["clippy::unwrap_used", "clippy::expect_used"]
    );
}

#[test]
fn multiple_suppress_attrs() {
    let content = "#[allow(dead_code)]\nfn a() {}\n\n#[expect(unused)]\nfn b() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 2);
    assert_eq!(attrs[0].kind, "allow");
    assert_eq!(attrs[1].kind, "expect");
}

#[test]
fn line_numbers_are_zero_indexed() {
    let content = "\n\n#[allow(dead_code)]\nfn f() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].line, 2); // 0-indexed, third line
}
