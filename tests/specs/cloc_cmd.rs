// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for the `quench cloc` command.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::prelude::*;

// =============================================================================
// Basic output
// =============================================================================

/// `quench cloc` produces a table with Language, files, blank, comment, code columns
#[test]
fn cloc_cmd_produces_table_output() {
    let mut cmd = quench_cmd();
    cmd.arg("cloc");
    cmd.current_dir(fixture("cloc-cmd"));
    let output = cmd.output().expect("command should run");
    assert!(output.status.success(), "expected cloc to succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain header columns
    assert!(stdout.contains("Language"), "should have Language column");
    assert!(stdout.contains("files"), "should have files column");
    assert!(stdout.contains("blank"), "should have blank column");
    assert!(stdout.contains("comment"), "should have comment column");
    assert!(stdout.contains("code"), "should have code column");
}

/// `quench cloc` shows source and test rows
#[test]
fn cloc_cmd_splits_source_and_test() {
    let mut cmd = quench_cmd();
    cmd.arg("cloc");
    cmd.current_dir(fixture("cloc-cmd"));
    let output = cmd.output().expect("command should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("(source)"), "should have source rows");
    assert!(stdout.contains("(tests)"), "should have test rows");
}

/// `quench cloc` shows summary totals
#[test]
fn cloc_cmd_shows_totals() {
    let mut cmd = quench_cmd();
    cmd.arg("cloc");
    cmd.current_dir(fixture("cloc-cmd"));
    let output = cmd.output().expect("command should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Source total"), "should have Source total");
    assert!(stdout.contains("Test total"), "should have Test total");
    assert!(stdout.contains("Total"), "should have grand Total");
}

// =============================================================================
// JSON output
// =============================================================================

/// `quench cloc --output json` produces valid JSON
#[test]
fn cloc_cmd_json_output_is_valid() {
    let mut cmd = quench_cmd();
    cmd.args(["cloc", "--output", "json"]);
    cmd.current_dir(fixture("cloc-cmd"));
    let output = cmd.output().expect("command should run");
    assert!(output.status.success());
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("should be valid JSON");
    assert!(
        json.get("languages").is_some(),
        "should have languages array"
    );
    assert!(json.get("totals").is_some(), "should have totals object");
}

/// JSON output contains language entries with expected fields
#[test]
fn cloc_cmd_json_has_language_entries() {
    let mut cmd = quench_cmd();
    cmd.args(["cloc", "--output", "json"]);
    cmd.current_dir(fixture("cloc-cmd"));
    let output = cmd.output().expect("command should run");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let languages = json["languages"].as_array().unwrap();
    assert!(!languages.is_empty(), "should have language entries");

    // Each entry should have required fields
    for lang in languages {
        assert!(lang.get("language").is_some(), "should have language field");
        assert!(lang.get("kind").is_some(), "should have kind field");
        assert!(lang.get("files").is_some(), "should have files field");
        assert!(lang.get("blank").is_some(), "should have blank field");
        assert!(lang.get("comment").is_some(), "should have comment field");
        assert!(lang.get("code").is_some(), "should have code field");
    }
}

/// JSON totals contain source, test, and total sections
#[test]
fn cloc_cmd_json_totals_structure() {
    let mut cmd = quench_cmd();
    cmd.args(["cloc", "--output", "json"]);
    cmd.current_dir(fixture("cloc-cmd"));
    let output = cmd.output().expect("command should run");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let totals = &json["totals"];
    assert!(totals.get("source").is_some(), "should have source totals");
    assert!(totals.get("test").is_some(), "should have test totals");
    assert!(totals.get("total").is_some(), "should have grand total");
}

// =============================================================================
// File classification
// =============================================================================

/// Source/test split matches adapter classification
#[test]
fn cloc_cmd_fixture_counts() {
    let mut cmd = quench_cmd();
    cmd.args(["cloc", "--output", "json"]);
    cmd.current_dir(fixture("cloc-cmd"));
    let output = cmd.output().expect("command should run");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let totals = &json["totals"];

    // Fixture has src/lib.rs (source) and tests/basic_test.rs (test)
    let source_files = totals["source"]["files"].as_u64().unwrap();
    let test_files = totals["test"]["files"].as_u64().unwrap();
    assert!(source_files >= 1, "should have at least 1 source file");
    assert!(test_files >= 1, "should have at least 1 test file");
}

// =============================================================================
// Respects excludes and gitignore
// =============================================================================

/// `quench cloc` respects project.exclude from config
#[test]
fn cloc_cmd_respects_project_exclude() {
    let temp = default_project();
    temp.config(
        r#"
[project]
exclude = ["generated"]
"#,
    );
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    temp.file("src/lib.rs", "pub fn main() {}\n");
    temp.file(
        "generated/big.rs",
        "fn generated_code() {}\nfn more() {}\nfn even_more() {}\n",
    );

    let mut cmd = quench_cmd();
    cmd.args(["cloc", "--output", "json"]);
    cmd.current_dir(temp.path());
    let output = cmd.output().expect("command should run");
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let total = &json["totals"]["total"];
    // Only src/lib.rs should be counted (1 file), generated/ should be excluded
    let total_files = total["files"].as_u64().unwrap();
    assert_eq!(total_files, 1, "excluded dir should not be counted");
}

/// `quench cloc` respects check.cloc.exclude from config
#[test]
fn cloc_cmd_respects_cloc_exclude() {
    let temp = default_project();
    temp.config(
        r#"
[check.cloc]
exclude = ["generated/**"]
"#,
    );
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    temp.file("src/lib.rs", "pub fn main() {}\n");
    temp.file("generated/big.rs", "fn generated() {}\n");

    let mut cmd = quench_cmd();
    cmd.args(["cloc", "--output", "json"]);
    cmd.current_dir(temp.path());
    let output = cmd.output().expect("command should run");
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let total = &json["totals"]["total"];
    let total_files = total["files"].as_u64().unwrap();
    assert_eq!(total_files, 1, "cloc-excluded dir should not be counted");
}

// =============================================================================
// Per-package output
// =============================================================================

/// `quench cloc` shows per-package breakdown inline beneath each language row
#[test]
fn cloc_cmd_shows_package_breakdown() {
    let mut cmd = quench_cmd();
    cmd.arg("cloc");
    cmd.current_dir(fixture("cloc-cmd-packages"));
    let output = cmd.output().expect("command should run");
    assert!(output.status.success(), "expected cloc to succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Package rows are indented with 2 spaces, no separate Package header
    assert!(
        !stdout.contains("Package"),
        "should not have Package header"
    );
    assert!(
        stdout.contains("  my-cli"),
        "should show cli package indented"
    );
    assert!(
        stdout.contains("  my-core"),
        "should show core package indented"
    );
    assert!(
        stdout.contains("  my-shared"),
        "should show shared package indented"
    );
}

/// `quench cloc --output json` includes packages array inside language entries
#[test]
fn cloc_cmd_json_includes_packages() {
    let mut cmd = quench_cmd();
    cmd.args(["cloc", "--output", "json"]);
    cmd.current_dir(fixture("cloc-cmd-packages"));
    let output = cmd.output().expect("command should run");
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    // No top-level packages object
    assert!(
        json.get("packages").is_none(),
        "should not have top-level packages"
    );
    // Packages are nested inside language entries
    let languages = json["languages"].as_array().unwrap();
    let pkg_names: Vec<String> = languages
        .iter()
        .flat_map(|l| {
            l.get("packages")
                .and_then(|p| p.as_array())
                .into_iter()
                .flatten()
                .filter_map(|p| p.get("name").and_then(|n| n.as_str()).map(String::from))
        })
        .collect();
    assert!(
        pkg_names.contains(&"my-cli".to_string()),
        "should have cli package"
    );
    assert!(
        pkg_names.contains(&"my-core".to_string()),
        "should have core package"
    );
    assert!(
        pkg_names.contains(&"my-shared".to_string()),
        "should have shared package"
    );
}

/// Per-package JSON contains name, files, blank, comment, code fields (no ratio)
#[test]
fn cloc_cmd_json_package_fields() {
    let mut cmd = quench_cmd();
    cmd.args(["cloc", "--output", "json"]);
    cmd.current_dir(fixture("cloc-cmd-packages"));
    let output = cmd.output().expect("command should run");
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let languages = json["languages"].as_array().unwrap();
    // Find a language entry that has packages
    let entry_with_pkgs = languages
        .iter()
        .find(|l| l.get("packages").is_some())
        .expect("should have at least one language entry with packages");
    let pkgs = entry_with_pkgs["packages"].as_array().unwrap();
    let pkg = &pkgs[0];
    assert!(pkg.get("name").is_some(), "should have name field");
    assert!(pkg.get("files").is_some(), "should have files field");
    assert!(pkg.get("blank").is_some(), "should have blank field");
    assert!(pkg.get("comment").is_some(), "should have comment field");
    assert!(pkg.get("code").is_some(), "should have code field");
    assert!(pkg.get("ratio").is_none(), "should not have ratio field");
}

/// Packages are omitted from language entries when no packages configured
#[test]
fn cloc_cmd_json_omits_packages_when_unconfigured() {
    let mut cmd = quench_cmd();
    cmd.args(["cloc", "--output", "json"]);
    cmd.current_dir(fixture("cloc-cmd"));
    let output = cmd.output().expect("command should run");
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    // cloc-cmd fixture has no packages configured and is a single crate
    assert!(
        json.get("packages").is_none(),
        "should not have top-level packages"
    );
    // No language entry should have packages either
    let languages = json["languages"].as_array().unwrap();
    for lang in languages {
        assert!(
            lang.get("packages").is_none(),
            "language entry should not have packages when unconfigured"
        );
    }
}

/// Auto-detected Rust workspace shows package breakdown inside language entries
#[test]
fn cloc_cmd_auto_detect_workspace_packages() {
    let mut cmd = quench_cmd();
    cmd.args(["cloc", "--output", "json"]);
    cmd.current_dir(fixture("cloc-cmd-auto-detect"));
    let output = cmd.output().expect("command should run");
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    // No top-level packages
    assert!(
        json.get("packages").is_none(),
        "should not have top-level packages"
    );
    // Packages are nested inside language entries
    let languages = json["languages"].as_array().unwrap();
    let pkg_names: Vec<String> = languages
        .iter()
        .flat_map(|l| {
            l.get("packages")
                .and_then(|p| p.as_array())
                .into_iter()
                .flatten()
                .filter_map(|p| p.get("name").and_then(|n| n.as_str()).map(String::from))
        })
        .collect();
    assert!(
        pkg_names.contains(&"alpha".to_string()),
        "should have alpha package"
    );
    assert!(
        pkg_names.contains(&"beta".to_string()),
        "should have beta package"
    );
}
