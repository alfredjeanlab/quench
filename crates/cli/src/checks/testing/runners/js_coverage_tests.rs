// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use std::time::Duration;
use yare::parameterized;

// =============================================================================
// LCOV PARSING TESTS
// =============================================================================

#[test]
fn parses_empty_lcov() {
    let content = "";
    let result = parse_lcov_report(content, Duration::ZERO);

    assert!(result.success);
    assert!(result.line_coverage.is_none());
    assert!(result.files.is_empty());
    assert!(result.packages.is_empty());
}

#[test]
fn parses_single_file_lcov() {
    let content = r#"TN:
SF:/project/src/lib.js
FN:1,covered
FN:5,uncovered
FNDA:1,covered
FNDA:0,uncovered
FNF:2
FNH:1
DA:1,1
DA:2,1
DA:5,0
DA:6,0
LF:4
LH:2
end_of_record
"#;
    let result = parse_lcov_report(content, Duration::from_secs(1));

    assert!(result.success);
    assert!(result.line_coverage.is_some());
    // 2 covered out of 4 lines = 50%
    let coverage = result.line_coverage.unwrap();
    assert!(
        (coverage - 50.0).abs() < 0.1,
        "Expected 50%, got {coverage}"
    );
}

#[test]
fn parses_multi_file_lcov() {
    let content = r#"TN:
SF:/project/src/math.js
DA:1,1
DA:2,1
LF:2
LH:2
end_of_record
SF:/project/src/utils.js
DA:1,1
DA:2,0
LF:2
LH:1
end_of_record
SF:/project/src/format.js
DA:1,0
DA:2,0
LF:2
LH:0
end_of_record
"#;
    let result = parse_lcov_report(content, Duration::from_secs(1));

    assert!(result.success);
    assert_eq!(result.files.len(), 3);

    // Check individual file coverage
    let math_coverage = result.files.get("src/math.js").unwrap();
    assert!((math_coverage - 100.0).abs() < 0.1);

    let utils_coverage = result.files.get("src/utils.js").unwrap();
    assert!((utils_coverage - 50.0).abs() < 0.1);

    let format_coverage = result.files.get("src/format.js").unwrap();
    assert!(format_coverage.abs() < 0.1);

    // Overall: 3 hit out of 6 lines = 50%
    let total_coverage = result.line_coverage.unwrap();
    assert!(
        (total_coverage - 50.0).abs() < 0.1,
        "Expected 50%, got {total_coverage}"
    );
}

#[parameterized(
    zero_coverage = { "DA:1,0\nDA:2,0\nDA:3,0", 0.0 },
    full_coverage = { "DA:1,1\nDA:2,1\nDA:3,1\nDA:4,1\nDA:5,1", 100.0 },
)]
fn parses_coverage_extremes(da_lines: &str, expected_coverage: f64) {
    let content = format!(
        "TN:\nSF:/project/src/test.js\n{}\nLF:5\nLH:{}\nend_of_record\n",
        da_lines,
        if expected_coverage > 0.0 { 5 } else { 0 }
    );
    let result = parse_lcov_report(&content, Duration::ZERO);

    assert!(result.success);
    let coverage = result.line_coverage.unwrap();
    assert!(
        (coverage - expected_coverage).abs() < 0.1,
        "Expected {expected_coverage}%, got {coverage}"
    );
}

// =============================================================================
// PACKAGE EXTRACTION TESTS
// =============================================================================

#[parameterized(
    packages_core = { "packages/core/src/index.ts", "packages/core" },
    apps_web = { "apps/web/components/Button.tsx", "apps/web" },
    libs_shared = { "libs/shared/src/utils.js", "libs/shared" },
    src_root = { "src/utils/format.js", "root" },
    top_level = { "index.js", "root" },
)]
fn extract_js_package_cases(path: &str, expected: &str) {
    assert_eq!(extract_js_package(path), expected);
}

// =============================================================================
// PATH NORMALIZATION TESTS
// =============================================================================

#[parameterized(
    src_path = { "/Users/dev/project/src/utils/format.js", "src/utils/format.js" },
    lib_path = { "/Users/dev/project/lib/helpers.js", "lib/helpers.js" },
    packages_path = { "/Users/dev/project/packages/core/src/index.ts", "packages/core/src/index.ts" },
    apps_path = { "/Users/dev/project/apps/web/components/Button.tsx", "apps/web/components/Button.tsx" },
    tests_path = { "/Users/dev/project/tests/unit/math.test.js", "tests/unit/math.test.js" },
    unknown_structure = { "/Users/dev/project/helpers.js", "helpers.js" },
    node_modules = { "/project/node_modules/lodash/index.js", "" },
)]
fn normalize_js_path_cases(path: &str, expected: &str) {
    assert_eq!(normalize_js_path(path), expected);
}

// =============================================================================
// NODE_MODULES EXCLUSION TESTS
// =============================================================================

#[parameterized(
    node_modules_full = { "/project/node_modules/lodash/index.js", false },
    node_modules_short = { "node_modules/react/index.js", false },
    src_file = { "/project/src/app.js", true },
    relative_src = { "src/utils/format.js", true },
)]
fn should_include_file_cases(path: &str, expected: bool) {
    assert_eq!(should_include_file(path), expected);
}

#[test]
fn excludes_node_modules_from_coverage() {
    let content = r#"TN:
SF:/project/src/app.js
DA:1,1
LF:1
LH:1
end_of_record
SF:/project/node_modules/lodash/index.js
DA:1,1
DA:2,1
LF:2
LH:2
end_of_record
"#;
    let result = parse_lcov_report(content, Duration::ZERO);

    assert!(result.success);
    // Should only have 1 file (node_modules excluded)
    assert_eq!(result.files.len(), 1);
    assert!(result.files.contains_key("src/app.js"));
    assert!(!result.files.keys().any(|k| k.contains("node_modules")));
}

// =============================================================================
// MONOREPO PACKAGE AGGREGATION TESTS
// =============================================================================

#[test]
fn aggregates_coverage_by_package() {
    let content = r#"TN:
SF:/project/packages/core/src/index.js
DA:1,1
DA:2,1
LF:2
LH:2
end_of_record
SF:/project/packages/core/src/utils.js
DA:1,1
DA:2,0
LF:2
LH:1
end_of_record
SF:/project/packages/ui/src/Button.tsx
DA:1,0
DA:2,0
LF:2
LH:0
end_of_record
"#;
    let result = parse_lcov_report(content, Duration::ZERO);

    assert!(result.success);
    assert_eq!(result.packages.len(), 2);

    // packages/core: 3 hit out of 4 = 75%
    let core_coverage = result.packages.get("packages/core").unwrap();
    assert!(
        (core_coverage - 75.0).abs() < 0.1,
        "Expected 75%, got {core_coverage}"
    );

    // packages/ui: 0 hit out of 2 = 0%
    let ui_coverage = result.packages.get("packages/ui").unwrap();
    assert!(ui_coverage.abs() < 0.1, "Expected 0%, got {ui_coverage}");
}

// =============================================================================
// COVERAGE DIRECTORY ISOLATION
// =============================================================================

#[parameterized(
    unit_tests = { Some("tests/unit/"), ".coverage-tests-unit" },
    integration = { Some("tests/integration/"), ".coverage-tests-integration" },
    nested_path = { Some("src/tests/e2e/"), ".coverage-src-tests-e2e" },
    no_path = { None, "coverage" },
)]
fn coverage_dir_for_cases(test_path: Option<&str>, expected: &str) {
    assert_eq!(coverage_dir_for(test_path), expected);
}

#[test]
fn coverage_dirs_are_unique_per_suite() {
    let dir_a = coverage_dir_for(Some("tests/unit/"));
    let dir_b = coverage_dir_for(Some("tests/integration/"));
    assert_ne!(dir_a, dir_b);
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn handles_malformed_lcov_gracefully() {
    let content = r#"TN:
SF:/project/src/app.js
LH:not_a_number
LF:2
end_of_record
"#;
    let result = parse_lcov_report(content, Duration::ZERO);

    // Should succeed but have 0 hit lines (parsed as 0)
    assert!(result.success);
    let coverage = result.line_coverage.unwrap();
    assert!(coverage.abs() < 0.1);
}

#[test]
fn handles_missing_end_of_record() {
    let content = r#"TN:
SF:/project/src/app.js
DA:1,1
LF:1
LH:1
"#;
    let result = parse_lcov_report(content, Duration::ZERO);

    // Without end_of_record, file should not be added
    assert!(result.success);
    assert!(result.files.is_empty());
}

#[test]
fn handles_multiple_test_names() {
    let content = r#"TN:unit tests
SF:/project/src/math.js
DA:1,1
LF:1
LH:1
end_of_record
TN:integration tests
SF:/project/src/api.js
DA:1,1
DA:2,1
LF:2
LH:2
end_of_record
"#;
    let result = parse_lcov_report(content, Duration::ZERO);

    assert!(result.success);
    assert_eq!(result.files.len(), 2);
    let coverage = result.line_coverage.unwrap();
    assert!(
        (coverage - 100.0).abs() < 0.1,
        "Expected 100%, got {coverage}"
    );
}

#[test]
fn parses_bun_style_lcov() {
    // Bun's LCOV output format (similar to other JS tools)
    let lcov = r#"TN:
SF:/project/src/lib.ts
FN:1,foo
FN:2,bar
FNDA:5,foo
FNDA:0,bar
FNF:2
FNH:1
DA:1,5
DA:2,0
LH:1
LF:2
end_of_record
"#;
    let result = parse_lcov_report(lcov, Duration::from_millis(100));
    assert!(result.success);
    assert!(result.line_coverage.is_some());
    let pct = result.line_coverage.unwrap();
    assert!((pct - 50.0).abs() < 0.1, "Expected 50%, got {}", pct);
}
