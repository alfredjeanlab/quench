// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JavaScript test runner auto-detection.
//!
//! Detection priority (first match wins):
//! 1. Config files (most specific signal)
//! 2. package.json devDependencies
//! 3. package.json scripts.test command

use std::path::Path;

/// Detected JavaScript test runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsRunner {
    Vitest,
    Jest,
    Bun,
}

impl JsRunner {
    /// Convert to runner name string used in TestSuiteConfig.
    pub fn name(&self) -> &'static str {
        match self {
            JsRunner::Vitest => "vitest",
            JsRunner::Jest => "jest",
            JsRunner::Bun => "bun",
        }
    }
}

/// Detection result with confidence signal.
#[derive(Debug)]
pub struct DetectionResult {
    pub runner: JsRunner,
    pub source: DetectionSource,
}

/// How the runner was detected.
#[derive(Debug)]
pub enum DetectionSource {
    /// Detected from a config file (e.g., "vitest.config.ts").
    ConfigFile(String),
    /// Detected from devDependencies (e.g., "vitest").
    DevDependency(String),
    /// Detected from scripts.test command (e.g., "vitest run").
    TestScript(String),
}

impl DetectionSource {
    /// Convert to a string for metrics.
    pub fn to_metric_string(&self) -> String {
        match self {
            DetectionSource::ConfigFile(name) => format!("config_file:{}", name),
            DetectionSource::DevDependency(name) => format!("dev_dependency:{}", name),
            DetectionSource::TestScript(cmd) => format!("test_script:{}", cmd),
        }
    }
}

/// Detect JavaScript test runner for a project.
///
/// Returns None if no runner can be detected.
pub fn detect_js_runner(root: &Path) -> Option<DetectionResult> {
    // 1. Check config files (highest priority)
    if let Some(result) = detect_from_config_files(root) {
        return Some(result);
    }

    // 2. Check package.json
    let package_json = root.join("package.json");
    if !package_json.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&package_json).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    // 2a. Check devDependencies
    if let Some(result) = detect_from_dependencies(&json) {
        return Some(result);
    }

    // 2b. Check scripts.test
    detect_from_test_script(&json)
}

fn detect_from_config_files(root: &Path) -> Option<DetectionResult> {
    // Vitest config files
    const VITEST_CONFIGS: &[&str] = &[
        "vitest.config.ts",
        "vitest.config.js",
        "vitest.config.mts",
        "vitest.config.mjs",
    ];
    for config in VITEST_CONFIGS {
        if root.join(config).exists() {
            return Some(DetectionResult {
                runner: JsRunner::Vitest,
                source: DetectionSource::ConfigFile((*config).to_string()),
            });
        }
    }

    // Jest config files
    const JEST_CONFIGS: &[&str] = &[
        "jest.config.ts",
        "jest.config.js",
        "jest.config.mjs",
        "jest.config.json",
    ];
    for config in JEST_CONFIGS {
        if root.join(config).exists() {
            return Some(DetectionResult {
                runner: JsRunner::Jest,
                source: DetectionSource::ConfigFile((*config).to_string()),
            });
        }
    }

    // Bun config (bun.toml with [test] section - less common)
    // Bun detection primarily via dependencies/scripts

    None
}

fn detect_from_dependencies(json: &serde_json::Value) -> Option<DetectionResult> {
    let dev_deps = json.get("devDependencies")?;

    // Check in priority order
    if dev_deps.get("vitest").is_some() {
        return Some(DetectionResult {
            runner: JsRunner::Vitest,
            source: DetectionSource::DevDependency("vitest".to_string()),
        });
    }

    if dev_deps.get("jest").is_some() {
        return Some(DetectionResult {
            runner: JsRunner::Jest,
            source: DetectionSource::DevDependency("jest".to_string()),
        });
    }

    // Bun is typically used as a runtime, not a devDependency
    // Check dependencies too for bun-specific test setup
    let deps = json.get("dependencies");
    if dev_deps.get("bun-types").is_some() || deps.and_then(|d| d.get("bun-types")).is_some() {
        return Some(DetectionResult {
            runner: JsRunner::Bun,
            source: DetectionSource::DevDependency("bun-types".to_string()),
        });
    }

    None
}

fn detect_from_test_script(json: &serde_json::Value) -> Option<DetectionResult> {
    let test_script = json.get("scripts")?.get("test")?.as_str()?;

    // Parse the test script command
    if test_script.contains("vitest") {
        return Some(DetectionResult {
            runner: JsRunner::Vitest,
            source: DetectionSource::TestScript(test_script.to_string()),
        });
    }

    if test_script.contains("jest") {
        return Some(DetectionResult {
            runner: JsRunner::Jest,
            source: DetectionSource::TestScript(test_script.to_string()),
        });
    }

    if test_script.contains("bun test") {
        return Some(DetectionResult {
            runner: JsRunner::Bun,
            source: DetectionSource::TestScript(test_script.to_string()),
        });
    }

    None
}

#[cfg(test)]
#[path = "js_detect_tests.rs"]
mod tests;
