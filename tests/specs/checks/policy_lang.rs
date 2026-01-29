// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for per-language policy check level.
//!
//! Tests that quench correctly:
//! - Respects {lang}.policy.check = "off" to disable policy for that language
//! - Respects {lang}.policy.check = "warn" to report without failing
//! - Allows independent check levels per language
//!
//! Reference: docs/specs/langs/{rust,golang,javascript,shell}.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

/// Helper to setup git repo with initial commit
fn setup_git_repo(temp: &Project) {
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(temp.path())
        .output()
        .unwrap();
}

fn git_add_all(temp: &Project) {
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();
}

fn git_commit(temp: &Project, message: &str) {
    std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(temp.path())
        .output()
        .unwrap();
}

// =============================================================================
// RUST POLICY CONFIG SPECS
// =============================================================================

/// Spec: docs/specs/langs/rust.md#policy
///
/// > [rust.policy]
/// > check = "off" disables policy for Rust files
#[test]
fn rust_policy_check_off_disables_policy() {
    let temp = Project::empty();

    // Setup quench.toml with check = "off"
    temp.config(
        r#"[rust.policy]
check = "off"
lint_changes = "standalone"
"#,
    );

    // Setup Cargo.toml
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    // Initialize git repo and create initial commit
    setup_git_repo(&temp);
    temp.file("src/lib.rs", "pub fn f() {}");
    git_add_all(&temp);
    git_commit(&temp, "initial");

    // Make both lint config and source changes
    temp.file("rustfmt.toml", "max_width = 100\n");
    temp.file("src/lib.rs", "pub fn f() {}\npub fn g() {}");
    git_add_all(&temp);

    // Should pass - check is "off"
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .passes();
}

/// Spec: docs/specs/langs/rust.md#policy
///
/// > [rust.policy]
/// > check = "warn" reports but doesn't fail
#[test]
fn rust_policy_check_warn_reports_without_failing() {
    let temp = Project::empty();

    // Setup quench.toml with check = "warn"
    temp.config(
        r#"[rust.policy]
check = "warn"
lint_changes = "standalone"
"#,
    );

    // Setup Cargo.toml
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    // Initialize git repo and create initial commit
    setup_git_repo(&temp);
    temp.file("src/lib.rs", "pub fn f() {}");
    git_add_all(&temp);
    git_commit(&temp, "initial");

    // Make both lint config and source changes
    temp.file("rustfmt.toml", "max_width = 100\n");
    temp.file("src/lib.rs", "pub fn f() {}\npub fn g() {}");
    git_add_all(&temp);

    // Should pass (warning only) but output warning message
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .passes()
        .stdout_has("lint_policy");
}

// =============================================================================
// GOLANG POLICY CONFIG SPECS
// =============================================================================

/// Spec: docs/specs/langs/golang.md#policy
///
/// > [golang.policy]
/// > check = "off" disables policy for Go files
#[test]
fn golang_policy_check_off_disables_policy() {
    let temp = Project::empty();

    // Setup quench.toml with check = "off"
    temp.config(
        r#"[golang.policy]
check = "off"
lint_changes = "standalone"
"#,
    );

    // Setup go.mod
    temp.file("go.mod", "module example.com/test\n\ngo 1.21\n");

    // Initialize git repo and create initial commit
    setup_git_repo(&temp);
    temp.file("main.go", "package main\n\nfunc main() {}\n");
    git_add_all(&temp);
    git_commit(&temp, "initial");

    // Make both lint config and source changes
    temp.file(".golangci.yml", "run:\n  timeout: 5m\n");
    temp.file("main.go", "package main\n\nfunc main() {}\nfunc f() {}\n");
    git_add_all(&temp);

    // Should pass - check is "off"
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .passes();
}

/// Spec: docs/specs/langs/golang.md#policy
///
/// > [golang.policy]
/// > check = "warn" reports but doesn't fail
#[test]
fn golang_policy_check_warn_reports_without_failing() {
    let temp = Project::empty();

    // Setup quench.toml with check = "warn"
    temp.config(
        r#"[golang.policy]
check = "warn"
lint_changes = "standalone"
"#,
    );

    // Setup go.mod
    temp.file("go.mod", "module example.com/test\n\ngo 1.21\n");

    // Initialize git repo and create initial commit
    setup_git_repo(&temp);
    temp.file("main.go", "package main\n\nfunc main() {}\n");
    git_add_all(&temp);
    git_commit(&temp, "initial");

    // Make both lint config and source changes
    temp.file(".golangci.yml", "run:\n  timeout: 5m\n");
    temp.file("main.go", "package main\n\nfunc main() {}\nfunc f() {}\n");
    git_add_all(&temp);

    // Should pass (warning only) but output warning message
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .passes()
        .stdout_has("lint_policy");
}

// =============================================================================
// JAVASCRIPT POLICY CONFIG SPECS
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#policy
///
/// > [javascript.policy]
/// > check = "off" disables policy for JS/TS files
#[test]
fn javascript_policy_check_off_disables_policy() {
    let temp = Project::empty();

    // Setup quench.toml with check = "off"
    temp.config(
        r#"[javascript.policy]
check = "off"
lint_changes = "standalone"
"#,
    );

    // Setup package.json
    temp.file(
        "package.json",
        r#"{"name": "test", "version": "1.0.0", "type": "module"}"#,
    );

    // Initialize git repo and create initial commit
    setup_git_repo(&temp);
    temp.file("src/index.ts", "export const x = 1;\n");
    git_add_all(&temp);
    git_commit(&temp, "initial");

    // Make both lint config and source changes
    temp.file("eslint.config.js", "export default [];\n");
    temp.file("src/index.ts", "export const x = 1;\nexport const y = 2;\n");
    git_add_all(&temp);

    // Should pass - check is "off"
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .passes();
}

// =============================================================================
// SHELL POLICY CONFIG SPECS
// =============================================================================

/// Spec: docs/specs/langs/shell.md#policy
///
/// > [shell.policy]
/// > check = "off" disables policy for shell scripts
#[test]
fn shell_policy_check_off_disables_policy() {
    let temp = Project::empty();

    // Setup quench.toml with check = "off"
    temp.config(
        r#"[shell.policy]
check = "off"
lint_changes = "standalone"
"#,
    );

    // Initialize git repo and create initial commit
    setup_git_repo(&temp);
    temp.file("scripts/deploy.sh", "#!/bin/bash\necho hello\n");
    git_add_all(&temp);
    git_commit(&temp, "initial");

    // Make both lint config and source changes
    temp.file(".shellcheckrc", "disable=SC2034\n");
    temp.file("scripts/deploy.sh", "#!/bin/bash\necho hello\necho world\n");
    git_add_all(&temp);

    // Should pass - check is "off"
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .passes();
}

// =============================================================================
// INDEPENDENT CHECK LEVEL SPECS
// =============================================================================

/// Spec: docs/specs/10-language-adapters.md
///
/// > Each language can have independent policy check level
#[test]
fn each_language_can_have_independent_policy_check_level() {
    let temp = Project::empty();

    // Setup quench.toml with mixed levels
    temp.config(
        r#"[rust.policy]
check = "error"
lint_changes = "standalone"

[golang.policy]
check = "warn"
lint_changes = "standalone"

[javascript.policy]
check = "off"
lint_changes = "standalone"
"#,
    );

    // Setup Cargo.toml (Rust is the primary detected language)
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    // Initialize git repo and create initial commit
    setup_git_repo(&temp);
    temp.file("src/lib.rs", "pub fn f() {}");
    git_add_all(&temp);
    git_commit(&temp, "initial");

    // Make both lint config and source changes
    temp.file("rustfmt.toml", "max_width = 100\n");
    temp.file("src/lib.rs", "pub fn f() {}\npub fn g() {}");
    git_add_all(&temp);

    // Rust policy violation should cause failure
    let result = check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .json()
        .fails();

    let violations = result.require("violations").as_array().unwrap();
    assert!(violations.iter().any(|v| {
        v.get("type")
            .and_then(|t| t.as_str())
            .map(|t| t == "lint_policy")
            .unwrap_or(false)
    }));
}

/// Spec: docs/specs/10-language-adapters.md
///
/// > Mixed project: Go policy warns, Rust policy errors
#[test]
fn mixed_levels_go_warn_rust_error() {
    let temp = Project::empty();

    // Setup quench.toml with mixed levels
    temp.config(
        r#"[rust.policy]
check = "error"
lint_changes = "standalone"

[golang.policy]
check = "warn"
lint_changes = "standalone"
"#,
    );

    // Setup Cargo.toml (Rust is the primary detected language)
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    // Initialize git repo and create initial commit
    setup_git_repo(&temp);
    temp.file("src/lib.rs", "pub fn f() {}");
    git_add_all(&temp);
    git_commit(&temp, "initial");

    // Make both lint config and source changes for Rust
    temp.file("rustfmt.toml", "max_width = 100\n");
    temp.file("src/lib.rs", "pub fn f() {}\npub fn g() {}");
    git_add_all(&temp);

    // Rust violations should cause failure
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .fails()
        .stdout_has("lint_policy");
}
