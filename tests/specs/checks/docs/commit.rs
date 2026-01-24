//! Behavioral specs for commit checking in CI mode.
//!
//! Reference: docs/specs/checks/docs.md#ci-mode-commit-checking

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;
use std::process::Command;

// =============================================================================
// CI MODE COMMIT CHECKING SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#how-it-works
///
/// > Identify commits with `feat:` or `feat(area):` prefixes.
/// > Report when feature commits lack corresponding doc changes.
#[test]
#[ignore = "TODO: Phase 603 - Docs Check CI Mode"]
fn feature_commit_without_doc_change_generates_violation_ci_mode() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[check.docs.commit]
check = "error"
"#,
    )
    .unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create initial commit on main
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial commit"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create feature branch
    Command::new("git")
        .args(["checkout", "-b", "feature/new-thing"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Add feature commit without docs
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/feature.rs"), "pub fn new_feature() {}").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feat: add new feature"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    check("docs")
        .pwd(dir.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("feature commits without documentation");
}

/// Spec: docs/specs/checks/docs.md#area-mapping
///
/// > Use area mappings to require specific documentation for scoped commits.
#[test]
#[ignore = "TODO: Phase 603 - Docs Check CI Mode"]
fn area_mapping_restricts_doc_requirement_to_specific_paths() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[check.docs.commit]
check = "error"

[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"
"#,
    )
    .unwrap();

    // Initialize git repo with main branch
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Initial commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Feature branch with api scope
    Command::new("git")
        .args(["checkout", "-b", "feature/api-endpoint"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::fs::create_dir_all(dir.path().join("src/api")).unwrap();
    std::fs::write(
        dir.path().join("src/api/endpoint.rs"),
        "pub fn endpoint() {}",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feat(api): add endpoint"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    check("docs")
        .pwd(dir.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("feat(api)")
        .stdout_has("docs/api/**");
}

/// Spec: docs/specs/checks/docs.md#check-levels
///
/// > `off` - Disable commit checking (default).
#[test]
#[ignore = "TODO: Phase 603 - Docs Check CI Mode"]
fn commit_checking_disabled_by_default() {
    let dir = temp_project();
    // No [check.docs.commit] section - should be disabled

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["checkout", "-b", "feature/thing"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    std::fs::write(dir.path().join("new.rs"), "fn new() {}").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feat: new thing"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // With commit checking disabled, should pass even without docs
    check("docs").pwd(dir.path()).args(&["--ci"]).passes();
}
