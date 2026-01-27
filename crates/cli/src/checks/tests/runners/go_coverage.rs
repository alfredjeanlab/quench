// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Go coverage profile parsing and collection.
//!
//! Parses Go's coverage profile format and collects coverage via `go test -coverprofile`.

use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use super::CoverageResult;

// Cache Go availability to avoid repeated checks
static GO_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if Go is available (cached).
///
/// The result is cached using OnceLock to avoid repeated subprocess
/// invocations during test suite execution.
pub fn go_available() -> bool {
    *GO_AVAILABLE.get_or_init(|| {
        Command::new("go")
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

/// Collect Go coverage for a project.
///
/// Executes `go test -coverprofile` and parses the resulting coverage profile.
/// Returns a skipped result if Go is not available.
pub fn collect_go_coverage(root: &Path, test_path: Option<&str>) -> CoverageResult {
    if !go_available() {
        return CoverageResult::skipped();
    }

    let start = Instant::now();

    // Create temp file for coverage profile
    let cover_file = root.join(".quench-coverage.out");

    let mut cmd = Command::new("go");
    cmd.args(["test", "-coverprofile"]);
    cmd.arg(&cover_file);
    cmd.arg(test_path.unwrap_or("./..."));
    cmd.current_dir(root);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = match cmd.output() {
        Ok(out) => out,
        Err(e) => {
            return CoverageResult::failed(start.elapsed(), format!("failed to run go test: {e}"));
        }
    };

    let duration = start.elapsed();

    // Read and cleanup coverage file
    let content = match std::fs::read_to_string(&cover_file) {
        Ok(c) => {
            // Cleanup the coverage file
            std::fs::remove_file(&cover_file).ok();
            c
        }
        Err(e) => {
            // If tests failed, go may not have written a coverage file
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let msg = stderr.lines().take(10).collect::<Vec<_>>().join("\n");
                return CoverageResult::failed(duration, format!("go test failed:\n{msg}"));
            }
            return CoverageResult::failed(
                duration,
                format!("failed to read coverage profile: {e}"),
            );
        }
    };

    parse_cover_profile(&content, duration)
}

/// Parse Go coverage profile format.
///
/// Go coverage profile format:
/// ```text
/// mode: set
/// github.com/example/pkg/math/math.go:5.14,7.2 1 1
/// github.com/example/pkg/math/math.go:9.14,11.2 1 0
/// ```
///
/// Format: `<file>:<startLine>.<startCol>,<endLine>.<endCol> <numStatements> <count>`
///
/// Returns per-file coverage as (covered_statements / total_statements) * 100.
pub fn parse_cover_profile(content: &str, duration: Duration) -> CoverageResult {
    let mut file_stats: HashMap<String, (u64, u64)> = HashMap::new(); // (covered, total)

    for line in content.lines().skip(1) {
        // Skip mode line
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some((file, statements, count)) = parse_profile_line(line) {
            let entry = file_stats.entry(file).or_default();
            entry.1 += statements; // total
            if count > 0 {
                entry.0 += statements; // covered
            }
        }
    }

    if file_stats.is_empty() {
        return CoverageResult {
            success: true,
            error: None,
            duration,
            line_coverage: None,
            files: HashMap::new(),
            packages: HashMap::new(),
        };
    }

    // Convert to percentages
    let mut files: HashMap<String, f64> = HashMap::new();
    let mut package_stats: HashMap<String, (u64, u64)> = HashMap::new(); // (covered, total)
    let mut total_covered: u64 = 0;
    let mut total_statements: u64 = 0;

    for (path, (covered, total)) in &file_stats {
        if *total > 0 {
            let pct = (*covered as f64 / *total as f64) * 100.0;
            let normalized_path = normalize_go_path(path);
            files.insert(normalized_path, pct);

            // Aggregate by package
            let package = extract_go_package(path);
            let pkg_entry = package_stats.entry(package).or_default();
            pkg_entry.0 += covered;
            pkg_entry.1 += total;

            // Aggregate totals
            total_covered += covered;
            total_statements += total;
        }
    }

    // Calculate per-package percentages
    let packages: HashMap<String, f64> = package_stats
        .into_iter()
        .filter(|(_, (_, total))| *total > 0)
        .map(|(pkg, (covered, total))| {
            let pct = (covered as f64 / total as f64) * 100.0;
            (pkg, pct)
        })
        .collect();

    // Calculate overall line coverage
    let line_coverage = if total_statements > 0 {
        Some((total_covered as f64 / total_statements as f64) * 100.0)
    } else {
        None
    };

    CoverageResult {
        success: true,
        error: None,
        duration,
        line_coverage,
        files,
        packages,
    }
}

/// Parse a single line from Go's coverage profile.
///
/// Format: `<file>:<startLine>.<startCol>,<endLine>.<endCol> <numStatements> <count>`
/// Example: `github.com/user/repo/pkg/math.go:5.14,7.2 1 1`
///
/// Returns (file_path, statements, count) or None if parsing fails.
fn parse_profile_line(line: &str) -> Option<(String, u64, u64)> {
    // Find last colon (file paths can contain colons on Windows)
    let colon_idx = line.rfind(':')?;
    let file = &line[..colon_idx];
    let rest = &line[colon_idx + 1..];

    // Split "5.14,7.2 1 1" into parts
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() != 3 {
        return None;
    }

    let statements: u64 = parts[1].parse().ok()?;
    let count: u64 = parts[2].parse().ok()?;

    Some((file.to_string(), statements, count))
}

/// Normalize Go coverage paths to workspace-relative.
///
/// Go reports full module paths like `github.com/user/repo/pkg/math/math.go`.
/// We normalize to just the relative portion starting from common markers.
fn normalize_go_path(path: &str) -> String {
    // Look for common Go project markers and keep from there
    for marker in ["pkg/", "internal/", "cmd/", "src/"] {
        if let Some(idx) = path.find(marker) {
            return path[idx..].to_string();
        }
    }

    // Fallback: use filename only
    path.rsplit('/').next().unwrap_or(path).to_string()
}

/// Extract Go package name from file path.
///
/// Go module paths use forward slashes. Extract package from file path.
///
/// Examples:
/// - `github.com/user/repo/pkg/math/math.go` -> `pkg/math`
/// - `github.com/user/repo/internal/core/core.go` -> `internal/core`
/// - `github.com/user/repo/main.go` -> `root`
fn extract_go_package(path: &str) -> String {
    // Find common patterns: pkg/, internal/, cmd/
    for marker in ["pkg/", "internal/", "cmd/"] {
        if let Some(idx) = path.find(marker) {
            // Extract up to filename
            let package_path = &path[idx..];
            if let Some(file_idx) = package_path.rfind('/') {
                return package_path[..file_idx].to_string();
            }
        }
    }
    "root".to_string()
}

#[cfg(test)]
#[path = "go_coverage_tests.rs"]
mod tests;
