#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
    assert_eq!(
        normalize_coverage_path("/home/user/project/src/lib.rs"),
        "src/lib.rs"
    );
    assert_eq!(
        normalize_coverage_path("/workspace/tests/basic.rs"),
        "tests/basic.rs"
    );
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
}
