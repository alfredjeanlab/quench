// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn parses_llvm_cov_json_report() {
    let json = r#"{
        "data": [{
            "totals": { "lines": { "count": 100, "covered": 75, "percent": 75.0 } },
            "files": [
                {
                    "filename": "/home/user/project/src/lib.rs",
                    "summary": { "lines": { "count": 60, "covered": 50, "percent": 83.33 } }
                },
                {
                    "filename": "/home/user/project/src/utils.rs",
                    "summary": { "lines": { "count": 40, "covered": 25, "percent": 62.5 } }
                }
            ]
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;

    let result = parse_llvm_cov_json(json, Duration::from_secs(1));

    assert!(result.success);
    assert_eq!(result.line_coverage, Some(75.0));
    assert_eq!(result.files.len(), 2);
    assert_eq!(result.files.get("src/lib.rs"), Some(&83.33));
    assert_eq!(result.files.get("src/utils.rs"), Some(&62.5));
}

#[test]
fn handles_empty_coverage_data() {
    let json = r#"{ "data": [], "type": "llvm.coverage.json.export", "version": "2.0.1" }"#;
    let result = parse_llvm_cov_json(json, Duration::from_secs(0));

    assert!(!result.success);
    assert!(result.error.is_some());
}

#[test]
fn handles_malformed_json() {
    let result = parse_llvm_cov_json("not json", Duration::from_secs(0));

    assert!(!result.success);
    assert!(result.error.unwrap().contains("failed to parse"));
}

#[test]
fn normalizes_coverage_paths() {
    assert_eq!(normalize_coverage_path("/home/user/project/src/lib.rs"), "src/lib.rs");
    assert_eq!(normalize_coverage_path("/workspace/tests/basic.rs"), "tests/basic.rs");
    assert_eq!(normalize_coverage_path("/unknown/path/file.rs"), "file.rs");
}

#[test]
fn extracts_overall_line_coverage() {
    let json = r#"{
        "data": [{
            "totals": { "lines": { "count": 200, "covered": 180, "percent": 90.0 } },
            "files": []
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;

    let result = parse_llvm_cov_json(json, Duration::from_millis(500));

    assert!(result.success);
    assert_eq!(result.line_coverage, Some(90.0));
    assert!(result.files.is_empty());
}

#[test]
fn coverage_result_failed_captures_error() {
    let result = CoverageResult::failed(Duration::from_secs(2), "test error");

    assert!(!result.success);
    assert_eq!(result.error, Some("test error".to_string()));
    assert_eq!(result.duration, Duration::from_secs(2));
    assert!(result.line_coverage.is_none());
    assert!(result.files.is_empty());
}

#[test]
fn coverage_result_skipped_returns_success_without_data() {
    let result = CoverageResult::skipped();

    assert!(result.success);
    assert!(result.error.is_none());
    assert_eq!(result.duration, Duration::ZERO);
    assert!(result.line_coverage.is_none());
    assert!(result.files.is_empty());
    assert!(result.packages.is_empty());
}

// =============================================================================
// PACKAGE EXTRACTION TESTS
// =============================================================================

#[test]
fn extracts_package_name_from_crates_pattern() {
    assert_eq!(extract_package_name("/project/crates/core/src/lib.rs"), "core");
    assert_eq!(extract_package_name("/project/crates/cli/src/main.rs"), "cli");
    assert_eq!(extract_package_name("/home/user/workspace/crates/utils/src/helpers.rs"), "utils");
}

#[test]
fn extracts_package_name_from_packages_pattern() {
    assert_eq!(extract_package_name("/project/packages/utils/index.ts"), "utils");
    assert_eq!(extract_package_name("/workspace/packages/api/src/routes.rs"), "api");
}

#[test]
fn extracts_package_name_falls_back_to_root() {
    assert_eq!(extract_package_name("/project/src/lib.rs"), "root");
    assert_eq!(extract_package_name("/home/user/app/main.rs"), "root");
    assert_eq!(extract_package_name("relative/path/file.rs"), "root");
}

#[test]
fn parses_per_package_coverage_from_llvm_cov() {
    let json = r#"{
        "data": [{
            "totals": { "lines": { "count": 100, "covered": 80, "percent": 80.0 } },
            "files": [
                {
                    "filename": "/home/user/project/crates/core/src/lib.rs",
                    "summary": { "lines": { "count": 60, "covered": 54, "percent": 90.0 } }
                },
                {
                    "filename": "/home/user/project/crates/core/src/utils.rs",
                    "summary": { "lines": { "count": 20, "covered": 16, "percent": 80.0 } }
                },
                {
                    "filename": "/home/user/project/crates/cli/src/main.rs",
                    "summary": { "lines": { "count": 20, "covered": 10, "percent": 50.0 } }
                }
            ]
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;

    let result = parse_llvm_cov_json(json, Duration::from_secs(1));

    assert!(result.success);
    assert_eq!(result.packages.len(), 2);

    // core: average of 90.0 and 80.0 = 85.0
    assert_eq!(result.packages.get("core"), Some(&85.0));

    // cli: 50.0
    assert_eq!(result.packages.get("cli"), Some(&50.0));
}

#[test]
fn parses_per_package_coverage_with_root_fallback() {
    let json = r#"{
        "data": [{
            "totals": { "lines": { "count": 100, "covered": 75, "percent": 75.0 } },
            "files": [
                {
                    "filename": "/home/user/project/src/lib.rs",
                    "summary": { "lines": { "count": 100, "covered": 75, "percent": 75.0 } }
                }
            ]
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;

    let result = parse_llvm_cov_json(json, Duration::from_secs(1));

    assert!(result.success);
    assert_eq!(result.packages.len(), 1);
    assert_eq!(result.packages.get("root"), Some(&75.0));
}
