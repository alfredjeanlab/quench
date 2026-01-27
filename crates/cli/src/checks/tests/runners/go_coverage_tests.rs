#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::time::Duration;

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
    assert!(
        (coverage - 50.0).abs() < 0.1,
        "Expected 50%, got {coverage}"
    );
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

#[test]
fn parses_zero_coverage() {
    let content = r#"mode: set
github.com/example/pkg/math/math.go:5.14,7.2 5 0
"#;
    let result = parse_cover_profile(content, Duration::ZERO);

    assert!(result.success);
    let coverage = result.line_coverage.unwrap();
    assert!(coverage.abs() < 0.1, "Expected 0%, got {coverage}");
}

#[test]
fn parses_full_coverage() {
    let content = r#"mode: set
github.com/example/pkg/math/math.go:5.14,7.2 5 1
github.com/example/pkg/math/math.go:9.14,11.2 3 1
"#;
    let result = parse_cover_profile(content, Duration::ZERO);

    assert!(result.success);
    let coverage = result.line_coverage.unwrap();
    assert!(
        (coverage - 100.0).abs() < 0.1,
        "Expected 100%, got {coverage}"
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
    assert!(
        (coverage - 50.0).abs() < 0.1,
        "Expected 50%, got {coverage}"
    );
}

// =============================================================================
// PROFILE LINE PARSING TESTS
// =============================================================================

#[test]
fn parses_profile_line_basic() {
    let line = "github.com/example/pkg/math/math.go:5.14,7.2 1 1";
    let (file, statements, count) = parse_profile_line(line).unwrap();

    assert_eq!(file, "github.com/example/pkg/math/math.go");
    assert_eq!(statements, 1);
    assert_eq!(count, 1);
}

#[test]
fn parses_profile_line_zero_count() {
    let line = "github.com/example/pkg/math/math.go:5.14,7.2 3 0";
    let (_file, statements, count) = parse_profile_line(line).unwrap();

    assert_eq!(statements, 3);
    assert_eq!(count, 0);
}

#[test]
fn parses_profile_line_large_numbers() {
    let line = "github.com/example/pkg/math/math.go:5.14,7.2 100 50";
    let (_, statements, count) = parse_profile_line(line).unwrap();

    assert_eq!(statements, 100);
    assert_eq!(count, 50);
}

#[test]
fn rejects_malformed_profile_line() {
    // Missing count
    assert!(parse_profile_line("file.go:5.14,7.2 1").is_none());
    // Missing statements
    assert!(parse_profile_line("file.go:5.14,7.2").is_none());
    // Empty line
    assert!(parse_profile_line("").is_none());
    // Invalid numbers
    assert!(parse_profile_line("file.go:5.14,7.2 abc def").is_none());
}

// =============================================================================
// PACKAGE EXTRACTION TESTS
// =============================================================================

#[test]
fn extracts_package_from_pkg_path() {
    let path = "github.com/user/repo/pkg/math/math.go";
    assert_eq!(extract_go_package(path), "pkg/math");
}

#[test]
fn extracts_package_from_internal_path() {
    let path = "github.com/user/repo/internal/core/core.go";
    assert_eq!(extract_go_package(path), "internal/core");
}

#[test]
fn extracts_package_from_cmd_path() {
    let path = "github.com/user/repo/cmd/server/main.go";
    assert_eq!(extract_go_package(path), "cmd/server");
}

#[test]
fn extracts_root_for_top_level_files() {
    let path = "github.com/user/repo/main.go";
    assert_eq!(extract_go_package(path), "root");
}

#[test]
fn extracts_nested_package() {
    let path = "github.com/user/repo/pkg/api/v2/handlers/user.go";
    assert_eq!(extract_go_package(path), "pkg/api/v2/handlers");
}

// =============================================================================
// PATH NORMALIZATION TESTS
// =============================================================================

#[test]
fn normalizes_pkg_path() {
    let path = "github.com/user/repo/pkg/math/math.go";
    assert_eq!(normalize_go_path(path), "pkg/math/math.go");
}

#[test]
fn normalizes_internal_path() {
    let path = "github.com/user/repo/internal/core/core.go";
    assert_eq!(normalize_go_path(path), "internal/core/core.go");
}

#[test]
fn normalizes_cmd_path() {
    let path = "github.com/user/repo/cmd/server/main.go";
    assert_eq!(normalize_go_path(path), "cmd/server/main.go");
}

#[test]
fn normalizes_top_level_to_filename() {
    let path = "github.com/user/repo/main.go";
    assert_eq!(normalize_go_path(path), "main.go");
}
