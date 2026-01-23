//! Behavioral specs for the escapes (escape hatches) check.
//!
//! Tests that quench correctly:
//! - Detects pattern matches in source files
//! - Applies actions (count, comment, forbid)
//! - Separates source and test code
//! - Generates correct violation types
//! - Outputs metrics in JSON format
//!
//! Reference: docs/specs/checks/escape-hatches.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// PATTERN DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/checks/escape-hatches.md#pattern-matching
///
/// > The escapes check detects patterns that bypass type safety or error handling.
#[test]
fn escapes_detects_pattern_matches_in_source() {
    check("escapes")
        .on("escapes/basic")
        .fails()
        .stdout_has("escapes: FAIL");
}

/// Spec: docs/specs/checks/escape-hatches.md#output
///
/// > src/parser.rs:47: unsafe block without // SAFETY: comment
#[test]
fn escapes_reports_line_number_of_match() {
    let escapes = check("escapes").on("escapes/basic").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("line").and_then(|l| l.as_u64()).is_some() }),
        "violations should include line numbers"
    );
}

// =============================================================================
// COUNT ACTION SPECS
// =============================================================================

/// Spec: docs/specs/checks/escape-hatches.md#count
///
/// > Just count occurrences.
#[test]
fn escapes_count_action_counts_occurrences() {
    let escapes = check("escapes").on("escapes/count-ok").json().passes();
    let metrics = escapes.require("metrics");
    let source = metrics.get("source").unwrap();

    assert!(
        source.get("todo").and_then(|v| v.as_u64()).unwrap() > 0,
        "should count TODO occurrences"
    );
}

/// Spec: docs/specs/checks/escape-hatches.md#count
///
/// > Fail if count exceeds per-pattern threshold (default: 0).
#[test]
fn escapes_count_action_fails_when_threshold_exceeded() {
    let escapes = check("escapes").on("escapes/count-fail").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("threshold_exceeded") }),
        "should have threshold_exceeded violation"
    );
}

// =============================================================================
// COMMENT ACTION SPECS
// =============================================================================

/// Spec: docs/specs/checks/escape-hatches.md#comment
///
/// > Pattern is allowed if accompanied by a justification comment.
#[test]
fn escapes_comment_action_passes_when_comment_on_same_line() {
    check("escapes").on("escapes/comment-ok").passes();
}

/// Spec: docs/specs/checks/escape-hatches.md#comment-detection
///
/// > On preceding lines, searching upward until a non-blank, non-comment line is found
#[test]
fn escapes_comment_action_passes_when_comment_on_preceding_line() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"
comment = "// SAFETY:"
"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        r#"
// SAFETY: Pointer guaranteed valid by caller
unsafe { *ptr }
"#,
    )
    .unwrap();

    check("escapes").pwd(dir.path()).passes();
}

/// Spec: docs/specs/checks/escape-hatches.md#comment
///
/// > Require a justification comment.
#[test]
fn escapes_comment_action_fails_when_no_comment_found() {
    let escapes = check("escapes").on("escapes/comment-fail").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("missing_comment") }),
        "should have missing_comment violation"
    );
}

// =============================================================================
// FORBID ACTION SPECS
// =============================================================================

/// Spec: docs/specs/checks/escape-hatches.md#forbid
///
/// > Pattern is never allowed in source code.
#[test]
fn escapes_forbid_action_always_fails_in_source_code() {
    let escapes = check("escapes").on("escapes/forbid-source").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("forbidden") }),
        "should have forbidden violation"
    );
}

/// Spec: docs/specs/checks/escape-hatches.md#forbid
///
/// > Always allowed in test code.
#[test]
fn escapes_forbid_action_allowed_in_test_code() {
    check("escapes").on("escapes/forbid-test").passes();
}

// =============================================================================
// SOURCE VS TEST SPECS
// =============================================================================

/// Spec: docs/specs/checks/escape-hatches.md#source-vs-test
///
/// > Escape hatches are counted separately for source and test code.
#[test]
fn escapes_test_code_counted_separately_in_metrics() {
    let escapes = check("escapes").on("escapes/metrics").json().passes();
    let metrics = escapes.require("metrics");

    let source = metrics.get("source").expect("should have source metrics");
    let test = metrics.get("test").expect("should have test metrics");

    // Both should have counts (actual values depend on fixture)
    assert!(source.is_object(), "source should be object");
    assert!(test.is_object(), "test should be object");
}

/// Spec: docs/specs/checks/escape-hatches.md#configurable-advice
///
/// > Each pattern can have custom advice
#[test]
fn escapes_per_pattern_advice_shown_in_violation() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"
advice = "Use .context() from anyhow instead."
"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn f() { None::<i32>.unwrap(); }",
    )
    .unwrap();

    let escapes = check("escapes").pwd(dir.path()).json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    let advice = violations[0]
        .get("advice")
        .and_then(|a| a.as_str())
        .unwrap();
    assert_eq!(advice, "Use .context() from anyhow instead.");
}

// =============================================================================
// JSON OUTPUT SPECS
// =============================================================================

/// Spec: docs/specs/checks/escape-hatches.md#json-output
///
/// > metrics: { source: {...}, test: {...} }
#[test]
fn escapes_json_includes_source_test_breakdown_per_pattern() {
    let escapes = check("escapes").on("escapes/metrics").json().passes();
    let metrics = escapes.require("metrics");

    // Source metrics by pattern name
    let source = metrics.get("source").unwrap();
    assert!(
        source.get("unwrap").is_some() || source.get("todo").is_some(),
        "source should have pattern counts"
    );

    // Test metrics by pattern name
    let test = metrics.get("test").unwrap();
    assert!(test.is_object(), "test should have pattern counts");
}

/// Spec: docs/specs/checks/escape-hatches.md#json-output
///
/// > Violation types: missing_comment, forbidden, threshold_exceeded
#[test]
fn escapes_violation_type_is_one_of_expected_values() {
    let escapes = check("escapes").on("violations").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    let valid_types = ["missing_comment", "forbidden", "threshold_exceeded"];
    for violation in violations {
        let vtype = violation.get("type").and_then(|t| t.as_str()).unwrap();
        assert!(
            valid_types.contains(&vtype),
            "unexpected violation type: {}",
            vtype
        );
    }
}

// =============================================================================
// TEXT OUTPUT FORMAT SPECS
// =============================================================================

/// Spec: docs/specs/checks/escape-hatches.md#text-output
///
/// > Text output shows violations with file path, line, and advice
#[test]
fn escapes_text_output_format_on_missing_comment() {
    check("escapes")
        .on("escapes/comment-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("lib.rs")
        .stdout_has("missing_comment");
}

/// Spec: docs/specs/checks/escape-hatches.md#text-output
///
/// > Forbidden violations show pattern name and advice
#[test]
fn escapes_text_output_format_on_forbidden() {
    check("escapes")
        .on("escapes/forbid-source")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("forbidden");
}

/// Spec: docs/specs/checks/escape-hatches.md#text-output
///
/// > Threshold exceeded shows count vs limit
#[test]
fn escapes_text_output_format_on_threshold_exceeded() {
    check("escapes")
        .on("escapes/count-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("threshold_exceeded");
}

// =============================================================================
// JSON OUTPUT STRUCTURE SPECS
// =============================================================================

/// Spec: docs/specs/checks/escape-hatches.md#json-output
///
/// > JSON output includes all required fields for violations
#[test]
fn escapes_json_violation_structure_complete() {
    let escapes = check("escapes").on("escapes/forbid-source").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(!violations.is_empty(), "should have violations");

    // Each violation must have all required fields
    for violation in violations {
        assert!(violation.get("file").is_some(), "missing file");
        assert!(violation.get("line").is_some(), "missing line");
        assert!(violation.get("type").is_some(), "missing type");
        assert!(violation.get("pattern").is_some(), "missing pattern");
        assert!(violation.get("advice").is_some(), "missing advice");
    }
}

/// Spec: docs/specs/checks/escape-hatches.md#json-output
///
/// > JSON metrics include source and test breakdowns per pattern
#[test]
fn escapes_json_metrics_structure_complete() {
    let escapes = check("escapes").on("escapes/metrics").json().passes();
    let metrics = escapes.require("metrics");

    // Verify structure
    assert!(metrics.get("source").is_some(), "missing source metrics");
    assert!(metrics.get("test").is_some(), "missing test metrics");

    // Source and test should be objects with pattern counts
    let source = metrics.get("source").unwrap();
    let test = metrics.get("test").unwrap();
    assert!(source.is_object(), "source should be object");
    assert!(test.is_object(), "test should be object");
}

// =============================================================================
// EDGE CASE SPECS - Checkpoint 3C Fixes
// =============================================================================

/// Spec: Edge case - pattern in both code and comment
///
/// > When escape pattern appears in code AND in a comment on the same line,
/// > only one violation should be reported for that line.
#[test]
fn escapes_single_violation_per_line_even_with_pattern_in_comment() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"
"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    // Pattern appears twice on same line: in code AND in comment
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn f() { None::<i32>.unwrap() } // using .unwrap() here\n",
    )
    .unwrap();

    let escapes = check("escapes").pwd(dir.path()).json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    // Should only have ONE violation, not two
    assert_eq!(
        violations.len(),
        1,
        "should have exactly one violation, not multiple for same line"
    );
}

/// Spec: Edge case - embedded comment pattern
///
/// > Comment pattern embedded in other text should NOT satisfy the requirement.
/// > For example, `// VIOLATION: missing // SAFETY:` should not match `// SAFETY:`.
#[test]
fn escapes_comment_embedded_in_text_does_not_satisfy() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"
comment = "// SAFETY:"
"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    // The // SAFETY: is embedded in another comment, not at comment start
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "unsafe { }  // VIOLATION: missing // SAFETY: comment\n",
    )
    .unwrap();

    // This should FAIL because the embedded // SAFETY: should not count
    let escapes = check("escapes").pwd(dir.path()).json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(
        !violations.is_empty(),
        "should have violation - embedded pattern should not satisfy requirement"
    );
    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("missing_comment") }),
        "should be missing_comment violation"
    );
}

/// Spec: Edge case - comment at start is valid
///
/// > Comment pattern at start of inline comment should satisfy requirement.
#[test]
fn escapes_comment_at_start_of_inline_comment_satisfies() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"
comment = "// SAFETY:"
"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    // The // SAFETY: is at start of the inline comment
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "unsafe { }  // SAFETY: pointer is valid\n",
    )
    .unwrap();

    // This should PASS
    check("escapes").pwd(dir.path()).passes();
}
