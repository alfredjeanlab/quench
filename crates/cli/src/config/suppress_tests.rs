// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tests for suppress configuration parsing.

use super::*;
use std::path::PathBuf;

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

#[test]
fn javascript_suppress_check_allow_is_respected() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[javascript.suppress]
check = "allow"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.javascript.suppress.check, SuppressLevel::Allow);
    // Source scope should not have Rust-specific patterns for JavaScript
    assert!(
        config.javascript.suppress.source.patterns.is_empty(),
        "JavaScript suppress source should not inherit Rust-specific patterns, got: {:?}",
        config.javascript.suppress.source.patterns
    );
}

#[test]
fn javascript_suppress_source_has_no_defaults() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();

    // JavaScript should NOT have Rust-specific patterns
    assert!(
        config.javascript.suppress.source.patterns.is_empty(),
        "JavaScript suppress source should not inherit Rust-specific patterns, got: {:?}",
        config.javascript.suppress.source.patterns
    );
}

#[test]
fn javascript_suppress_test_defaults_to_allow() {
    let path = PathBuf::from("quench.toml");
    // When [javascript.suppress] is present, test scope should still default to allow
    let content = r#"
version = 1

[javascript.suppress]
check = "comment"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(
        config.javascript.suppress.test.check,
        Some(SuppressLevel::Allow),
        "JavaScript test scope should default to allow even when [javascript.suppress] is present"
    );
}
