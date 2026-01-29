// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Zero-config defaults specs.
//!
//! These tests verify the default behavior with minimal or no configuration.
//! Reference: docs/specs/checks/agents.md#zero-config-defaults

use crate::prelude::*;

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > required = ["*"] - At least one agent file must exist
#[test]
fn default_requires_at_least_one_agent_file() {
    let temp = Project::empty();
    temp.config("");
    // No agent files created

    let result = check("agents").pwd(temp.path()).json().fails();
    assert!(
        result.has_violation("missing_file"),
        "should fail with missing_file when no agent file exists"
    );
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > sync = true - Multiple agent files must stay in sync
#[test]
fn default_sync_enabled_detects_out_of_sync_files() {
    let temp = Project::empty();
    temp.config("");
    // Create two agent files with different content
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Directory Structure\n\nLayout A\n\n## Landing the Plane\n\n- Done\n",
    );
    temp.file(
        ".cursorrules",
        "# Project\n\n## Directory Structure\n\nLayout B\n\n## Landing the Plane\n\n- Done\n",
    );

    let result = check("agents").pwd(temp.path()).json().fails();
    assert!(
        result.has_violation("out_of_sync"),
        "should fail with out_of_sync when files differ (sync enabled by default)"
    );
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > tables = "allow" - Markdown tables allowed by default
#[test]
fn default_allows_markdown_tables() {
    let temp = Project::empty();
    temp.config("");
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Directory Structure\n\nLayout\n\n## Commands\n\n| Cmd | Desc |\n|-----|------|\n| a | b |\n\n## Landing the Plane\n\n- Done\n",
    );

    // Should pass - tables allowed by default
    check("agents").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > max_lines = 500 - Files over 500 lines generate violations
#[test]
fn default_max_lines_500() {
    let temp = Project::empty();
    temp.config("");

    // Create agent file with 501 lines
    let mut content = String::from(
        "# Project\n\n## Directory Structure\n\nLayout\n\n## Landing the Plane\n\n- Done\n\n## Extra\n\n",
    );
    for i in 0..490 {
        content.push_str(&format!("Line {}\n", i));
    }
    temp.file("CLAUDE.md", &content);

    let result = check("agents").pwd(temp.path()).json().fails();
    assert!(
        result.has_violation("file_too_large"),
        "should fail with file_too_large when over 500 lines (default max_lines)"
    );
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > max_tokens = 20000 - Files over ~20k tokens generate violations
#[test]
fn default_max_tokens_20000() {
    let temp = Project::empty();
    temp.config("");

    // Create agent file with ~21k tokens (84k chars / 4 = 21k tokens)
    let mut content = String::from(
        "# Project\n\n## Directory Structure\n\nLayout\n\n## Landing the Plane\n\n- Done\n\n## Content\n\n",
    );
    // Add enough content to exceed 20k tokens (need > 80k chars)
    for _ in 0..850 {
        content.push_str("This is a line of content that adds tokens to the file for testing. ");
        content.push_str("More content here to bulk up the file size significantly.\n");
    }
    temp.file("CLAUDE.md", &content);

    let result = check("agents").pwd(temp.path()).json().fails();
    assert!(
        result.has_violation("file_too_large"),
        "should fail with file_too_large when over 20k tokens (default max_tokens)"
    );
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > sections.required = ["Directory Structure", "Landing the Plane"]
#[test]
fn default_requires_directory_structure_section() {
    let temp = Project::empty();
    temp.config("");
    temp.file("CLAUDE.md", "# Project\n\n## Landing the Plane\n\n- Done\n");

    let result = check("agents").pwd(temp.path()).json().fails();
    let violations = result.violations();

    let has_missing_dir_structure = violations.iter().any(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("missing_section")
            && v.get("advice")
                .and_then(|a| a.as_str())
                .map(|a| a.contains("Directory Structure"))
                .unwrap_or(false)
    });

    assert!(
        has_missing_dir_structure,
        "should fail with missing_section for 'Directory Structure'"
    );
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > sections.required = ["Directory Structure", "Landing the Plane"]
#[test]
fn default_requires_landing_the_plane_section() {
    let temp = Project::empty();
    temp.config("");
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Directory Structure\n\nLayout\n",
    );

    let result = check("agents").pwd(temp.path()).json().fails();
    let violations = result.violations();

    let has_missing_landing = violations.iter().any(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("missing_section")
            && v.get("advice")
                .and_then(|a| a.as_str())
                .map(|a| a.contains("Landing the Plane"))
                .unwrap_or(false)
    });

    assert!(
        has_missing_landing,
        "should fail with missing_section for 'Landing the Plane'"
    );
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > box_diagrams = "allow" - ASCII diagrams allowed by default
#[test]
fn default_allows_box_diagrams() {
    let temp = Project::empty();
    temp.config("");
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Directory Structure\n\n┌─────┐\n│ Box │\n└─────┘\n\n## Landing the Plane\n\n- Done\n",
    );

    // Should pass - box diagrams allowed by default
    check("agents").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > mermaid = "allow" - Mermaid blocks allowed by default
#[test]
fn default_allows_mermaid_blocks() {
    let temp = Project::empty();
    temp.config("");
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Directory Structure\n\n```mermaid\ngraph TD\n  A --> B\n```\n\n## Landing the Plane\n\n- Done\n",
    );

    // Should pass - mermaid allowed by default
    check("agents").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > A valid project with all defaults satisfied should pass
#[test]
fn default_passes_with_valid_agent_file() {
    let temp = Project::empty();
    temp.config("");
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Directory Structure\n\nLayout here.\n\n## Landing the Plane\n\n- Run tests\n",
    );

    // Should pass with all defaults
    check("agents").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > Disabling defaults with explicit config should work
#[test]
fn can_disable_defaults_with_explicit_config() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
required = []
sync = false
tables = "allow"
max_lines = false
max_tokens = false
sections.required = []
"#,
    );

    // No agent file, but required = [] so it's fine
    // Should pass with all checks disabled
    check("agents").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/agents.md#section-validation
///
/// > Required sections are only enforced at root scope, not packages/modules
#[test]
fn default_sections_only_enforced_at_root_scope() {
    let temp = Project::empty();
    temp.config(
        r#"[project]
packages = ["crates/mylib"]
"#,
    );
    // Root file has required sections - should pass
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n",
    );
    // Package file MISSING required sections - should still pass
    // because sections are only enforced at root scope
    temp.file(
        "crates/mylib/CLAUDE.md",
        "# Package Notes\n\nJust some notes, no required sections.\n",
    );

    // Should pass - package file doesn't need required sections
    check("agents").pwd(temp.path()).passes();
}
