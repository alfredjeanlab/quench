// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn parses_minimal_config() {
    let path = PathBuf::from("quench.toml");
    let config = parse("version = 1\n", &path).unwrap();
    assert_eq!(config.version, 1);
}

#[test]
fn parses_config_with_project() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[project]
name = "test-project"
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.version, 1);
    assert_eq!(config.project.name, Some("test-project".to_string()));
}

#[test]
fn rejects_missing_version() {
    let path = PathBuf::from("quench.toml");
    let result = parse("", &path);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("missing required field: version"));
}

#[test]
fn rejects_unsupported_version() {
    let path = PathBuf::from("quench.toml");
    let result = parse("version = 2\n", &path);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("unsupported config version 2"));
}

#[test]
fn rejects_version_zero() {
    let path = PathBuf::from("quench.toml");
    let result = parse("version = 0\n", &path);
    assert!(result.is_err());
}

#[test]
fn load_reads_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("quench.toml");
    fs::write(&config_path, "version = 1\n").unwrap();

    let config = load(&config_path).unwrap();
    assert_eq!(config.version, 1);
}

#[test]
fn load_fails_on_missing_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("nonexistent.toml");

    let result = load(&config_path);
    assert!(result.is_err());
}

// Unknown key validation tests

#[test]
fn parse_rejects_unknown_top_level_key() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1
unknown_key = true
"#;
    let result = parse(content, &path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unknown field"));
}

#[test]
fn parse_rejects_unknown_nested_key() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.unknown]
field = "value"
"#;
    let result = parse(content, &path);
    assert!(result.is_err());
}

#[test]
fn parse_preserves_known_fields() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[project]
name = "test"
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.version, 1);
    assert_eq!(config.project.name, Some("test".to_string()));
}

#[test]
fn parse_rejects_invalid_version() {
    let path = PathBuf::from("quench.toml");
    let result = parse("version = 99\n", &path);
    assert!(result.is_err());
}

// max_tokens config tests

#[test]
fn parse_max_tokens_default() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.check.cloc.max_tokens, Some(20000));
}

#[test]
fn parse_max_tokens_custom() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.cloc]
max_tokens = 10000
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.check.cloc.max_tokens, Some(10000));
}

#[test]
fn parse_max_tokens_false_disables() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.cloc]
max_tokens = false
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.check.cloc.max_tokens, None);
}

// Rust policy config tests

#[test]
fn parse_rust_policy_default() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.rust.policy.lint_changes, LintChangesPolicy::None);
    assert_eq!(config.rust.policy.lint_config.len(), 4);
    assert!(
        config
            .rust
            .policy
            .lint_config
            .contains(&"rustfmt.toml".to_string())
    );
}

#[test]
fn parse_rust_policy_standalone() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust.policy]
lint_changes = "standalone"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(
        config.rust.policy.lint_changes,
        LintChangesPolicy::Standalone
    );
}

#[test]
fn parse_rust_policy_custom_lint_config() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust.policy]
lint_changes = "standalone"
lint_config = ["rustfmt.toml", "custom-lint.toml"]
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(
        config.rust.policy.lint_changes,
        LintChangesPolicy::Standalone
    );
    assert_eq!(config.rust.policy.lint_config.len(), 2);
    assert!(
        config
            .rust
            .policy
            .lint_config
            .contains(&"custom-lint.toml".to_string())
    );
}

#[test]
fn parse_rust_policy_none_explicit() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust.policy]
lint_changes = "none"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.rust.policy.lint_changes, LintChangesPolicy::None);
}

// Shell config tests

#[test]
fn shell_suppress_defaults_to_forbid() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.shell.suppress.check, SuppressLevel::Forbid);
}

#[test]
fn shell_suppress_test_defaults_to_allow() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.shell.suppress.test.check, Some(SuppressLevel::Allow));
}

#[test]
fn shell_suppress_can_be_comment() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[shell.suppress]
check = "comment"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.shell.suppress.check, SuppressLevel::Comment);
}

#[test]
fn shell_suppress_allow_list() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[shell.suppress.source]
allow = ["SC2034", "SC2086"]
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert!(
        config
            .shell
            .suppress
            .source
            .allow
            .contains(&"SC2034".to_string())
    );
    assert!(
        config
            .shell
            .suppress
            .source
            .allow
            .contains(&"SC2086".to_string())
    );
}

#[test]
fn shell_suppress_forbid_list() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[shell.suppress.source]
forbid = ["SC2006"]
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert!(
        config
            .shell
            .suppress
            .source
            .forbid
            .contains(&"SC2006".to_string())
    );
}

#[test]
fn shell_suppress_comment_pattern() {
    let path = PathBuf::from("quench.toml");
    let content = r##"
version = 1

[shell.suppress]
check = "comment"
comment = "# OK:"
"##;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.shell.suppress.comment, Some("# OK:".to_string()));
}

#[test]
fn shell_default_source_patterns() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();
    assert!(config.shell.source.contains(&"**/*.sh".to_string()));
    assert!(config.shell.source.contains(&"**/*.bash".to_string()));
}

#[test]
fn shell_default_test_patterns() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();
    assert!(config.shell.tests.contains(&"tests/**/*.bats".to_string()));
    assert!(config.shell.tests.contains(&"**/*_test.sh".to_string()));
}

// Per-lint pattern tests

#[test]
fn rust_suppress_per_lint_patterns() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust.suppress.source.dead_code]
comment = "// NOTE(compat):"

[rust.suppress.source.unused_variables]
comment = "// KEEP:"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(
        config.rust.suppress.source.patterns.get("dead_code"),
        Some(&vec!["// NOTE(compat):".to_string()])
    );
    assert_eq!(
        config.rust.suppress.source.patterns.get("unused_variables"),
        Some(&vec!["// KEEP:".to_string()])
    );
}

#[test]
fn shell_suppress_per_lint_patterns() {
    let path = PathBuf::from("quench.toml");
    let content = r##"
version = 1

[shell.suppress.source.SC2034]
comment = "# UNUSED_VAR:"

[shell.suppress.source.SC2086]
comment = "# UNQUOTED_OK:"
"##;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(
        config.shell.suppress.source.patterns.get("SC2034"),
        Some(&vec!["# UNUSED_VAR:".to_string()])
    );
    assert_eq!(
        config.shell.suppress.source.patterns.get("SC2086"),
        Some(&vec!["# UNQUOTED_OK:".to_string()])
    );
}

#[test]
fn suppress_patterns_override_defaults() {
    // When user specifies a [rust.suppress.source] section, defaults are preserved
    // unless explicitly overridden per-lint
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust.suppress.source]
allow = ["dead_code"]
forbid = ["unsafe_code"]
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    // Default patterns should still be present
    assert!(
        config
            .rust
            .suppress
            .source
            .patterns
            .contains_key("dead_code")
    );
}

#[test]
fn suppress_patterns_coexist_with_allow_forbid() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust.suppress.source]
allow = ["clippy::unwrap_used"]
forbid = ["unsafe_code"]

[rust.suppress.source.dead_code]
comment = "// LEGACY:"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert!(
        config
            .rust
            .suppress
            .source
            .allow
            .contains(&"clippy::unwrap_used".to_string())
    );
    assert!(
        config
            .rust
            .suppress
            .source
            .forbid
            .contains(&"unsafe_code".to_string())
    );
    // User-specified pattern overrides default
    assert_eq!(
        config.rust.suppress.source.patterns.get("dead_code"),
        Some(&vec!["// LEGACY:".to_string()])
    );
}

#[test]
fn suppress_patterns_inline_array_syntax() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust.suppress.source]
dead_code = ["// KEEP UNTIL:", "// NOTE(compat):"]
deprecated = "// TODO(refactor):"
"#;
    let config = parse_with_warnings(content, &path).unwrap();

    // Array syntax
    assert_eq!(
        config.rust.suppress.source.patterns.get("dead_code"),
        Some(&vec![
            "// KEEP UNTIL:".to_string(),
            "// NOTE(compat):".to_string()
        ])
    );

    // String syntax (converted to single-element array)
    assert_eq!(
        config.rust.suppress.source.patterns.get("deprecated"),
        Some(&vec!["// TODO(refactor):".to_string()])
    );
}

#[test]
fn rust_suppress_source_has_defaults() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();

    // Default patterns should be present
    assert!(
        config
            .rust
            .suppress
            .source
            .patterns
            .contains_key("dead_code")
    );
    assert!(
        config
            .rust
            .suppress
            .source
            .patterns
            .contains_key("clippy::too_many_arguments")
    );
    assert!(
        config
            .rust
            .suppress
            .source
            .patterns
            .contains_key("clippy::cast_possible_truncation")
    );
    assert!(
        config
            .rust
            .suppress
            .source
            .patterns
            .contains_key("deprecated")
    );

    // Check dead_code has multiple patterns
    let dead_code_patterns = config
        .rust
        .suppress
        .source
        .patterns
        .get("dead_code")
        .unwrap();
    assert!(dead_code_patterns.contains(&"// KEEP UNTIL:".to_string()));
    assert!(dead_code_patterns.contains(&"// NOTE(compat):".to_string()));
    assert!(dead_code_patterns.contains(&"// NOTE(lifetime):".to_string()));
}

#[test]
fn shell_suppress_source_has_no_defaults() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();

    // Shell should have empty patterns (defaults to forbid anyway)
    assert!(config.shell.suppress.source.patterns.is_empty());
}

#[test]
fn go_suppress_source_has_no_defaults() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();

    // Go should have empty patterns
    assert!(config.golang.suppress.source.patterns.is_empty());
}

// Per-language cloc config tests

#[test]
fn rust_cloc_check_level_parses_warn() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust.cloc]
check = "warn"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert!(config.rust.cloc.is_some());
    assert_eq!(
        config.rust.cloc.as_ref().unwrap().check,
        Some(CheckLevel::Warn)
    );
    assert_eq!(
        config.cloc_check_level_for_language("rust"),
        CheckLevel::Warn
    );
}

#[test]
fn rust_cloc_check_level_parses_off() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust.cloc]
check = "off"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(
        config.cloc_check_level_for_language("rust"),
        CheckLevel::Off
    );
}

#[test]
fn rust_cloc_check_level_inherits_from_global() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.cloc]
check = "warn"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    // No [rust.cloc] section, so should inherit global
    assert_eq!(
        config.cloc_check_level_for_language("rust"),
        CheckLevel::Warn
    );
}

#[test]
fn rust_cloc_advice_new_style_takes_precedence() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust.cloc]
advice = "New style advice"

[rust]
cloc_advice = "Old style advice"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    // New style [rust.cloc].advice should take precedence over old style [rust].cloc_advice
    assert_eq!(config.cloc_advice_for_language("rust"), "New style advice");
}

// cloc advice tests

#[test]
fn default_cloc_advice_includes_avoid_picking() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();

    assert!(config.check.cloc.advice.contains("Avoid picking"));
    assert!(
        config
            .check
            .cloc
            .advice
            .contains("refactoring out testable code blocks")
    );
}

#[test]
fn cloc_advice_for_language_uses_language_defaults() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();

    // Known languages should use their language-specific defaults
    assert_eq!(
        config.cloc_advice_for_language("rust"),
        RustConfig::default_cloc_advice()
    );
    assert_eq!(
        config.cloc_advice_for_language("go"),
        GoConfig::default_cloc_advice()
    );
    assert_eq!(
        config.cloc_advice_for_language("shell"),
        ShellConfig::default_cloc_advice()
    );
    // Unknown languages fall back to generic advice
    assert_eq!(
        config.cloc_advice_for_language("unknown"),
        &config.check.cloc.advice
    );
}

#[test]
fn rust_cloc_advice_overrides_default() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust]
cloc_advice = "Custom Rust advice here"
"#;
    let config = parse_with_warnings(content, &path).unwrap();

    assert_eq!(
        config.cloc_advice_for_language("rust"),
        "Custom Rust advice here"
    );
    // Other languages still use their defaults
    assert_eq!(
        config.cloc_advice_for_language("go"),
        GoConfig::default_cloc_advice()
    );
}

#[test]
fn go_cloc_advice_overrides_default() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[golang]
cloc_advice = "Custom Go advice here"
"#;
    let config = parse_with_warnings(content, &path).unwrap();

    assert_eq!(
        config.cloc_advice_for_language("go"),
        "Custom Go advice here"
    );
    // Other languages still use their defaults
    assert_eq!(
        config.cloc_advice_for_language("rust"),
        RustConfig::default_cloc_advice()
    );
}

#[test]
fn shell_cloc_advice_overrides_default() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[shell]
cloc_advice = "Custom Shell advice here"
"#;
    let config = parse_with_warnings(content, &path).unwrap();

    assert_eq!(
        config.cloc_advice_for_language("shell"),
        "Custom Shell advice here"
    );
    // Other languages still use their defaults
    assert_eq!(
        config.cloc_advice_for_language("rust"),
        RustConfig::default_cloc_advice()
    );
}

// Ignore config tests

#[test]
fn ignore_shorthand_array() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[project]
ignore = ["*.snapshot", "testdata/**"]
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(
        config.project.ignore.patterns,
        vec!["*.snapshot", "testdata/**"]
    );
}

#[test]
fn ignore_full_table() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[project.ignore]
patterns = ["*.snapshot"]
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.project.ignore.patterns, vec!["*.snapshot"]);
}

// Specs content validation config tests

#[test]
fn specs_default_content_rules() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse(content, &path).unwrap();

    // Specs defaults: tables, box_diagrams, mermaid all allowed
    assert_eq!(config.check.docs.specs.tables, ContentRule::Allow);
    assert_eq!(config.check.docs.specs.box_diagrams, ContentRule::Allow);
    assert_eq!(config.check.docs.specs.mermaid, ContentRule::Allow);
}

#[test]
fn specs_default_size_limits() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse(content, &path).unwrap();

    // Specs defaults: 1000 max lines, 20000 max tokens
    assert_eq!(config.check.docs.specs.max_lines, Some(1000));
    assert_eq!(config.check.docs.specs.max_tokens, Some(20000));
}

#[test]
fn specs_tables_forbid() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.docs.specs]
tables = "forbid"
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.check.docs.specs.tables, ContentRule::Forbid);
}

#[test]
fn specs_box_diagrams_forbid() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.docs.specs]
box_diagrams = "forbid"
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.check.docs.specs.box_diagrams, ContentRule::Forbid);
}

#[test]
fn specs_mermaid_forbid() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.docs.specs]
mermaid = "forbid"
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.check.docs.specs.mermaid, ContentRule::Forbid);
}

#[test]
fn specs_custom_max_lines() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.docs.specs]
max_lines = 500
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.check.docs.specs.max_lines, Some(500));
}

#[test]
fn specs_max_lines_disabled() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.docs.specs]
max_lines = false
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.check.docs.specs.max_lines, None);
}

#[test]
fn specs_max_tokens_disabled() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.docs.specs]
max_tokens = false
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.check.docs.specs.max_tokens, None);
}

#[test]
fn specs_sections_required_simple() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.docs.specs]
sections.required = ["Purpose", "Overview"]
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.check.docs.specs.sections.required.len(), 2);
    assert_eq!(config.check.docs.specs.sections.required[0].name, "Purpose");
    assert_eq!(
        config.check.docs.specs.sections.required[1].name,
        "Overview"
    );
}

#[test]
fn specs_sections_required_extended() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[[check.docs.specs.sections.required]]
name = "Purpose"
advice = "Explain why this spec exists"
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.check.docs.specs.sections.required.len(), 1);
    assert_eq!(config.check.docs.specs.sections.required[0].name, "Purpose");
    assert_eq!(
        config.check.docs.specs.sections.required[0].advice,
        Some("Explain why this spec exists".to_string())
    );
}

#[test]
fn specs_sections_forbid() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.docs.specs]
sections.forbid = ["TODO", "Draft*"]
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.check.docs.specs.sections.forbid.len(), 2);
    assert!(
        config
            .check
            .docs
            .specs
            .sections
            .forbid
            .contains(&"TODO".to_string())
    );
    assert!(
        config
            .check
            .docs
            .specs
            .sections
            .forbid
            .contains(&"Draft*".to_string())
    );
}

#[test]
fn specs_default_sections_empty() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse(content, &path).unwrap();

    // Specs defaults: no required sections (unlike agents)
    assert!(config.check.docs.specs.sections.required.is_empty());
    assert!(config.check.docs.specs.sections.forbid.is_empty());
}
