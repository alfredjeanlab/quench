// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use std::collections::HashMap;
use std::path::PathBuf;

use super::*;

#[test]
fn coverage_env_sets_profile_file() {
    let build =
        InstrumentedBuild { profile_dir: PathBuf::from("/tmp/coverage"), binaries: HashMap::new() };

    let env = coverage_env(&build);
    assert!(env.contains_key("LLVM_PROFILE_FILE"));
    assert!(env["LLVM_PROFILE_FILE"].contains("coverage"));
    assert!(env["LLVM_PROFILE_FILE"].contains("%p-%m.profraw"));
}

#[test]
fn truncate_lines_limits_output() {
    let text = "line1\nline2\nline3\nline4\nline5";
    assert_eq!(truncate_lines(text, 2), "line1\nline2");
    assert_eq!(truncate_lines(text, 10), text);
}

#[test]
fn normalize_path_extracts_src() {
    assert_eq!(normalize_path("/home/user/project/src/lib.rs"), "src/lib.rs");
    assert_eq!(normalize_path("/workspace/myapp/src/main.rs"), "src/main.rs");
}

#[test]
fn normalize_path_extracts_tests() {
    assert_eq!(normalize_path("/home/user/project/tests/basic.rs"), "tests/basic.rs");
}

#[test]
fn normalize_path_falls_back_to_filename() {
    assert_eq!(normalize_path("/unknown/path/file.rs"), "file.rs");
}

#[test]
fn build_instrumented_fails_with_no_targets() {
    let temp = tempfile::TempDir::new().unwrap();
    let result = build_instrumented(&[], temp.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no targets"));
}

#[test]
fn parse_llvm_cov_export_extracts_coverage() {
    let json = r#"{
        "data": [{
            "totals": { "lines": { "count": 100, "covered": 75, "percent": 75.0 } },
            "files": [
                {
                    "filename": "/home/user/project/src/lib.rs",
                    "summary": { "lines": { "count": 60, "covered": 50, "percent": 83.33 } }
                }
            ]
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;

    let result = parse_llvm_cov_export(json);
    assert!(result.is_ok());

    let (total, files) = result.unwrap();
    assert_eq!(total, 75.0);
    assert_eq!(files.get("src/lib.rs"), Some(&83.33));
}

#[test]
fn parse_llvm_cov_export_handles_no_files() {
    let json = r#"{
        "data": [{
            "totals": { "lines": { "count": 100, "covered": 80, "percent": 80.0 } }
        }]
    }"#;

    let result = parse_llvm_cov_export(json);
    assert!(result.is_ok());

    let (total, files) = result.unwrap();
    assert_eq!(total, 80.0);
    assert!(files.is_empty());
}

#[test]
fn parse_llvm_cov_export_fails_on_empty_data() {
    let json = r#"{ "data": [] }"#;
    let result = parse_llvm_cov_export(json);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no coverage data"));
}

#[test]
fn parse_llvm_cov_export_fails_on_invalid_json() {
    let result = parse_llvm_cov_export("not json");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("failed to parse"));
}
