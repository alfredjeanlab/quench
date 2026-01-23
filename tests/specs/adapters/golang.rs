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
#[ignore = "TODO: Phase 455+"]
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
#[ignore = "TODO: Phase 455+"]
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
#[ignore = "TODO: Phase 455+"]
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
#[ignore = "TODO: Phase 455+"]
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
#[ignore = "TODO: Phase 455+"]
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
#[ignore = "TODO: Phase 455+"]
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
#[ignore = "TODO: Phase 455+"]
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
#[ignore = "TODO: Phase 455+"]
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
#[ignore = "TODO: Phase 455+"]
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
#[ignore = "TODO: Phase 455+"]
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
#[ignore = "TODO: Phase 455+"]
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
#[ignore = "TODO: Phase 455+"]
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
#[ignore = "TODO: Phase 455+"]
fn nolint_without_comment_fails_when_comment_required() {
    check("suppress").on("golang/nolint-comment-fail").fails();
}

/// Spec: docs/specs/langs/golang.md#suppress
///
/// > `//nolint` with justification comment passes.
#[test]
#[ignore = "TODO: Phase 455+"]
fn nolint_with_comment_passes() {
    check("suppress").on("golang/nolint-comment-ok").passes();
}

// =============================================================================
// LINT CONFIG POLICY SPECS
// =============================================================================

/// Spec: docs/specs/langs/golang.md#policy
///
/// > `lint_changes = "standalone"` requires lint config in separate PRs.
#[test]
#[ignore = "TODO: Phase 455+"]
fn lint_config_changes_with_source_fails_standalone_policy() {
    check("policy").on("golang/lint-policy-fail").fails();
}
