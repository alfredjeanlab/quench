//! Behavioral specs for the Rust language adapter.
//!
//! Tests that quench correctly:
//! - Detects Rust projects via Cargo.toml
//! - Applies default source/test patterns
//! - Handles inline #[cfg(test)] blocks
//! - Applies Rust-specific escape patterns
//!
//! Reference: docs/specs/langs/rust.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// AUTO-DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/10-language-adapters.md#adapter-selection
///
/// > rust | Cargo.toml exists | **/*.rs
#[test]
fn rust_adapter_auto_detected_when_cargo_toml_present() {
    // Project has Cargo.toml but no quench.toml [rust] section
    // Should still apply Rust defaults
    let result = cli().on("rust/auto-detect").json().passes();
    let checks = result.checks();

    // escapes check should have rust-specific patterns active
    // (will verify by checking that .unwrap() is detected)
    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/langs/rust.md#default-patterns
///
/// > source = ["**/*.rs"]
#[test]
fn rust_adapter_default_source_pattern_matches_rs_files() {
    let cloc = check("cloc").on("rust/auto-detect").json().passes();
    let metrics = cloc.require("metrics");

    // Should count .rs files as source
    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(source_lines > 0, "should count .rs files as source");
}

/// Spec: docs/specs/langs/rust.md#default-patterns
///
/// > ignore = ["target/"]
#[test]
fn rust_adapter_default_ignores_target_directory() {
    // Fixture has files in target/ that should be ignored
    let cloc = check("cloc").on("rust/auto-detect").json().passes();
    let files = cloc.get("files").and_then(|f| f.as_array());

    if let Some(files) = files {
        assert!(
            !files
                .iter()
                .any(|f| { f.as_str().map(|s| s.contains("target/")).unwrap_or(false) }),
            "target/ directory should be ignored"
        );
    }
}

// =============================================================================
// WORKSPACE DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/langs/rust.md#default-patterns
///
/// > Detected when Cargo.toml exists in project root.
/// > Auto-detects workspace packages from Cargo.toml [workspace] members.
#[test]
fn rust_adapter_detects_workspace_packages_from_cargo_toml() {
    // Fixture has Cargo.toml with [workspace] members = ["crates/*"]
    let cloc = check("cloc").on("rust/workspace-auto").json().passes();
    let by_package = cloc.get("by_package");

    assert!(by_package.is_some(), "should have by_package breakdown");
    let by_package = by_package.unwrap();

    // Should detect packages from workspace members
    assert!(
        by_package.get("core").is_some(),
        "should detect 'core' package"
    );
    assert!(
        by_package.get("cli").is_some(),
        "should detect 'cli' package"
    );
}

// =============================================================================
// TEST CODE DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/langs/rust.md#test-code-detection
///
/// > Lines inside #[cfg(test)] blocks are counted as test LOC
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_cfg_test_blocks_counted_as_test_loc() {
    let cloc = check("cloc").on("rust/cfg-test").json().passes();
    let metrics = cloc.require("metrics");

    // Source file has both source and #[cfg(test)] code
    let source_loc = metrics
        .get("source_loc")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let test_loc = metrics
        .get("test_loc")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    assert!(source_loc > 0, "should have source LOC");
    assert!(test_loc > 0, "should have test LOC from #[cfg(test)]");
}

/// Spec: docs/specs/langs/rust.md#test-code-detection
///
/// > Configurable: split_cfg_test = true (default)
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_split_cfg_test_can_be_disabled() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[rust]
split_cfg_test = false
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        r#"
pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    #[test]
    fn test_add() { assert_eq!(super::add(1, 2), 3); }
}
"#,
    )
    .unwrap();

    let cloc = check("cloc").pwd(dir.path()).json().passes();
    let metrics = cloc.require("metrics");

    // With split_cfg_test = false, all lines should be counted as source
    let test_loc = metrics
        .get("test_loc")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert_eq!(test_loc, 0, "should not split #[cfg(test)] when disabled");
}

// =============================================================================
// ESCAPE PATTERN SPECS
// =============================================================================

/// Spec: docs/specs/langs/rust.md#default-escape-patterns
///
/// > unsafe { } | comment | // SAFETY:
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_unsafe_without_safety_comment_fails() {
    check("escapes")
        .on("rust/unsafe-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("// SAFETY:");
}

/// Spec: docs/specs/langs/rust.md#default-escape-patterns
///
/// > unsafe { } | comment | // SAFETY:
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_unsafe_with_safety_comment_passes() {
    check("escapes").on("rust/unsafe-ok").passes();
}

/// Spec: docs/specs/langs/rust.md#default-escape-patterns
///
/// > .unwrap() | forbid | -
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_unwrap_in_source_code_fails() {
    let escapes = check("escapes").on("rust/unwrap-source").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("pattern").and_then(|p| p.as_str()) == Some("unwrap") }),
        "should have unwrap violation"
    );
}

/// Spec: docs/specs/checks/escape-hatches.md#forbid
///
/// > Always allowed in test code.
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_unwrap_in_test_code_allowed() {
    // .unwrap() only appears in test files or #[cfg(test)] blocks
    check("escapes").on("rust/unwrap-test").passes();
}

/// Spec: docs/specs/langs/rust.md#default-escape-patterns
///
/// > .expect( | forbid | -
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_expect_in_source_code_fails() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn f() { Some(1).expect(\"should have value\"); }",
    )
    .unwrap();

    check("escapes").pwd(dir.path()).fails();
}

/// Spec: docs/specs/langs/rust.md#default-escape-patterns
///
/// > mem::transmute | comment | // SAFETY:
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_transmute_without_safety_comment_fails() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "use std::mem; pub fn f() -> u64 { unsafe { mem::transmute(1i64) } }",
    )
    .unwrap();

    check("escapes")
        .pwd(dir.path())
        .fails()
        .stdout_has("// SAFETY:");
}

// =============================================================================
// SUPPRESS ATTRIBUTE SPECS
// =============================================================================

/// Spec: docs/specs/langs/rust.md#suppress
///
/// > "comment" - Requires justification comment (default for source)
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_allow_without_comment_fails_when_configured() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[rust.suppress]
check = "comment"
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "#[allow(dead_code)]\nfn unused() {}",
    )
    .unwrap();

    check("escapes")
        .pwd(dir.path())
        .fails()
        .stdout_has("#[allow");
}

/// Spec: docs/specs/langs/rust.md#suppress
///
/// > "comment" - Requires justification comment
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_allow_with_comment_passes() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[rust.suppress]
check = "comment"
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "// This function is reserved for future use\n#[allow(dead_code)]\nfn unused() {}",
    )
    .unwrap();

    check("escapes").pwd(dir.path()).passes();
}

/// Spec: docs/specs/langs/rust.md#suppress
///
/// > [rust.suppress.test] check = "allow" - tests can suppress freely
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_allow_in_test_code_always_passes() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[rust.suppress]
check = "comment"
[rust.suppress.test]
check = "allow"
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("tests")).unwrap();
    std::fs::write(
        dir.path().join("tests/test.rs"),
        "#[allow(unused)]\n#[test]\nfn test_something() {}",
    )
    .unwrap();

    check("escapes").pwd(dir.path()).passes();
}

/// Spec: docs/specs/langs/rust.md#suppress
///
/// > allow = ["dead_code"] - no comment needed for specific codes
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_allow_list_skips_comment_check() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[rust.suppress]
check = "comment"
[rust.suppress.source]
allow = ["dead_code"]
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "#[allow(dead_code)]\nfn unused() {}", // No comment, but dead_code is in allow list
    )
    .unwrap();

    check("escapes").pwd(dir.path()).passes();
}

/// Spec: docs/specs/langs/rust.md#suppress
///
/// > forbid = ["unsafe_code"] - never allowed
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_forbid_list_always_fails() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[rust.suppress.source]
forbid = ["unsafe_code"]
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "// Even with comment, forbidden\n#[allow(unsafe_code)]\nfn allow_unsafe() {}",
    )
    .unwrap();

    check("escapes").pwd(dir.path()).fails();
}

// =============================================================================
// LINT CONFIG POLICY SPECS
// =============================================================================

/// Spec: docs/specs/langs/rust.md#policy
///
/// > lint_changes = "standalone" - lint config changes must be standalone PRs
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_lint_config_changes_with_source_fails_standalone_policy() {
    // This requires git state - fixture has both rustfmt.toml and src changes staged
    check("escapes")
        .on("rust/lint-policy")
        .args(&["--base", "HEAD~1"])
        .fails()
        .stdout_has("lint config changes must be standalone");
}

/// Spec: docs/specs/langs/rust.md#policy
///
/// > lint_config = ["rustfmt.toml", ".rustfmt.toml", "clippy.toml", ".clippy.toml"]
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_lint_config_standalone_passes() {
    // Only lint config changed, no source files
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[rust.policy]
lint_changes = "standalone"
lint_config = ["rustfmt.toml"]
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("rustfmt.toml"), "max_width = 100\n").unwrap();

    // Initialize git repo with initial commit
    // Then add only rustfmt.toml change
    // This would need git setup - may need fixture

    check("escapes").pwd(dir.path()).passes();
}
