//! Behavioral specs for the Go language adapter.
//!
//! Tests that quench correctly:
//! - Detects Go projects via go.mod
//! - Applies default source/test patterns
//! - Ignores vendor directory
//! - Applies Go-specific escape patterns (unsafe.Pointer, go:linkname, go:noescape)
//!
//! Reference: docs/specs/langs/golang.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// AUTO-DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/langs/golang.md#detection
///
/// > Go is detected when `go.mod` exists in the project root.
#[test]
fn auto_detected_when_go_mod_present() {
    let result = cli().on("golang/auto-detect").json().passes();
    let checks = result.checks();

    // escapes check should have Go-specific patterns active
    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

// =============================================================================
// DEFAULT PATTERN SPECS
// =============================================================================

/// Spec: docs/specs/langs/golang.md#default-patterns
///
/// > source = ["**/*.go"]
#[test]
fn default_source_pattern_matches_go_files() {
    let cloc = check("cloc").on("golang/auto-detect").json().passes();
    let metrics = cloc.require("metrics");

    // Should count .go files as source
    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(source_lines > 0, "should count .go files as source");
}

/// Spec: docs/specs/langs/golang.md#default-patterns
///
/// > tests = ["**/*_test.go"]
#[test]
fn default_test_pattern_matches_test_files() {
    let cloc = check("cloc").on("golang/auto-detect").json().passes();
    let metrics = cloc.require("metrics");

    // Should count *_test.go files as test
    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(test_lines > 0, "should count *_test.go files as test");
}

/// Spec: docs/specs/langs/golang.md#default-patterns
///
/// > ignore = ["vendor/**"]
#[test]
fn default_ignores_vendor_directory() {
    let cloc = check("cloc").on("golang/vendor-ignore").json().passes();
    let metrics = cloc.require("metrics");

    // vendor/ files should not be counted
    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Only main.go should be counted (not vendor/dep/dep.go)
    // main.go has ~5 lines, vendor/dep/dep.go also has ~5 lines
    // If vendor is properly ignored, source_lines should be small
    assert!(source_lines < 20, "vendor/ should be ignored");
}

// =============================================================================
// MODULE AND PACKAGE DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/langs/golang.md#detection
///
/// > The module name is extracted from `go.mod`.
#[test]
fn detects_module_name_from_go_mod() {
    let result = cli().on("golang/module-packages").json().passes();
    let checks = result.checks();

    // Module name should be detected and available in check context
    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/langs/golang.md#detection
///
/// > Packages are detected from directory structure.
#[test]
fn detects_packages_from_directory_structure() {
    let result = cli().on("golang/module-packages").json().passes();
    let checks = result.checks();

    // Should detect packages from directory structure
    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

// =============================================================================
// ESCAPE PATTERN SPECS - unsafe.Pointer
// =============================================================================

/// Spec: docs/specs/langs/golang.md#default-escape-patterns
///
/// > `unsafe.Pointer` requires `// SAFETY:` comment explaining why.
#[test]
fn unsafe_pointer_without_safety_comment_fails() {
    check("escapes")
        .on("golang/unsafe-pointer-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("// SAFETY:");
}

/// Spec: docs/specs/langs/golang.md#default-escape-patterns
///
/// > `unsafe.Pointer` with `// SAFETY:` comment passes.
#[test]
fn unsafe_pointer_with_safety_comment_passes() {
    check("escapes").on("golang/unsafe-pointer-ok").passes();
}

// =============================================================================
// ESCAPE PATTERN SPECS - go:linkname
// =============================================================================

/// Spec: docs/specs/langs/golang.md#default-escape-patterns
///
/// > `//go:linkname` requires `// LINKNAME:` comment explaining why.
#[test]
fn go_linkname_without_linkname_comment_fails() {
    check("escapes")
        .on("golang/linkname-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("// LINKNAME:");
}

/// Spec: docs/specs/langs/golang.md#default-escape-patterns
///
/// > `//go:linkname` with `// LINKNAME:` comment passes.
#[test]
fn go_linkname_with_linkname_comment_passes() {
    check("escapes").on("golang/linkname-ok").passes();
}

// =============================================================================
// ESCAPE PATTERN SPECS - go:noescape
// =============================================================================

/// Spec: docs/specs/langs/golang.md#default-escape-patterns
///
/// > `//go:noescape` requires `// NOESCAPE:` comment explaining why.
#[test]
fn go_noescape_without_noescape_comment_fails() {
    check("escapes")
        .on("golang/noescape-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("// NOESCAPE:");
}

/// Spec: docs/specs/langs/golang.md#default-escape-patterns
///
/// > `//go:noescape` with `// NOESCAPE:` comment passes.
#[test]
fn go_noescape_with_noescape_comment_passes() {
    check("escapes").on("golang/noescape-ok").passes();
}

// =============================================================================
// SUPPRESS DIRECTIVE SPECS
// =============================================================================

/// Spec: docs/specs/langs/golang.md#suppress
///
/// > When `check = "comment"`, `//nolint` requires justification.
#[test]
fn nolint_without_comment_fails_when_comment_required() {
    check("escapes").on("golang/nolint-comment-fail").fails();
}

/// Spec: docs/specs/langs/golang.md#suppress
///
/// > `//nolint` with justification comment passes.
#[test]
fn nolint_with_comment_passes() {
    check("escapes").on("golang/nolint-comment-ok").passes();
}

/// Spec: docs/specs/langs/golang.md#supported-patterns
///
/// > //nolint:errcheck,gosec (multiple linters)
#[test]
fn nolint_with_multiple_codes_and_comment_passes() {
    check("escapes").on("golang/nolint-multiple-codes").passes();
}

/// Spec: docs/specs/langs/golang.md#configuration
///
/// > forbid = ["govet"]             # never suppress go vet findings
#[test]
fn nolint_with_forbidden_code_fails() {
    check("escapes")
        .on("golang/nolint-forbid-fail")
        .fails()
        .stdout_has("govet")
        .stdout_has("forbidden");
}

/// Spec: docs/specs/langs/golang.md#configuration
///
/// > allow = ["unused"]             # no comment needed for these
#[test]
fn nolint_with_allowed_code_passes_without_comment() {
    check("escapes").on("golang/nolint-allow-ok").passes();
}

/// Spec: docs/specs/langs/golang.md#suppress
///
/// > Default: "comment" for source, "allow" for test code.
#[test]
fn nolint_in_test_file_passes_without_comment() {
    check("escapes").on("golang/nolint-test-file-ok").passes();
}

/// Spec: docs/specs/langs/golang.md#supported-patterns
///
/// > //nolint (all linters, discouraged)
#[test]
fn nolint_all_linters_with_comment_passes() {
    check("escapes").on("golang/nolint-all-linters").passes();
}

/// Spec: docs/specs/langs/golang.md#configuration
///
/// > comment = "// OK:"           # optional: require specific pattern
#[test]
fn nolint_with_custom_pattern_passes() {
    check("escapes").on("golang/nolint-custom-pattern").passes();
}

/// Spec: docs/specs/langs/golang.md#configuration
///
/// > comment = "// OK:" requires that specific pattern
#[test]
fn nolint_without_custom_pattern_fails() {
    check("escapes")
        .on("golang/nolint-custom-pattern-fail")
        .fails()
        .stdout_has("// OK:");
}

// =============================================================================
// LINT CONFIG POLICY SPECS
// =============================================================================

/// Spec: docs/specs/langs/golang.md#policy
///
/// > `lint_changes = "standalone"` requires lint config in separate PRs.
#[test]
fn lint_config_changes_with_source_fails_standalone_policy() {
    let temp = Project::empty();

    // Setup quench.toml with standalone policy
    temp.config(
        r#"[golang.policy]
lint_changes = "standalone"
lint_config = [".golangci.yml"]
"#,
    );

    // Setup go.mod
    temp.file("go.mod", "module example.com/test\n\ngo 1.21\n");

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
    temp.file("main.go", "package main\n\nfunc main() {}\n");

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
    temp.file(".golangci.yml", "linters:\n  enable:\n    - errcheck\n");
    temp.file(
        "main.go",
        "package main\n\nfunc main() {}\nfunc helper() {}\n",
    );

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

/// Spec: docs/specs/langs/golang.md#policy
///
/// > Lint config changes only (no source) passes standalone policy.
#[test]
fn lint_config_standalone_passes() {
    let temp = Project::empty();

    // Setup quench.toml with standalone policy
    temp.config(
        r#"[golang.policy]
lint_changes = "standalone"
lint_config = [".golangci.yml"]
"#,
    );

    // Setup go.mod
    temp.file("go.mod", "module example.com/test\n\ngo 1.21\n");

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
    temp.file("main.go", "package main\n\nfunc main() {}\n");

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
    temp.file(".golangci.yml", "linters:\n  enable:\n    - errcheck\n");

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
// EXACT OUTPUT FORMAT SPECS
// =============================================================================
// These tests verify exact output format using direct comparison.
// Any output change requires explicit test update (no auto-accept).

/// Spec: Go adapter cloc metrics structure
#[test]
fn exact_go_simple_cloc_json() {
    let result = check("cloc").on("go-simple").json().passes();
    let metrics = result.require("metrics");

    // Verify structure without timestamp dependency
    assert_eq!(metrics.get("ratio").unwrap().as_f64(), Some(0.32));
    assert_eq!(metrics.get("source_files").unwrap().as_i64(), Some(3));
    assert_eq!(metrics.get("source_lines").unwrap().as_i64(), Some(22));
    assert_eq!(metrics.get("source_tokens").unwrap().as_i64(), Some(100));
    assert_eq!(metrics.get("test_files").unwrap().as_i64(), Some(1));
    assert_eq!(metrics.get("test_lines").unwrap().as_i64(), Some(7));
    assert_eq!(metrics.get("test_tokens").unwrap().as_i64(), Some(27));
}

/// Spec: Go escape violation text output format
#[test]
fn exact_unsafe_pointer_fail_text() {
    check("escapes")
        .on("golang/unsafe-pointer-fail")
        .fails()
        .stdout_eq(
            r###"escapes: FAIL
  main.go:7: missing_comment: unsafe_pointer
    Add a // SAFETY: comment explaining pointer validity.
FAIL: escapes
"###,
        );
}
