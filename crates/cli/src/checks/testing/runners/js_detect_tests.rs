// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use std::fs;
use tempfile::TempDir;

fn temp_project() -> TempDir {
    tempfile::tempdir().unwrap()
}

// =============================================================================
// CONFIG FILE DETECTION
// =============================================================================

#[test]
fn detects_vitest_from_config_ts() {
    let temp = temp_project();
    fs::write(temp.path().join("vitest.config.ts"), "export default {}").unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.runner, JsRunner::Vitest);
    assert!(matches!(result.source, DetectionSource::ConfigFile(ref s) if s == "vitest.config.ts"));
}

#[test]
fn detects_vitest_from_config_js() {
    let temp = temp_project();
    fs::write(temp.path().join("vitest.config.js"), "module.exports = {}").unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.runner, JsRunner::Vitest);
    assert!(matches!(result.source, DetectionSource::ConfigFile(ref s) if s == "vitest.config.js"));
}

#[test]
fn detects_jest_from_config_js() {
    let temp = temp_project();
    fs::write(temp.path().join("jest.config.js"), "module.exports = {}").unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.runner, JsRunner::Jest);
    assert!(matches!(result.source, DetectionSource::ConfigFile(ref s) if s == "jest.config.js"));
}

#[test]
fn detects_jest_from_config_json() {
    let temp = temp_project();
    fs::write(temp.path().join("jest.config.json"), "{}").unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.runner, JsRunner::Jest);
    assert!(matches!(result.source, DetectionSource::ConfigFile(ref s) if s == "jest.config.json"));
}

// =============================================================================
// DEV DEPENDENCIES DETECTION
// =============================================================================

#[test]
fn detects_vitest_from_dev_dependencies() {
    let temp = temp_project();
    fs::write(temp.path().join("package.json"), r#"{"devDependencies": {"vitest": "^2.0.0"}}"#)
        .unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.runner, JsRunner::Vitest);
    assert!(matches!(result.source, DetectionSource::DevDependency(ref s) if s == "vitest"));
}

#[test]
fn detects_jest_from_dev_dependencies() {
    let temp = temp_project();
    fs::write(temp.path().join("package.json"), r#"{"devDependencies": {"jest": "^29.0.0"}}"#)
        .unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.runner, JsRunner::Jest);
    assert!(matches!(result.source, DetectionSource::DevDependency(ref s) if s == "jest"));
}

#[test]
fn detects_bun_from_bun_types() {
    let temp = temp_project();
    fs::write(temp.path().join("package.json"), r#"{"devDependencies": {"bun-types": "^1.0.0"}}"#)
        .unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.runner, JsRunner::Bun);
    assert!(matches!(result.source, DetectionSource::DevDependency(ref s) if s == "bun-types"));
}

// =============================================================================
// TEST SCRIPT DETECTION
// =============================================================================

#[test]
fn detects_vitest_from_test_script() {
    let temp = temp_project();
    fs::write(temp.path().join("package.json"), r#"{"scripts": {"test": "vitest run"}}"#).unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.runner, JsRunner::Vitest);
    assert!(matches!(result.source, DetectionSource::TestScript(ref s) if s == "vitest run"));
}

#[test]
fn detects_jest_from_test_script() {
    let temp = temp_project();
    fs::write(temp.path().join("package.json"), r#"{"scripts": {"test": "jest --coverage"}}"#)
        .unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.runner, JsRunner::Jest);
    assert!(matches!(result.source, DetectionSource::TestScript(ref s) if s == "jest --coverage"));
}

#[test]
fn detects_bun_from_test_script() {
    let temp = temp_project();
    fs::write(temp.path().join("package.json"), r#"{"scripts": {"test": "bun test"}}"#).unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.runner, JsRunner::Bun);
    assert!(matches!(result.source, DetectionSource::TestScript(ref s) if s == "bun test"));
}

// =============================================================================
// PRIORITY TESTS
// =============================================================================

#[test]
fn config_file_takes_priority_over_dependencies() {
    let temp = temp_project();
    // Has both vitest config AND jest in devDependencies
    fs::write(temp.path().join("vitest.config.ts"), "export default {}").unwrap();
    fs::write(temp.path().join("package.json"), r#"{"devDependencies": {"jest": "^29.0.0"}}"#)
        .unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();
    // Should detect vitest from config file, not jest from deps
    assert_eq!(result.runner, JsRunner::Vitest);
    assert!(matches!(result.source, DetectionSource::ConfigFile(_)));
}

#[test]
fn dependencies_take_priority_over_test_script() {
    let temp = temp_project();
    // Has vitest in deps AND jest in test script
    fs::write(
        temp.path().join("package.json"),
        r#"{"devDependencies": {"vitest": "^2.0.0"}, "scripts": {"test": "jest"}}"#,
    )
    .unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();
    // Should detect vitest from deps, not jest from script
    assert_eq!(result.runner, JsRunner::Vitest);
    assert!(matches!(result.source, DetectionSource::DevDependency(_)));
}

#[test]
fn vitest_takes_priority_over_jest_in_dependencies() {
    let temp = temp_project();
    // Has both vitest AND jest in devDependencies
    fs::write(
        temp.path().join("package.json"),
        r#"{"devDependencies": {"vitest": "^2.0.0", "jest": "^29.0.0"}}"#,
    )
    .unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();
    // Should detect vitest (higher priority)
    assert_eq!(result.runner, JsRunner::Vitest);
}

// =============================================================================
// NO DETECTION CASES
// =============================================================================

#[test]
fn returns_none_when_no_package_json() {
    let temp = temp_project();
    // No package.json and no config files

    let result = detect_js_runner(temp.path());
    assert!(result.is_none());
}

#[test]
fn returns_none_when_no_runner_detected() {
    let temp = temp_project();
    // package.json with no test runner info
    fs::write(
        temp.path().join("package.json"),
        r#"{"name": "test", "scripts": {"test": "echo 'no tests'"}}"#,
    )
    .unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_none());
}

#[test]
fn returns_none_for_invalid_json() {
    let temp = temp_project();
    fs::write(temp.path().join("package.json"), "not valid json").unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_none());
}

// =============================================================================
// HELPER METHOD TESTS
// =============================================================================

#[test]
fn js_runner_name_returns_correct_strings() {
    assert_eq!(JsRunner::Vitest.name(), "vitest");
    assert_eq!(JsRunner::Jest.name(), "jest");
    assert_eq!(JsRunner::Bun.name(), "bun");
}

#[test]
fn detection_source_to_metric_string() {
    let pkg = DetectionSource::PackageManager(PackageManager::Bun);
    assert_eq!(pkg.to_metric_string(), "package_manager:bun");

    let config = DetectionSource::ConfigFile("vitest.config.ts".to_string());
    assert_eq!(config.to_metric_string(), "config_file:vitest.config.ts");

    let dep = DetectionSource::DevDependency("vitest".to_string());
    assert_eq!(dep.to_metric_string(), "dev_dependency:vitest");

    let script = DetectionSource::TestScript("vitest run".to_string());
    assert_eq!(script.to_metric_string(), "test_script:vitest run");
}

// =============================================================================
// PACKAGE MANAGER DETECTION
// =============================================================================

/// Note: Package manager detection tests for Bun depend on bun being installed.
/// When bun is installed and bun.lock exists, detect_js_runner returns Bun.
/// When bun is not installed, it falls through to other detection methods.
#[test]
fn bun_lock_detection_respects_bun_installation() {
    let temp = temp_project();
    fs::write(temp.path().join("bun.lock"), "").unwrap();
    fs::write(temp.path().join("package.json"), r#"{"devDependencies": {"vitest": "^2.0.0"}}"#)
        .unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();

    // If bun is installed, should detect from package manager
    // If not, should fall through to vitest from devDependencies
    if is_bun_installed() {
        assert_eq!(result.runner, JsRunner::Bun);
        assert!(matches!(result.source, DetectionSource::PackageManager(_)));
    } else {
        assert_eq!(result.runner, JsRunner::Vitest);
        assert!(matches!(result.source, DetectionSource::DevDependency(_)));
    }
}

#[test]
fn bun_lockb_detection_respects_bun_installation() {
    let temp = temp_project();
    fs::write(temp.path().join("bun.lockb"), "").unwrap();
    fs::write(temp.path().join("package.json"), r#"{"scripts": {"test": "jest"}}"#).unwrap();

    let result = detect_js_runner(temp.path());
    assert!(result.is_some());
    let result = result.unwrap();

    // If bun is installed, should detect from package manager
    // If not, should fall through to jest from test script
    if is_bun_installed() {
        assert_eq!(result.runner, JsRunner::Bun);
        assert!(matches!(result.source, DetectionSource::PackageManager(_)));
    } else {
        assert_eq!(result.runner, JsRunner::Jest);
        assert!(matches!(result.source, DetectionSource::TestScript(_)));
    }
}
