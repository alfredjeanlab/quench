// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use super::*;
use crate::adapter::{Adapter, EscapeAction};
use yare::parameterized;

#[parameterized(
    src_lib = { "src/lib.rs", FileKind::Source },
    src_main = { "src/main.rs", FileKind::Source },
    src_nested = { "src/foo/bar.rs", FileKind::Source },
    tests_integration = { "tests/integration.rs", FileKind::Test },
    tests_nested = { "tests/foo/bar.rs", FileKind::Test },
    test_suffix_single = { "src/lib_test.rs", FileKind::Test },
    test_suffix_plural = { "src/lib_tests.rs", FileKind::Test },
    target_debug = { "target/debug/deps/foo.rs", FileKind::Other },
    target_release = { "target/release/build/bar.rs", FileKind::Other },
    cargo_toml = { "Cargo.toml", FileKind::Other },
    readme = { "README.md", FileKind::Other },
)]
fn classify_path(path: &str, expected: FileKind) {
    let adapter = RustAdapter::new();
    assert_eq!(
        adapter.classify(Path::new(path)),
        expected,
        "path {:?} should be {:?}",
        path,
        expected
    );
}

#[parameterized(
    target_debug = { "target/debug/foo.rs", true },
    target_release = { "target/release/bar.rs", true },
    src_lib = { "src/lib.rs", false },
    tests_test = { "tests/test.rs", false },
)]
fn should_ignore_path(path: &str, expected: bool) {
    let adapter = RustAdapter::new();
    assert_eq!(
        adapter.should_ignore(Path::new(path)),
        expected,
        "path {:?} should_ignore = {}",
        path,
        expected
    );
}

mod line_classification {
    use super::*;

    #[test]
    fn source_file_with_inline_tests() {
        let adapter = RustAdapter::new();
        let content = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(1, 2), 3);
    }
}
"#;
        let classification = adapter.classify_lines(Path::new("src/lib.rs"), content);

        // Source: pub fn add, a + b, } = 3 non-blank lines before #[cfg(test)]
        // Test: #[cfg(test)], mod tests, use super::*, #[test], fn test_add, assert_eq, }, } = 8 lines
        assert!(classification.source_lines > 0, "should have source lines");
        assert!(
            classification.test_lines > 0,
            "should have test lines from #[cfg(test)]"
        );
    }

    #[test]
    fn test_file_all_test_loc() {
        let adapter = RustAdapter::new();
        let content = r#"
use super::*;

#[test]
fn test_something() {
    assert!(true);
}
"#;
        let classification = adapter.classify_lines(Path::new("tests/test.rs"), content);

        assert_eq!(
            classification.source_lines, 0,
            "test file should have no source lines"
        );
        assert!(
            classification.test_lines > 0,
            "test file should have test lines"
        );
    }

    #[test]
    fn source_file_no_inline_tests() {
        let adapter = RustAdapter::new();
        let content = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#;
        let classification = adapter.classify_lines(Path::new("src/lib.rs"), content);

        assert!(classification.source_lines > 0, "should have source lines");
        assert_eq!(classification.test_lines, 0, "should have no test lines");
    }
}

#[parameterized(
    unsafe_requires_comment = { "unsafe", EscapeAction::Comment, Some("// SAFETY:") },
    transmute_requires_comment = { "transmute", EscapeAction::Comment, Some("// SAFETY:") },
    unwrap_forbidden = { "unwrap", EscapeAction::Forbid, None },
    expect_forbidden = { "expect", EscapeAction::Forbid, None },
)]
fn default_escape_pattern(
    name: &str,
    expected_action: EscapeAction,
    expected_comment: Option<&str>,
) {
    let adapter = RustAdapter::new();
    let patterns = adapter.default_escapes();
    let pattern = patterns
        .iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("pattern {:?} not found", name));

    assert_eq!(pattern.action, expected_action, "pattern {:?} action", name);
    assert_eq!(
        pattern.comment, expected_comment,
        "pattern {:?} comment",
        name
    );
}

#[test]
fn returns_four_default_patterns() {
    let adapter = RustAdapter::new();
    assert_eq!(adapter.default_escapes().len(), 4);
}

#[test]
fn all_patterns_have_advice() {
    let adapter = RustAdapter::new();
    for pattern in adapter.default_escapes() {
        assert!(
            !pattern.advice.is_empty(),
            "Pattern {} should have advice",
            pattern.name
        );
    }
}

// Integration tests for methods on RustAdapter that delegate to submodules

mod adapter_check_lint_policy {
    use super::*;
    use crate::config::{LintChangesPolicy, RustPolicyConfig};

    fn default_policy() -> RustPolicyConfig {
        RustPolicyConfig {
            lint_changes: LintChangesPolicy::Standalone,
            lint_config: vec![
                "rustfmt.toml".to_string(),
                ".rustfmt.toml".to_string(),
                "clippy.toml".to_string(),
                ".clippy.toml".to_string(),
            ],
        }
    }

    #[test]
    fn uses_adapter_classify_for_files() {
        let adapter = RustAdapter::new();
        let policy = default_policy();
        // Use test file paths that RustAdapter specifically classifies
        let files = [Path::new("src/lib.rs"), Path::new("tests/test.rs")];
        let file_refs: Vec<&Path> = files.to_vec();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        // Both should be classified as source/test by RustAdapter
        assert_eq!(result.changed_source.len(), 2);
    }

    #[test]
    fn delegates_to_policy_module() {
        let adapter = RustAdapter::new();
        let policy = default_policy();
        let files = [Path::new("rustfmt.toml"), Path::new("src/lib.rs")];
        let file_refs: Vec<&Path> = files.to_vec();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        // Verify policy module logic is invoked
        assert!(result.standalone_violated);
    }
}
