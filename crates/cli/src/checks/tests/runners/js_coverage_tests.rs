#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::time::Duration;

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

#[test]
fn parses_zero_coverage_lcov() {
    let content = r#"TN:
SF:/project/src/uncovered.js
DA:1,0
DA:2,0
DA:3,0
LF:3
LH:0
end_of_record
"#;
    let result = parse_lcov_report(content, Duration::ZERO);

    assert!(result.success);
    let coverage = result.line_coverage.unwrap();
    assert!(coverage.abs() < 0.1, "Expected 0%, got {coverage}");
}

#[test]
fn parses_full_coverage_lcov() {
    let content = r#"TN:
SF:/project/src/covered.js
DA:1,1
DA:2,1
DA:3,1
DA:4,1
DA:5,1
LF:5
LH:5
end_of_record
"#;
    let result = parse_lcov_report(content, Duration::ZERO);

    assert!(result.success);
    let coverage = result.line_coverage.unwrap();
    assert!(
        (coverage - 100.0).abs() < 0.1,
        "Expected 100%, got {coverage}"
    );
}

// =============================================================================
// PACKAGE EXTRACTION TESTS
// =============================================================================

#[test]
fn extracts_package_from_packages_path() {
    let path = "packages/core/src/index.ts";
    assert_eq!(extract_js_package(path), "packages/core");
}

#[test]
fn extracts_package_from_apps_path() {
    let path = "apps/web/components/Button.tsx";
    assert_eq!(extract_js_package(path), "apps/web");
}

#[test]
fn extracts_package_from_libs_path() {
    let path = "libs/shared/src/utils.js";
    assert_eq!(extract_js_package(path), "libs/shared");
}

#[test]
fn extracts_root_for_src_path() {
    let path = "src/utils/format.js";
    assert_eq!(extract_js_package(path), "root");
}

#[test]
fn extracts_root_for_top_level_file() {
    let path = "index.js";
    assert_eq!(extract_js_package(path), "root");
}

// =============================================================================
// PATH NORMALIZATION TESTS
// =============================================================================

#[test]
fn normalizes_src_path() {
    let path = "/Users/dev/project/src/utils/format.js";
    assert_eq!(normalize_js_path(path), "src/utils/format.js");
}

#[test]
fn normalizes_lib_path() {
    let path = "/Users/dev/project/lib/helpers.js";
    assert_eq!(normalize_js_path(path), "lib/helpers.js");
}

#[test]
fn normalizes_packages_path() {
    let path = "/Users/dev/project/packages/core/src/index.ts";
    assert_eq!(normalize_js_path(path), "packages/core/src/index.ts");
}

#[test]
fn normalizes_apps_path() {
    let path = "/Users/dev/project/apps/web/components/Button.tsx";
    assert_eq!(normalize_js_path(path), "apps/web/components/Button.tsx");
}

#[test]
fn normalizes_tests_path() {
    let path = "/Users/dev/project/tests/unit/math.test.js";
    assert_eq!(normalize_js_path(path), "tests/unit/math.test.js");
}

#[test]
fn normalizes_to_filename_for_unknown_structure() {
    let path = "/Users/dev/project/helpers.js";
    assert_eq!(normalize_js_path(path), "helpers.js");
}

// =============================================================================
// NODE_MODULES EXCLUSION TESTS
// =============================================================================

#[test]
fn excludes_node_modules_from_path() {
    let path = "/project/node_modules/lodash/index.js";
    assert_eq!(normalize_js_path(path), "");
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

#[test]
fn should_include_file_excludes_node_modules() {
    assert!(!should_include_file(
        "/project/node_modules/lodash/index.js"
    ));
    assert!(!should_include_file("node_modules/react/index.js"));
    assert!(should_include_file("/project/src/app.js"));
    assert!(should_include_file("src/utils/format.js"));
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
