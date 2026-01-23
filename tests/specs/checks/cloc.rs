//! Behavioral specs for the CLOC (Count Lines of Code) check.
//!
//! Tests that quench correctly:
//! - Counts non-blank lines as LOC
//! - Separates source and test files by pattern
//! - Calculates source-to-test ratio
//! - Generates violations for oversized files
//! - Outputs metrics in JSON format
//!
//! Reference: docs/specs/checks/cloc.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// LOC COUNTING SPECS
// =============================================================================

/// Spec: docs/specs/checks/cloc.md#metrics-output
///
/// > The source_lines and test_lines metrics count non-blank lines
/// > (lines with at least one non-whitespace character).
#[test]
fn cloc_counts_nonblank_lines_as_loc() {
    let cloc = check("cloc").on("cloc/basic").json().passes();
    let metrics = cloc.require("metrics");

    // src/counted.rs has exactly 10 non-blank lines
    assert_eq!(
        metrics.get("source_lines").and_then(|v| v.as_u64()),
        Some(10)
    );
}

/// Spec: docs/specs/checks/cloc.md#metrics-output
///
/// > The source_lines and test_lines metrics count non-blank lines
#[test]
fn cloc_does_not_count_blank_lines() {
    let cloc = check("cloc").on("cloc/basic").json().passes();
    let metrics = cloc.require("metrics");

    // File has 15 total lines but only 10 non-blank
    // If blank lines were counted, we'd see 15
    assert_eq!(
        metrics.get("source_lines").and_then(|v| v.as_u64()),
        Some(10)
    );
}

// =============================================================================
// SOURCE/TEST SEPARATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/cloc.md#pattern-based-language-agnostic
///
/// > Files matching any test pattern are counted as test code.
/// > All other files matching source patterns are counted as source code.
#[test]
fn cloc_separates_source_and_test_by_pattern() {
    let cloc = check("cloc").on("cloc/source-test").json().passes();
    let metrics = cloc.require("metrics");

    assert_eq!(
        metrics.get("source_lines").and_then(|v| v.as_u64()),
        Some(10)
    );
    assert_eq!(metrics.get("test_lines").and_then(|v| v.as_u64()), Some(8));
    assert_eq!(
        metrics.get("source_files").and_then(|v| v.as_u64()),
        Some(1)
    );
    assert_eq!(metrics.get("test_files").and_then(|v| v.as_u64()), Some(1));
}

/// Spec: docs/specs/checks/cloc.md#ratio-direction
///
/// > Ratio is test LOC / source LOC.
#[test]
fn cloc_calculates_source_to_test_ratio() {
    let cloc = check("cloc").on("cloc/source-test").json().passes();
    let metrics = cloc.require("metrics");

    // 8 test lines / 10 source lines = 0.8
    let ratio = metrics.get("ratio").and_then(|v| v.as_f64()).unwrap();
    assert!(
        (ratio - 0.8).abs() < 0.01,
        "Expected ratio ~0.8, got {}",
        ratio
    );
}

// =============================================================================
// JSON OUTPUT SPECS
// =============================================================================

/// Spec: docs/specs/checks/cloc.md#json-output
///
/// > JSON metrics always include: source_lines, source_files, test_lines, test_files, ratio
#[test]
fn cloc_json_includes_required_metrics() {
    let cloc = check("cloc").on("cloc/basic").json().passes();
    let metrics = cloc.require("metrics");

    assert!(
        metrics.get("source_lines").is_some(),
        "missing source_lines"
    );
    assert!(
        metrics.get("source_files").is_some(),
        "missing source_files"
    );
    assert!(metrics.get("test_lines").is_some(), "missing test_lines");
    assert!(metrics.get("test_files").is_some(), "missing test_files");
    assert!(metrics.get("ratio").is_some(), "missing ratio");
}

/// Spec: docs/specs/checks/cloc.md#json-output
///
/// > violations only present when file size limits exceeded
#[test]
fn cloc_json_omits_violations_when_none() {
    let cloc = check("cloc").on("cloc/basic").json().passes();

    // No oversized files in basic fixture
    assert!(
        cloc.get("violations")
            .map(|v| v.as_array().unwrap().is_empty())
            .unwrap_or(true),
        "violations should be empty or omitted"
    );
}

// =============================================================================
// VIOLATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/cloc.md#file-size-limits
///
/// > violation.type is "file_too_large" (for metric = "lines", the default)
/// > or "file_too_large_nonblank" (for metric = "nonblank")
#[test]
fn cloc_violation_type_is_file_too_large() {
    let cloc = check("cloc").on("cloc/oversized-source").json().fails();
    let violations = cloc.require("violations").as_array().unwrap();

    for violation in violations {
        assert_eq!(
            violation.get("type").and_then(|v| v.as_str()),
            Some("file_too_large"),
            "default metric=lines uses file_too_large type"
        );
    }
}

/// Spec: docs/specs/checks/cloc.md#file-size-limits
///
/// > max_lines = 750 (default for source files)
#[test]
fn cloc_fails_on_source_file_over_max_lines() {
    let cloc = check("cloc").on("cloc/oversized-source").json().fails();

    assert_eq!(cloc.require("passed").as_bool(), Some(false));

    let violations = cloc.require("violations").as_array().unwrap();
    assert!(!violations.is_empty(), "should have violations");
    assert!(
        violations.iter().any(|v| {
            v.get("file")
                .and_then(|f| f.as_str())
                .unwrap()
                .ends_with("big.rs")
        }),
        "violation should reference oversized file"
    );
}

/// Spec: docs/specs/checks/cloc.md#file-size-limits
///
/// > max_lines_test = 1100 (default for test files)
#[test]
fn cloc_fails_on_test_file_over_max_lines_test() {
    let cloc = check("cloc").on("cloc/oversized-test").json().fails();

    assert_eq!(cloc.require("passed").as_bool(), Some(false));

    let violations = cloc.require("violations").as_array().unwrap();
    assert!(
        violations.iter().any(|v| {
            v.get("file")
                .and_then(|f| f.as_str())
                .unwrap()
                .ends_with("big_test.rs")
        }),
        "violation should reference oversized test file"
    );
}

/// Spec: docs/specs/checks/cloc.md#file-size-limits
///
/// > max_tokens = 20000 (default)
#[test]
fn cloc_fails_on_file_over_max_tokens() {
    let cloc = check("cloc").on("cloc/high-tokens").json().fails();

    assert_eq!(cloc.require("passed").as_bool(), Some(false));

    // Fixture has max_tokens = 100, verify a violation with that threshold exists
    let violations = cloc.require("violations").as_array().unwrap();
    assert!(
        violations
            .iter()
            .any(|v| { v.get("threshold").and_then(|t| t.as_i64()) == Some(100) }),
        "should have violation with token threshold (100)"
    );
}

// =============================================================================
// ADVICE SPECS
// =============================================================================

/// Spec: docs/specs/checks/cloc.md#configuration
///
/// > advice = "Can the code be made more concise? If not, split large source files
/// > into sibling modules or submodules in a folder; consider refactoring to be more
/// > unit testable." (default for source files)
#[test]
fn cloc_source_violation_has_default_advice() {
    let cloc = check("cloc").on("cloc/oversized-source").json().fails();
    let violations = cloc.require("violations").as_array().unwrap();

    let source_violation = violations
        .iter()
        .find(|v| {
            v.get("file")
                .and_then(|f| f.as_str())
                .map(|f| !f.contains("_test.rs") && !f.contains("/tests/"))
                .unwrap_or(false)
        })
        .expect("should have source file violation");

    let advice = source_violation
        .get("advice")
        .and_then(|a| a.as_str())
        .unwrap();
    assert_eq!(
        advice,
        "Can the code be made more concise? If not, split large source files into sibling modules or submodules in a folder; consider refactoring to be more unit testable."
    );
}

/// Spec: docs/specs/checks/cloc.md#configuration
///
/// > advice_test = "Can tests be parameterized or use shared fixtures to be more
/// > concise? If not, split large test files into a folder." (default for test files)
#[test]
fn cloc_test_violation_has_default_advice() {
    let cloc = check("cloc").on("cloc/oversized-test").json().fails();
    let violations = cloc.require("violations").as_array().unwrap();

    let test_violation = violations
        .iter()
        .find(|v| {
            v.get("file")
                .and_then(|f| f.as_str())
                .map(|f| f.contains("_test.rs") || f.contains("/tests/"))
                .unwrap_or(false)
        })
        .expect("should have test file violation");

    let advice = test_violation
        .get("advice")
        .and_then(|a| a.as_str())
        .unwrap();
    assert_eq!(
        advice,
        "Can tests be parameterized or use shared fixtures to be more concise? If not, split large test files into a folder."
    );
}

/// Spec: docs/specs/checks/cloc.md#configuration
///
/// > advice = "..." - custom advice for source file violations
#[test]
fn cloc_uses_custom_advice_for_source() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[check.cloc]
max_lines = 5
advice = "Custom source advice here."
"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/big.rs"),
        "fn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\nfn e() {}\nfn f() {}\n",
    )
    .unwrap();

    let cloc = check("cloc").pwd(dir.path()).json().fails();
    let violations = cloc.require("violations").as_array().unwrap();

    let advice = violations[0]
        .get("advice")
        .and_then(|a| a.as_str())
        .unwrap();
    assert_eq!(advice, "Custom source advice here.");
}

/// Spec: docs/specs/checks/cloc.md#configuration
///
/// > advice_test = "..." - custom advice for test file violations
#[test]
fn cloc_uses_custom_advice_for_test() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[check.cloc]
max_lines_test = 5
advice_test = "Custom test advice here."
"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("tests")).unwrap();
    std::fs::write(
        dir.path().join("tests/big_test.rs"),
        "fn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\nfn e() {}\nfn f() {}\n",
    )
    .unwrap();

    let cloc = check("cloc").pwd(dir.path()).json().fails();
    let violations = cloc.require("violations").as_array().unwrap();

    let advice = violations[0]
        .get("advice")
        .and_then(|a| a.as_str())
        .unwrap();
    assert_eq!(advice, "Custom test advice here.");
}

// =============================================================================
// CONFIGURATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/cloc.md#configuration
///
/// > exclude = [...] - patterns don't generate violations
#[test]
fn cloc_excluded_patterns_dont_generate_violations() {
    let cloc = check("cloc").on("cloc/with-excludes").json().passes();

    // Should pass because huge.rs is in excluded generated/ directory
    assert_eq!(cloc.require("passed").as_bool(), Some(true));

    // Violations should be empty or not mention excluded files
    if let Some(violations) = cloc.get("violations").and_then(|v| v.as_array()) {
        for v in violations {
            let file = v.get("file").and_then(|f| f.as_str()).unwrap_or("");
            assert!(
                !file.contains("/generated/"),
                "excluded files should not appear in violations"
            );
        }
    }
}

/// Spec: docs/specs/checks/cloc.md#json-output
///
/// > by_package omitted if no packages configured
#[test]
fn cloc_omits_by_package_when_not_configured() {
    let cloc = check("cloc").on("cloc/basic").json().passes();

    assert!(
        cloc.get("by_package").is_none(),
        "by_package should be omitted when packages not configured"
    );
}

/// Spec: docs/specs/checks/cloc.md#json-output
///
/// > by_package present with per-package metrics when packages configured
#[test]
fn cloc_includes_by_package_when_configured() {
    let cloc = check("cloc").on("cloc/with-packages").json().passes();
    let by_package = cloc.require("by_package");

    assert!(by_package.get("cli").is_some(), "should have cli package");
    assert!(by_package.get("core").is_some(), "should have core package");

    // Each package should have metrics
    let cli = by_package.get("cli").unwrap();
    assert!(cli.get("source_lines").is_some());
    assert!(cli.get("test_lines").is_some());
    assert!(cli.get("ratio").is_some());
}

/// Spec: docs/specs/checks/cloc.md#json-output
///
/// > JSON metrics include: source_tokens, test_tokens
#[test]
fn cloc_json_includes_token_metrics() {
    let cloc = check("cloc").on("cloc/basic").json().passes();
    let metrics = cloc.require("metrics");

    assert!(
        metrics.get("source_tokens").is_some(),
        "missing source_tokens"
    );
    assert!(metrics.get("test_tokens").is_some(), "missing test_tokens");
}

// =============================================================================
// OUTPUT FORMAT SPECS
// =============================================================================

/// Spec: docs/specs/checks/cloc.md#text-output
///
/// > Text output shows violations with file path, line count, and advice
#[test]
fn cloc_text_output_format_on_violation() {
    check("cloc")
        .on("cloc/oversized-source")
        .fails()
        .stdout_has("cloc: FAIL")
        .stdout_has("big.rs")
        .stdout_has("file_too_large")
        .stdout_has("750");
}

/// Spec: docs/specs/checks/cloc.md#json-output
///
/// > JSON output includes all required fields for violations
#[test]
fn cloc_json_violation_structure_complete() {
    let cloc = check("cloc").on("cloc/oversized-source").json().fails();
    let violations = cloc.require("violations").as_array().unwrap();

    assert!(!violations.is_empty(), "should have violations");

    // Each violation must have all required fields
    for violation in violations {
        assert!(violation.get("file").is_some(), "missing file");
        assert!(violation.get("type").is_some(), "missing type");
        assert!(violation.get("value").is_some(), "missing value");
        assert!(violation.get("threshold").is_some(), "missing threshold");
        assert!(violation.get("advice").is_some(), "missing advice");
        // Both line counts included for convenience
        assert!(violation.get("lines").is_some(), "missing lines");
        assert!(violation.get("nonblank").is_some(), "missing nonblank");
    }
}

// =============================================================================
// METRIC CONFIGURATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/cloc.md#metric-configuration
///
/// > Configure which metric to check via `metric` (default: `lines`)
#[test]
fn cloc_default_metric_is_total_lines() {
    // Default config uses total lines (matches wc -l)
    let cloc = check("cloc").on("cloc/oversized-source").json().fails();
    let violations = cloc.require("violations").as_array().unwrap();
    let v = &violations[0];

    // value should equal lines (total), not nonblank
    let value = v.get("value").and_then(|v| v.as_i64()).unwrap();
    let lines = v.get("lines").and_then(|v| v.as_i64()).unwrap();
    assert_eq!(value, lines, "default metric should use total lines");
}

/// Spec: docs/specs/checks/cloc.md#metric-configuration
///
/// > metric = "nonblank" uses non-blank lines for threshold
#[test]
fn cloc_metric_nonblank_uses_nonblank_for_threshold() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[check.cloc]
max_lines = 5
metric = "nonblank"
"#,
    )
    .unwrap();
    // Create a file with 10 total lines but only 6 non-blank
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "fn a() {}\n\nfn b() {}\n\nfn c() {}\n\nfn d() {}\n\nfn e() {}\n\nfn f() {}\n",
    )
    .unwrap();

    let cloc = check("cloc").pwd(dir.path()).json().fails();
    let violations = cloc.require("violations").as_array().unwrap();
    let v = &violations[0];

    // violation type should be file_too_large_nonblank
    assert_eq!(
        v.get("type").and_then(|v| v.as_str()),
        Some("file_too_large_nonblank")
    );

    // value should equal nonblank, not total lines
    let value = v.get("value").and_then(|v| v.as_i64()).unwrap();
    let nonblank = v.get("nonblank").and_then(|v| v.as_i64()).unwrap();
    assert_eq!(
        value, nonblank,
        "metric=nonblank should use nonblank for value"
    );
}
