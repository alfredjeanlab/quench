//! Behavioral specs for the agents check.
//!
//! Tests that quench correctly:
//! - Detects agent context files (CLAUDE.md, .cursorrules)
//! - Validates file synchronization
//! - Checks required/forbidden sections
//! - Enforces content rules (tables, max_lines, max_tokens)
//! - Generates correct violation types
//! - Outputs metrics in JSON format
//!
//! Reference: docs/specs/checks/agents.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// ZERO-CONFIG DEFAULTS SPECS
// =============================================================================
// These tests verify the default behavior with minimal or no configuration.
// Reference: docs/specs/checks/agents.md#zero-config-defaults

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

// =============================================================================
// FILE DETECTION SPECS (Phase 1)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#agent-files
///
/// > The agents check detects CLAUDE.md at the project root.
#[test]
fn agents_detects_claude_md_at_project_root() {
    let agents = check("agents").on("agents/basic").json().passes();
    let metrics = agents.require("metrics");
    let files_found = metrics.get("files_found").unwrap().as_array().unwrap();
    assert!(
        files_found.iter().any(|f| f.as_str() == Some("CLAUDE.md")),
        "should detect CLAUDE.md"
    );
}

/// Spec: docs/specs/checks/agents.md#agent-files
///
/// > The agents check detects .cursorrules at the project root.
#[test]
fn agents_detects_cursorrules_at_project_root() {
    let agents = check("agents").on("agents/basic").json().passes();
    let metrics = agents.require("metrics");
    let files_found = metrics.get("files_found").unwrap().as_array().unwrap();
    assert!(
        files_found
            .iter()
            .any(|f| f.as_str() == Some(".cursorrules")),
        "should detect .cursorrules"
    );
}

/// Spec: docs/specs/checks/agents.md#passing-check
///
/// > Check passes when all configured files exist and are valid.
#[test]
fn agents_passes_on_valid_project() {
    check("agents").on("agents/basic").passes();
}

// =============================================================================
// VIOLATION DETECTION SPECS (Phase 2)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#required-files
///
/// > Missing a required file generates a violation.
#[test]
fn agents_missing_required_file_generates_violation() {
    let agents = check("agents").on("agents/missing-file").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("missing_file") }),
        "should have missing_file violation"
    );
}

/// Spec: docs/specs/checks/agents.md#forbidden-files
///
/// > Having a forbidden file generates a violation.
#[test]
fn agents_forbidden_file_generates_violation() {
    let agents = check("agents").on("agents/forbidden-file").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("forbidden_file") }),
        "should have forbidden_file violation"
    );
}

/// Spec: docs/specs/checks/agents.md#sync-behavior
///
/// > Files out of sync with sync_source generate a violation.
#[test]
fn agents_out_of_sync_generates_violation() {
    let agents = check("agents").on("agents/out-of-sync").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("out_of_sync") }),
        "should have out_of_sync violation"
    );
}

/// Spec: docs/specs/checks/agents.md#required-sections
///
/// > Missing a required section generates a violation with advice.
#[test]
fn agents_missing_section_generates_violation_with_advice() {
    let agents = check("agents").on("agents/missing-section").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    let missing_section = violations
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("missing_section"));

    assert!(
        missing_section.is_some(),
        "should have missing_section violation"
    );

    let advice = missing_section
        .unwrap()
        .get("advice")
        .and_then(|a| a.as_str());
    assert!(
        advice.is_some() && !advice.unwrap().is_empty(),
        "missing_section violation should have advice"
    );

    // Verify advice includes section name and configured advice
    let advice_text = advice.unwrap();
    assert!(
        advice_text.contains("Landing the Plane"),
        "advice should include section name"
    );
    assert!(
        advice_text.contains("Checklist"),
        "advice should include configured advice text"
    );
}

/// Spec: docs/specs/checks/agents.md#forbidden-sections
///
/// > Having a forbidden section generates a violation.
#[test]
fn agents_forbidden_section_generates_violation() {
    let agents = check("agents")
        .on("agents/forbidden-section")
        .json()
        .fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("forbidden_section") }),
        "should have forbidden_section violation"
    );
}

/// Spec: docs/specs/checks/agents.md#glob-patterns
///
/// > Glob patterns match multiple section names.
#[test]
fn agents_forbidden_section_glob_matches() {
    let agents = check("agents")
        .on("agents/forbidden-section")
        .json()
        .fails();
    let violations = agents.require("violations").as_array().unwrap();

    let matches_test = violations.iter().any(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("forbidden_section")
            && v.get("advice")
                .and_then(|a| a.as_str())
                .map(|a| a.contains("Test*"))
                .unwrap_or(false)
    });

    assert!(matches_test, "should match Test* glob pattern");
}

// =============================================================================
// CONTENT RULES SPECS (Phase 3)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#tables
///
/// > Markdown tables generate a violation when tables = "forbid".
#[test]
fn agents_markdown_table_generates_violation() {
    let agents = check("agents").on("agents/with-table").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("forbidden_table") }),
        "should have forbidden_table violation"
    );
}

/// Spec: docs/specs/checks/agents.md#max-lines
///
/// > File exceeding max_lines generates a violation.
#[test]
fn agents_file_over_max_lines_generates_violation() {
    let agents = check("agents").on("agents/oversized-lines").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("file_too_large") }),
        "should have file_too_large violation"
    );
}

/// Spec: docs/specs/checks/agents.md#max-tokens
///
/// > File exceeding max_tokens generates a violation.
#[test]
fn agents_file_over_max_tokens_generates_violation() {
    let agents = check("agents").on("agents/oversized-tokens").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("file_too_large") }),
        "should have file_too_large violation"
    );
}

/// Spec: docs/specs/checks/agents.md#box-diagrams
///
/// > Box diagrams generate a violation when box_diagrams = "forbid".
#[test]
fn agents_box_diagram_generates_violation() {
    let agents = check("agents").on("agents/with-box-diagram").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("forbidden_diagram")),
        "should have forbidden_diagram violation"
    );
}

/// Spec: docs/specs/checks/agents.md#mermaid
///
/// > Mermaid blocks generate a violation when mermaid = "forbid".
#[test]
fn agents_mermaid_block_generates_violation() {
    let agents = check("agents").on("agents/with-mermaid").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("forbidden_mermaid")),
        "should have forbidden_mermaid violation"
    );
}

/// Spec: docs/specs/checks/agents.md#size-limits
///
/// > Violations include value and threshold in JSON output.
#[test]
fn agents_size_violation_includes_threshold() {
    let agents = check("agents").on("agents/oversized-lines").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    let size_violation = violations
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("file_too_large"));

    assert!(
        size_violation.is_some(),
        "should have file_too_large violation"
    );

    let v = size_violation.unwrap();
    assert!(v.get("value").is_some(), "should have value field");
    assert!(v.get("threshold").is_some(), "should have threshold field");
}

// =============================================================================
// JSON OUTPUT SPECS (Phase 4)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#json-output
///
/// > JSON output includes files_found and in_sync metrics.
#[test]
fn agents_json_includes_files_found_and_in_sync_metrics() {
    let agents = check("agents").on("agents/metrics").json().passes();
    let metrics = agents.require("metrics");

    assert!(
        metrics.get("files_found").is_some(),
        "should have files_found metric"
    );
    assert!(
        metrics.get("in_sync").is_some(),
        "should have in_sync metric"
    );
}

/// Spec: docs/specs/checks/agents.md#json-output
///
/// > Violation types are one of the expected values.
#[test]
fn agents_violation_type_is_valid() {
    let agents = check("agents").on("agents/missing-file").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    let valid_types = [
        "missing_file",
        "forbidden_file",
        "out_of_sync",
        "missing_section",
        "forbidden_section",
        "forbidden_table",
        "forbidden_diagram",
        "forbidden_mermaid",
        "file_too_large",
    ];

    for v in violations {
        let vtype = v.get("type").and_then(|t| t.as_str()).unwrap();
        assert!(valid_types.contains(&vtype), "unexpected type: {}", vtype);
    }
}

// =============================================================================
// FIX BEHAVIOR SPECS (Phase 4)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#sync-behavior
///
/// > Running with --fix syncs files from sync_source.
#[test]
fn agents_fix_syncs_files_from_sync_source() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    )
    .unwrap();
    let source =
        "# Source\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n";
    std::fs::write(dir.path().join("CLAUDE.md"), source).unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Different content").unwrap();

    // Run with --fix
    check("agents").pwd(dir.path()).args(&["--fix"]).passes();

    // Verify files are now synced
    let cursorrules = std::fs::read_to_string(dir.path().join(".cursorrules")).unwrap();
    assert_eq!(cursorrules, source);
}

// =============================================================================
// TEXT OUTPUT FORMAT SPECS (Phase 525)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#output
///
/// > Missing file shows human-readable description.
#[test]
fn agents_missing_file_text_output() {
    check("agents")
        .on("agents/missing-file")
        .fails()
        .stdout_has("missing required file");
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Out of sync shows other file name.
#[test]
fn agents_out_of_sync_text_output() {
    check("agents")
        .on("agents/out-of-sync")
        .fails()
        .stdout_has("out of sync with");
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Out of sync with differing preamble shows "(preamble)" not empty string.
#[test]
fn agents_out_of_sync_preamble_text_output() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    // Create two files with different preambles (content before ## headings)
    std::fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project A\n\nFirst preamble.\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join(".cursorrules"),
        "# Project B\n\nDifferent preamble.\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n",
    )
    .unwrap();

    check("agents")
        .pwd(dir.path())
        .fails()
        .stdout_has("(preamble)")
        .stdout_lacks("Section \"\"");
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Missing section includes section name and advice.
#[test]
fn agents_missing_section_text_output() {
    check("agents")
        .on("agents/missing-section")
        .fails()
        .stdout_has("Landing the Plane")
        .stdout_has("Checklist");
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Forbidden table shows line number.
#[test]
fn agents_forbidden_table_text_output() {
    let output = check("agents").on("agents/with-table").fails();
    // Verify line number is present (format: CLAUDE.md:N: forbidden table)
    let stdout = output.stdout();
    assert!(
        stdout.contains(":") && stdout.contains("forbidden table"),
        "should show file:line: forbidden table, got: {}",
        stdout
    );
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > File too large shows value vs threshold.
#[test]
fn agents_file_too_large_text_output() {
    check("agents")
        .on("agents/oversized-lines")
        .fails()
        .stdout_has("vs");
}

// =============================================================================
// FIXED STATUS SPECS (Phase 525)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#fixed
///
/// > Running with --fix shows FIXED status when files are synced.
#[test]
fn agents_fix_shows_fixed_status() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    )
    .unwrap();
    let source =
        "# Source\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n";
    std::fs::write(dir.path().join("CLAUDE.md"), source).unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Different content").unwrap();

    check("agents")
        .pwd(dir.path())
        .args(&["--fix"])
        .passes()
        .stdout_has("FIXED")
        .stdout_has("Synced");
}

/// Spec: docs/specs/checks/agents.md#json-output
///
/// > JSON includes fixed:true when --fix applies changes.
#[test]
fn agents_fix_json_includes_fixed_field() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    )
    .unwrap();
    let source =
        "# Source\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n";
    std::fs::write(dir.path().join("CLAUDE.md"), source).unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Different content").unwrap();

    let result = check("agents")
        .pwd(dir.path())
        .args(&["--fix"])
        .json()
        .passes();

    assert_eq!(
        result.require("fixed").as_bool(),
        Some(true),
        "should have fixed: true"
    );
}

// =============================================================================
// SNAPSHOT TESTS
// =============================================================================
// These tests use insta to capture exact output format for regression testing.

use insta::assert_snapshot;

/// Snapshot: Text output for missing file violation
#[test]
fn snapshot_missing_file_text() {
    let result = check("agents").on("agents/missing-file").fails();
    assert_snapshot!(result.stdout());
}

/// Snapshot: Text output for out-of-sync files
#[test]
fn snapshot_out_of_sync_text() {
    let result = check("agents").on("agents/out-of-sync").fails();
    assert_snapshot!(result.stdout());
}

/// Snapshot: Text output for forbidden table
#[test]
fn snapshot_forbidden_table_text() {
    let result = check("agents").on("agents/with-table").fails();
    assert_snapshot!(result.stdout());
}

/// Snapshot: Text output for missing section
#[test]
fn snapshot_missing_section_text() {
    let result = check("agents").on("agents/missing-section").fails();
    assert_snapshot!(result.stdout());
}

/// Snapshot: Text output for oversized file (lines)
#[test]
fn snapshot_oversized_lines_text() {
    let result = check("agents").on("agents/oversized-lines").fails();
    assert_snapshot!(result.stdout());
}

/// Snapshot: JSON output for multi-scope project (with timestamp redacted)
#[test]
fn snapshot_agents_project_json() {
    let result = check("agents").on("agents-project").json().passes();
    // Redact the timestamp for deterministic snapshots
    let json = result.raw_json();
    let redacted = regex::Regex::new(r#""timestamp": "[^"]+""#)
        .expect("valid regex")
        .replace(&json, r#""timestamp": "[REDACTED]""#);
    assert_snapshot!(redacted);
}
