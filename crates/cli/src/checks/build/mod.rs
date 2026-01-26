// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Build check: binary size and build time metrics.
//!
//! CI-only check that measures:
//! - Binary sizes for configured targets
//! - Cold build time (clean build)
//! - Hot build time (incremental build)

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::adapter::{ProjectLanguage, detect_language};
use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::tolerance::{parse_duration, parse_size};

/// Default advice for cold build time threshold violations.
const TIME_COLD_ADVICE: &str =
    "Cold build time exceeded threshold. Consider optimizing dependencies or build configuration.";

/// Default advice for hot build time threshold violations.
const TIME_HOT_ADVICE: &str =
    "Hot build time exceeded threshold. Consider optimizing incremental build setup.";

pub struct BuildCheck;

impl Check for BuildCheck {
    fn name(&self) -> &'static str {
        "build"
    }

    fn description(&self) -> &'static str {
        "Build metrics (size, time)"
    }

    fn default_enabled(&self) -> bool {
        false // CI-only by default
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        // Skip if not in CI mode
        if !ctx.ci_mode {
            return CheckResult::stub(self.name());
        }

        let mut metrics = BuildMetrics::default();
        let mut violations = Vec::new();
        let language = detect_language(ctx.root);
        let build_config = &ctx.config.check.build;

        // Parse time thresholds
        let time_cold_max = build_config
            .time_cold_max
            .as_ref()
            .and_then(|s| parse_duration(s).ok());
        let time_hot_max = build_config
            .time_hot_max
            .as_ref()
            .and_then(|s| parse_duration(s).ok());

        // Resolve targets: explicit config > auto-detection
        let targets = resolve_targets(ctx, language);
        let explicit_targets = !ctx.config.check.build.targets.is_empty();

        // Measure binary sizes and check thresholds
        for target in &targets {
            if let Some(size) = measure_binary_size(ctx.root, target, language) {
                metrics.sizes.insert(target.clone(), size);

                // Check threshold
                if let Some(threshold) = get_size_threshold(ctx, target)
                    && size > threshold
                {
                    violations.push(Violation {
                        file: None,
                        line: None,
                        violation_type: "size_exceeded".to_string(),
                        advice: "Reduce binary size. Check for unnecessary dependencies."
                            .to_string(),
                        value: Some(size as i64),
                        threshold: Some(threshold as i64),
                        pattern: None,
                        lines: None,
                        nonblank: None,
                        other_file: None,
                        section: None,
                        commit: None,
                        message: None,
                        expected_docs: None,
                        area: None,
                        area_match: None,
                        path: None,
                        target: Some(target.clone()),
                        change_type: None,
                        lines_changed: None,
                        scope: None,
                    });
                }
            }
        }

        // Check for missing targets (only when explicitly configured)
        if explicit_targets {
            for target in &targets {
                if !metrics.sizes.contains_key(target) {
                    violations.push(Violation {
                        file: None,
                        line: None,
                        violation_type: "missing_target".to_string(),
                        advice:
                            "Configured build target not found. Verify target exists and builds successfully."
                                .to_string(),
                        value: None,
                        threshold: None,
                        pattern: None,
                        lines: None,
                        nonblank: None,
                        other_file: None,
                        section: None,
                        commit: None,
                        message: None,
                        expected_docs: None,
                        area: None,
                        area_match: None,
                        path: None,
                        target: Some(target.clone()),
                        change_type: None,
                        lines_changed: None,
                        scope: None,
                    });
                }
            }
        }

        // Measure build times (only if configured or thresholds set)
        let should_measure_cold = ctx.config.ratchet.build_time_cold || time_cold_max.is_some();
        let should_measure_hot = ctx.config.ratchet.build_time_hot || time_hot_max.is_some();

        if should_measure_cold {
            metrics.time_cold = measure_cold_build(ctx.root, language);

            // Check cold build time threshold
            if let (Some(duration), Some(max)) = (metrics.time_cold, time_cold_max)
                && duration > max
            {
                violations.push(
                    Violation::file_only("build", "time_cold_exceeded", TIME_COLD_ADVICE)
                        .with_threshold(duration.as_millis() as i64, max.as_millis() as i64),
                );
            }
        }

        if should_measure_hot {
            metrics.time_hot = measure_hot_build(ctx.root, language);

            // Check hot build time threshold
            if let (Some(duration), Some(max)) = (metrics.time_hot, time_hot_max)
                && duration > max
            {
                violations.push(
                    Violation::file_only("build", "time_hot_exceeded", TIME_HOT_ADVICE)
                        .with_threshold(duration.as_millis() as i64, max.as_millis() as i64),
                );
            }
        }

        // Return result with metrics
        let has_metrics =
            !metrics.sizes.is_empty() || metrics.time_cold.is_some() || metrics.time_hot.is_some();

        if !violations.is_empty() {
            // Violations found - fail with metrics if available
            if has_metrics {
                CheckResult::failed(self.name(), violations).with_metrics(metrics.to_json())
            } else {
                CheckResult::failed(self.name(), violations)
            }
        } else if has_metrics {
            CheckResult::passed(self.name()).with_metrics(metrics.to_json())
        } else {
            // No metrics collected and no violations - return stub
            CheckResult::stub(self.name())
        }
    }
}

#[derive(Default)]
struct BuildMetrics {
    sizes: HashMap<String, u64>,
    time_cold: Option<Duration>,
    time_hot: Option<Duration>,
}

impl BuildMetrics {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "size": self.sizes,
            "time": {
                "cold": self.time_cold.map(|d| d.as_secs_f64()),
                "hot": self.time_hot.map(|d| d.as_secs_f64()),
            }
        })
    }
}

/// Get build targets for the project.
fn get_build_targets(root: &Path, language: ProjectLanguage) -> Vec<String> {
    match language {
        ProjectLanguage::Rust => get_rust_targets(root),
        ProjectLanguage::Go => get_go_targets(root),
        _ => Vec::new(),
    }
}

/// Get Rust binary targets from Cargo.toml.
fn get_rust_targets(root: &Path) -> Vec<String> {
    let cargo_toml = root.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Vec::new();
    }

    // Try parsing Cargo.toml to find [[bin]] sections
    if let Ok(content) = std::fs::read_to_string(&cargo_toml)
        && let Ok(manifest) = content.parse::<toml::Table>()
    {
        let mut targets = Vec::new();

        // Check for [[bin]] sections
        if let Some(bins) = manifest.get("bin").and_then(|v| v.as_array()) {
            for bin in bins {
                if let Some(name) = bin.get("name").and_then(|v| v.as_str()) {
                    targets.push(name.to_string());
                }
            }
        }

        // Check for [package] name as default binary
        if targets.is_empty()
            && let Some(pkg) = manifest.get("package").and_then(|v| v.as_table())
            && let Some(name) = pkg.get("name").and_then(|v| v.as_str())
            && root.join("src/main.rs").exists()
        {
            targets.push(name.to_string());
        }

        return targets;
    }

    Vec::new()
}

/// Get Go binary targets (package name from go.mod).
fn get_go_targets(root: &Path) -> Vec<String> {
    let go_mod = root.join("go.mod");
    if !go_mod.exists() {
        return Vec::new();
    }

    if let Ok(content) = std::fs::read_to_string(&go_mod) {
        for line in content.lines() {
            if let Some(module) = line.strip_prefix("module ") {
                // Use the last part of the module path as the binary name
                let name = module.trim().rsplit('/').next().unwrap_or(module.trim());
                return vec![name.to_string()];
            }
        }
    }

    Vec::new()
}

/// Resolve build targets: explicit config > auto-detection.
fn resolve_targets(ctx: &CheckContext, language: ProjectLanguage) -> Vec<String> {
    // Use explicit config if provided
    if !ctx.config.check.build.targets.is_empty() {
        return ctx.config.check.build.targets.clone();
    }

    // Fall back to auto-detection
    get_build_targets(ctx.root, language)
}

/// Get size threshold for a target: per-target > global > None.
fn get_size_threshold(ctx: &CheckContext, target: &str) -> Option<u64> {
    // Check per-target config first
    if let Some(target_config) = ctx.config.check.build.target.get(target)
        && let Some(ref size_str) = target_config.size_max
    {
        return parse_size(size_str).ok();
    }

    // Fall back to global threshold
    ctx.config
        .check
        .build
        .size_max
        .as_ref()
        .and_then(|s| parse_size(s).ok())
}

/// Measure binary size for a target.
fn measure_binary_size(root: &Path, target: &str, language: ProjectLanguage) -> Option<u64> {
    let binary_path = match language {
        ProjectLanguage::Rust => root.join("target/release").join(target),
        ProjectLanguage::Go => root.join(target),
        _ => return None,
    };

    std::fs::metadata(&binary_path).ok().map(|m| m.len())
}

/// Measure cold build time (clean build).
fn measure_cold_build(root: &Path, language: ProjectLanguage) -> Option<Duration> {
    let (clean_cmd, build_cmd) = match language {
        ProjectLanguage::Rust => (vec!["cargo", "clean"], vec!["cargo", "build", "--release"]),
        ProjectLanguage::Go => (vec!["go", "clean", "-cache"], vec!["go", "build", "./..."]),
        _ => return None,
    };

    // Clean first
    let output = Command::new(clean_cmd[0])
        .args(&clean_cmd[1..])
        .current_dir(root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    // Time the build
    let start = Instant::now();
    let output = Command::new(build_cmd[0])
        .args(&build_cmd[1..])
        .current_dir(root)
        .output()
        .ok()?;

    if output.status.success() {
        Some(start.elapsed())
    } else {
        None
    }
}

/// Measure hot build time (incremental build).
fn measure_hot_build(root: &Path, language: ProjectLanguage) -> Option<Duration> {
    let (touch_path, build_cmd) = match language {
        ProjectLanguage::Rust => {
            let lib_rs = root.join("src/lib.rs");
            let main_rs = root.join("src/main.rs");
            let touch = if lib_rs.exists() { lib_rs } else { main_rs };
            (touch, vec!["cargo", "build", "--release"])
        }
        ProjectLanguage::Go => {
            let main_go = root.join("main.go");
            (main_go, vec!["go", "build", "./..."])
        }
        _ => return None,
    };

    // Touch a source file to trigger incremental rebuild
    if touch_path.exists() {
        let _ = Command::new("touch")
            .arg(&touch_path)
            .current_dir(root)
            .output();
    }

    // Time the build
    let start = Instant::now();
    let output = Command::new(build_cmd[0])
        .args(&build_cmd[1..])
        .current_dir(root)
        .output()
        .ok()?;

    if output.status.success() {
        Some(start.elapsed())
    } else {
        None
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
