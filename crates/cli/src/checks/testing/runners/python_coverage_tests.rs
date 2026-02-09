// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use super::*;

// =============================================================================
// JSON Parser Tests
// =============================================================================

#[test]
fn parses_coverage_json_complete() {
    let json = r#"{
        "meta": {
            "version": "7.4.0",
            "branch_coverage": true
        },
        "files": {
            "/home/user/project/src/myproject/math.py": {
                "executed_lines": [1, 2, 3, 5, 6],
                "missing_lines": [10, 11],
                "excluded_lines": [],
                "summary": {
                    "covered_lines": 5,
                    "num_statements": 7,
                    "percent_covered": 71.43
                }
            },
            "/home/user/project/src/myproject/utils.py": {
                "executed_lines": [1, 2, 3, 4],
                "missing_lines": [],
                "excluded_lines": [],
                "summary": {
                    "covered_lines": 4,
                    "num_statements": 4,
                    "percent_covered": 100.0
                }
            }
        },
        "totals": {
            "covered_lines": 9,
            "num_statements": 11,
            "percent_covered": 81.82
        }
    }"#;

    let result = parse_coverage_json(json, Duration::ZERO);

    assert!(result.success);
    assert!(result.error.is_none());
    assert_eq!(result.line_coverage, Some(81.82));
    assert_eq!(result.files.len(), 2);
    assert_eq!(result.files.get("src/myproject/math.py"), Some(&71.43));
    assert_eq!(result.files.get("src/myproject/utils.py"), Some(&100.0));
    assert!(result.packages.contains_key("myproject"));
}

#[test]
fn parses_coverage_json_minimal() {
    let json = r#"{
        "meta": {},
        "files": {
            "src/app.py": {
                "executed_lines": [1, 2, 3],
                "missing_lines": [5],
                "summary": {
                    "covered_lines": 3,
                    "num_statements": 4,
                    "percent_covered": 75.0
                }
            }
        },
        "totals": {
            "covered_lines": 3,
            "num_statements": 4,
            "percent_covered": 75.0
        }
    }"#;

    let result = parse_coverage_json(json, Duration::ZERO);

    assert!(result.success);
    assert_eq!(result.line_coverage, Some(75.0));
    assert_eq!(result.files.len(), 1);
}

#[test]
fn parses_coverage_json_empty_files() {
    let json = r#"{
        "meta": {"version": "7.4.0"},
        "files": {},
        "totals": {
            "covered_lines": 0,
            "num_statements": 0,
            "percent_covered": 0.0
        }
    }"#;

    let result = parse_coverage_json(json, Duration::ZERO);

    assert!(result.success);
    assert_eq!(result.line_coverage, Some(0.0));
    assert!(result.files.is_empty());
}

#[test]
fn parses_coverage_json_invalid() {
    let json = "not valid json";

    let result = parse_coverage_json(json, Duration::ZERO);

    assert!(!result.success);
    assert!(result.error.is_some());
    assert!(result.error.unwrap().contains("failed to parse"));
}

// =============================================================================
// XML Parser Tests
// =============================================================================

#[test]
fn parses_cobertura_xml_complete() {
    let xml = r#"<?xml version="1.0" ?>
<coverage version="7.4.0" timestamp="1234567890" lines-valid="30" lines-covered="25" line-rate="0.8333" branch-rate="0">
    <packages>
        <package name="myproject" line-rate="0.8333" branch-rate="0" complexity="0">
            <classes>
                <class name="math.py" filename="src/myproject/math.py" line-rate="0.75" branch-rate="0" complexity="0">
                    <methods/>
                    <lines>
                        <line number="1" hits="1"/>
                        <line number="2" hits="1"/>
                        <line number="3" hits="1"/>
                        <line number="10" hits="0"/>
                    </lines>
                </class>
                <class name="utils.py" filename="src/myproject/utils.py" line-rate="1.0" branch-rate="0" complexity="0">
                    <methods/>
                    <lines>
                        <line number="1" hits="1"/>
                    </lines>
                </class>
            </classes>
        </package>
    </packages>
</coverage>"#;

    let result = parse_cobertura_xml(xml, Duration::ZERO);

    assert!(result.success);
    assert!(result.error.is_none());
    // line-rate is 0.8333, which should become 83.33%
    assert!((result.line_coverage.unwrap() - 83.33).abs() < 0.01);
    assert_eq!(result.files.len(), 2);
    assert_eq!(result.files.get("src/myproject/math.py"), Some(&75.0));
    assert_eq!(result.files.get("src/myproject/utils.py"), Some(&100.0));
    assert!(result.packages.contains_key("myproject"));
}

#[test]
fn parses_cobertura_xml_minimal() {
    let xml = r#"<?xml version="1.0" ?>
<coverage line-rate="0.75">
    <packages>
        <package name="root">
            <classes>
                <class filename="app.py" line-rate="0.75"/>
            </classes>
        </package>
    </packages>
</coverage>"#;

    let result = parse_cobertura_xml(xml, Duration::ZERO);

    assert!(result.success);
    assert_eq!(result.line_coverage, Some(75.0));
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files.get("app.py"), Some(&75.0));
}

#[test]
fn parses_cobertura_xml_self_closing_classes() {
    let xml = r#"<?xml version="1.0" ?>
<coverage line-rate="0.80">
    <packages>
        <package name="mypackage">
            <classes>
                <class filename="src/mypackage/core.py" line-rate="0.80"/>
            </classes>
        </package>
    </packages>
</coverage>"#;

    let result = parse_cobertura_xml(xml, Duration::ZERO);

    assert!(result.success);
    assert_eq!(result.line_coverage, Some(80.0));
    assert_eq!(result.files.len(), 1);
}

// =============================================================================
// Path Normalization Tests
// =============================================================================

#[test]
fn normalizes_absolute_src_layout_path() {
    assert_eq!(
        normalize_python_path("/home/user/project/src/myproject/math.py"),
        "src/myproject/math.py"
    );
}

#[test]
fn normalizes_absolute_tests_path() {
    assert_eq!(
        normalize_python_path("/home/user/project/tests/test_math.py"),
        "tests/test_math.py"
    );
}

#[test]
fn normalizes_relative_path() {
    assert_eq!(normalize_python_path("src/myproject/math.py"), "src/myproject/math.py");
}

#[test]
fn normalizes_lib_path() {
    assert_eq!(normalize_python_path("/home/user/project/lib/helpers.py"), "lib/helpers.py");
}

#[test]
fn normalizes_app_path() {
    assert_eq!(normalize_python_path("/home/user/project/app/models.py"), "app/models.py");
}

#[test]
fn normalizes_site_packages_to_filename() {
    let path = "/usr/lib/python3.11/site-packages/requests/api.py";
    assert_eq!(normalize_python_path(path), "api.py");
}

#[test]
fn normalizes_unknown_absolute_to_filename() {
    assert_eq!(normalize_python_path("/some/unknown/path/file.py"), "file.py");
}

// =============================================================================
// Package Extraction Tests
// =============================================================================

#[test]
fn extracts_package_from_src_layout() {
    assert_eq!(extract_python_package("src/myproject/math.py"), "myproject");
    assert_eq!(extract_python_package("src/myproject/subpackage/utils.py"), "myproject");
}

#[test]
fn extracts_package_from_flat_layout() {
    assert_eq!(extract_python_package("myproject/math.py"), "myproject");
    assert_eq!(extract_python_package("myproject/subpackage/utils.py"), "myproject");
}

#[test]
fn extracts_tests_package() {
    assert_eq!(extract_python_package("tests/test_math.py"), "tests");
    assert_eq!(extract_python_package("tests/unit/test_core.py"), "tests");
}

#[test]
fn extracts_root_for_single_file() {
    assert_eq!(extract_python_package("app.py"), "root");
    assert_eq!(extract_python_package("conftest.py"), "root");
}

#[test]
fn extracts_package_from_src_single_file() {
    // When src/ contains a single file directly
    assert_eq!(extract_python_package("src/main.py"), "main.py");
}

// =============================================================================
// Coverage Result Tests
// =============================================================================

#[test]
fn json_reports_multiple_packages() {
    let json = r#"{
        "meta": {},
        "files": {
            "src/core/engine.py": {
                "executed_lines": [1, 2],
                "missing_lines": [],
                "summary": {"covered_lines": 2, "num_statements": 2, "percent_covered": 100.0}
            },
            "src/api/routes.py": {
                "executed_lines": [1],
                "missing_lines": [2],
                "summary": {"covered_lines": 1, "num_statements": 2, "percent_covered": 50.0}
            },
            "src/api/handlers.py": {
                "executed_lines": [1, 2, 3],
                "missing_lines": [4],
                "summary": {"covered_lines": 3, "num_statements": 4, "percent_covered": 75.0}
            }
        },
        "totals": {
            "covered_lines": 6,
            "num_statements": 8,
            "percent_covered": 75.0
        }
    }"#;

    let result = parse_coverage_json(json, Duration::ZERO);

    assert!(result.success);
    assert_eq!(result.packages.len(), 2);
    assert_eq!(result.packages.get("core"), Some(&100.0));
    // api package average: (50 + 75) / 2 = 62.5
    assert_eq!(result.packages.get("api"), Some(&62.5));
}
