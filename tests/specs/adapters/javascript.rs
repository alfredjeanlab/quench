// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for the JavaScript/TypeScript language adapter.
//!
//! Tests that quench correctly:
//! - Detects JS/TS projects via package.json, tsconfig.json, jsconfig.json
//! - Applies default source/test patterns
//! - Ignores node_modules, dist, build directories
//! - Applies JS/TS-specific escape patterns
//!
//! Reference: docs/specs/langs/javascript.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// AUTO-DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#detection
///
/// > Detected when any of these exist in project root:
/// > - `package.json`
#[test]
fn auto_detected_when_package_json_present() {
    let result = cli().on("javascript/auto-detect").json().passes();
    let checks = result.checks();

    // escapes check should have JS-specific patterns active
    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/langs/javascript.md#detection
///
/// > - `tsconfig.json`
#[test]
fn auto_detected_when_tsconfig_json_present() {
    let result = cli().on("javascript/tsconfig-detect").json().passes();
    let checks = result.checks();

    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/langs/javascript.md#detection
///
/// > - `jsconfig.json`
#[test]
fn auto_detected_when_jsconfig_json_present() {
    let result = cli().on("javascript/jsconfig-detect").json().passes();
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

/// Spec: docs/specs/langs/javascript.md#default-patterns
///
/// > source = ["**/*.js", "**/*.jsx", "**/*.ts", "**/*.tsx", "**/*.mjs", "**/*.mts", "**/*.cjs", "**/*.cts"]
#[test]
fn default_source_pattern_matches_js_ts_files() {
    let cloc = check("cloc")
        .on("javascript/default-patterns")
        .json()
        .passes();
    let metrics = cloc.require("metrics");

    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(source_lines > 0, "should count .js/.ts files as source");
}

/// Spec: docs/specs/langs/javascript.md#default-patterns
///
/// > tests = [
/// >   "**/*.test.*", "**/*.spec.*",
/// >   "**/*_test.*", "**/*_tests.*", "**/test_*.*",
/// >   "**/__tests__/**",
/// >   "**/test/**", "**/tests/**"
/// > ]
#[test]
fn default_test_pattern_matches_test_files() {
    let cloc = check("cloc")
        .on("javascript/default-patterns")
        .json()
        .passes();
    let metrics = cloc.require("metrics");

    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(
        test_lines > 0,
        "should count *.test.*, *.spec.*, __tests__/** as test"
    );
}

/// Spec: docs/specs/langs/javascript.md#default-patterns
///
/// > ignore = ["node_modules/", "dist/", "build/", ".next/", "coverage/"]
#[test]
fn default_ignores_node_modules_directory() {
    let cloc = check("cloc")
        .on("javascript/node-modules-ignore")
        .json()
        .passes();
    let metrics = cloc.require("metrics");

    // Only src/index.js should be counted, not node_modules or dist
    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(
        source_lines < 50,
        "node_modules/, dist/, build/ should be ignored"
    );
}

/// Spec: docs/specs/langs/javascript.md#exclude
///
/// > Custom exclude patterns are respected via [javascript].exclude config.
#[test]
fn custom_exclude_patterns_respected() {
    let cloc = check("cloc")
        .on("javascript/custom-exclude")
        .json()
        .passes();
    let metrics = cloc.require("metrics");

    // Only index.js should be counted (not node_modules/pkg.js)
    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(
        source_lines < 20,
        "node_modules/ should be excluded via config, got {} source lines",
        source_lines
    );
}

// =============================================================================
// WORKSPACE DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/langs/javascript.md (implied by Default Patterns)
///
/// > Detects workspaces from package.json or pnpm-workspace.yaml
#[test]
fn detects_npm_workspaces_from_package_json() {
    let cloc = check("cloc").on("javascript/workspace-npm").json().passes();
    let by_package = cloc.get("by_package");

    assert!(by_package.is_some(), "should have by_package breakdown");
    let by_package = by_package.unwrap();

    assert!(
        by_package.get("core").is_some(),
        "should detect 'core' package"
    );
    assert!(
        by_package.get("cli").is_some(),
        "should detect 'cli' package"
    );
}

/// Spec: docs/specs/langs/javascript.md (implied by Default Patterns)
///
/// > Detects pnpm workspaces from pnpm-workspace.yaml
#[test]
fn detects_pnpm_workspaces() {
    let cloc = check("cloc")
        .on("javascript/workspace-pnpm")
        .json()
        .passes();
    let by_package = cloc.get("by_package");

    assert!(by_package.is_some(), "should have by_package breakdown");
}

// =============================================================================
// ESCAPE PATTERN SPECS - as unknown
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#default-escape-patterns
///
/// > `as unknown` requires `// CAST:` comment explaining why.
#[test]
fn as_unknown_without_cast_comment_fails() {
    check("escapes")
        .on("javascript/as-unknown-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("// CAST:");
}

/// Spec: docs/specs/langs/javascript.md#default-escape-patterns
///
/// > `as unknown` with `// CAST:` comment passes.
#[test]
fn as_unknown_with_cast_comment_passes() {
    check("escapes").on("javascript/as-unknown-ok").passes();
}

// =============================================================================
// ESCAPE PATTERN SPECS - @ts-ignore
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#default-escape-patterns
///
/// > `@ts-ignore` is forbidden in source code.
#[test]
fn ts_ignore_forbidden_in_source() {
    check("escapes")
        .on("javascript/ts-ignore-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("@ts-ignore")
        .stdout_has("forbidden");
}

/// Spec: docs/specs/langs/javascript.md#escapes-in-test-code
///
/// > Escape patterns are allowed in test code.
#[test]
fn ts_ignore_allowed_in_test_code() {
    check("escapes").on("javascript/ts-ignore-test-ok").passes();
}

// =============================================================================
// SUPPRESS DIRECTIVE SPECS - ESLint
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#suppress
///
/// > When `check = "comment"`, `eslint-disable` requires justification.
#[test]
fn eslint_disable_without_comment_fails_when_comment_required() {
    check("escapes")
        .on("javascript/eslint-disable-fail")
        .fails()
        .stdout_has("eslint-disable");
}

/// Spec: docs/specs/langs/javascript.md#suppress
///
/// > `eslint-disable` with justification comment passes.
#[test]
fn eslint_disable_with_comment_passes() {
    check("escapes").on("javascript/eslint-disable-ok").passes();
}

/// Spec: docs/specs/langs/javascript.md#supported-patterns
///
/// > eslint-disable-next-line no-unused-vars
#[test]
fn eslint_disable_next_line_with_comment_passes() {
    let temp = Project::empty();
    temp.config(
        r#"[javascript.suppress]
check = "comment"
"#,
    );
    temp.file("package.json", r#"{"name": "test", "version": "1.0.0"}"#);
    temp.file(
        "src/index.ts",
        r#"
// Legacy code requires this pattern
// eslint-disable-next-line no-console
console.log('debug');
"#,
    );

    check("escapes").pwd(temp.path()).passes();
}

// =============================================================================
// SUPPRESS DIRECTIVE SPECS - Biome
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#supported-patterns
///
/// > biome-ignore lint/suspicious/noExplicitAny: explanation required
#[test]
fn biome_ignore_without_explanation_fails() {
    check("escapes")
        .on("javascript/biome-ignore-fail")
        .fails()
        .stdout_has("biome-ignore");
}

/// Spec: docs/specs/langs/javascript.md#supported-patterns
///
/// > biome-ignore with explanation passes
#[test]
fn biome_ignore_with_explanation_passes() {
    check("escapes").on("javascript/biome-ignore-ok").passes();
}

/// Spec: docs/specs/langs/javascript.md#suppress
///
/// > Default: "comment" for source, "allow" for test code.
#[test]
fn eslint_disable_in_test_file_passes_without_comment() {
    check("escapes").on("javascript/eslint-test-ok").passes();
}

// =============================================================================
// LINT CONFIG POLICY SPECS
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#policy
///
/// > `lint_changes = "standalone"` requires lint config in separate PRs.
#[test]
fn lint_config_changes_with_source_fails_standalone_policy() {
    let temp = Project::empty();

    // Setup quench.toml with standalone policy
    temp.config(
        r#"[javascript.policy]
lint_changes = "standalone"
lint_config = ["eslint.config.js"]
"#,
    );

    // Setup package.json
    temp.file("package.json", r#"{"name": "test", "version": "1.0.0"}"#);

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
    temp.file("src/index.ts", "export function main() {}\n");

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
    temp.file("eslint.config.js", "export default [];\n");
    temp.file(
        "src/index.ts",
        "export function main() {}\nexport function helper() {}\n",
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

/// Spec: docs/specs/langs/javascript.md#policy
///
/// > Lint config changes only (no source) passes standalone policy.
#[test]
fn lint_config_standalone_passes() {
    let temp = Project::empty();

    // Setup quench.toml with standalone policy
    temp.config(
        r#"[javascript.policy]
lint_changes = "standalone"
lint_config = ["eslint.config.js"]
"#,
    );

    // Setup package.json
    temp.file("package.json", r#"{"name": "test", "version": "1.0.0"}"#);

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
    temp.file("src/index.ts", "export function main() {}\n");

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
    temp.file("eslint.config.js", "export default [];\n");

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
// VALIDATION SNAPSHOT SPECS (Checkpoint 49B)
// =============================================================================

/// Checkpoint 49B: Validate js-simple produces expected JSON structure
///
/// Verifies that the JavaScript adapter auto-detects correctly and produces
/// useful output with source/test file counts and line counts.
#[test]
fn js_simple_produces_expected_json_structure() {
    let result = cli().on("js-simple").json().fails(); // fails because of agents check
    let checks = result.checks();

    // Find cloc check and verify metrics
    let cloc = checks
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("cloc"))
        .expect("cloc check should exist");

    let metrics = cloc.get("metrics").expect("cloc should have metrics");

    // Verify key structure elements
    assert!(
        metrics.get("source_files").is_some(),
        "should have source_files"
    );
    assert!(
        metrics.get("test_files").is_some(),
        "should have test_files"
    );
    assert!(
        metrics.get("source_lines").is_some(),
        "should have source_lines"
    );
    assert!(
        metrics.get("test_lines").is_some(),
        "should have test_lines"
    );

    // Verify counts match expected
    // Note: vitest.config.ts is also counted as a source file
    assert_eq!(
        metrics.get("source_files").and_then(|v| v.as_u64()),
        Some(3),
        "should have 3 source files (src/index.ts, src/utils.ts, vitest.config.ts)"
    );
    assert_eq!(
        metrics.get("test_files").and_then(|v| v.as_u64()),
        Some(2),
        "should have 2 test files"
    );

    // Verify escapes check exists and passes
    let escapes = checks
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
        .expect("escapes check should exist");
    assert_eq!(
        escapes.get("passed").and_then(|v| v.as_bool()),
        Some(true),
        "escapes should pass"
    );
}

/// Checkpoint 49B: Validate js-monorepo detects all workspace packages
///
/// Verifies that the pnpm workspace detection correctly enumerates
/// packages and produces per-package metrics.
#[test]
fn js_monorepo_produces_by_package_metrics() {
    let result = cli().on("js-monorepo").json().fails(); // fails because of agents check
    let checks = result.checks();

    // Find cloc check
    let cloc = checks
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("cloc"))
        .expect("cloc check should exist");

    // Verify by_package exists
    let by_package = cloc.get("by_package").expect("should have by_package");

    // Verify both packages detected
    assert!(
        by_package.get("core").is_some(),
        "should detect 'core' package"
    );
    assert!(
        by_package.get("cli").is_some(),
        "should detect 'cli' package"
    );

    // Verify each package has metrics
    let core = by_package.get("core").unwrap();
    assert!(
        core.get("source_files").is_some(),
        "core should have source_files"
    );
    assert!(
        core.get("test_files").is_some(),
        "core should have test_files"
    );

    let cli_pkg = by_package.get("cli").unwrap();
    assert!(
        cli_pkg.get("source_files").is_some(),
        "cli should have source_files"
    );
    assert!(
        cli_pkg.get("test_files").is_some(),
        "cli should have test_files"
    );
}

/// Checkpoint 49B: Validate JavaScript-specific escapes detected in violations
///
/// Verifies that JavaScript escape patterns are correctly detected in the
/// violations fixture's js/ subdirectory.
#[test]
fn violations_js_escapes_detected() {
    let result = check("escapes").on("violations").json().fails();

    // Verify JS-specific files are detected
    assert!(
        result.has_violation_for_file("as-unknown.ts"),
        "should detect as-unknown.ts violation"
    );
    assert!(
        result.has_violation_for_file("ts-ignore.ts"),
        "should detect ts-ignore.ts violation"
    );
    assert!(
        result.has_violation_for_file("eslint-disable.ts"),
        "should detect eslint-disable.ts violation"
    );

    // Count JS violations (at least 3 from js/ directory)
    let js_violations: Vec<_> = result
        .violations()
        .iter()
        .filter(|v| {
            v.get("file")
                .and_then(|f| f.as_str())
                .map(|f| f.starts_with("js/"))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        js_violations.len() >= 3,
        "should have at least 3 JS violations, found {}",
        js_violations.len()
    );
}
