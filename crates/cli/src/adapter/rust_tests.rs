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

mod cfg_test_parsing {
    use super::*;

    #[test]
    fn basic_cfg_test_block() {
        let content = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_add() {
        assert_eq!(super::add(1, 2), 3);
    }
}
"#;
        let info = CfgTestInfo::parse(content);

        // Lines 0-4 are source (empty, pub fn, a+b, }, empty)
        // Lines 5-11 are test (#[cfg(test)], mod tests, #[test], fn, assert, }, })
        assert!(!info.is_test_line(1)); // pub fn add
        assert!(!info.is_test_line(2)); // a + b
        assert!(info.is_test_line(5)); // #[cfg(test)]
        assert!(info.is_test_line(6)); // mod tests
        assert!(info.is_test_line(11)); // closing brace
    }

    #[test]
    fn nested_braces_in_test() {
        let content = r#"
pub fn main() {}

#[cfg(test)]
mod tests {
    fn helper() {
        if true {
            println!("nested");
        }
    }
}
"#;
        let info = CfgTestInfo::parse(content);

        assert!(!info.is_test_line(1)); // pub fn main
        assert!(info.is_test_line(3)); // #[cfg(test)]
        assert!(info.is_test_line(7)); // nested println
        assert!(info.is_test_line(10)); // closing brace of mod tests
    }

    #[test]
    fn multiple_cfg_test_blocks() {
        let content = r#"
fn a() {}

#[cfg(test)]
mod tests_a {
    #[test]
    fn test_a() {}
}

fn b() {}

#[cfg(test)]
mod tests_b {
    #[test]
    fn test_b() {}
}
"#;
        let info = CfgTestInfo::parse(content);

        assert_eq!(info.test_ranges.len(), 2);
        assert!(!info.is_test_line(1)); // fn a()
        assert!(info.is_test_line(3)); // first #[cfg(test)]
        assert!(!info.is_test_line(9)); // fn b()
        assert!(info.is_test_line(11)); // second #[cfg(test)]
    }

    #[test]
    fn no_cfg_test_blocks() {
        let content = r#"
pub fn main() {
    println!("Hello");
}
"#;
        let info = CfgTestInfo::parse(content);

        assert!(info.test_ranges.is_empty());
        assert!(!info.is_test_line(0));
        assert!(!info.is_test_line(1));
    }

    #[test]
    fn cfg_test_with_spaces() {
        // #[cfg(test)] with extra whitespace inside
        let content = r#"
pub fn main() {}

#[cfg( test )]
mod tests {
    fn test() {}
}
"#;
        let info = CfgTestInfo::parse(content);

        assert!(!info.test_ranges.is_empty());
        assert!(info.is_test_line(3)); // #[cfg( test )]
    }

    #[test]
    fn string_literals_with_braces() {
        // Note: This test documents a known limitation
        // Braces in string literals may confuse the parser
        let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let s = "{ not a real brace }";
        assert!(true);
    }
}
"#;
        let info = CfgTestInfo::parse(content);

        // The parser may or may not handle this correctly
        // We just verify it doesn't panic and returns at least one range
        assert!(!info.test_ranges.is_empty());
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

mod suppress_parsing {
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
}

mod workspace {
    use super::*;
    use tempfile::TempDir;

    fn create_workspace(dir: &Path, manifest: &str) {
        std::fs::write(dir.join("Cargo.toml"), manifest).unwrap();
    }

    fn create_package(dir: &Path, name: &str) {
        let pkg_dir = dir.join(name);
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("Cargo.toml"),
            format!(
                r#"[package]
name = "{name}"
version = "0.1.0"
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn single_package() {
        let dir = TempDir::new().unwrap();
        create_workspace(
            dir.path(),
            r#"[package]
name = "my-project"
version = "0.1.0"
"#,
        );

        let workspace = CargoWorkspace::from_root(dir.path());
        assert!(!workspace.is_workspace);
        assert_eq!(workspace.packages, vec!["my-project"]);
        assert!(workspace.member_patterns.is_empty());
    }

    #[test]
    fn workspace_with_explicit_members() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("crates")).unwrap();
        create_package(&dir.path().join("crates"), "core");
        create_package(&dir.path().join("crates"), "cli");

        create_workspace(
            dir.path(),
            r#"[workspace]
members = ["crates/core", "crates/cli"]
"#,
        );

        let workspace = CargoWorkspace::from_root(dir.path());
        assert!(workspace.is_workspace);
        assert_eq!(workspace.packages, vec!["cli", "core"]);
        assert_eq!(workspace.member_patterns, vec!["crates/core", "crates/cli"]);
    }

    #[test]
    fn workspace_with_glob_members() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("crates")).unwrap();
        create_package(&dir.path().join("crates"), "core");
        create_package(&dir.path().join("crates"), "cli");

        create_workspace(
            dir.path(),
            r#"[workspace]
members = ["crates/*"]
"#,
        );

        let workspace = CargoWorkspace::from_root(dir.path());
        assert!(workspace.is_workspace);
        assert_eq!(workspace.packages, vec!["cli", "core"]);
        assert_eq!(workspace.member_patterns, vec!["crates/*"]);
    }

    #[test]
    fn no_cargo_toml() {
        let dir = TempDir::new().unwrap();
        let workspace = CargoWorkspace::from_root(dir.path());
        assert!(!workspace.is_workspace);
        assert!(workspace.packages.is_empty());
        assert!(workspace.member_patterns.is_empty());
    }
}

mod policy_checking {
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
    fn no_violation_when_only_source_changed() {
        let adapter = RustAdapter::new();
        let policy = default_policy();
        let files = [Path::new("src/lib.rs"), Path::new("src/main.rs")];
        let file_refs: Vec<&Path> = files.to_vec();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        assert!(!result.standalone_violated);
        assert!(result.changed_lint_config.is_empty());
        assert_eq!(result.changed_source.len(), 2);
    }

    #[test]
    fn no_violation_when_only_lint_config_changed() {
        let adapter = RustAdapter::new();
        let policy = default_policy();
        let files = [Path::new("rustfmt.toml"), Path::new("clippy.toml")];
        let file_refs: Vec<&Path> = files.to_vec();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        assert!(!result.standalone_violated);
        assert_eq!(result.changed_lint_config.len(), 2);
        assert!(result.changed_source.is_empty());
    }

    #[test]
    fn violation_when_both_changed() {
        let adapter = RustAdapter::new();
        let policy = default_policy();
        let files = [Path::new("rustfmt.toml"), Path::new("src/lib.rs")];
        let file_refs: Vec<&Path> = files.to_vec();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        assert!(result.standalone_violated);
        assert_eq!(result.changed_lint_config.len(), 1);
        assert_eq!(result.changed_source.len(), 1);
    }

    #[test]
    fn no_violation_when_policy_disabled() {
        let adapter = RustAdapter::new();
        let policy = RustPolicyConfig {
            lint_changes: LintChangesPolicy::None,
            ..default_policy()
        };
        let files = [Path::new("rustfmt.toml"), Path::new("src/lib.rs")];
        let file_refs: Vec<&Path> = files.to_vec();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        assert!(!result.standalone_violated);
    }

    #[test]
    fn detects_hidden_lint_config_files() {
        let adapter = RustAdapter::new();
        let policy = default_policy();
        let files = [Path::new(".rustfmt.toml"), Path::new("src/lib.rs")];
        let file_refs: Vec<&Path> = files.to_vec();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        assert!(result.standalone_violated);
        assert_eq!(result.changed_lint_config, vec![".rustfmt.toml"]);
    }

    #[test]
    fn detects_nested_lint_config_files() {
        let adapter = RustAdapter::new();
        let policy = default_policy();
        let files = [
            Path::new("crates/foo/rustfmt.toml"),
            Path::new("src/lib.rs"),
        ];
        let file_refs: Vec<&Path> = files.to_vec();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        assert!(result.standalone_violated);
        assert_eq!(result.changed_lint_config.len(), 1);
        assert!(result.changed_lint_config[0].contains("rustfmt.toml"));
    }

    #[test]
    fn test_files_count_as_source_for_policy() {
        let adapter = RustAdapter::new();
        let policy = default_policy();
        let files = [Path::new("rustfmt.toml"), Path::new("tests/test.rs")];
        let file_refs: Vec<&Path> = files.to_vec();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        // Test files should also trigger the violation
        assert!(result.standalone_violated);
        assert_eq!(result.changed_source.len(), 1);
    }

    #[test]
    fn custom_lint_config_list() {
        let adapter = RustAdapter::new();
        let policy = RustPolicyConfig {
            lint_changes: LintChangesPolicy::Standalone,
            lint_config: vec!["custom-lint.toml".to_string()],
        };
        let files = [Path::new("custom-lint.toml"), Path::new("src/lib.rs")];
        let file_refs: Vec<&Path> = files.to_vec();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        assert!(result.standalone_violated);
        assert_eq!(result.changed_lint_config, vec!["custom-lint.toml"]);
    }

    #[test]
    fn non_source_non_lint_files_ignored() {
        let adapter = RustAdapter::new();
        let policy = default_policy();
        let files = [
            Path::new("rustfmt.toml"),
            Path::new("README.md"),
            Path::new("Cargo.toml"),
        ];
        let file_refs: Vec<&Path> = files.to_vec();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        // Only lint config, no source files -> no violation
        assert!(!result.standalone_violated);
        assert_eq!(result.changed_lint_config.len(), 1);
        assert!(result.changed_source.is_empty());
    }
}
