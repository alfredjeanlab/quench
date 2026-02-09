// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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
    Command::new("git").args(["init"]).current_dir(temp.path()).output().unwrap();
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
    Command::new("git").args(["add", "."]).current_dir(temp.path()).output().unwrap();
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
    Command::new("git").args(["add", "."]).current_dir(temp.path()).output().unwrap();
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
    Command::new("git").args(["init"]).current_dir(temp.path()).output().unwrap();
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
    Command::new("git").args(["add", "."]).current_dir(temp.path()).output().unwrap();
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
    Command::new("git").args(["add", "."]).current_dir(temp.path()).output().unwrap();
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
    Command::new("git").args(["init"]).current_dir(temp.path()).output().unwrap();
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
    Command::new("git").args(["add", "."]).current_dir(temp.path()).output().unwrap();
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
    Command::new("git").args(["add", "."]).current_dir(temp.path()).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "feat: new thing"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // With commit checking disabled, should pass even without docs
    check("docs").pwd(temp.path()).args(&["--ci"]).passes();
}

// =============================================================================
// SOURCE-BASED AREA MATCHING SPECS
// =============================================================================

/// Helper to initialize a git repo with user config.
fn init_git_repo(path: &std::path::Path) {
    Command::new("git").args(["init", "-b", "main"]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git").args(["config", "user.name", "Test"]).current_dir(path).output().unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial"])
        .current_dir(path)
        .output()
        .unwrap();
}

/// Helper to add and commit files.
fn git_add_commit(path: &std::path::Path, msg: &str) {
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git").args(["commit", "-m", msg]).current_dir(path).output().unwrap();
}

/// Spec: docs/specs/checks/docs.md#source-based-area-matching
///
/// > When source files matching an area's `source` pattern are changed,
/// > require documentation changes matching that area's `docs` pattern.
#[test]
fn source_change_triggers_area_doc_requirement() {
    let temp = Project::empty();
    temp.config(
        r#"[check.docs.commit]
check = "error"

[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"
"#,
    );

    init_git_repo(temp.path());

    // Feature branch with source change but no scope
    Command::new("git")
        .args(["checkout", "-b", "feature/api-change"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    temp.file("src/api/handler.rs", "pub fn handler() {}");
    git_add_commit(temp.path(), "feat: add api handler");

    check("docs")
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("docs/api/**")
        .stdout_has("changes in api area"); // Source-based match message
}

/// Spec: docs/specs/checks/docs.md#multiple-area-matching
///
/// > When source changes match multiple areas, require docs for all.
#[test]
fn multiple_source_areas_require_all_docs() {
    let temp = Project::empty();
    temp.config(
        r#"[check.docs.commit]
check = "error"

[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"

[check.docs.area.cli]
docs = "docs/cli/**"
source = "src/cli/**"
"#,
    );

    init_git_repo(temp.path());

    Command::new("git")
        .args(["checkout", "-b", "feature/multi-area"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    temp.file("src/api/handler.rs", "pub fn api() {}");
    temp.file("src/cli/main.rs", "pub fn cli() {}");
    git_add_commit(temp.path(), "feat: add api and cli");

    check("docs")
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("docs/api/**")
        .stdout_has("docs/cli/**");
}

/// Spec: docs/specs/checks/docs.md#scope-priority
///
/// > Scope-based matching takes priority over source-based matching.
#[test]
fn scope_takes_priority_over_source() {
    let temp = Project::empty();
    temp.config(
        r#"[check.docs.commit]
check = "error"

[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"
"#,
    );

    init_git_repo(temp.path());

    Command::new("git")
        .args(["checkout", "-b", "feature/scoped"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    temp.file("src/api/handler.rs", "pub fn api() {}");
    temp.file("docs/api/handler.md", "# Handler");
    git_add_commit(temp.path(), "feat(api): add handler with docs");

    // Should pass - scope matched and docs exist
    check("docs").pwd(temp.path()).args(&["--ci"]).passes();
}

/// Spec: docs/specs/checks/docs.md#source-with-docs
///
/// > Source-matched areas pass when corresponding docs exist.
#[test]
fn source_match_passes_with_docs() {
    let temp = Project::empty();
    temp.config(
        r#"[check.docs.commit]
check = "error"

[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"
"#,
    );

    init_git_repo(temp.path());

    Command::new("git")
        .args(["checkout", "-b", "feature/with-docs"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    temp.file("src/api/handler.rs", "pub fn api() {}");
    temp.file("docs/api/handler.md", "# Handler");
    git_add_commit(temp.path(), "feat: add handler with docs");

    // Should pass - source matched and docs exist
    check("docs").pwd(temp.path()).args(&["--ci"]).passes();
}
