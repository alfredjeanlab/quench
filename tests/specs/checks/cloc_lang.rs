//! Behavioral specs for per-language CLOC configuration.
//!
//! Tests that quench correctly:
//! - Respects {lang}.cloc.check = "off" to disable cloc for that language
//! - Respects {lang}.cloc.check = "warn" to report without failing
//! - Uses {lang}.cloc.advice for custom violation advice
//! - Allows independent check levels per language
//! - Falls back to check.cloc.check when {lang}.cloc.check is unset
//!
//! Reference: docs/specs/langs/{rust,golang,javascript,shell}.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// RUST CLOC CONFIG SPECS
// =============================================================================

/// Spec: docs/specs/langs/rust.md#configuration
///
/// > [rust.cloc]
/// > check = "error" | "warn" | "off"
#[test]
fn rust_cloc_check_off_skips_rust_files() {
    // Fixture has oversized Rust file but rust.cloc.check = "off"
    check("cloc").on("cloc-lang/rust-off").passes();
}

/// Spec: docs/specs/langs/rust.md#configuration
///
/// > [rust.cloc]
/// > check = "warn" reports but doesn't fail
#[test]
fn rust_cloc_check_warn_reports_without_failing() {
    check("cloc")
        .on("cloc-lang/rust-warn")
        .passes()
        .stdout_has("big.rs")
        .stdout_has("file_too_large");
}

/// Spec: docs/specs/langs/rust.md#configuration
///
/// > [rust.cloc]
/// > advice = "..." - Custom advice for oversized Rust files
#[test]
fn rust_cloc_advice_overrides_default() {
    let cloc = check("cloc").on("cloc-lang/rust-advice").json().fails();
    let violations = cloc.require("violations").as_array().unwrap();

    let advice = violations[0]
        .get("advice")
        .and_then(|a| a.as_str())
        .unwrap();
    assert_eq!(advice, "Rust files should be split into smaller modules.");
}

// =============================================================================
// GOLANG CLOC CONFIG SPECS
// =============================================================================

/// Spec: docs/specs/langs/golang.md#configuration
///
/// > [golang.cloc]
/// > check = "off" disables cloc for Go files
#[test]
fn golang_cloc_check_off_skips_go_files() {
    check("cloc").on("cloc-lang/golang-off").passes();
}

/// Spec: docs/specs/langs/golang.md#configuration
///
/// > [golang.cloc]
/// > check = "warn" reports but doesn't fail
#[test]
fn golang_cloc_check_warn_reports_without_failing() {
    check("cloc")
        .on("cloc-lang/golang-warn")
        .passes()
        .stdout_has("big.go")
        .stdout_has("file_too_large");
}

// =============================================================================
// JAVASCRIPT CLOC CONFIG SPECS
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#configuration
///
/// > [javascript.cloc]
/// > check = "off" disables cloc for JS/TS files
#[test]
fn javascript_cloc_check_off_skips_js_files() {
    check("cloc").on("cloc-lang/javascript-off").passes();
}

// =============================================================================
// SHELL CLOC CONFIG SPECS
// =============================================================================

/// Spec: docs/specs/langs/shell.md#configuration
///
/// > [shell.cloc]
/// > check = "off" disables cloc for shell scripts
#[test]
fn shell_cloc_check_off_skips_shell_files() {
    check("cloc").on("cloc-lang/shell-off").passes();
}

// =============================================================================
// INDEPENDENT CHECK LEVEL SPECS
// =============================================================================

/// Spec: docs/specs/10-language-adapters.md
///
/// > Each language can have independent cloc check level
#[test]
fn each_language_can_have_independent_cloc_check_level() {
    // Fixture has: rust=error (fails), golang=warn (reports), javascript=off (skipped)
    let result = check("cloc").on("cloc-lang/mixed-levels").json().fails();

    // Rust file should cause failure
    let violations = result.require("violations").as_array().unwrap();
    assert!(violations.iter().any(|v| {
        v.get("file")
            .and_then(|f| f.as_str())
            .map(|f| f.ends_with(".rs"))
            .unwrap_or(false)
    }));

    // Go file should not appear in violations (only warned)
    // JS file should not appear at all (skipped)
    assert!(!violations.iter().any(|v| {
        v.get("file")
            .and_then(|f| f.as_str())
            .map(|f| f.ends_with(".js"))
            .unwrap_or(false)
    }));
}

/// Spec: docs/specs/10-language-adapters.md
///
/// > Mixed project: Go file over limit warns, Rust file over limit fails
#[test]
fn mixed_levels_go_warn_rust_error() {
    // When golang.cloc.check = "warn" and rust.cloc.check = "error"
    // Go violations should be reported but not cause failure
    // Rust violations should cause failure
    check("cloc")
        .on("cloc-lang/mixed-levels")
        .fails()
        .stdout_has("big.rs")
        .stdout_has("file_too_large");
}

// =============================================================================
// INHERITANCE SPECS
// =============================================================================

/// Spec: docs/specs/10-language-adapters.md
///
/// > Unset {lang}.cloc.check inherits from check.cloc.check
#[test]
fn unset_lang_cloc_inherits_from_global() {
    // Fixture has check.cloc.check = "error" but no [rust.cloc] section
    // Rust files should still be checked with error level
    check("cloc").on("cloc-lang/inherits").fails();
}

/// Spec: docs/specs/10-language-adapters.md
///
/// > Global check.cloc.check = "off" disables all languages unless overridden
#[test]
fn global_off_disables_all_unless_overridden() {
    let temp = Project::empty();
    temp.config(
        r#"[check.cloc]
check = "off"
max_lines = 5

[rust.cloc]
check = "error"
"#,
    );
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    // Oversized Rust file should fail (rust.cloc overrides global off)
    temp.file(
        "src/big.rs",
        "fn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\nfn e() {}\nfn f() {}\n",
    );
    // Oversized Go file should pass (inherits global off)
    temp.file("go.mod", "module test\n");
    temp.file(
        "big.go",
        "package main\nfunc a() {}\nfunc b() {}\nfunc c() {}\nfunc d() {}\nfunc e() {}\nfunc f() {}\n",
    );

    // Should fail only because of Rust file
    let result = check("cloc").pwd(temp.path()).json().fails();
    let violations = result.require("violations").as_array().unwrap();

    // Only Rust violation, not Go
    assert!(violations.iter().all(|v| {
        v.get("file")
            .and_then(|f| f.as_str())
            .map(|f| f.ends_with(".rs"))
            .unwrap_or(false)
    }));
}
