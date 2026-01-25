//! Behavioral specs for quench report command.
//!
//! Tests that quench report correctly:
//! - Reads baseline files
//! - Outputs metrics in various formats
//!
//! Reference: docs/specs/01-cli.md#quench-report

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// BASELINE READING
// =============================================================================

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Reports read from .quench/baseline.json
#[test]
fn report_reads_baseline_file() {
    report()
        .on("report/with-baseline")
        .runs()
        .stdout_has("coverage");
}

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Report without baseline shows appropriate message
#[test]
fn report_without_baseline_shows_message() {
    report()
        .on("report/no-baseline")
        .runs()
        .stdout_has("No baseline");
}

// =============================================================================
// TEXT FORMAT
// =============================================================================

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Default output format is text
#[test]
fn report_default_format_is_text() {
    // Should not be JSON or HTML
    report()
        .on("report/with-baseline")
        .runs()
        .stdout_lacks("{")
        .stdout_lacks("<html");
}

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Text format shows summary with metrics
#[test]
fn report_text_shows_summary() {
    report()
        .on("report/with-baseline")
        .runs()
        .stdout_has("coverage: 85.5%")
        .stdout_has("escapes.unsafe: 3");
}

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Text format shows baseline commit and timestamp
#[test]
fn report_text_shows_baseline_info() {
    report()
        .on("report/with-baseline")
        .runs()
        .stdout_has("abc1234")
        .stdout_has("2026-01-20");
}

// =============================================================================
// JSON FORMAT
// =============================================================================

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > JSON format outputs machine-readable metrics
#[test]
fn report_json_outputs_metrics() {
    let output = report().on("report/with-baseline").json().runs();

    let json: serde_json::Value = serde_json::from_str(&output.stdout()).unwrap();

    assert!(json.get("metrics").is_some(), "should have metrics field");
    assert!(
        json["metrics"]["coverage"]["total"].as_f64() == Some(85.5),
        "should have coverage metric"
    );
}

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > JSON format includes baseline metadata
#[test]
fn report_json_includes_metadata() {
    let output = report().on("report/with-baseline").json().runs();

    let json: serde_json::Value = serde_json::from_str(&output.stdout()).unwrap();

    assert!(json.get("updated").is_some(), "should have updated field");
    assert!(json.get("commit").is_some(), "should have commit field");
}

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > JSON format with no baseline outputs empty metrics
#[test]
fn report_json_no_baseline_empty_metrics() {
    let output = report().on("report/no-baseline").json().runs();

    let json: serde_json::Value = serde_json::from_str(&output.stdout()).unwrap();

    assert!(json.get("metrics").is_some(), "should have metrics field");
    // Metrics should be empty object or have null/empty values
}

// =============================================================================
// HTML FORMAT
// =============================================================================

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > HTML format produces valid HTML document
#[test]
fn report_html_produces_valid_html() {
    report()
        .on("report/with-baseline")
        .html()
        .runs()
        .stdout_has("<!DOCTYPE html>")
        .stdout_has("<html")
        .stdout_has("</html>");
}

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > HTML format includes metrics data
#[test]
fn report_html_includes_metrics() {
    report()
        .on("report/with-baseline")
        .html()
        .runs()
        .stdout_has("85.5") // coverage value
        .stdout_has("coverage"); // metric name
}

// =============================================================================
// MARKDOWN FORMAT
// =============================================================================

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Markdown format produces valid markdown table
#[test]
fn report_markdown_produces_table() {
    report()
        .on("report/with-baseline")
        .markdown()
        .runs()
        .stdout_has("| Metric |")
        .stdout_has("|---");
}

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Markdown format includes all metric values
#[test]
fn report_markdown_includes_metrics() {
    report()
        .on("report/with-baseline")
        .markdown()
        .runs()
        .stdout_has("Coverage")
        .stdout_has("85.5");
}

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Markdown format with no baseline shows appropriate message
#[test]
fn report_markdown_no_baseline_shows_message() {
    report()
        .on("report/no-baseline")
        .markdown()
        .runs()
        .stdout_has("No baseline");
}

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Markdown format includes baseline metadata
#[test]
fn report_markdown_includes_metadata() {
    report()
        .on("report/with-baseline")
        .markdown()
        .runs()
        .stdout_has("abc1234"); // commit hash
}

// =============================================================================
// FILE OUTPUT
// =============================================================================

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > -o report.html writes to file instead of stdout
#[test]
fn report_writes_to_file() {
    let temp = Project::with_defaults();

    // Create baseline
    temp.file(
        ".quench/baseline.json",
        r#"{
        "version": 1,
        "updated": "2026-01-20T12:00:00Z",
        "metrics": {"coverage": {"total": 75.0}}
    }"#,
    );

    quench_cmd()
        .args(["report", "-o", "report.html"])
        .current_dir(temp.path())
        .assert()
        .success();

    let output_path = temp.path().join("report.html");
    assert!(output_path.exists(), "report.html should be created");

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("<!DOCTYPE html>"), "should be HTML");
    assert!(content.contains("75.0"), "should include metrics");
}
