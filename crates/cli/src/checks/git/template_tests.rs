// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for git template generation.

use super::*;
use crate::config::GitCommitConfig;

// =============================================================================
// TEMPLATE CONTENT TESTS
// =============================================================================

#[test]
fn generates_template_with_default_config() {
    let config = GitCommitConfig::default();
    let template = generate_template(&config);

    assert!(template.contains("# <type>(<scope>): <description>"));
    assert!(template.contains("# Types: feat, fix, chore"));
    assert!(template.contains("# Scope: optional"));
    assert!(template.contains("# Examples:"));
}

#[test]
fn generates_template_with_custom_types() {
    let config = GitCommitConfig {
        types: Some(vec!["feat".to_string(), "fix".to_string()]),
        ..Default::default()
    };
    let template = generate_template(&config);

    assert!(template.contains("# Types: feat, fix"));
    assert!(!template.contains("chore"));
}

#[test]
fn generates_template_with_scopes() {
    let config = GitCommitConfig {
        scopes: Some(vec!["api".to_string(), "cli".to_string()]),
        ..Default::default()
    };
    let template = generate_template(&config);

    assert!(template.contains("# <type>(<scope>): <description>"));
    assert!(template.contains("# Scope: optional (api, cli)"));
    assert!(template.contains("(api):")); // Example uses first scope
}

#[test]
fn generates_template_with_empty_types() {
    let config = GitCommitConfig { types: Some(vec![]), ..Default::default() };
    let template = generate_template(&config);

    assert!(template.contains("# Types: (any)"));
}

#[test]
fn generates_template_with_empty_scopes() {
    let config = GitCommitConfig { scopes: Some(vec![]), ..Default::default() };
    let template = generate_template(&config);

    assert!(template.contains("# Scope: optional"));
    assert!(!template.contains("# Scope: optional ("));
}

#[test]
fn template_ends_with_newline() {
    let config = GitCommitConfig::default();
    let template = generate_template(&config);

    assert!(template.ends_with('\n'));
}

#[test]
fn template_starts_with_blank_line() {
    let config = GitCommitConfig::default();
    let template = generate_template(&config);

    // Leading blank line so humans can start typing immediately
    assert!(template.starts_with('\n'));
}

#[test]
fn generates_examples_with_scopes() {
    let config = GitCommitConfig {
        types: Some(vec!["feat".to_string(), "fix".to_string()]),
        scopes: Some(vec!["api".to_string(), "cli".to_string()]),
        ..Default::default()
    };
    let template = generate_template(&config);

    // First example with scope
    assert!(template.contains("#   feat(api): add new feature"));
    // Second example without scope
    assert!(template.contains("#   fix: handle edge case"));
}

#[test]
fn generates_examples_without_scopes() {
    let config = GitCommitConfig {
        types: Some(vec!["feat".to_string(), "fix".to_string()]),
        scopes: None,
        ..Default::default()
    };
    let template = generate_template(&config);

    assert!(template.contains("#   feat: add new feature"));
    assert!(template.contains("#   fix: handle edge case"));
}
