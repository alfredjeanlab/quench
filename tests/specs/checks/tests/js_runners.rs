//! JavaScript test runner auto-detection specs.
//!
//! Reference: PLAN.md - JS Runner Auto-Detection

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// AUTO-DETECTION FROM CONFIG FILES
// =============================================================================

/// Spec: Auto-detect vitest from vitest.config.ts
#[test]
fn auto_detects_vitest_from_config_file() {
    let temp = Project::empty();
    temp.file("package.json", r#"{"name": "test"}"#);
    temp.file("vitest.config.ts", "export default {}");

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    assert_eq!(metrics.get("auto_detected"), Some(&serde_json::json!(true)));
    assert_eq!(metrics.get("runner"), Some(&serde_json::json!("vitest")));
    assert!(
        metrics
            .get("detection_source")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.contains("config_file"))
    );
}

/// Spec: Auto-detect jest from jest.config.js
#[test]
fn auto_detects_jest_from_config_file() {
    let temp = Project::empty();
    temp.file("package.json", r#"{"name": "test"}"#);
    temp.file("jest.config.js", "module.exports = {}");

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    assert_eq!(metrics.get("auto_detected"), Some(&serde_json::json!(true)));
    assert_eq!(metrics.get("runner"), Some(&serde_json::json!("jest")));
}

// =============================================================================
// AUTO-DETECTION FROM DEVDEPENDENCIES
// =============================================================================

/// Spec: Auto-detect vitest from devDependencies
#[test]
fn auto_detects_vitest_from_dev_dependencies() {
    let temp = Project::empty();
    temp.file(
        "package.json",
        r#"{"name": "test", "devDependencies": {"vitest": "^2.0.0"}}"#,
    );

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    assert_eq!(metrics.get("auto_detected"), Some(&serde_json::json!(true)));
    assert_eq!(metrics.get("runner"), Some(&serde_json::json!("vitest")));
    assert!(
        metrics
            .get("detection_source")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.contains("dev_dependency"))
    );
}

/// Spec: Auto-detect jest from devDependencies
#[test]
fn auto_detects_jest_from_dev_dependencies() {
    let temp = Project::empty();
    temp.file(
        "package.json",
        r#"{"name": "test", "devDependencies": {"jest": "^29.0.0"}}"#,
    );

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    assert_eq!(metrics.get("auto_detected"), Some(&serde_json::json!(true)));
    assert_eq!(metrics.get("runner"), Some(&serde_json::json!("jest")));
}

// =============================================================================
// AUTO-DETECTION FROM TEST SCRIPTS
// =============================================================================

/// Spec: Auto-detect vitest from scripts.test
#[test]
fn auto_detects_vitest_from_test_script() {
    let temp = Project::empty();
    temp.file(
        "package.json",
        r#"{"name": "test", "scripts": {"test": "vitest run"}}"#,
    );

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    assert_eq!(metrics.get("auto_detected"), Some(&serde_json::json!(true)));
    assert_eq!(metrics.get("runner"), Some(&serde_json::json!("vitest")));
    assert!(
        metrics
            .get("detection_source")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.contains("test_script"))
    );
}

// =============================================================================
// EXPLICIT CONFIG TAKES PRECEDENCE
// =============================================================================

/// Spec: Explicit config takes precedence over auto-detection
#[test]
fn explicit_config_takes_precedence() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
"#,
    );
    temp.file(
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"",
    );
    temp.file("src/lib.rs", "");
    // Also has package.json with vitest - should be ignored
    temp.file(
        "package.json",
        r#"{"name": "test", "devDependencies": {"vitest": "^2.0.0"}}"#,
    );

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should NOT have auto_detected flag since explicit config was used
    assert!(metrics.get("auto_detected").is_none());
}

// =============================================================================
// NO DETECTION CASES
// =============================================================================

/// Spec: No auto-detection when no package.json
#[test]
fn no_auto_detection_without_package_json() {
    let temp = Project::empty();
    // No package.json, no quench.toml suite config
    // Should just pass with correlation check (no runner to detect)

    let result = check("tests").pwd(temp.path()).json().passes();

    // Should NOT have auto_detected flag - may or may not have metrics
    // depending on whether correlation check runs
    let auto_detected = result.get("metrics").and_then(|m| m.get("auto_detected"));
    assert!(auto_detected.is_none());
}

/// Spec: No auto-detection when no runner can be detected
#[test]
fn no_auto_detection_when_no_runner_found() {
    let temp = Project::empty();
    temp.file(
        "package.json",
        r#"{"name": "test", "scripts": {"test": "echo 'no tests'"}}"#,
    );

    let result = check("tests").pwd(temp.path()).json().passes();

    // Should NOT have auto_detected flag (no runner could be detected)
    let auto_detected = result.get("metrics").and_then(|m| m.get("auto_detected"));
    assert!(auto_detected.is_none());
}

// =============================================================================
// DETECTION PRIORITY
// =============================================================================

/// Spec: Config file takes priority over devDependencies
#[test]
fn config_file_priority_over_dependencies() {
    let temp = Project::empty();
    temp.file(
        "package.json",
        r#"{"name": "test", "devDependencies": {"jest": "^29.0.0"}}"#,
    );
    // vitest config file should win over jest devDependency
    temp.file("vitest.config.ts", "export default {}");

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    assert_eq!(metrics.get("runner"), Some(&serde_json::json!("vitest")));
    assert!(
        metrics
            .get("detection_source")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.contains("config_file"))
    );
}

// =============================================================================
// FIXTURE INTEGRATION TEST
// =============================================================================

/// Spec: Auto-detection works on js-simple fixture
///
/// This test requires npm install to have been run on the fixture.
/// Marked as ignored since fixtures may not have node_modules.
#[test]
#[ignore = "TODO: requires npm install on fixture"]
fn auto_detects_vitest_on_js_simple_fixture() {
    check("tests")
        .on("js-simple")
        .passes()
        .stdout_has("vitest (auto-detected)");
}
