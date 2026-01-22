//! Behavioral specs for file walking functionality.
//!
//! Tests that quench correctly discovers files while respecting:
//! - .gitignore rules
//! - Custom ignore patterns from configuration
//! - Symlink loop detection
//! - Directory depth limits
//!
//! Reference: docs/specs/20-performance.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// Gitignore Handling
// =============================================================================

/// Spec: docs/specs/20-performance.md#parallel-gitignore-aware-file-walking
///
/// > Respects `.gitignore`, `.ignore`, global ignores
#[test]
fn file_walking_respects_gitignore() {
    // Files in target/ should not appear in scan results
    // src/lib.rs should be scanned
    // target/debug.rs should be ignored
    quench_cmd()
        .args(["check", "--debug-files"]) // hypothetical flag to list scanned files
        .current_dir(fixture("gitignore-test"))
        .assert()
        .success()
        .stdout(predicates::str::contains("src/lib.rs"))
        .stdout(predicates::str::contains("target/").not());
}

/// Spec: docs/specs/20-performance.md#parallel-gitignore-aware-file-walking
///
/// > Gitignore filtering must happen during traversal, not after
#[test]
fn file_walking_ignores_gitignore_glob_patterns() {
    // Files matching *.generated.rs should be ignored
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("gitignore-test"))
        .assert()
        .success()
        .stdout(predicates::str::contains(".generated.rs").not());
}

/// Spec: docs/specs/20-performance.md#parallel-gitignore-aware-file-walking
///
/// > Respects `.gitignore` in subdirectories
#[test]
fn file_walking_respects_nested_gitignore() {
    // Nested .gitignore files should also be respected
    // This tests that the walker properly inherits gitignore rules
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("gitignore-test"))
        .assert()
        .success()
        .stdout(predicates::str::contains("vendor/").not());
}

// =============================================================================
// Custom Ignore Patterns
// =============================================================================

/// Spec: docs/specs/20-performance.md (custom ignore patterns)
///
/// > Custom ignore patterns from quench.toml should be respected
#[test]
fn file_walking_respects_custom_ignore_patterns() {
    // Files matching patterns in [project.ignore] should be ignored
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("custom-ignore"))
        .assert()
        .success()
        .stdout(predicates::str::contains("src/lib.rs"))
        .stdout(predicates::str::contains(".snapshot").not());
}

/// Spec: docs/specs/20-performance.md (custom ignore patterns)
///
/// > Directory patterns should exclude entire directories
#[test]
fn file_walking_respects_custom_directory_patterns() {
    // testdata/ directory should be completely ignored
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("custom-ignore"))
        .assert()
        .success()
        .stdout(predicates::str::contains("testdata/").not());
}

/// Spec: docs/specs/20-performance.md (custom ignore patterns)
///
/// > Glob patterns with ** should match at any depth
#[test]
fn file_walking_respects_double_star_patterns() {
    // **/fixtures/** should match fixtures at any depth
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("custom-ignore"))
        .assert()
        .success()
        .stdout(predicates::str::contains("fixtures/").not());
}

// =============================================================================
// Symlink Loop Detection
// =============================================================================

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Detect and skip symlink loops
#[test]
fn file_walking_detects_symlink_loops() {
    // A symlink pointing to itself or parent should not cause infinite recursion
    // The test should complete without hanging (timeout enforced by test runner)
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("symlink-loop"))
        .assert()
        .success(); // Should complete, not hang
}

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Symlink loops should be reported when verbose
#[test]
fn file_walking_reports_symlink_loops_in_verbose_mode() {
    // With --verbose, symlink loops should be mentioned
    quench_cmd()
        .args(["check", "--verbose"])
        .current_dir(fixture("symlink-loop"))
        .assert()
        .success()
        .stderr(predicates::str::contains("symlink").or(predicates::str::contains("loop")));
}

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Normal files should still be scanned when symlink loops exist
#[test]
fn file_walking_scans_normal_files_despite_symlink_loops() {
    // src/lib.rs should still be scanned even though a loop exists
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("symlink-loop"))
        .assert()
        .success()
        .stdout(predicates::str::contains("src/lib.rs"));
}

// =============================================================================
// Directory Depth Limits
// =============================================================================

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Limit directory depth (default: 100 levels)
#[test]
#[ignore = "TODO: Create bench-deep fixture"]
fn file_walking_respects_default_depth_limit() {
    // Files beyond depth 100 should not be scanned
    // bench-deep has files at level 50 (within limit) and 120 (beyond)
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("bench-deep"))
        .assert()
        .success()
        .stdout(predicates::str::contains("mid.rs")) // level 50, within limit
        .stdout(predicates::str::contains("deep.rs").not()); // level 120, beyond limit
}

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Depth limit should be configurable
#[test]
#[ignore = "TODO: Create bench-deep fixture"]
fn file_walking_respects_custom_depth_limit() {
    // With a lower depth limit, fewer files should be scanned
    // This tests the --max-depth flag or config option
    quench_cmd()
        .args(["check", "--debug-files", "--max-depth", "25"])
        .current_dir(fixture("bench-deep"))
        .assert()
        .success()
        .stdout(predicates::str::contains("mid.rs").not()); // level 50, now beyond limit
}

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Depth limit warnings in verbose mode
#[test]
#[ignore = "TODO: Create bench-deep fixture"]
fn file_walking_warns_on_depth_limit_in_verbose() {
    // When files are skipped due to depth, verbose mode should mention it
    quench_cmd()
        .args(["check", "--verbose"])
        .current_dir(fixture("bench-deep"))
        .assert()
        .success()
        .stderr(predicates::str::contains("depth").or(predicates::str::contains("limit")));
}

// =============================================================================
// Edge Cases
// =============================================================================

/// Spec: docs/specs/20-performance.md#large-file-counts
///
/// > Never build unbounded in-memory file lists
#[test]
fn file_walking_handles_empty_directory() {
    // Empty directories should not cause errors
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("minimal"))
        .assert()
        .success();
}

/// Spec: docs/specs/20-performance.md#parallel-gitignore-aware-file-walking
///
/// > Use iterative traversal, not recursive
#[test]
#[ignore = "TODO: Create bench-deep fixture"]
fn file_walking_uses_iterative_traversal() {
    // This is tested implicitly by bench-deep - recursive traversal
    // would cause stack overflow at 120 levels on most systems
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("bench-deep"))
        .assert()
        .success(); // Should complete without stack overflow
}
