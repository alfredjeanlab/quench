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

/// Spec: docs/specs/checks/cloc.md#what-counts-as-a-line
///
/// > A line is counted if it contains at least one non-whitespace character.
#[test]
fn cloc_counts_nonblank_lines_as_loc() {
    let json = check_json(&fixture("cloc/basic"));
    let cloc = find_check(&json, "cloc");
    let metrics = cloc.get("metrics").unwrap();

    // src/counted.rs has exactly 10 non-blank lines
    assert_eq!(
        metrics.get("source_lines").and_then(|v| v.as_u64()),
        Some(10)
    );
}

/// Spec: docs/specs/checks/cloc.md#what-counts-as-a-line
///
/// > Blank lines (whitespace-only) are not counted.
#[test]
fn cloc_does_not_count_blank_lines() {
    let json = check_json(&fixture("cloc/basic"));
    let cloc = find_check(&json, "cloc");
    let metrics = cloc.get("metrics").unwrap();

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
    let json = check_json(&fixture("cloc/source-test"));
    let cloc = find_check(&json, "cloc");
    let metrics = cloc.get("metrics").unwrap();

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
    let json = check_json(&fixture("cloc/source-test"));
    let cloc = find_check(&json, "cloc");
    let metrics = cloc.get("metrics").unwrap();

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
    let json = check_json(&fixture("cloc/basic"));
    let cloc = find_check(&json, "cloc");
    let metrics = cloc.get("metrics").expect("cloc should have metrics");

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
    let json = check_json(&fixture("cloc/basic"));
    let cloc = find_check(&json, "cloc");

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
/// > violation.type is always "file_too_large"
#[test]
fn cloc_violation_type_is_file_too_large() {
    let json = check_json(&fixture("cloc/oversized-source"));
    let cloc = find_check(&json, "cloc");
    let violations = cloc.get("violations").and_then(|v| v.as_array()).unwrap();

    for violation in violations {
        assert_eq!(
            violation.get("type").and_then(|v| v.as_str()),
            Some("file_too_large"),
            "all cloc violations should be file_too_large"
        );
    }
}

/// Spec: docs/specs/checks/cloc.md#file-size-limits
///
/// > max_lines = 750 (default for source files)
#[test]
fn cloc_fails_on_source_file_over_max_lines() {
    let json = check_json(&fixture("cloc/oversized-source"));
    let cloc = find_check(&json, "cloc");

    assert_eq!(cloc.get("passed").and_then(|v| v.as_bool()), Some(false));

    let violations = cloc.get("violations").and_then(|v| v.as_array()).unwrap();
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
    let json = check_json(&fixture("cloc/oversized-test"));
    let cloc = find_check(&json, "cloc");

    assert_eq!(cloc.get("passed").and_then(|v| v.as_bool()), Some(false));

    let violations = cloc.get("violations").and_then(|v| v.as_array()).unwrap();
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
    let json = check_json(&fixture("cloc/high-tokens"));
    let cloc = find_check(&json, "cloc");

    assert_eq!(cloc.get("passed").and_then(|v| v.as_bool()), Some(false));

    // Fixture has max_tokens = 100, verify a violation with that threshold exists
    let violations = cloc.get("violations").and_then(|v| v.as_array()).unwrap();
    assert!(
        violations.iter().any(|v| {
            v.get("threshold").and_then(|t| t.as_i64()) == Some(100)
        }),
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
    let json = check_json(&fixture("cloc/oversized-source"));
    let cloc = find_check(&json, "cloc");
    let violations = cloc.get("violations").and_then(|v| v.as_array()).unwrap();

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
    let json = check_json(&fixture("cloc/oversized-test"));
    let cloc = find_check(&json, "cloc");
    let violations = cloc.get("violations").and_then(|v| v.as_array()).unwrap();

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

    let json = check_json(dir.path());
    let cloc = find_check(&json, "cloc");
    let violations = cloc.get("violations").and_then(|v| v.as_array()).unwrap();

    let advice = violations[0].get("advice").and_then(|a| a.as_str()).unwrap();
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

    let json = check_json(dir.path());
    let cloc = find_check(&json, "cloc");
    let violations = cloc.get("violations").and_then(|v| v.as_array()).unwrap();

    let advice = violations[0].get("advice").and_then(|a| a.as_str()).unwrap();
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
    let json = check_json(&fixture("cloc/with-excludes"));
    let cloc = find_check(&json, "cloc");

    // Should pass because huge.rs is in excluded generated/ directory
    assert_eq!(cloc.get("passed").and_then(|v| v.as_bool()), Some(true));

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
    let json = check_json(&fixture("cloc/basic"));
    let cloc = find_check(&json, "cloc");

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
    let json = check_json(&fixture("cloc/with-packages"));
    let cloc = find_check(&json, "cloc");
    let by_package = cloc
        .get("by_package")
        .expect("by_package should be present");

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
    let json = check_json(&fixture("cloc/basic"));
    let cloc = find_check(&json, "cloc");
    let metrics = cloc.get("metrics").expect("cloc should have metrics");

    assert!(
        metrics.get("source_tokens").is_some(),
        "missing source_tokens"
    );
    assert!(metrics.get("test_tokens").is_some(), "missing test_tokens");
}
