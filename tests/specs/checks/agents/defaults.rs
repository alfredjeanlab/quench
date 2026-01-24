//! Zero-config defaults specs.
//!
//! These tests verify the default behavior with minimal or no configuration.
//! Reference: docs/specs/checks/agents.md#zero-config-defaults

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > required = ["*"] - At least one agent file must exist
#[test]
fn default_requires_at_least_one_agent_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();
    // No agent files created

    let result = check("agents").pwd(dir.path()).json().fails();
    let violations = result.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("missing_file")),
        "should fail with missing_file when no agent file exists"
    );
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > sync = true - Multiple agent files must stay in sync
#[test]
fn default_sync_enabled_detects_out_of_sync_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    // Create two agent files with different content
    std::fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project\n\n## Directory Structure\n\nLayout A\n\n## Landing the Plane\n\n- Done\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join(".cursorrules"),
        "# Project\n\n## Directory Structure\n\nLayout B\n\n## Landing the Plane\n\n- Done\n",
    )
    .unwrap();

    let result = check("agents").pwd(dir.path()).json().fails();
    let violations = result.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("out_of_sync")),
        "should fail with out_of_sync when files differ (sync enabled by default)"
    );
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > tables = "forbid" - Markdown tables generate violations
#[test]
fn default_forbids_markdown_tables() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    // Create agent file with a table
    std::fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project\n\n## Directory Structure\n\nLayout\n\n## Commands\n\n| Cmd | Desc |\n|-----|------|\n| a | b |\n\n## Landing the Plane\n\n- Done\n",
    )
    .unwrap();

    let result = check("agents").pwd(dir.path()).json().fails();
    let violations = result.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("forbidden_table")),
        "should fail with forbidden_table (tables forbidden by default)"
    );
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > max_lines = 500 - Files over 500 lines generate violations
#[test]
fn default_max_lines_500() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    // Create agent file with 501 lines
    let mut content = String::from(
        "# Project\n\n## Directory Structure\n\nLayout\n\n## Landing the Plane\n\n- Done\n\n## Extra\n\n",
    );
    for i in 0..490 {
        content.push_str(&format!("Line {}\n", i));
    }
    std::fs::write(dir.path().join("CLAUDE.md"), &content).unwrap();

    let result = check("agents").pwd(dir.path()).json().fails();
    let violations = result.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("file_too_large")),
        "should fail with file_too_large when over 500 lines (default max_lines)"
    );
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > max_tokens = 20000 - Files over ~20k tokens generate violations
#[test]
fn default_max_tokens_20000() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    // Create agent file with ~21k tokens (84k chars / 4 = 21k tokens)
    let mut content = String::from(
        "# Project\n\n## Directory Structure\n\nLayout\n\n## Landing the Plane\n\n- Done\n\n## Content\n\n",
    );
    // Add enough content to exceed 20k tokens (need > 80k chars)
    for _ in 0..850 {
        content.push_str("This is a line of content that adds tokens to the file for testing. ");
        content.push_str("More content here to bulk up the file size significantly.\n");
    }
    std::fs::write(dir.path().join("CLAUDE.md"), &content).unwrap();

    let result = check("agents").pwd(dir.path()).json().fails();
    let violations = result.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("file_too_large")),
        "should fail with file_too_large when over 20k tokens (default max_tokens)"
    );
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > sections.required = ["Directory Structure", "Landing the Plane"]
#[test]
fn default_requires_directory_structure_section() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    // Create agent file missing "Directory Structure"
    std::fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project\n\n## Landing the Plane\n\n- Done\n",
    )
    .unwrap();

    let result = check("agents").pwd(dir.path()).json().fails();
    let violations = result.require("violations").as_array().unwrap();

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
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    // Create agent file missing "Landing the Plane"
    std::fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project\n\n## Directory Structure\n\nLayout\n",
    )
    .unwrap();

    let result = check("agents").pwd(dir.path()).json().fails();
    let violations = result.require("violations").as_array().unwrap();

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
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    // Create agent file with box diagram
    std::fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project\n\n## Directory Structure\n\n┌─────┐\n│ Box │\n└─────┘\n\n## Landing the Plane\n\n- Done\n",
    )
    .unwrap();

    // Should pass - box diagrams allowed by default
    check("agents").pwd(dir.path()).passes();
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > mermaid = "allow" - Mermaid blocks allowed by default
#[test]
fn default_allows_mermaid_blocks() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    // Create agent file with mermaid block
    std::fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project\n\n## Directory Structure\n\n```mermaid\ngraph TD\n  A --> B\n```\n\n## Landing the Plane\n\n- Done\n",
    )
    .unwrap();

    // Should pass - mermaid allowed by default
    check("agents").pwd(dir.path()).passes();
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > A valid project with all defaults satisfied should pass
#[test]
fn default_passes_with_valid_agent_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    // Create minimal valid agent file
    std::fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project\n\n## Directory Structure\n\nLayout here.\n\n## Landing the Plane\n\n- Run tests\n",
    )
    .unwrap();

    // Should pass with all defaults
    check("agents").pwd(dir.path()).passes();
}

/// Spec: docs/specs/checks/agents.md#zero-config-defaults
///
/// > Disabling defaults with explicit config should work
#[test]
fn can_disable_defaults_with_explicit_config() {
    let dir = tempfile::tempdir().unwrap();

    // Disable all defaults
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
required = []
sync = false
tables = "allow"
max_lines = false
max_tokens = false
sections.required = []
"#,
    )
    .unwrap();

    // No agent file, but required = [] so it's fine
    // Should pass with all checks disabled
    check("agents").pwd(dir.path()).passes();
}

/// Spec: docs/specs/checks/agents.md#section-validation
///
/// > Required sections are only enforced at root scope, not packages/modules
#[test]
fn default_sections_only_enforced_at_root_scope() {
    let dir = tempfile::tempdir().unwrap();

    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[workspace]
packages = ["crates/mylib"]
"#,
    )
    .unwrap();

    // Root file has required sections - should pass
    std::fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n",
    )
    .unwrap();

    // Package file MISSING required sections - should still pass
    // because sections are only enforced at root scope
    std::fs::create_dir_all(dir.path().join("crates/mylib")).unwrap();
    std::fs::write(
        dir.path().join("crates/mylib/CLAUDE.md"),
        "# Package Notes\n\nJust some notes, no required sections.\n",
    )
    .unwrap();

    // Should pass - package file doesn't need required sections
    check("agents").pwd(dir.path()).passes();
}
