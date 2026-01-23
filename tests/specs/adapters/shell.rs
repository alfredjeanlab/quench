//! Behavioral specs for the Shell language adapter.
//!
//! Tests that quench correctly:
//! - Detects Shell projects via *.sh files in root, bin/, or scripts/
//! - Applies default source/test patterns
//! - Applies Shell-specific escape patterns
//!
//! Reference: docs/specs/langs/shell.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// AUTO-DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/10-language-adapters.md#adapter-selection
///
/// > shell | *.sh files in root, bin/, or scripts/ | **/*.sh, **/*.bash
#[test]
fn shell_adapter_auto_detected_when_sh_files_in_scripts() {
    // Project has .sh files in scripts/ but no quench.toml [shell] section
    // Should still apply Shell defaults
    let result = cli().on("shell/auto-detect").json().passes();
    let checks = result.checks();

    // escapes check should have shell-specific patterns active
    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/10-language-adapters.md#adapter-selection
///
/// > shell | *.sh files in root, bin/, or scripts/
#[test]
fn shell_adapter_auto_detected_when_sh_files_in_bin() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("bin")).unwrap();
    std::fs::write(
        dir.path().join("bin/build"),
        "#!/bin/bash\necho 'building'\n",
    )
    .unwrap();

    let result = cli().pwd(dir.path()).json().passes();
    let checks = result.checks();

    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/10-language-adapters.md#adapter-selection
///
/// > shell | *.sh files in root, bin/, or scripts/
#[test]
fn shell_adapter_auto_detected_when_sh_files_in_root() {
    let dir = temp_project();
    std::fs::write(dir.path().join("setup.sh"), "#!/bin/bash\necho 'setup'\n").unwrap();

    let result = cli().pwd(dir.path()).json().passes();
    let checks = result.checks();

    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

// =============================================================================
// DEFAULT PATTERN SPECS
// =============================================================================

/// Spec: docs/specs/langs/shell.md#default-patterns
///
/// > source = ["**/*.sh", "**/*.bash"]
#[test]
fn shell_adapter_default_source_pattern_matches_sh_files() {
    let cloc = check("cloc").on("shell/auto-detect").json().passes();
    let metrics = cloc.require("metrics");

    // Should count .sh files as source
    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(source_lines > 0, "should count .sh files as source");
}

/// Spec: docs/specs/langs/shell.md#default-patterns
///
/// > source = ["**/*.sh", "**/*.bash"]
#[test]
fn shell_adapter_default_source_pattern_matches_bash_files() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    std::fs::write(
        dir.path().join("scripts/deploy.bash"),
        "#!/bin/bash\necho 'deploying'\n",
    )
    .unwrap();

    let cloc = check("cloc").pwd(dir.path()).json().passes();
    let metrics = cloc.require("metrics");

    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(source_lines > 0, "should count .bash files as source");
}

/// Spec: docs/specs/langs/shell.md#default-patterns
///
/// > tests = ["tests/**/*.bats", "test/**/*.bats", "*_test.sh"]
#[test]
fn shell_adapter_default_test_pattern_matches_bats_files() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    std::fs::create_dir_all(dir.path().join("tests")).unwrap();
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\necho 'building'\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("tests/build.bats"),
        "#!/usr/bin/env bats\n@test 'builds' { run ./scripts/build.sh; }\n",
    )
    .unwrap();

    let cloc = check("cloc").pwd(dir.path()).json().passes();
    let metrics = cloc.require("metrics");

    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(test_lines > 0, "should count .bats files as test");
}

/// Spec: docs/specs/langs/shell.md#test-code-detection
///
/// > *_test.sh files
#[test]
fn shell_adapter_default_test_pattern_matches_test_sh_files() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\necho 'building'\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("scripts/build_test.sh"),
        "#!/bin/bash\n./scripts/build.sh && echo 'passed'\n",
    )
    .unwrap();

    let cloc = check("cloc").pwd(dir.path()).json().passes();
    let metrics = cloc.require("metrics");

    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(test_lines > 0, "should count *_test.sh files as test");
}

// =============================================================================
// ESCAPE PATTERN SPECS
// =============================================================================

/// Spec: docs/specs/langs/shell.md#default-escape-patterns
///
/// > set +e | comment | # OK:
#[test]
fn shell_adapter_set_plus_e_without_ok_comment_fails() {
    check("escapes")
        .on("shell/set-e-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("# OK:");
}

/// Spec: docs/specs/langs/shell.md#default-escape-patterns
///
/// > set +e | comment | # OK:
#[test]
fn shell_adapter_set_plus_e_with_ok_comment_passes() {
    check("escapes").on("shell/set-e-ok").passes();
}

/// Spec: docs/specs/langs/shell.md#default-escape-patterns
///
/// > eval | comment | # OK:
#[test]
fn shell_adapter_eval_without_ok_comment_fails() {
    check("escapes")
        .on("shell/eval-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("# OK:");
}

/// Spec: docs/specs/langs/shell.md#default-escape-patterns
///
/// > eval | comment | # OK:
#[test]
fn shell_adapter_eval_with_ok_comment_passes() {
    check("escapes").on("shell/eval-ok").passes();
}

/// Spec: docs/specs/langs/shell.md#default-escape-patterns
///
/// > set +e and eval allowed in test code without comment
#[test]
fn shell_adapter_escape_patterns_allowed_in_tests() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    std::fs::create_dir_all(dir.path().join("tests")).unwrap();
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\necho 'building'\n",
    )
    .unwrap();
    // Test file with set +e and eval, no comments
    std::fs::write(
        dir.path().join("tests/integration.bats"),
        "#!/usr/bin/env bats\nset +e\neval \"echo test\"\n@test 'works' { true; }\n",
    )
    .unwrap();

    check("escapes").pwd(dir.path()).passes();
}

// =============================================================================
// SHELLCHECK SUPPRESS SPECS
// =============================================================================

/// Spec: docs/specs/langs/shell.md#suppress
///
/// > "forbid" - Never allowed (default)
#[test]
fn shell_adapter_shellcheck_disable_forbidden_by_default() {
    check("escapes")
        .on("shell/shellcheck-forbid")
        .fails()
        .stdout_has("shellcheck");
}

/// Spec: docs/specs/langs/shell.md#suppress
///
/// > [shell.suppress.test] check = "allow" - tests can suppress freely
#[test]
fn shell_adapter_shellcheck_disable_allowed_in_tests() {
    check("escapes").on("shell/shellcheck-test").passes();
}

/// Spec: docs/specs/langs/shell.md#suppress
///
/// > "comment" - Requires justification comment
#[test]
fn shell_adapter_shellcheck_disable_with_comment_when_configured() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[shell.suppress]
check = "comment"
"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    // Has justification comment before shellcheck disable
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\n# This variable is exported for subprocesses\n# shellcheck disable=SC2034\nUNUSED_VAR=1\n",
    )
    .unwrap();

    check("escapes").pwd(dir.path()).passes();
}

/// Spec: docs/specs/langs/shell.md#suppress
///
/// > [shell.suppress.source] allow = ["SC2034"]
#[test]
fn shell_adapter_shellcheck_allow_list_skips_check() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[shell.suppress]
check = "forbid"
[shell.suppress.source]
allow = ["SC2034"]
"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    // SC2034 is in allow list, no comment needed
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\n# shellcheck disable=SC2034\nUNUSED_VAR=1\n",
    )
    .unwrap();

    check("escapes").pwd(dir.path()).passes();
}

// =============================================================================
// LINT CONFIG POLICY SPECS
// =============================================================================

/// Spec: docs/specs/langs/shell.md#policy
///
/// > lint_changes = "standalone" - lint config changes must be standalone PRs
#[test]
fn shell_adapter_lint_config_changes_with_source_fails_standalone_policy() {
    let dir = temp_project();

    // Setup quench.toml with standalone policy
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[shell.policy]
lint_changes = "standalone"
lint_config = [".shellcheckrc"]
"#,
    )
    .unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create initial commit with source
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\necho 'building'\n",
    )
    .unwrap();

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Add both lint config and source changes
    std::fs::write(dir.path().join(".shellcheckrc"), "enable=all\n").unwrap();
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\necho 'building'\necho 'more'\n",
    )
    .unwrap();

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Check with --base HEAD should detect mixed changes
    check("escapes")
        .pwd(dir.path())
        .args(&["--base", "HEAD"])
        .fails()
        .stdout_has("lint config")
        .stdout_has("separate PR");
}

/// Spec: docs/specs/langs/shell.md#policy
///
/// > lint_config = [".shellcheckrc"] files that trigger standalone requirement
#[test]
fn shell_adapter_lint_config_standalone_passes() {
    let dir = temp_project();

    // Setup quench.toml with standalone policy
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[shell.policy]
lint_changes = "standalone"
lint_config = [".shellcheckrc"]
"#,
    )
    .unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create initial commit
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\necho 'building'\n",
    )
    .unwrap();

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Add ONLY lint config change (no source changes)
    std::fs::write(dir.path().join(".shellcheckrc"), "enable=all\n").unwrap();

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should pass - only lint config changed
    check("escapes")
        .pwd(dir.path())
        .args(&["--base", "HEAD"])
        .passes();
}

/// Spec: docs/specs/langs/shell.md#policy
///
/// > Policy is disabled when lint_changes = "none"
#[test]
fn shell_adapter_lint_policy_disabled_allows_mixed_changes() {
    let dir = temp_project();

    // Setup quench.toml with policy disabled
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[shell.policy]
lint_changes = "none"
"#,
    )
    .unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create initial commit
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\necho 'building'\n",
    )
    .unwrap();

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Add both lint config and source changes
    std::fs::write(dir.path().join(".shellcheckrc"), "enable=all\n").unwrap();
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\necho 'building'\necho 'more'\n",
    )
    .unwrap();

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should pass - policy is disabled
    check("escapes")
        .pwd(dir.path())
        .args(&["--base", "HEAD"])
        .passes();
}

/// Spec: docs/specs/langs/shell.md#policy
///
/// > Source-only changes pass the standalone policy
#[test]
fn shell_adapter_source_only_changes_pass_standalone_policy() {
    let dir = temp_project();

    // Setup quench.toml with standalone policy
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[shell.policy]
lint_changes = "standalone"
lint_config = [".shellcheckrc"]
"#,
    )
    .unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create initial commit
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\necho 'building'\n",
    )
    .unwrap();

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Add ONLY source changes (no lint config)
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\necho 'building'\necho 'more'\n",
    )
    .unwrap();

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should pass - only source changed
    check("escapes")
        .pwd(dir.path())
        .args(&["--base", "HEAD"])
        .passes();
}
