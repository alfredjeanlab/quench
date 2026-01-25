//! Behavioral specs for the git check.
//!
//! Tests that quench correctly:
//! - Validates commit message format (conventional commits)
//! - Enforces type restrictions
//! - Enforces scope restrictions (when configured)
//! - Checks for commit format documentation in agent files
//! - Creates .gitmessage template with --fix
//!
//! Reference: docs/specs/checks/git.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Initialize a git repo with main branch
fn init_git_repo(project: &Project) {
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(project.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(project.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(project.path())
        .output()
        .unwrap();
}

/// Create main branch with initial commit
fn create_main_branch(project: &Project) {
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(project.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "feat: initial commit"])
        .current_dir(project.path())
        .output()
        .unwrap();
}

/// Create a feature branch
fn create_branch(project: &Project, name: &str) {
    std::process::Command::new("git")
        .args(["checkout", "-b", name])
        .current_dir(project.path())
        .output()
        .unwrap();
}

/// Add a commit with the given message
fn add_commit(project: &Project, message: &str) {
    // Touch a file to make a change
    let dummy_file = project.path().join(format!("dummy_{}.txt", rand_id()));
    std::fs::write(&dummy_file, "dummy").unwrap();

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(project.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(project.path())
        .output()
        .unwrap();
}

/// Generate a random ID for unique files
fn rand_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

// =============================================================================
// COMMIT FORMAT VALIDATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/git.md#conventional-format
///
/// > When `format = "conventional"`, commits must match: <type>(<scope>): <description>
#[test]
fn git_validates_conventional_commit_format() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
agents = false
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    init_git_repo(&temp);
    create_main_branch(&temp);
    create_branch(&temp, "feature");
    add_commit(&temp, "feat: add new feature");

    // Valid format should pass
    check("git").pwd(temp.path()).args(&["--ci"]).passes();
}

/// Spec: docs/specs/checks/git.md#output
///
/// > abc123: "update stuff" - missing type prefix
#[test]
fn git_invalid_format_generates_violation() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
agents = false
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    init_git_repo(&temp);
    create_main_branch(&temp);
    create_branch(&temp, "feature");
    add_commit(&temp, "update stuff"); // Invalid format!

    let git = check("git").pwd(temp.path()).args(&["--ci"]).json().fails();
    let violations = git.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("invalid_format")),
        "should have invalid_format violation"
    );
}

// =============================================================================
// TYPE RESTRICTION SPECS
// =============================================================================

/// Spec: docs/specs/checks/git.md#types
///
/// > `["feat", "fix"]` - Only these types allowed
#[test]
fn git_invalid_type_generates_violation() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
types = ["feat", "fix"]
agents = false
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat: or fix: only\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    init_git_repo(&temp);
    create_main_branch(&temp);
    create_branch(&temp, "feature");
    add_commit(&temp, "chore: do something"); // Invalid type!

    let git = check("git").pwd(temp.path()).args(&["--ci"]).json().fails();
    let violations = git.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("invalid_type")),
        "should have invalid_type violation"
    );
}

// =============================================================================
// SCOPE RESTRICTION SPECS
// =============================================================================

/// Spec: docs/specs/checks/git.md#scopes
///
/// > `["api", "cli"]` - Only these scopes allowed
#[test]
fn git_invalid_scope_generates_violation_when_scopes_configured() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
scopes = ["api", "cli"]
agents = false
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat(api): or feat(cli): only\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    init_git_repo(&temp);
    create_main_branch(&temp);
    create_branch(&temp, "feature");
    add_commit(&temp, "feat(unknown): add something"); // Invalid scope!

    let git = check("git").pwd(temp.path()).args(&["--ci"]).json().fails();
    let violations = git.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("invalid_scope")),
        "should have invalid_scope violation"
    );
}

/// Spec: docs/specs/checks/git.md#scopes
///
/// > omitted - Any scope allowed (or none)
#[test]
fn git_any_scope_allowed_when_not_configured() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
agents = false
# scopes not specified - any scope allowed
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat(anything): allowed\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    init_git_repo(&temp);
    create_main_branch(&temp);
    create_branch(&temp, "feature");
    add_commit(&temp, "feat(random): add feature");

    check("git").pwd(temp.path()).args(&["--ci"]).passes();
}

// =============================================================================
// AGENT DOCUMENTATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/git.md#agent-documentation-check
///
/// > When `agents = true` (default), quench verifies that commit format
/// > is documented in agent-readable files.
#[test]
fn git_missing_format_documentation_generates_violation() {
    let git = check("git").on("git/missing-docs").json().fails();
    assert!(
        git.has_violation("missing_docs"),
        "should have missing_docs violation"
    );
}

/// Spec: docs/specs/checks/git.md#detection
///
/// > Searches for type prefixes followed by `:` or `(` (e.g., `feat:`, `fix(`)
#[test]
fn git_detects_commit_format_via_type_prefixes() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
agents = true
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nUse `feat:` or `fix(scope):` format.\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    check("git").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/git.md#detection
///
/// > Searches for the phrase "conventional commits" (case-insensitive)
#[test]
fn git_detects_commit_format_via_conventional_commits_phrase() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
agents = true
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nWe use Conventional Commits.\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    check("git").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/git.md#disable
///
/// > agents = false - Don't check CLAUDE.md
#[test]
fn git_skips_docs_check_when_agents_disabled() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
agents = false
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n(No commit docs)\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    // Should pass even without commit format documentation
    check("git").pwd(temp.path()).passes();
}

// =============================================================================
// FIX BEHAVIOR SPECS
// =============================================================================

/// Spec: docs/specs/checks/git.md#template-creation
///
/// > When `template = true` (default), `--fix` creates a `.gitmessage` file.
#[test]
fn git_fix_creates_gitmessage_template() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
template = true
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat: format\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    // Initialize git repo for template config
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    check("git").pwd(temp.path()).args(&["--fix"]).passes();

    // Verify .gitmessage was created
    let gitmessage_path = temp.path().join(".gitmessage");
    assert!(gitmessage_path.exists(), ".gitmessage should be created");

    let content = std::fs::read_to_string(&gitmessage_path).unwrap();
    assert!(
        content.contains("# <type>"),
        ".gitmessage should contain template"
    );
}

/// Spec: docs/specs/checks/git.md#git-config
///
/// > `--fix` also runs: git config commit.template .gitmessage
#[test]
fn git_fix_configures_commit_template() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
template = true
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat: format\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    check("git").pwd(temp.path()).args(&["--fix"]).passes();

    // Verify git config was set
    let output = std::process::Command::new("git")
        .args(["config", "commit.template"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    let template_value = String::from_utf8_lossy(&output.stdout);
    assert!(
        template_value.trim() == ".gitmessage",
        "commit.template should be set to .gitmessage"
    );
}

/// Spec: docs/specs/checks/git.md#behavior
///
/// > `.gitmessage` exists - Leave it alone
#[test]
fn git_fix_does_not_overwrite_existing_gitmessage() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
template = true
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat: format\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );
    temp.file(".gitmessage", "# Custom template\n");

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    check("git").pwd(temp.path()).args(&["--fix"]).passes();

    // Verify original content preserved
    let content = std::fs::read_to_string(temp.path().join(".gitmessage")).unwrap();
    assert_eq!(content, "# Custom template\n");
}

// =============================================================================
// JSON OUTPUT SPECS
// =============================================================================

/// Spec: docs/specs/checks/git.md#json-output
///
/// > Violation types: `invalid_format`, `invalid_type`, `invalid_scope`, `missing_docs`
#[test]
fn git_violation_type_is_one_of_expected_values() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
agents = false
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    init_git_repo(&temp);
    create_main_branch(&temp);
    create_branch(&temp, "feature");
    add_commit(&temp, "update stuff"); // Invalid format triggers violation

    let git = check("git").pwd(temp.path()).args(&["--ci"]).json().fails();
    let violations = git.require("violations").as_array().unwrap();

    let valid_types = [
        "invalid_format",
        "invalid_type",
        "invalid_scope",
        "missing_docs",
    ];

    for v in violations {
        let vtype = v.get("type").and_then(|t| t.as_str()).unwrap();
        assert!(
            valid_types.contains(&vtype),
            "unexpected violation type: {}",
            vtype
        );
    }
}

/// Spec: docs/specs/checks/git.md#json-output
///
/// > Commit-related violations have `file: null` with `commit` field instead.
#[test]
fn git_commit_violations_have_commit_field() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
agents = false
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    init_git_repo(&temp);
    create_main_branch(&temp);
    create_branch(&temp, "feature");
    add_commit(&temp, "update stuff"); // Invalid format

    let git = check("git").pwd(temp.path()).args(&["--ci"]).json().fails();
    let violations = git.require("violations").as_array().unwrap();

    let commit_violation = violations
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("invalid_format"))
        .expect("should have invalid_format violation");

    assert!(
        commit_violation.get("commit").is_some(),
        "commit violation should have commit field"
    );
    assert!(
        commit_violation
            .get("file")
            .map(|f| f.is_null())
            .unwrap_or(true),
        "commit violation should have null file"
    );
}

/// Spec: docs/specs/checks/git.md#json-output
///
/// > missing_docs violations reference the agent file
#[test]
fn git_missing_docs_violation_references_file() {
    let git = check("git").on("git/missing-docs").json().fails();
    let violations = git.require("violations").as_array().unwrap();

    let docs_violation = violations
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("missing_docs"))
        .expect("should have missing_docs violation");

    assert!(
        docs_violation
            .get("file")
            .and_then(|f| f.as_str())
            .is_some(),
        "missing_docs violation should reference file"
    );
}

// =============================================================================
// EXACT OUTPUT FORMAT SPECS
// =============================================================================

/// Spec: docs/specs/checks/git.md#output
///
/// > Missing docs shows human-readable violation with file reference.
#[test]
fn exact_missing_docs_text() {
    check("git").on("git/missing-docs").fails().stdout_eq(
        r###"git: FAIL
  CLAUDE.md: feature commits without documentation
    Add a Commits section describing the format, e.g.:

    ## Commits

    Use conventional commit format: `type(scope): description`
    Types: feat, fix, chore, docs, test, refactor

FAIL: git
"###,
    );
}

/// Spec: docs/specs/checks/git.md#output
///
/// > PASS status when no violations.
#[test]
fn exact_git_pass_text() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
agents = false
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    // Initialize git repo so check doesn't skip
    init_git_repo(&temp);
    create_main_branch(&temp);

    check("git")
        .pwd(temp.path())
        .passes()
        .stdout_eq("PASS: git\n");
}

/// Spec: docs/specs/checks/git.md#fix-output
///
/// > FIXED status shows actions taken.
#[test]
fn exact_fix_creates_template_text() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
template = true
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat: format\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    check("git")
        .pwd(temp.path())
        .args(&["--fix"])
        .passes()
        .stdout_has("FIXED");

    // Verify .gitmessage was actually created
    assert!(
        temp.path().join(".gitmessage").exists(),
        ".gitmessage should be created by fix"
    );
}

/// Spec: docs/specs/checks/git.md#fix-output
///
/// > JSON output includes fixed:true and actions array.
#[test]
fn exact_fix_json_structure() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
template = true
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat: format\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    let result = check("git")
        .pwd(temp.path())
        .args(&["--fix"])
        .json()
        .passes();

    assert_eq!(
        result.require("fixed").as_bool(),
        Some(true),
        "should have fixed: true"
    );
}
