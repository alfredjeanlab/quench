// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Pattern detection, count/comment/forbid actions, source vs test, and exclude pattern specs.

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
    check("escapes").on("escapes/basic").fails().stdout_has("escapes: FAIL");
}

/// Spec: docs/specs/checks/escape-hatches.md#output
///
/// > src/parser.rs:47: unsafe block without // SAFETY: comment
#[test]
fn escapes_reports_line_number_of_match() {
    let escapes = check("escapes").on("escapes/basic").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(
        violations.iter().any(|v| { v.get("line").and_then(|l| l.as_u64()).is_some() }),
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
    let temp = Project::empty();
    temp.config(
        r#"[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"
comment = "// SAFETY:"
"#,
    );
    temp.file(
        "src/lib.rs",
        r#"
// SAFETY: Pointer guaranteed valid by caller
unsafe { *ptr }
"#,
    );

    check("escapes").pwd(temp.path()).passes();
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
        violations.iter().any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("forbidden") }),
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
    let temp = Project::empty();
    temp.config(
        r#"[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"
advice = "Use .context() from anyhow instead."
"#,
    );
    temp.file("src/lib.rs", "pub fn f() { None::<i32>.unwrap(); }");

    let escapes = check("escapes").pwd(temp.path()).json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    let advice = violations[0].get("advice").and_then(|a| a.as_str()).unwrap();
    assert_eq!(advice, "Use .context() from anyhow instead.");
}

// =============================================================================
// CHECK OFF SPECS
// =============================================================================

/// Spec: docs/specs/checks/escape-hatches.md#check-level
///
/// > [check.escapes].check = "off" disables all escape checking for JavaScript.
#[test]
fn escapes_check_off_disables_javascript_violations() {
    let temp = Project::empty();
    temp.config(
        r#"[check.escapes]
check = "off"

[javascript.suppress]
check = "comment"
"#,
    );
    temp.file("package.json", r#"{"name": "test", "version": "1.0.0"}"#);
    temp.file(
        "src/app.ts",
        "const x = foo as unknown as Bar;\n// @ts-ignore\nconst y = 1;\n// eslint-disable-next-line no-console\nconsole.log(\"test\");\n",
    );
    check("escapes").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/escape-hatches.md#check-level
///
/// > [check.escapes].check = "off" disables all escape checking for Rust.
#[test]
fn escapes_check_off_disables_rust_violations() {
    let temp = Project::empty();
    temp.config(
        r#"[check.escapes]
check = "off"

[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"
"#,
    );
    temp.file("src/lib.rs", "pub fn f() { None::<i32>.unwrap(); }");
    check("escapes").pwd(temp.path()).passes();
}

// =============================================================================
// EXCLUDE PATTERN SPECS
// =============================================================================

/// Spec: docs/specs/checks/escape-hatches.md#exclude-patterns
///
/// > Skip files matching glob patterns from escape checks.
#[test]
fn escapes_exclude_skips_matching_files() {
    let temp = super::exclude_project();
    temp.file("src/generated/bindings.rs", "pub fn f() { None::<i32>.unwrap(); }");
    check("escapes").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/escape-hatches.md#exclude-patterns
///
/// > Non-excluded files still trigger violations.
#[test]
fn escapes_exclude_does_not_skip_non_matching_files() {
    let temp = super::exclude_project();
    temp.file("src/lib.rs", "pub fn f() { None::<i32>.unwrap(); }");
    check("escapes").pwd(temp.path()).fails().stdout_has("forbidden");
}
