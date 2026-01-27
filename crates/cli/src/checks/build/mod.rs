// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Build check: binary size and build time metrics.
//!
//! CI-only check that measures:
//! - Binary sizes for configured targets (raw and gzipped for JS)
//! - Cold build time (clean build)
//! - Hot build time (incremental build)
//!
//! Supports Rust, Go, and JavaScript/TypeScript projects.

mod javascript;

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::adapter::{ProjectLanguage, detect_bundler, detect_language};
use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::tolerance::{parse_duration, parse_size};

use javascript::{has_build_script, measure_bundle_size, resolve_js_targets};

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

        // Measure binary/bundle sizes and check thresholds
        for target in &targets {
            let size_result = if language == ProjectLanguage::JavaScript {
                // JavaScript: measure bundle size (raw + gzip)
                let target_path = ctx.root.join(target);
                measure_bundle_size(&target_path).ok().map(|bundle_size| {
                    metrics
                        .sizes_gzip
                        .insert(target.clone(), bundle_size.gzipped);
                    bundle_size.raw
                })
            } else {
                // Rust/Go: measure binary size
                measure_binary_size(ctx.root, target, language)
            };

            if let Some(size) = size_result {
                metrics.sizes.insert(target.clone(), size);

                // Check threshold
                if let Some(threshold) = get_size_threshold(ctx, target)
                    && size > threshold
                {
                    let advice = if language == ProjectLanguage::JavaScript {
                        "Reduce bundle size. Check for large dependencies or unused code."
                    } else {
                        "Reduce binary size. Check for unnecessary dependencies."
                    };
                    violations.push(Violation {
                        file: None,
                        line: None,
                        violation_type: "size_exceeded".to_string(),
                        advice: advice.to_string(),
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
                        expected: None,
                        found: None,
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
                        expected: None,
                        found: None,
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
        if !violations.is_empty() {
            // Violations found - fail with metrics if available
            if metrics.has_metrics() {
                CheckResult::failed(self.name(), violations).with_metrics(metrics.to_json())
            } else {
                CheckResult::failed(self.name(), violations)
            }
        } else if metrics.has_metrics() {
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
    sizes_gzip: HashMap<String, u64>,
    time_cold: Option<Duration>,
    time_hot: Option<Duration>,
}

impl BuildMetrics {
    fn to_json(&self) -> serde_json::Value {
        let mut result = json!({
            "size": self.sizes,
            "time": {
                "cold": self.time_cold.map(|d| d.as_secs_f64()),
                "hot": self.time_hot.map(|d| d.as_secs_f64()),
            }
        });

        // Only include size_gzip if there are gzipped sizes
        if !self.sizes_gzip.is_empty() {
            result["size_gzip"] = json!(self.sizes_gzip);
        }

        result
    }

    fn has_metrics(&self) -> bool {
        !self.sizes.is_empty() || self.time_cold.is_some() || self.time_hot.is_some()
    }
}

/// Get build targets for the project.
fn get_build_targets(root: &Path, language: ProjectLanguage) -> Vec<String> {
    match language {
        ProjectLanguage::Rust => get_rust_targets(root),
        ProjectLanguage::Go => get_go_targets(root),
        ProjectLanguage::JavaScript => {
            let bundler = detect_bundler(root);
            resolve_js_targets(root, &[], bundler)
        }
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
    match language {
        ProjectLanguage::Rust => {
            // Clean first
            let output = Command::new("cargo")
                .args(["clean"])
                .current_dir(root)
                .output()
                .ok()?;

            if !output.status.success() {
                return None;
            }

            // Time the build
            let start = Instant::now();
            let output = Command::new("cargo")
                .args(["build", "--release"])
                .current_dir(root)
                .output()
                .ok()?;

            if output.status.success() {
                Some(start.elapsed())
            } else {
                None
            }
        }
        ProjectLanguage::Go => {
            // Clean first
            let output = Command::new("go")
                .args(["clean", "-cache"])
                .current_dir(root)
                .output()
                .ok()?;

            if !output.status.success() {
                return None;
            }

            // Time the build
            let start = Instant::now();
            let output = Command::new("go")
                .args(["build", "./..."])
                .current_dir(root)
                .output()
                .ok()?;

            if output.status.success() {
                Some(start.elapsed())
            } else {
                None
            }
        }
        ProjectLanguage::JavaScript => {
            // Check if build script exists
            if !has_build_script(root) {
                return None;
            }

            // Clean output directory
            let bundler = detect_bundler(root);
            let output_dir = root.join(bundler.default_output_dir());
            if output_dir.exists() {
                let _ = std::fs::remove_dir_all(&output_dir);
            }

            // Time the build
            let start = Instant::now();
            let output = Command::new("npm")
                .args(["run", "build"])
                .current_dir(root)
                .output()
                .ok()?;

            if output.status.success() {
                Some(start.elapsed())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Measure hot build time (incremental build).
fn measure_hot_build(root: &Path, language: ProjectLanguage) -> Option<Duration> {
    match language {
        ProjectLanguage::Rust => {
            let lib_rs = root.join("src/lib.rs");
            let main_rs = root.join("src/main.rs");
            let touch_path = if lib_rs.exists() { lib_rs } else { main_rs };

            // Touch a source file to trigger incremental rebuild
            if touch_path.exists() {
                let _ = Command::new("touch")
                    .arg(&touch_path)
                    .current_dir(root)
                    .output();
            }

            // Time the build
            let start = Instant::now();
            let output = Command::new("cargo")
                .args(["build", "--release"])
                .current_dir(root)
                .output()
                .ok()?;

            if output.status.success() {
                Some(start.elapsed())
            } else {
                None
            }
        }
        ProjectLanguage::Go => {
            let touch_path = root.join("main.go");

            // Touch a source file to trigger incremental rebuild
            if touch_path.exists() {
                let _ = Command::new("touch")
                    .arg(&touch_path)
                    .current_dir(root)
                    .output();
            }

            // Time the build
            let start = Instant::now();
            let output = Command::new("go")
                .args(["build", "./..."])
                .current_dir(root)
                .output()
                .ok()?;

            if output.status.success() {
                Some(start.elapsed())
            } else {
                None
            }
        }
        ProjectLanguage::JavaScript => {
            // Check if build script exists
            if !has_build_script(root) {
                return None;
            }

            // Touch a source file to trigger rebuild (common entry points)
            let touch_candidates = [
                root.join("src/index.ts"),
                root.join("src/index.js"),
                root.join("src/main.ts"),
                root.join("src/main.js"),
                root.join("index.ts"),
                root.join("index.js"),
            ];
            for touch_path in touch_candidates {
                if touch_path.exists() {
                    let _ = Command::new("touch").arg(&touch_path).output();
                    break;
                }
            }

            // Time the build
            let start = Instant::now();
            let output = Command::new("npm")
                .args(["run", "build"])
                .current_dir(root)
                .output()
                .ok()?;

            if output.status.success() {
                Some(start.elapsed())
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
