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
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
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
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
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
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
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
/// > source = ["**/*.js", "**/*.jsx", "**/*.ts", "**/*.tsx", "**/*.mjs", "**/*.mts"]
#[test]
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
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
/// >   "**/*.test.js", "**/*.test.ts", "**/*.test.jsx", "**/*.test.tsx",
/// >   "**/*.spec.js", "**/*.spec.ts", "**/*.spec.jsx", "**/*.spec.tsx",
/// >   "**/__tests__/**",
/// >   "test/**", "tests/**"
/// > ]
#[test]
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
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
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
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

// =============================================================================
// WORKSPACE DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/langs/javascript.md (implied by Default Patterns)
///
/// > Detects workspaces from package.json or pnpm-workspace.yaml
#[test]
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
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
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
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
#[ignore = "TODO: Phase 495 - JavaScript Adapter Escapes"]
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
#[ignore = "TODO: Phase 495 - JavaScript Adapter Escapes"]
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
#[ignore = "TODO: Phase 495 - JavaScript Adapter Escapes"]
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
#[ignore = "TODO: Phase 495 - JavaScript Adapter Escapes"]
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
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
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
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
fn eslint_disable_with_comment_passes() {
    check("escapes").on("javascript/eslint-disable-ok").passes();
}

/// Spec: docs/specs/langs/javascript.md#supported-patterns
///
/// > eslint-disable-next-line no-unused-vars
#[test]
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
fn eslint_disable_next_line_with_comment_passes() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[javascript.suppress]
check = "comment"
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"name": "test", "version": "1.0.0"}"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/index.ts"),
        r#"
// Legacy code requires this pattern
// eslint-disable-next-line no-console
console.log('debug');
"#,
    )
    .unwrap();

    check("escapes").pwd(dir.path()).passes();
}

// =============================================================================
// SUPPRESS DIRECTIVE SPECS - Biome
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#supported-patterns
///
/// > biome-ignore lint/suspicious/noExplicitAny: explanation required
#[test]
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
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
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
fn biome_ignore_with_explanation_passes() {
    check("escapes").on("javascript/biome-ignore-ok").passes();
}

/// Spec: docs/specs/langs/javascript.md#suppress
///
/// > Default: "comment" for source, "allow" for test code.
#[test]
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
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
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
fn lint_config_changes_with_source_fails_standalone_policy() {
    let dir = temp_project();

    // Setup quench.toml with standalone policy
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[javascript.policy]
lint_changes = "standalone"
lint_config = ["eslint.config.js"]
"#,
    )
    .unwrap();

    // Setup package.json
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"name": "test", "version": "1.0.0"}"#,
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
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/index.ts"),
        "export function main() {}\n",
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
    std::fs::write(dir.path().join("eslint.config.js"), "export default [];\n").unwrap();
    std::fs::write(
        dir.path().join("src/index.ts"),
        "export function main() {}\nexport function helper() {}\n",
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

/// Spec: docs/specs/langs/javascript.md#policy
///
/// > Lint config changes only (no source) passes standalone policy.
#[test]
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
fn lint_config_standalone_passes() {
    let dir = temp_project();

    // Setup quench.toml with standalone policy
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[javascript.policy]
lint_changes = "standalone"
lint_config = ["eslint.config.js"]
"#,
    )
    .unwrap();

    // Setup package.json
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"name": "test", "version": "1.0.0"}"#,
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
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/index.ts"),
        "export function main() {}\n",
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
    std::fs::write(dir.path().join("eslint.config.js"), "export default [];\n").unwrap();

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
