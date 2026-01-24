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
fn rust_adapter_cfg_test_blocks_counted_as_test_loc() {
    let cloc = check("cloc").on("rust/cfg-test").json().passes();
    let metrics = cloc.require("metrics");

    // Source file has both source and #[cfg(test)] code
    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    assert!(source_lines > 0, "should have source LOC");
    assert!(test_lines > 0, "should have test LOC from #[cfg(test)]");
}

/// Spec: docs/specs/langs/rust.md#test-code-detection
///
/// > Configurable: cfg_test_split = true (default)
#[test]
fn rust_adapter_cfg_test_split_can_be_disabled() {
    let temp = Project::empty();
    temp.config(
        r#"[rust]
cfg_test_split = false
"#,
    );
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file(
        "src/lib.rs",
        r#"
pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    #[test]
    fn test_add() { assert_eq!(super::add(1, 2), 3); }
}
"#,
    );

    let cloc = check("cloc").pwd(temp.path()).json().passes();
    let metrics = cloc.require("metrics");

    // With cfg_test_split = false, all lines should be counted as source
    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert_eq!(test_lines, 0, "should not split #[cfg(test)] when disabled");
}

/// Spec: docs/specs/langs/rust.md#test-code-detection
///
/// > External test modules are detected via file patterns
/// > `tests/*.rs` → matched by `tests/**` pattern
#[test]
fn rust_adapter_external_test_modules_detected_via_file_patterns() {
    // external-tests fixture has tests/integration.rs (external test module)
    let cloc = check("cloc").on("rust/external-tests").json().passes();
    let metrics = cloc.require("metrics");

    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // tests/test.rs should be counted as test LOC via tests/** pattern
    assert!(
        test_lines > 0,
        "external test files in tests/ should be counted as test LOC"
    );
}

/// Spec: docs/specs/langs/rust.md#supported-patterns
///
/// > Multi-line #[cfg(test)] attributes should be detected
#[test]
fn rust_adapter_multiline_cfg_test_detected() {
    let cloc = check("cloc").on("rust/multiline-cfg-test").json().passes();
    let metrics = cloc.require("metrics");

    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Multi-line #[cfg(test)] block should be counted as test LOC
    assert!(
        test_lines > 0,
        "multi-line #[cfg(test)] blocks should be counted as test LOC"
    );
}

// =============================================================================
// ESCAPE PATTERN SPECS
// =============================================================================

/// Spec: docs/specs/langs/rust.md#default-escape-patterns
///
/// > unsafe { } | comment | // SAFETY:
#[test]
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
fn rust_adapter_unsafe_with_safety_comment_passes() {
    check("escapes").on("rust/unsafe-ok").passes();
}

// Note: .unwrap() and .expect() are not checked by quench.
// Use Clippy's unwrap_used and expect_used lints for that.
// Quench ensures escapes and suppressions are commented.

/// Spec: docs/specs/langs/rust.md#default-escape-patterns
///
/// > mem::transmute | comment | // SAFETY:
#[test]
fn rust_adapter_transmute_without_safety_comment_fails() {
    let temp = Project::empty();
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file(
        "src/lib.rs",
        "use std::mem; pub fn f() -> u64 { unsafe { mem::transmute(1i64) } }",
    );

    check("escapes")
        .pwd(temp.path())
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
fn rust_adapter_allow_without_comment_fails_when_configured() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "comment"
"#,
    );
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file("src/lib.rs", "#[allow(dead_code)]\nfn unused() {}");

    check("escapes")
        .pwd(temp.path())
        .fails()
        .stdout_has("#[allow");
}

/// Spec: docs/specs/langs/rust.md#suppress
///
/// > "comment" - Requires justification comment
#[test]
fn rust_adapter_allow_with_comment_passes() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "comment"
"#,
    );
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    // dead_code default patterns include "// KEEP UNTIL:" and "// NOTE(compat):"
    temp.file(
        "src/lib.rs",
        "// KEEP UNTIL: v2.0 removes this API\n#[allow(dead_code)]\nfn unused() {}",
    );

    check("escapes").pwd(temp.path()).passes();
}

/// Spec: docs/specs/langs/rust.md#suppress
///
/// > [rust.suppress.test] check = "allow" - tests can suppress freely
#[test]
fn rust_adapter_allow_in_test_code_always_passes() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "comment"
[rust.suppress.test]
check = "allow"
"#,
    );
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file(
        "tests/test.rs",
        "#[allow(unused)]\n#[test]\nfn test_something() {}",
    );

    check("escapes").pwd(temp.path()).passes();
}

/// Spec: docs/specs/langs/rust.md#suppress
///
/// > allow = ["dead_code"] - no comment needed for specific codes
#[test]
fn rust_adapter_allow_list_skips_comment_check() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "comment"
[rust.suppress.source]
allow = ["dead_code"]
"#,
    );
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    // No comment, but dead_code is in allow list
    temp.file("src/lib.rs", "#[allow(dead_code)]\nfn unused() {}");

    check("escapes").pwd(temp.path()).passes();
}

/// Spec: docs/specs/langs/rust.md#suppress
///
/// > forbid = ["unsafe_code"] - never allowed
#[test]
fn rust_adapter_forbid_list_always_fails() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress.source]
forbid = ["unsafe_code"]
"#,
    );
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file(
        "src/lib.rs",
        "// Even with comment, forbidden\n#[allow(unsafe_code)]\nfn allow_unsafe() {}",
    );

    check("escapes").pwd(temp.path()).fails();
}

/// Spec: docs/specs/langs/rust.md#suppress
///
/// > Source scope override should apply even when base level is "allow"
/// > Regression test for: source.check override ignored in early return
#[test]
fn rust_adapter_source_check_override_applied_when_base_is_allow() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "allow"
[rust.suppress.source]
check = "comment"
"#,
    );
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    // No comment - should fail with source.check = "comment"
    temp.file("src/lib.rs", "#[allow(dead_code)]\nfn unused() {}");

    // Should fail because source files require comments, even though base is "allow"
    check("escapes").pwd(temp.path()).fails();
}

/// Spec: docs/specs/langs/rust.md#supported-patterns
///
/// > Multi-line #[allow(...)] attributes should be detected
#[test]
fn rust_adapter_multiline_allow_detected() {
    // multiline-allow fixture has multi-line #[allow(dead_code, unused_variables)]
    // with check = "forbid", so it should fail if detected
    check("escapes").on("rust/multiline-allow").fails();
}

// =============================================================================
// INNER ATTRIBUTE SPECS
// =============================================================================

/// Spec: docs/specs/langs/rust.md#supported-patterns
///
/// > Inner attributes (#![...]) apply to the module or crate they appear in.
#[test]
fn rust_adapter_inner_allow_without_comment_fails_when_configured() {
    // Fixture has #![allow(dead_code)] - inner attribute
    // Violation output normalizes to #[allow(...)] format
    check("escapes")
        .on("rust/module-suppress")
        .fails()
        .stdout_has("#[allow(dead_code)");
}

/// Spec: docs/specs/langs/rust.md#supported-patterns
///
/// > They follow the same comment requirement rules as outer attributes.
#[test]
fn rust_adapter_inner_allow_with_comment_passes() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "comment"
"#,
    );
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    // dead_code default patterns include "// NOTE(compat):"
    temp.file(
        "src/lib.rs",
        "// NOTE(compat): Module-wide suppression for backwards compatibility\n#![allow(dead_code)]\n\nfn helper() {}",
    );

    check("escapes").pwd(temp.path()).passes();
}

/// Spec: docs/specs/langs/rust.md#supported-patterns
///
/// > Inner expect attributes are also supported
#[test]
fn rust_adapter_inner_expect_without_comment_fails() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "comment"
"#,
    );
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file("src/lib.rs", "#![expect(unused)]\n\nfn f() {}");

    // Violation output normalizes to #[expect(...)] format
    check("escapes")
        .pwd(temp.path())
        .fails()
        .stdout_has("#[expect(unused)");
}

/// Spec: docs/specs/langs/rust.md#supported-patterns
///
/// > #[allow(...)] inside macro_rules! definitions should be detected
#[test]
fn rust_adapter_allow_in_macro_rules_detected() {
    // macro-escape fixture has #[allow(dead_code)] inside a macro_rules! definition
    // with check = "forbid", so it should fail
    check("escapes")
        .on("rust/macro-escape")
        .fails()
        .stdout_has("#[allow(dead_code)]");
}

// =============================================================================
// LINT CONFIG POLICY SPECS
// =============================================================================

/// Spec: docs/specs/langs/rust.md#policy
///
/// > lint_changes = "standalone" - lint config changes must be standalone PRs
#[test]
fn rust_adapter_lint_config_changes_with_source_fails_standalone_policy() {
    let temp = Project::empty();

    // Setup quench.toml with standalone policy
    temp.config(
        r#"[rust.policy]
lint_changes = "standalone"
lint_config = ["rustfmt.toml"]
"#,
    );

    // Setup Cargo.toml
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    // Initialize git repo
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

    // Create initial commit with source
    temp.file("src/lib.rs", "pub fn f() {}");

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Add both lint config and source changes
    temp.file("rustfmt.toml", "max_width = 100\n");
    temp.file("src/lib.rs", "pub fn f() {}\npub fn g() {}");

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Check with --base HEAD should detect mixed changes
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .fails()
        .stdout_has("lint config")
        .stdout_has("separate PR");
}

/// Spec: docs/specs/langs/rust.md#policy
///
/// > lint_config = ["rustfmt.toml", ...] files that trigger standalone requirement
#[test]
fn rust_adapter_lint_config_standalone_passes() {
    let temp = Project::empty();

    // Setup quench.toml with standalone policy
    temp.config(
        r#"[rust.policy]
lint_changes = "standalone"
lint_config = ["rustfmt.toml"]
"#,
    );

    // Setup Cargo.toml
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    // Initialize git repo
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

    // Create initial commit
    temp.file("src/lib.rs", "pub fn f() {}");

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Add ONLY lint config change (no source changes)
    temp.file("rustfmt.toml", "max_width = 100\n");

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Should pass - only lint config changed
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .passes();
}

// =============================================================================
// CFG_TEST_SPLIT MODE SPECS
// =============================================================================

/// Spec: docs/specs/langs/rust.md#cfg-test-split-modes
///
/// > cfg_test_split = "count" (default): Split #[cfg(test)] blocks into test LOC
#[test]
fn rust_cfg_test_split_count_separates_source_and_test() {
    let cloc = check("cloc")
        .on("rust/inline-cfg-test-count")
        .json()
        .passes();
    let metrics = cloc.require("metrics");

    // Source and test lines should be separated
    assert!(
        metrics
            .get("source_lines")
            .and_then(|v| v.as_u64())
            .unwrap()
            > 0
    );
    assert!(metrics.get("test_lines").and_then(|v| v.as_u64()).unwrap() > 0);
}

/// Spec: docs/specs/langs/rust.md#cfg-test-split-modes
///
/// > cfg_test_split = "require": Fail if source files contain inline #[cfg(test)]
#[test]
fn rust_cfg_test_split_require_fails_on_inline_tests() {
    let cloc = check("cloc").on("rust/inline-cfg-test").json().fails();

    assert!(cloc.has_violation("inline_cfg_test"));
    let v = cloc.require_violation("inline_cfg_test");
    assert!(v.get("line").is_some(), "violation should have line number");
}

/// Spec: docs/specs/langs/rust.md#cfg-test-split-modes
///
/// > cfg_test_split = "require": Sibling _tests.rs pattern passes
#[test]
fn rust_cfg_test_split_require_passes_with_sibling_tests() {
    check("cloc").on("rust/sibling-tests").passes();
}

/// Spec: docs/specs/langs/rust.md#cfg-test-split-modes
///
/// > cfg_test_split = "off": Count all lines as source LOC
#[test]
fn rust_cfg_test_split_off_counts_all_as_source() {
    let cloc = check("cloc").on("rust/inline-cfg-test-off").json().passes();
    let metrics = cloc.require("metrics");

    // All lines counted as source, none as test
    assert!(
        metrics
            .get("source_lines")
            .and_then(|v| v.as_u64())
            .unwrap()
            > 0
    );
    assert_eq!(metrics.get("test_lines").and_then(|v| v.as_u64()), Some(0));
}

/// Spec: docs/specs/langs/rust.md#cfg-test-split-modes
///
/// > cfg_test_split = true (legacy): Same as "count"
#[test]
fn rust_cfg_test_split_true_is_count() {
    let cloc = check("cloc").on("rust/cfg-test-split-true").json().passes();
    let metrics = cloc.require("metrics");

    // Should split like "count" mode
    assert!(metrics.get("test_lines").and_then(|v| v.as_u64()).unwrap() > 0);
}

/// Spec: docs/specs/langs/rust.md#cfg-test-split-modes
///
/// > cfg_test_split = false (legacy): Same as "off"
#[test]
fn rust_cfg_test_split_false_is_off() {
    let cloc = check("cloc")
        .on("rust/cfg-test-split-false")
        .json()
        .passes();
    let metrics = cloc.require("metrics");

    // Should count all as source like "off" mode
    assert_eq!(metrics.get("test_lines").and_then(|v| v.as_u64()), Some(0));
}
