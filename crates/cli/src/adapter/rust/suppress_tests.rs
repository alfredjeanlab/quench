// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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
    assert_eq!(attrs[0].comment_text, Some("This is needed for FFI compatibility".to_string()));
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

    assert_eq!(attrs[0].codes, vec!["clippy::unwrap_used", "clippy::expect_used"]);
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

// =============================================================================
// INNER ATTRIBUTE TESTS
// =============================================================================

#[test]
fn detects_inner_allow_attribute() {
    let content = "#![allow(dead_code)]\nfn unused() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].kind, "allow");
    assert_eq!(attrs[0].codes, vec!["dead_code"]);
}

#[test]
fn detects_inner_expect_attribute() {
    let content = "#![expect(unused)]\nlet _x = 42;";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].kind, "expect");
    assert_eq!(attrs[0].codes, vec!["unused"]);
}

#[test]
fn detects_inner_attribute_with_comment() {
    let content = "// Module suppression for FFI compatibility\n#![allow(unsafe_code)]\n";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert!(attrs[0].has_comment);
    assert_eq!(attrs[0].comment_text, Some("Module suppression for FFI compatibility".to_string()));
}

#[test]
fn detects_mixed_inner_and_outer_attributes() {
    let content = "#![allow(dead_code)]\n\n#[allow(unused)]\nfn f() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 2);
    assert_eq!(attrs[0].codes, vec!["dead_code"]);
    assert_eq!(attrs[1].codes, vec!["unused"]);
}

#[test]
fn inner_attribute_with_multiple_codes() {
    let content = "#![allow(dead_code, unused_variables, clippy::unwrap_used)]\n";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].codes, vec!["dead_code", "unused_variables", "clippy::unwrap_used"]);
}

// =============================================================================
// MULTI-LINE ATTRIBUTE TESTS
// =============================================================================

#[test]
fn multiline_allow_attribute() {
    let content = "#[allow(\n    dead_code,\n    unused_variables\n)]\nfn unused() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1, "should detect multi-line allow attribute");
    assert_eq!(attrs[0].kind, "allow");
    assert_eq!(attrs[0].codes, vec!["dead_code", "unused_variables"]);
    assert_eq!(attrs[0].line, 0, "line should be start of attribute");
}

#[test]
fn multiline_expect_attribute() {
    let content = "#[expect(\n    unused\n)]\nfn f() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].kind, "expect");
    assert_eq!(attrs[0].codes, vec!["unused"]);
}

#[test]
fn multiline_allow_with_comment() {
    let content = "// This is a justification comment\n#[allow(\n    dead_code\n)]\nfn f() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert!(attrs[0].has_comment);
    assert_eq!(attrs[0].comment_text, Some("This is a justification comment".to_string()));
}

#[test]
fn multiline_inner_allow_attribute() {
    let content = "#![allow(\n    dead_code,\n    unused\n)]\nfn f() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].kind, "allow");
    assert_eq!(attrs[0].codes, vec!["dead_code", "unused"]);
}

#[test]
fn multiline_allow_with_clippy_lints() {
    let content = "#[allow(\n    clippy::unwrap_used,\n    clippy::expect_used\n)]\nfn f() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].codes, vec!["clippy::unwrap_used", "clippy::expect_used"]);
}

#[test]
fn mixed_single_and_multiline_attributes() {
    let content = "#[allow(dead_code)]\nfn a() {}\n\n#[allow(\n    unused\n)]\nfn b() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 2);
    assert_eq!(attrs[0].codes, vec!["dead_code"]);
    assert_eq!(attrs[0].line, 0);
    assert_eq!(attrs[1].codes, vec!["unused"]);
    assert_eq!(attrs[1].line, 3);
}
