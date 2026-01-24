//! Behavioral specs for source/test correlation.
//!
//! Reference: docs/specs/checks/tests.md#commit-checking-fast-mode

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;
use std::process::Command;

/// Initialize a git repo with user config and initial commit.
fn init_git_repo(path: &std::path::Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial commit", "--allow-empty"])
        .current_dir(path)
        .output()
        .unwrap();
}

/// Stage files without committing.
fn git_stage(path: &std::path::Path) {
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
}

/// Add and commit all changes.
fn git_commit(path: &std::path::Path, msg: &str) {
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(path)
        .output()
        .unwrap();
}

/// Create a feature branch.
fn git_branch(path: &std::path::Path, name: &str) {
    Command::new("git")
        .args(["checkout", "-b", name])
        .current_dir(path)
        .output()
        .unwrap();
}

// =============================================================================
// GIT INTEGRATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/tests.md#git-integration
///
/// > `quench check --staged` - Staged changes (pre-commit)
///
/// Only staged files are checked, unstaged changes are ignored.
#[test]
fn staged_flag_checks_only_staged_files() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());

    // Add source file (unstaged)
    temp.file("src/unstaged.rs", "pub fn unstaged() {}");

    // Stage a different source file
    temp.file("src/staged.rs", "pub fn staged() {}");
    git_stage(temp.path());

    // Unstage the first file (only staged.rs should be checked)
    Command::new("git")
        .args(["restore", "--staged", "src/unstaged.rs"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Should fail only for staged.rs (no test)
    check("tests")
        .pwd(temp.path())
        .args(&["--staged"])
        .fails()
        .stdout_has("staged.rs")
        .stdout_lacks("unstaged.rs");
}

/// Spec: docs/specs/checks/tests.md#git-integration
///
/// > `quench check --base main` - Compare to branch (PR/CI)
/// > `quench check --base HEAD~5` - Compare to commits
#[test]
fn base_flag_compares_against_git_ref() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());

    // Create feature branch
    git_branch(temp.path(), "feature/new-thing");

    // Add source file without test
    temp.file("src/feature.rs", "pub fn feature() {}");
    git_commit(temp.path(), "feat: add feature");

    // Check against main - should fail (source without test)
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .fails()
        .stdout_has("feature.rs");
}

// =============================================================================
// SOURCE/TEST CORRELATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/tests.md#check-levels
///
/// > Source changes require corresponding test changes:
/// > - New source files → require new test file (or test additions)
/// > - Modified source files → require test changes
#[test]
fn source_change_without_test_change_generates_violation() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/no-tests");

    // Add source file without any tests
    temp.file("src/parser.rs", "pub fn parse() {}");
    git_commit(temp.path(), "feat: add parser");

    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .fails()
        .stdout_has("parser.rs")
        .stdout_has("missing_tests");
}

/// Spec: docs/specs/checks/tests.md#commit-scope
///
/// > Tests without code = **OK** (TDD recognized)
///
/// Writing tests before implementation is a valid workflow.
#[test]
fn test_change_without_source_change_passes_tdd() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/tdd");

    // Add test file without corresponding source
    temp.file(
        "tests/parser_tests.rs",
        r#"#[test]
fn test_parse() {
    // TDD: test written first
    assert!(true);
}
"#,
    );
    git_commit(temp.path(), "test: add parser tests");

    // Should pass - TDD workflow is valid
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}

// =============================================================================
// INLINE TEST DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/checks/tests.md#inline-test-changes-rust
///
/// > For Rust, changes to `#[cfg(test)]` blocks in the same file
/// > **satisfy the test requirement**
#[test]
fn inline_cfg_test_change_satisfies_test_requirement() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/inline-tests");

    // Add source file with inline tests
    temp.file(
        "src/parser.rs",
        r#"pub fn parse() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        assert!(parse());
    }
}
"#,
    );
    git_commit(temp.path(), "feat: add parser with tests");

    // Should pass - inline tests satisfy the requirement
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}

// =============================================================================
// PLACEHOLDER TEST SPECS
// =============================================================================

/// Spec: docs/specs/checks/tests.md#placeholder-tests
///
/// > Placeholder tests indicate planned test implementation.
/// > ```rust
/// > #[test]
/// > #[ignore = "TODO: implement parser"]
/// > fn test_parser() { todo!() }
/// > ```
#[test]
fn placeholder_test_satisfies_test_requirement() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
placeholders = "allow"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/placeholder");

    // Add source file
    temp.file("src/parser.rs", "pub fn parse() {}");

    // Add placeholder test
    temp.file(
        "tests/parser_tests.rs",
        r#"#[test]
#[ignore = "TODO: implement parser tests"]
fn test_parse() {
    todo!()
}
"#,
    );
    git_commit(temp.path(), "feat: add parser with placeholder test");

    // Should pass - placeholder indicates test intent
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}

// =============================================================================
// EXCLUSION SPECS
// =============================================================================

/// Spec: docs/specs/checks/tests.md#configuration
///
/// > exclude = [
/// >   "**/mod.rs",           # Module declarations
/// >   "**/lib.rs",           # Library roots
/// >   "**/main.rs",          # Binary entry points
/// > ]
#[test]
fn excluded_files_dont_require_tests() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/excluded");

    // Add excluded files without tests
    temp.file("src/mod.rs", "pub mod parser;");
    temp.file("src/main.rs", "fn main() {}");
    temp.file("src/lib.rs", "pub mod api;");
    git_commit(temp.path(), "feat: add module files");

    // Should pass - these files are excluded by default
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}

// =============================================================================
// JSON OUTPUT SPECS
// =============================================================================

/// Spec: docs/specs/checks/tests.md#json-output
///
/// > "metrics": {
/// >   "source_files_changed": 5,
/// >   "with_test_changes": 3,
/// >   ...
/// > }
#[test]
fn json_includes_source_files_changed_metrics() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/metrics");

    // Add source files: 2 with tests, 1 without
    temp.file("src/good1.rs", "pub fn good1() {}");
    temp.file("tests/good1_tests.rs", "#[test] fn t() {}");
    temp.file("src/good2.rs", "pub fn good2() {}");
    temp.file("tests/good2_tests.rs", "#[test] fn t() {}");
    temp.file("src/bad.rs", "pub fn bad() {}");
    git_commit(temp.path(), "feat: add files");

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .json()
        .fails();

    let metrics = result.require("metrics");
    assert_eq!(
        metrics.get("source_files_changed").and_then(|v| v.as_u64()),
        Some(3)
    );
    assert_eq!(
        metrics.get("with_test_changes").and_then(|v| v.as_u64()),
        Some(2)
    );
}

/// Spec: docs/specs/checks/tests.md#json-output
///
/// > **Violation types**: `missing_tests`
#[test]
fn tests_violation_type_is_always_missing_tests() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/violation-type");

    // Add multiple source files without tests
    temp.file("src/parser.rs", "pub fn parse() {}");
    temp.file("src/lexer.rs", "pub fn lex() {}");
    git_commit(temp.path(), "feat: add parser and lexer");

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .json()
        .fails();

    // All violations should be of type "missing_tests"
    for violation in result.violations() {
        assert_eq!(
            violation.get("type").and_then(|v| v.as_str()),
            Some("missing_tests"),
            "unexpected violation type: {:?}",
            violation
        );
    }

    // Should have at least 2 violations
    assert!(result.violations().len() >= 2);
}

// =============================================================================
// COMMIT SCOPE SPECS
// =============================================================================

/// Spec: docs/specs/checks/tests.md#commit-scope
///
/// > scope = "commit" # Per-commit with asymmetric rules
/// > - Tests without code = **OK** (TDD recognized)
/// > - Code without tests = **FAIL**
#[test]
fn commit_scope_fails_on_source_without_tests() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
scope = "commit"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/commit-scope");

    // First commit: tests only (TDD) - OK
    temp.file("tests/parser_tests.rs", "#[test] fn t() {}");
    git_commit(temp.path(), "test: add parser tests");

    // Second commit: source without tests - FAIL
    temp.file("src/lexer.rs", "pub fn lex() {}");
    git_commit(temp.path(), "feat: add lexer");

    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .fails()
        .stdout_has("lexer.rs")
        .stdout_lacks("parser"); // TDD commit should pass
}

/// Spec: docs/specs/checks/tests.md#commit-scope
///
/// > Tests without code = **OK** (TDD recognized)
#[test]
fn commit_scope_passes_test_only_commit_tdd() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
scope = "commit"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/tdd-commit");

    // Commit with only test changes - TDD workflow
    temp.file(
        "tests/parser_tests.rs",
        "#[test] fn test_parse() { assert!(true); }",
    );
    git_commit(temp.path(), "test: add parser tests first");

    // Should pass - TDD commit is valid
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}

/// Spec: docs/specs/checks/tests.md#commit-scope
///
/// > Each commit checked independently
#[test]
fn commit_scope_passes_when_each_commit_has_tests() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
scope = "commit"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/proper-commits");

    // First commit: source with tests
    temp.file("src/parser.rs", "pub fn parse() {}");
    temp.file("tests/parser_tests.rs", "#[test] fn test_parse() {}");
    git_commit(temp.path(), "feat: add parser with tests");

    // Second commit: source with tests
    temp.file("src/lexer.rs", "pub fn lex() {}");
    temp.file("tests/lexer_tests.rs", "#[test] fn test_lex() {}");
    git_commit(temp.path(), "feat: add lexer with tests");

    // Should pass - each commit has tests
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}

/// Spec: docs/specs/checks/tests.md#commit-scope
///
/// > Inline #[cfg(test)] changes count as test changes per commit
#[test]
fn commit_scope_inline_cfg_test_satisfies() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
scope = "commit"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/inline-tests");

    // Commit with source and inline tests
    temp.file(
        "src/parser.rs",
        r#"pub fn parse() -> bool { true }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        assert!(parse());
    }
}
"#,
    );
    git_commit(temp.path(), "feat: add parser with inline tests");

    // Should pass - inline tests satisfy requirement
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}

/// Spec: docs/specs/checks/tests.md#commit-scope
///
/// > scope = "branch" aggregates all changes (default)
#[test]
fn branch_scope_aggregates_all_changes() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
scope = "branch"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/branch-scope");

    // First commit: source only
    temp.file("src/parser.rs", "pub fn parse() {}");
    git_commit(temp.path(), "feat: add parser");

    // Second commit: tests only
    temp.file("tests/parser_tests.rs", "#[test] fn test_parse() {}");
    git_commit(temp.path(), "test: add parser tests");

    // Should pass in branch scope - tests exist somewhere in the branch
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}

/// Spec: docs/specs/checks/tests.md#commit-scope
///
/// > Commit scope with sibling test files
#[test]
fn commit_scope_sibling_test_file_satisfies() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
scope = "commit"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/sibling-tests");

    // Commit with source and sibling test file
    temp.file("src/parser.rs", "pub fn parse() {}");
    temp.file("src/parser_tests.rs", "#[test] fn test_parse() {}");
    git_commit(temp.path(), "feat: add parser with sibling tests");

    // Should pass - sibling test file satisfies requirement
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}

/// Spec: docs/specs/checks/tests.md#json-output
///
/// > Commit scope includes commits_checked and commits_failing metrics
#[test]
fn commit_scope_json_includes_commit_metrics() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
scope = "commit"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/commit-metrics");

    // Two commits: one passing, one failing
    temp.file("src/good.rs", "pub fn good() {}");
    temp.file("tests/good_tests.rs", "#[test] fn t() {}");
    git_commit(temp.path(), "feat: add good with tests");

    temp.file("src/bad.rs", "pub fn bad() {}");
    git_commit(temp.path(), "feat: add bad without tests");

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .json()
        .fails();

    let metrics = result.require("metrics");
    assert_eq!(
        metrics.get("commits_checked").and_then(|v| v.as_u64()),
        Some(2)
    );
    assert_eq!(
        metrics.get("commits_failing").and_then(|v| v.as_u64()),
        Some(1)
    );
    assert_eq!(
        metrics.get("scope").and_then(|v| v.as_str()),
        Some("commit")
    );
}
