// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use std::time::Duration;
use yare::parameterized;

// =============================================================================
// PROFILE PARSING TESTS
// =============================================================================

#[test]
fn parses_empty_profile() {
    let content = "mode: set\n";
    let result = parse_cover_profile(content, Duration::ZERO);

    assert!(result.success);
    assert!(result.line_coverage.is_none());
    assert!(result.files.is_empty());
    assert!(result.packages.is_empty());
}

#[test]
fn parses_single_file_profile() {
    let content = r#"mode: set
github.com/example/pkg/math/math.go:5.14,7.2 1 1
github.com/example/pkg/math/math.go:9.14,11.2 1 0
"#;
    let result = parse_cover_profile(content, Duration::from_secs(1));

    assert!(result.success);
    assert!(result.line_coverage.is_some());
    // 1 covered out of 2 statements = 50%
    let coverage = result.line_coverage.unwrap();
    assert!((coverage - 50.0).abs() < 0.1, "Expected 50%, got {coverage}");
}

#[test]
fn parses_multi_file_profile() {
    let content = r#"mode: set
github.com/example/pkg/math/add.go:5.14,7.2 2 1
github.com/example/pkg/math/sub.go:5.14,7.2 2 1
github.com/example/internal/core/core.go:5.14,7.2 2 0
"#;
    let result = parse_cover_profile(content, Duration::from_secs(1));

    assert!(result.success);
    assert_eq!(result.files.len(), 3);

    // Check package aggregation
    assert!(result.packages.contains_key("pkg/math"));
    assert!(result.packages.contains_key("internal/core"));

    // pkg/math: 4 statements covered out of 4 = 100%
    let math_coverage = result.packages.get("pkg/math").unwrap();
    assert!((math_coverage - 100.0).abs() < 0.1);

    // internal/core: 0 covered out of 2 = 0%
    let core_coverage = result.packages.get("internal/core").unwrap();
    assert!(core_coverage.abs() < 0.1);
}

#[parameterized(
    zero_coverage = { "github.com/example/pkg/math/math.go:5.14,7.2 5 0", 0.0 },
    full_coverage = { "github.com/example/pkg/math/math.go:5.14,7.2 5 1\ngithub.com/example/pkg/math/math.go:9.14,11.2 3 1", 100.0 },
)]
fn parses_coverage_extremes(lines: &str, expected_coverage: f64) {
    let content = format!("mode: set\n{}\n", lines);
    let result = parse_cover_profile(&content, Duration::ZERO);

    assert!(result.success);
    let coverage = result.line_coverage.unwrap();
    assert!(
        (coverage - expected_coverage).abs() < 0.1,
        "Expected {expected_coverage}%, got {coverage}"
    );
}

#[test]
fn parses_profile_with_multiple_coverages() {
    // Some blocks hit multiple times
    let content = r#"mode: set
github.com/example/pkg/math/math.go:5.14,7.2 1 5
github.com/example/pkg/math/math.go:9.14,11.2 1 0
"#;
    let result = parse_cover_profile(content, Duration::ZERO);

    assert!(result.success);
    // count > 0 means covered, regardless of how many times
    let coverage = result.line_coverage.unwrap();
    assert!((coverage - 50.0).abs() < 0.1, "Expected 50%, got {coverage}");
}

// =============================================================================
// PROFILE LINE PARSING TESTS
// =============================================================================

#[parameterized(
    basic = { "github.com/example/pkg/math/math.go:5.14,7.2 1 1", Some(("github.com/example/pkg/math/math.go", 1u64, 1u64)) },
    zero_count = { "github.com/example/pkg/math/math.go:5.14,7.2 3 0", Some(("github.com/example/pkg/math/math.go", 3u64, 0u64)) },
    large_numbers = { "github.com/example/pkg/math/math.go:5.14,7.2 100 50", Some(("github.com/example/pkg/math/math.go", 100u64, 50u64)) },
    missing_count = { "file.go:5.14,7.2 1", None },
    missing_statements = { "file.go:5.14,7.2", None },
    empty_line = { "", None },
    invalid_numbers = { "file.go:5.14,7.2 abc def", None },
)]
fn parse_profile_line_cases(line: &str, expected: Option<(&str, u64, u64)>) {
    let result = parse_profile_line(line);
    match expected {
        Some((file, statements, count)) => {
            let (f, s, c) = result.unwrap();
            assert_eq!(f, file);
            assert_eq!(s, statements);
            assert_eq!(c, count);
        }
        None => assert!(result.is_none()),
    }
}

// =============================================================================
// PACKAGE EXTRACTION TESTS
// =============================================================================

#[parameterized(
    pkg_path = { "github.com/user/repo/pkg/math/math.go", "pkg/math" },
    internal_path = { "github.com/user/repo/internal/core/core.go", "internal/core" },
    cmd_path = { "github.com/user/repo/cmd/server/main.go", "cmd/server" },
    top_level = { "github.com/user/repo/main.go", "root" },
    nested = { "github.com/user/repo/pkg/api/v2/handlers/user.go", "pkg/api/v2/handlers" },
)]
fn extract_go_package_cases(path: &str, expected: &str) {
    assert_eq!(extract_go_package(path), expected);
}

// =============================================================================
// PATH NORMALIZATION TESTS
// =============================================================================

#[parameterized(
    pkg_path = { "github.com/user/repo/pkg/math/math.go", "pkg/math/math.go" },
    internal_path = { "github.com/user/repo/internal/core/core.go", "internal/core/core.go" },
    cmd_path = { "github.com/user/repo/cmd/server/main.go", "cmd/server/main.go" },
    top_level = { "github.com/user/repo/main.go", "main.go" },
)]
fn normalize_go_path_cases(path: &str, expected: &str) {
    assert_eq!(normalize_go_path(path), expected);
}
