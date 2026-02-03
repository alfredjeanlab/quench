// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use std::path::PathBuf;

fn parse_config(content: &str) -> Config {
    let path = PathBuf::from("quench.toml");
    parse(content, &path).unwrap()
}

// =============================================================================
// CheckLevel
// =============================================================================

#[test]
fn check_level_defaults_to_error() {
    assert_eq!(CheckLevel::default(), CheckLevel::Error);
}

#[test]
fn check_level_deserializes_all_variants() {
    let config = parse_config(
        r#"
version = 1
[check.cloc]
check = "warn"
"#,
    );
    assert_eq!(config.check.cloc.check, CheckLevel::Warn);

    let config = parse_config(
        r#"
version = 1
[check.cloc]
check = "off"
"#,
    );
    assert_eq!(config.check.cloc.check, CheckLevel::Off);
}

// =============================================================================
// ClocConfig
// =============================================================================

#[test]
fn cloc_config_defaults() {
    let config = parse_config("version = 1\n");
    assert_eq!(config.check.cloc.max_lines, 750);
    assert_eq!(config.check.cloc.max_lines_test, 1000);
    assert_eq!(config.check.cloc.max_tokens, Some(20000));
    assert_eq!(config.check.cloc.metric, LineMetric::Lines);
    assert_eq!(config.check.cloc.check, CheckLevel::Error);
}

#[test]
fn cloc_config_custom_max_lines() {
    let config = parse_config(
        r#"
version = 1
[check.cloc]
max_lines = 500
max_lines_test = 800
"#,
    );
    assert_eq!(config.check.cloc.max_lines, 500);
    assert_eq!(config.check.cloc.max_lines_test, 800);
}

#[test]
fn cloc_config_nonblank_metric() {
    let config = parse_config(
        r#"
version = 1
[check.cloc]
metric = "nonblank"
"#,
    );
    assert_eq!(config.check.cloc.metric, LineMetric::Nonblank);
}

#[test]
fn cloc_config_max_tokens_false_disables() {
    let config = parse_config(
        r#"
version = 1
[check.cloc]
max_tokens = false
"#,
    );
    assert_eq!(config.check.cloc.max_tokens, None);
}

#[test]
fn cloc_config_max_tokens_number() {
    let config = parse_config(
        r#"
version = 1
[check.cloc]
max_tokens = 10000
"#,
    );
    assert_eq!(config.check.cloc.max_tokens, Some(10000));
}

#[test]
fn cloc_config_custom_advice() {
    let config = parse_config(
        r#"
version = 1
[check.cloc]
advice = "Split this file"
"#,
    );
    assert_eq!(config.check.cloc.advice, "Split this file");
}

// =============================================================================
// EscapesConfig
// =============================================================================

#[test]
fn escapes_config_defaults() {
    let config = parse_config("version = 1\n");
    assert_eq!(config.check.escapes.check, CheckLevel::Error);
    assert!(config.check.escapes.exclude.is_empty());
    assert!(config.check.escapes.patterns.is_empty());
}

#[test]
fn escapes_config_with_patterns() {
    let config = parse_config(
        r#"
version = 1

[[check.escapes.patterns]]
name = "todo"
pattern = "TODO"
action = "count"
threshold = 5
"#,
    );
    assert_eq!(config.check.escapes.patterns.len(), 1);
    let pattern = &config.check.escapes.patterns[0];
    assert_eq!(pattern.effective_name(), "todo");
    assert_eq!(pattern.pattern, "TODO");
    assert_eq!(pattern.action, EscapeAction::Count);
    assert_eq!(pattern.threshold, 5);
}

#[test]
fn escape_pattern_effective_name_uses_pattern_as_fallback() {
    let config = parse_config(
        r#"
version = 1

[[check.escapes.patterns]]
pattern = "unsafe"
action = "forbid"
"#,
    );
    assert_eq!(config.check.escapes.patterns[0].effective_name(), "unsafe");
}

#[test]
fn escape_action_defaults_to_forbid() {
    assert_eq!(EscapeAction::default(), EscapeAction::Forbid);
}

// =============================================================================
// DocsConfig
// =============================================================================

#[test]
fn docs_config_defaults() {
    let config = parse_config("version = 1\n");
    assert!(config.check.docs.check.is_none());
    assert!(config.check.docs.area.is_empty());
    assert_eq!(config.check.docs.commit.check, "off");
}

#[test]
fn docs_commit_config_defaults() {
    let commit = DocsCommitConfig::default();
    assert_eq!(commit.check, "off");
    assert_eq!(commit.types, vec!["feat", "feature", "story", "breaking"]);
}

#[test]
fn docs_toc_config_defaults() {
    let config = parse_config("version = 1\n");
    assert!(config.check.docs.toc.check.is_none());
    assert!(
        config
            .check
            .docs
            .toc
            .include
            .contains(&"**/*.md".to_string())
    );
    assert!(
        config
            .check
            .docs
            .toc
            .exclude
            .contains(&"plans/**".to_string())
    );
}

// =============================================================================
// SpecsConfig
// =============================================================================

#[test]
fn specs_config_defaults() {
    let specs = SpecsConfig::default();
    assert_eq!(specs.path, "docs/specs");
    assert_eq!(specs.extension, ".md");
    assert_eq!(specs.index, "exists");
    assert_eq!(specs.max_lines, Some(1000));
    assert_eq!(specs.max_tokens, Some(20000));
}

// =============================================================================
// LangClocConfig
// =============================================================================

#[test]
fn lang_cloc_config_parses() {
    let config = parse_config(
        r#"
version = 1
[rust.cloc]
check = "warn"
advice = "Custom advice"
"#,
    );
    let lang_cloc = config.rust.cloc.unwrap();
    assert_eq!(lang_cloc.check, Some(CheckLevel::Warn));
    assert_eq!(lang_cloc.advice.unwrap(), "Custom advice");
}

#[test]
fn lang_cloc_config_defaults_to_none() {
    let config = parse_config("version = 1\n");
    assert!(config.rust.cloc.is_none());
    assert!(config.golang.cloc.is_none());
    assert!(config.javascript.cloc.is_none());
}
