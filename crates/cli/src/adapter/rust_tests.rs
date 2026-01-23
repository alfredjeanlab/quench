#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use super::*;

mod classification {
    use super::*;

    #[test]
    fn source_file_in_src() {
        let adapter = RustAdapter::new();
        assert_eq!(adapter.classify(Path::new("src/lib.rs")), FileKind::Source);
        assert_eq!(adapter.classify(Path::new("src/main.rs")), FileKind::Source);
        assert_eq!(
            adapter.classify(Path::new("src/foo/bar.rs")),
            FileKind::Source
        );
    }

    #[test]
    fn test_file_in_tests_dir() {
        let adapter = RustAdapter::new();
        assert_eq!(
            adapter.classify(Path::new("tests/integration.rs")),
            FileKind::Test
        );
        assert_eq!(
            adapter.classify(Path::new("tests/foo/bar.rs")),
            FileKind::Test
        );
    }

    #[test]
    fn test_file_with_suffix() {
        let adapter = RustAdapter::new();
        assert_eq!(
            adapter.classify(Path::new("src/lib_test.rs")),
            FileKind::Test
        );
        assert_eq!(
            adapter.classify(Path::new("src/lib_tests.rs")),
            FileKind::Test
        );
    }

    #[test]
    fn ignored_target_dir() {
        let adapter = RustAdapter::new();
        assert_eq!(
            adapter.classify(Path::new("target/debug/deps/foo.rs")),
            FileKind::Other
        );
        assert_eq!(
            adapter.classify(Path::new("target/release/build/bar.rs")),
            FileKind::Other
        );
    }

    #[test]
    fn non_rust_file() {
        let adapter = RustAdapter::new();
        assert_eq!(adapter.classify(Path::new("Cargo.toml")), FileKind::Other);
        assert_eq!(adapter.classify(Path::new("README.md")), FileKind::Other);
    }
}

mod ignore_patterns {
    use super::*;

    #[test]
    fn target_dir_ignored() {
        let adapter = RustAdapter::new();
        assert!(adapter.should_ignore(Path::new("target/debug/foo.rs")));
        assert!(adapter.should_ignore(Path::new("target/release/bar.rs")));
    }

    #[test]
    fn src_not_ignored() {
        let adapter = RustAdapter::new();
        assert!(!adapter.should_ignore(Path::new("src/lib.rs")));
        assert!(!adapter.should_ignore(Path::new("tests/test.rs")));
    }
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

mod default_escapes {
    use super::*;
    use crate::adapter::{Adapter, EscapeAction};

    #[test]
    fn returns_four_default_patterns() {
        let adapter = RustAdapter::new();
        let patterns = adapter.default_escapes();
        assert_eq!(patterns.len(), 4);
    }

    #[test]
    fn unsafe_pattern_requires_safety_comment() {
        let adapter = RustAdapter::new();
        let patterns = adapter.default_escapes();
        let unsafe_pattern = patterns.iter().find(|p| p.name == "unsafe").unwrap();

        assert_eq!(unsafe_pattern.action, EscapeAction::Comment);
        assert_eq!(unsafe_pattern.comment, Some("// SAFETY:"));
    }

    #[test]
    fn unwrap_pattern_is_forbidden() {
        let adapter = RustAdapter::new();
        let patterns = adapter.default_escapes();
        let unwrap_pattern = patterns.iter().find(|p| p.name == "unwrap").unwrap();

        assert_eq!(unwrap_pattern.action, EscapeAction::Forbid);
    }

    #[test]
    fn expect_pattern_is_forbidden() {
        let adapter = RustAdapter::new();
        let patterns = adapter.default_escapes();
        let expect_pattern = patterns.iter().find(|p| p.name == "expect").unwrap();

        assert_eq!(expect_pattern.action, EscapeAction::Forbid);
    }

    #[test]
    fn transmute_pattern_requires_safety_comment() {
        let adapter = RustAdapter::new();
        let patterns = adapter.default_escapes();
        let transmute_pattern = patterns.iter().find(|p| p.name == "transmute").unwrap();

        assert_eq!(transmute_pattern.action, EscapeAction::Comment);
        assert_eq!(transmute_pattern.comment, Some("// SAFETY:"));
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
