// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use std::path::Path;

use super::*;
use crate::config::{ContentRule, RequiredSection, SpecsConfig, SpecsSectionsConfig};

fn default_config() -> SpecsConfig {
    SpecsConfig::default()
}

#[test]
fn validates_clean_content() {
    let config = default_config();
    let content = "# My Spec\n\nSome content here.\n";

    let violations = validate_spec_content(Path::new("test.md"), content, &config);
    assert!(violations.is_empty());
}

// Section validation tests

#[test]
fn detects_missing_required_section() {
    let config = SpecsConfig {
        sections: SpecsSectionsConfig {
            required: vec![RequiredSection { name: "Purpose".to_string(), advice: None }],
            forbid: vec![],
        },
        ..default_config()
    };
    let content = "# My Spec\n\n## Overview\n\nSome content.\n";

    let violations = validate_spec_content(Path::new("test.md"), content, &config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "missing_section");
    assert!(violations[0].advice.contains("Purpose"));
}

#[test]
fn accepts_present_required_section() {
    let config = SpecsConfig {
        sections: SpecsSectionsConfig {
            required: vec![RequiredSection { name: "Purpose".to_string(), advice: None }],
            forbid: vec![],
        },
        ..default_config()
    };
    let content = "# My Spec\n\n## Purpose\n\nExplains the feature.\n";

    let violations = validate_spec_content(Path::new("test.md"), content, &config);
    assert!(violations.is_empty());
}

#[test]
fn includes_advice_in_missing_section_violation() {
    let config = SpecsConfig {
        sections: SpecsSectionsConfig {
            required: vec![RequiredSection {
                name: "Purpose".to_string(),
                advice: Some("Explain why this spec exists".to_string()),
            }],
            forbid: vec![],
        },
        ..default_config()
    };
    let content = "# My Spec\n\n## Overview\n\nSome content.\n";

    let violations = validate_spec_content(Path::new("test.md"), content, &config);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].advice.contains("Explain why this spec exists"));
}

#[test]
fn detects_forbidden_section() {
    let config = SpecsConfig {
        sections: SpecsSectionsConfig { required: vec![], forbid: vec!["TODO".to_string()] },
        ..default_config()
    };
    let content = "# My Spec\n\n## TODO\n\nFix this later.\n";

    let violations = validate_spec_content(Path::new("test.md"), content, &config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "forbidden_section");
}

#[test]
fn forbidden_section_glob_pattern() {
    let config = SpecsConfig {
        sections: SpecsSectionsConfig { required: vec![], forbid: vec!["Draft*".to_string()] },
        ..default_config()
    };
    let content = "# My Spec\n\n## Draft Notes\n\nWork in progress.\n";

    let violations = validate_spec_content(Path::new("test.md"), content, &config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "forbidden_section");
}

// Content rule tests

#[test]
fn tables_allowed_by_default() {
    let config = default_config();
    let content = "# Spec\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";

    let violations = validate_spec_content(Path::new("test.md"), content, &config);
    assert!(violations.is_empty());
}

#[test]
fn tables_forbidden_when_configured() {
    let config = SpecsConfig { tables: ContentRule::Forbid, ..default_config() };
    let content = "# Spec\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";

    let violations = validate_spec_content(Path::new("test.md"), content, &config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "forbidden_table");
}

#[test]
fn box_diagrams_allowed_by_default() {
    let config = default_config();
    let content = "# Spec\n\n┌───┐\n│ A │\n└───┘\n";

    let violations = validate_spec_content(Path::new("test.md"), content, &config);
    assert!(violations.is_empty());
}

#[test]
fn box_diagrams_forbidden_when_configured() {
    let config = SpecsConfig { box_diagrams: ContentRule::Forbid, ..default_config() };
    let content = "# Spec\n\n┌───┐\n│ A │\n└───┘\n";

    let violations = validate_spec_content(Path::new("test.md"), content, &config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "forbidden_diagram");
}

#[test]
fn mermaid_allowed_by_default() {
    let config = default_config();
    let content = "# Spec\n\n```mermaid\ngraph TD;\nA-->B;\n```\n";

    let violations = validate_spec_content(Path::new("test.md"), content, &config);
    assert!(violations.is_empty());
}

#[test]
fn mermaid_forbidden_when_configured() {
    let config = SpecsConfig { mermaid: ContentRule::Forbid, ..default_config() };
    let content = "# Spec\n\n```mermaid\ngraph TD;\nA-->B;\n```\n";

    let violations = validate_spec_content(Path::new("test.md"), content, &config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "forbidden_mermaid");
}

// Size limit tests

#[test]
fn within_line_limit() {
    let config = SpecsConfig { max_lines: Some(100), ..default_config() };
    let content = "line\n".repeat(50);

    let violations = validate_spec_content(Path::new("test.md"), &content, &config);
    assert!(violations.is_empty());
}

#[test]
fn exceeds_line_limit() {
    let config = SpecsConfig { max_lines: Some(10), ..default_config() };
    let content = "line\n".repeat(20);

    let violations = validate_spec_content(Path::new("test.md"), &content, &config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "spec_too_large");
    assert!(violations[0].advice.contains("lines"));
}

#[test]
fn line_limit_disabled() {
    let config = SpecsConfig { max_lines: None, ..default_config() };
    let content = "line\n".repeat(2000);

    let violations = validate_spec_content(Path::new("test.md"), &content, &config);
    // Only token limit might trigger, check for line-specific violations
    let line_violations: Vec<_> =
        violations.iter().filter(|v| v.advice.contains("lines")).collect();
    assert!(line_violations.is_empty());
}

#[test]
fn exceeds_token_limit() {
    let config = SpecsConfig {
        max_tokens: Some(100),
        max_lines: None, // Disable to isolate token test
        ..default_config()
    };
    // Each char is ~0.25 tokens, need 400+ chars to exceed 100 tokens
    let content = "a".repeat(500);

    let violations = validate_spec_content(Path::new("test.md"), &content, &config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "spec_too_large");
    assert!(violations[0].advice.contains("tokens"));
}

#[test]
fn token_limit_disabled() {
    let config = SpecsConfig { max_tokens: None, max_lines: None, ..default_config() };
    let content = "a".repeat(100000);

    let violations = validate_spec_content(Path::new("test.md"), &content, &config);
    assert!(violations.is_empty());
}

// Combined validation tests

#[test]
fn multiple_violations() {
    let config = SpecsConfig {
        sections: SpecsSectionsConfig {
            required: vec![RequiredSection { name: "Purpose".to_string(), advice: None }],
            forbid: vec!["TODO".to_string()],
        },
        tables: ContentRule::Forbid,
        ..default_config()
    };
    let content = "# Spec\n\n## TODO\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";

    let violations = validate_spec_content(Path::new("test.md"), content, &config);
    // Should have: missing Purpose, forbidden TODO section, forbidden table
    assert_eq!(violations.len(), 3);
}
