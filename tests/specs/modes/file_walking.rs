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
use std::sync::Once;

static BENCH_DEEP_INIT: Once = Once::new();

/// Ensures bench-deep fixture exists (generates if missing).
fn ensure_bench_deep_fixture() {
    BENCH_DEEP_INIT.call_once(|| {
        let fixture_dir = fixture("bench-deep");
        if fixture_dir.join("deep").exists() {
            return;
        }

        std::fs::create_dir_all(&fixture_dir).unwrap();
        std::fs::write(
            fixture_dir.join("quench.toml"),
            "version = 1\n\n[git.commit]\nagents = false\n",
        )
        .unwrap();
        std::fs::write(
            fixture_dir.join("CLAUDE.md"),
            "# Bench Deep\n\n## Directory Structure\n\nDeep.\n\n## Landing the Plane\n\n- Done\n",
        )
        .unwrap();

        // Build nested path: deep/d1/d2/.../d120
        let mut deep_path = fixture_dir.join("deep");
        for i in 1..=120 {
            deep_path = deep_path.join(format!("d{}", i));
        }
        std::fs::create_dir_all(&deep_path).unwrap();
        std::fs::write(
            deep_path.join("deep.rs"),
            "//! File at maximum depth.\npub fn at_depth() -> &'static str { \"deep\" }\n",
        )
        .unwrap();

        // File at level 50
        let mut mid_path = fixture_dir.join("deep");
        for i in 1..=50 {
            mid_path = mid_path.join(format!("d{}", i));
        }
        std::fs::write(
            mid_path.join("mid.rs"),
            "//! File at mid depth.\npub fn at_mid() -> i32 { 50 }\n",
        )
        .unwrap();
    });
}

// =============================================================================
// Gitignore Handling
// =============================================================================

/// Spec: docs/specs/20-performance.md#parallel-gitignore-aware-file-walking
///
/// > Respects `.gitignore`, `.ignore`, global ignores
#[test]
fn file_walking_respects_gitignore() {
    // Files in target/ should not appear in scan results
    // src/lib.rs should be scanned, target/debug.rs should be ignored
    cli()
        .on("gitignore-test")
        .args(&["--debug-files"])
        .passes()
        .stdout_has("src/lib.rs")
        .stdout_lacks("target/");
}

/// Spec: docs/specs/20-performance.md#parallel-gitignore-aware-file-walking
///
/// > Gitignore filtering must happen during traversal, not after
#[test]
fn file_walking_ignores_gitignore_glob_patterns() {
    // Files matching *.generated.rs should be ignored
    cli()
        .on("gitignore-test")
        .args(&["--debug-files"])
        .passes()
        .stdout_lacks(".generated.rs");
}

/// Spec: docs/specs/20-performance.md#parallel-gitignore-aware-file-walking
///
/// > Respects `.gitignore` in subdirectories
#[test]
fn file_walking_respects_nested_gitignore() {
    // Nested .gitignore files should also be respected
    cli()
        .on("gitignore-test")
        .args(&["--debug-files"])
        .passes()
        .stdout_lacks("vendor/");
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
    cli()
        .on("custom-ignore")
        .args(&["--debug-files"])
        .passes()
        .stdout_has("src/lib.rs")
        .stdout_lacks(".snapshot");
}

/// Spec: docs/specs/20-performance.md (custom ignore patterns)
///
/// > Directory patterns should exclude entire directories
#[test]
fn file_walking_respects_custom_directory_patterns() {
    // testdata/ directory should be completely ignored
    cli()
        .on("custom-ignore")
        .args(&["--debug-files"])
        .passes()
        .stdout_lacks("testdata/");
}

/// Spec: docs/specs/20-performance.md (custom ignore patterns)
///
/// > Glob patterns with ** should match at any depth
#[test]
fn file_walking_respects_double_star_patterns() {
    // **/fixtures/** should match fixtures at any depth
    cli()
        .on("custom-ignore")
        .args(&["--debug-files"])
        .passes()
        .stdout_lacks("fixtures/");
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
    cli().on("symlink-loop").passes();
}

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Symlink loops should be reported when verbose
#[test]
fn file_walking_reports_symlink_loops_in_verbose_mode() {
    // With --verbose, symlink loops should be mentioned
    cli()
        .on("symlink-loop")
        .args(&["--verbose"])
        .passes()
        .stderr_has(predicates::str::contains("symlink").or(predicates::str::contains("loop")));
}

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Normal files should still be scanned when symlink loops exist
#[test]
fn file_walking_scans_normal_files_despite_symlink_loops() {
    // src/lib.rs should still be scanned even though a loop exists
    cli()
        .on("symlink-loop")
        .args(&["--debug-files"])
        .passes()
        .stdout_has("src/lib.rs");
}

// =============================================================================
// Directory Depth Limits
// =============================================================================

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Limit directory depth (default: 100 levels)
#[test]
fn file_walking_respects_default_depth_limit() {
    ensure_bench_deep_fixture();
    // Files beyond depth 100 should not be scanned
    // bench-deep has files at level 50 (within limit) and 120 (beyond)
    cli()
        .on("bench-deep")
        .args(&["--debug-files"])
        .passes()
        .stdout_has("mid.rs") // level 50, within limit
        .stdout_lacks("deep.rs"); // level 120, beyond limit
}

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Depth limit should be configurable
#[test]
fn file_walking_respects_custom_depth_limit() {
    ensure_bench_deep_fixture();
    // With a lower depth limit, fewer files should be scanned
    cli()
        .on("bench-deep")
        .args(&["--debug-files", "--max-depth", "25"])
        .passes()
        .stdout_lacks("mid.rs"); // level 50, now beyond limit
}

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Depth limit warnings in verbose mode
#[test]
fn file_walking_warns_on_depth_limit_in_verbose() {
    ensure_bench_deep_fixture();
    // When files are skipped due to depth, verbose mode should mention it
    cli()
        .on("bench-deep")
        .args(&["--verbose"])
        .passes()
        .stderr_has(predicates::str::contains("depth").or(predicates::str::contains("limit")));
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
    cli().on("minimal").passes();
}

/// Spec: docs/specs/20-performance.md#parallel-gitignore-aware-file-walking
///
/// > Use iterative traversal, not recursive
#[test]
fn file_walking_uses_iterative_traversal() {
    ensure_bench_deep_fixture();
    // This is tested implicitly by bench-deep - recursive traversal
    // would cause stack overflow at 120 levels on most systems
    cli().on("bench-deep").passes();
}
