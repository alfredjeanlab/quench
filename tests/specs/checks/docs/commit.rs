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
fn feature_commit_without_doc_change_generates_violation_ci_mode() {
    let temp = Project::empty();
    temp.config(
        r#"[check.docs.commit]
check = "error"
"#,
    );

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create initial commit on main
    Command::new("git")
        .args(["add", "."])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial commit"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create feature branch
    Command::new("git")
        .args(["checkout", "-b", "feature/new-thing"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Add feature commit without docs
    temp.file("src/feature.rs", "pub fn new_feature() {}");
    Command::new("git")
        .args(["add", "."])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feat: add new feature"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    check("docs")
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("feature commits without documentation");
}

/// Spec: docs/specs/checks/docs.md#area-mapping
///
/// > Use area mappings to require specific documentation for scoped commits.
#[test]
fn area_mapping_restricts_doc_requirement_to_specific_paths() {
    let temp = Project::empty();
    temp.config(
        r#"[check.docs.commit]
check = "error"

[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"
"#,
    );

    // Initialize git repo with main branch
    Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Initial commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Feature branch with api scope
    Command::new("git")
        .args(["checkout", "-b", "feature/api-endpoint"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    temp.file("src/api/endpoint.rs", "pub fn endpoint() {}");
    Command::new("git")
        .args(["add", "."])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feat(api): add endpoint"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    check("docs")
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("feat(api)")
        .stdout_has("docs/api/**");
}

/// Spec: docs/specs/checks/docs.md#check-levels
///
/// > `off` - Disable commit checking (default).
#[test]
fn commit_checking_disabled_by_default() {
    let temp = default_project();
    // No [check.docs.commit] section - should be disabled

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["checkout", "-b", "feature/thing"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    temp.file("new.rs", "fn new() {}");
    Command::new("git")
        .args(["add", "."])
        .current_dir(temp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feat: new thing"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // With commit checking disabled, should pass even without docs
    check("docs").pwd(temp.path()).args(&["--ci"]).passes();
}
