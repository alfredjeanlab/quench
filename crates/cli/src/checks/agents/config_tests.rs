// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn default_files_includes_claude_md() {
    let config = AgentsConfig::default();
    assert!(config.files.contains(&"CLAUDE.md".to_string()));
}

#[test]
fn default_files_includes_cursorrules() {
    let config = AgentsConfig::default();
    assert!(config.files.contains(&".cursorrules".to_string()));
}

#[test]
fn default_files_excludes_cursorignore() {
    let config = AgentsConfig::default();
    assert!(
        !config.files.contains(&".cursorignore".to_string()),
        ".cursorignore is an ignore file, not an agent context file"
    );
}

#[test]
fn default_files_includes_cursor_rules_md_glob() {
    let config = AgentsConfig::default();
    assert!(config.files.contains(&".cursor/rules/*.md".to_string()));
}

#[test]
fn default_files_includes_cursor_rules_mdc_glob() {
    let config = AgentsConfig::default();
    assert!(config.files.contains(&".cursor/rules/*.mdc".to_string()));
}

#[test]
fn default_config_requires_any_agent_file() {
    let config = AgentsConfig::default();
    assert_eq!(config.required, vec!["*".to_string()]);
}

#[test]
fn default_sync_is_enabled() {
    let config = AgentsConfig::default();
    assert!(config.sync);
    assert!(config.sync_from.is_none());
}

#[test]
fn default_tables_is_allow() {
    let config = AgentsConfig::default();
    assert_eq!(config.tables, ContentRule::Allow);
}

#[test]
fn default_box_diagrams_is_allow() {
    let config = AgentsConfig::default();
    assert_eq!(config.box_diagrams, ContentRule::Allow);
}

#[test]
fn default_mermaid_is_allow() {
    let config = AgentsConfig::default();
    assert_eq!(config.mermaid, ContentRule::Allow);
}

#[test]
fn default_max_lines_is_500() {
    let config = AgentsConfig::default();
    assert_eq!(config.max_lines, Some(500));
}

#[test]
fn default_max_tokens_is_20000() {
    let config = AgentsConfig::default();
    assert_eq!(config.max_tokens, Some(20000));
}

#[test]
fn content_rule_deserialize_allow() {
    let json = r#""allow""#;
    let rule: ContentRule = serde_json::from_str(json).unwrap();
    assert_eq!(rule, ContentRule::Allow);
}

#[test]
fn content_rule_deserialize_forbid() {
    let json = r#""forbid""#;
    let rule: ContentRule = serde_json::from_str(json).unwrap();
    assert_eq!(rule, ContentRule::Forbid);
}

#[test]
fn content_rule_deserialize_invalid() {
    let json = r#""invalid""#;
    let result: Result<ContentRule, _> = serde_json::from_str(json);
    assert!(result.is_err());
}
