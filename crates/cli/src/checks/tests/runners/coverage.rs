// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Coverage report parsing for cargo llvm-cov.

use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde::Deserialize;

/// Result of collecting coverage.
#[derive(Debug, Clone)]
pub struct CoverageResult {
    /// Whether coverage collection succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Collection time.
    pub duration: Duration,
    /// Overall line coverage percentage (0-100).
    pub line_coverage: Option<f64>,
    /// Per-file coverage data (path -> line coverage %).
    pub files: HashMap<String, f64>,
    /// Per-package coverage data (package name -> line coverage %).
    pub packages: HashMap<String, f64>,
}

impl CoverageResult {
    pub fn failed(duration: Duration, error: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(error.into()),
            duration,
            line_coverage: None,
            files: HashMap::new(),
            packages: HashMap::new(),
        }
    }

    pub fn skipped() -> Self {
        Self {
            success: true,
            error: None,
            duration: Duration::ZERO,
            line_coverage: None,
            files: HashMap::new(),
            packages: HashMap::new(),
        }
    }
}

// Cache llvm-cov availability to avoid repeated checks
static LLVM_COV_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if cargo-llvm-cov is available (cached).
///
/// The result is cached using OnceLock to avoid repeated subprocess
/// invocations during test suite execution.
pub fn llvm_cov_available() -> bool {
    *LLVM_COV_AVAILABLE.get_or_init(|| {
        Command::new("cargo")
            .args(["llvm-cov", "--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Collect coverage for a Rust project.
pub fn collect_rust_coverage(root: &Path, path: Option<&str>) -> CoverageResult {
    if !llvm_cov_available() {
        return CoverageResult::skipped();
    }

    let start = Instant::now();

    let mut cmd = Command::new("cargo");
    cmd.args(["llvm-cov", "--json", "--release"]);

    // Set working directory
    let work_dir = path
        .map(|p| root.join(p))
        .unwrap_or_else(|| root.to_path_buf());
    cmd.current_dir(&work_dir);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = match cmd.output() {
        Ok(out) => out,
        Err(e) => {
            return CoverageResult::failed(
                start.elapsed(),
                format!("failed to run cargo llvm-cov: {e}"),
            );
        }
    };

    let duration = start.elapsed();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.lines().take(10).collect::<Vec<_>>().join("\n");
        return CoverageResult::failed(duration, format!("cargo llvm-cov failed:\n{msg}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_llvm_cov_json(&stdout, duration)
}

// =============================================================================
// JSON Parsing
// =============================================================================

#[derive(Debug, Deserialize)]
struct LlvmCovReport {
    data: Vec<LlvmCovData>,
}

#[derive(Debug, Deserialize)]
struct LlvmCovData {
    totals: LlvmCovSummary,
    files: Vec<LlvmCovFile>,
}

#[derive(Debug, Deserialize)]
struct LlvmCovSummary {
    lines: LlvmCovLines,
}

#[derive(Debug, Deserialize)]
struct LlvmCovLines {
    #[allow(dead_code)] // Deserialized from JSON but not directly used
    count: u64,
    #[allow(dead_code)] // Deserialized from JSON but not directly used
    covered: u64,
    percent: f64,
}

#[derive(Debug, Deserialize)]
struct LlvmCovFile {
    filename: String,
    summary: LlvmCovSummary,
}

fn parse_llvm_cov_json(json: &str, duration: Duration) -> CoverageResult {
    let report: LlvmCovReport = match serde_json::from_str(json) {
        Ok(r) => r,
        Err(e) => {
            return CoverageResult::failed(duration, format!("failed to parse coverage JSON: {e}"));
        }
    };

    // Get first data entry (typically only one)
    let Some(data) = report.data.first() else {
        return CoverageResult::failed(duration, "no coverage data in report");
    };

    // Extract overall line coverage
    let line_coverage = data.totals.lines.percent;

    // Extract per-file coverage and group by package
    let mut files = HashMap::new();
    let mut package_files: HashMap<String, Vec<f64>> = HashMap::new();

    for file in &data.files {
        // Normalize path: remove workspace prefix, keep relative
        let path = normalize_coverage_path(&file.filename);
        let coverage = file.summary.lines.percent;
        files.insert(path, coverage);

        // Group by package
        let package = extract_package_name(&file.filename);
        package_files.entry(package).or_default().push(coverage);
    }

    // Calculate per-package averages
    let packages: HashMap<String, f64> = package_files
        .into_iter()
        .map(|(pkg, coverages)| {
            let avg = coverages.iter().sum::<f64>() / coverages.len() as f64;
            (pkg, avg)
        })
        .collect();

    CoverageResult {
        success: true,
        error: None,
        duration,
        line_coverage: Some(line_coverage),
        files,
        packages,
    }
}

/// Normalize coverage paths to workspace-relative.
fn normalize_coverage_path(path: &str) -> String {
    // llvm-cov reports absolute paths; extract relative portion
    // Heuristic: find "src/" or "tests/" and keep from there
    for marker in ["src/", "tests/"] {
        if let Some(idx) = path.find(marker) {
            return path[idx..].to_string();
        }
    }
    // Fallback: use filename only
    std::path::Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

/// Extract package name from file path.
///
/// Heuristics:
/// - Cargo workspace: look for "crates/<name>/" pattern
/// - Monorepo: look for "packages/<name>/" pattern
/// - Single package: use "root" as fallback
fn extract_package_name(path: &str) -> String {
    // Check for "crates/<name>/" pattern (Rust workspace)
    if let Some(idx) = path.find("/crates/") {
        let rest = &path[idx + 8..];
        if let Some(end) = rest.find('/') {
            return rest[..end].to_string();
        }
    }

    // Check for "packages/<name>/" pattern (monorepo)
    if let Some(idx) = path.find("/packages/") {
        let rest = &path[idx + 10..];
        if let Some(end) = rest.find('/') {
            return rest[..end].to_string();
        }
    }

    // Fallback to "root"
    "root".to_string()
}

#[cfg(test)]
#[path = "coverage_tests.rs"]
mod tests;
