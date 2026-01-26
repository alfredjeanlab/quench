//! Large file handling behavioral tests.
//!
//! Verifies that files >10MB are skipped per docs/specs/20-performance.md.
//!
//! Spec: docs/specs/20-performance.md#large-file-handling
//!
//! > Files exceeding 10MB are not read at all—skipped with a warning.

use crate::prelude::*;
use std::fs::File;

/// Files over 10MB are skipped with a warning, not processed.
///
/// Per docs/specs/20-performance.md:
/// > Files exceeding 10MB are not read at all—skipped with a warning.
#[test]
fn large_file_skipped_with_warning() {
    let project = default_project();

    // Create a normal small file
    project.file("src/lib.rs", "fn main() {}");

    // Create 15MB sparse file (larger than 10MB limit)
    let large_path = project.path().join("src/huge.rs");
    let large_file = File::create(&large_path).unwrap();
    large_file.set_len(15 * 1024 * 1024).unwrap();

    // Run with QUENCH_DEBUG and QUENCH_LOG=warn to see the tracing warning
    cli()
        .pwd(project.path())
        .env("QUENCH_DEBUG", "1")
        .env("QUENCH_LOG", "warn")
        .passes()
        .stderr_has("skipping")
        .stderr_has("huge.rs")
        .stderr_has("10MB limit");
}

/// Large file is not counted in check violations.
///
/// When a file exceeds 10MB, it should not appear in any check results
/// since it was skipped during file discovery.
#[test]
fn large_file_not_in_violations() {
    let project = default_project();

    // First create a small file to ensure src exists
    project.file("src/lib.rs", "fn main() {}");

    // Create 15MB sparse file that would trigger violations if processed
    let large_path = project.path().join("src/huge.rs");
    let large_file = File::create(&large_path).unwrap();
    large_file.set_len(15 * 1024 * 1024).unwrap();

    // Run all checks - should pass because huge.rs is skipped
    let result = cli().pwd(project.path()).json().passes();

    // No violations should reference the huge file
    for check in result.checks() {
        if let Some(violations) = check.get("violations").and_then(|v| v.as_array()) {
            for violation in violations {
                let file = violation.get("file").and_then(|f| f.as_str()).unwrap_or("");
                assert!(
                    !file.contains("huge.rs"),
                    "huge.rs should not appear in violations, but found in: {:?}",
                    violation
                );
            }
        }
    }
}

/// Files just under 10MB are still processed.
///
/// The 10MB limit is exclusive: files at exactly 10MB or less should
/// be processed normally.
#[test]
fn file_under_10mb_processed() {
    let project = default_project();

    // Create src directory first
    std::fs::create_dir_all(project.path().join("src")).unwrap();

    // Create file just under the limit (10MB - 1 byte)
    // This is 10,485,759 bytes
    let borderline_path = project.path().join("src/borderline.rs");
    let borderline_file = File::create(&borderline_path).unwrap();
    borderline_file.set_len(10 * 1024 * 1024 - 1).unwrap();

    // This file should be processed and trigger cloc violation
    // for being oversized (>1MB soft limit)
    let cloc = check("cloc").pwd(project.path()).json().fails();
    assert!(
        cloc.has_violation_for_file("borderline.rs"),
        "borderline.rs should be processed and trigger cloc violation"
    );
}

/// Files at exactly 10MB are processed (boundary condition).
///
/// The limit is "greater than 10MB", so exactly 10MB should be processed.
#[test]
fn file_at_10mb_boundary_processed() {
    let project = default_project();

    // Create src directory first
    std::fs::create_dir_all(project.path().join("src")).unwrap();

    // Create file at exactly 10MB
    let boundary_path = project.path().join("src/boundary.rs");
    let boundary_file = File::create(&boundary_path).unwrap();
    boundary_file.set_len(10 * 1024 * 1024).unwrap();

    // This file should be processed (not skipped)
    // It will trigger cloc violation for being oversized
    let cloc = check("cloc").pwd(project.path()).json().fails();
    assert!(
        cloc.has_violation_for_file("boundary.rs"),
        "boundary.rs at exactly 10MB should be processed"
    );
}

/// Multiple large files are all skipped with individual warnings.
#[test]
fn multiple_large_files_skipped() {
    let project = default_project();
    project.file("src/lib.rs", "fn main() {}");

    // Create multiple large files
    for name in ["huge1.rs", "huge2.rs", "huge3.rs"] {
        let path = project.path().join("src").join(name);
        let file = File::create(&path).unwrap();
        file.set_len(15 * 1024 * 1024).unwrap();
    }

    // All should be reported as skipped (with QUENCH_LOG=warn for tracing output)
    cli()
        .pwd(project.path())
        .env("QUENCH_DEBUG", "1")
        .env("QUENCH_LOG", "warn")
        .passes()
        .stderr_has("huge1.rs")
        .stderr_has("huge2.rs")
        .stderr_has("huge3.rs");
}
