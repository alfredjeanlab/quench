// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Python coverage collection and parsing.
//!
//! Supports coverage.py and pytest-cov, parsing both JSON and Cobertura XML formats.

use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::CoverageResult;

// Cache coverage.py availability to avoid repeated checks
static COVERAGE_AVAILABLE: OnceLock<bool> = OnceLock::new();

// Cache pytest-cov availability to avoid repeated checks
static PYTEST_COV_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if coverage.py is available (cached).
///
/// The result is cached using OnceLock to avoid repeated subprocess
/// invocations during test suite execution.
pub fn coverage_available() -> bool {
    *COVERAGE_AVAILABLE.get_or_init(|| {
        Command::new("coverage")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

/// Check if pytest-cov is available (cached).
///
/// The result is cached using OnceLock to avoid repeated subprocess
/// invocations during test suite execution.
pub fn pytest_cov_available() -> bool {
    *PYTEST_COV_AVAILABLE.get_or_init(|| {
        Command::new("pytest")
            .arg("--co")
            .arg("-q")
            .arg("--cov-help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

/// Collect Python coverage for a project.
///
/// Strategy:
/// 1. Run `pytest --cov` if pytest-cov is available
/// 2. Or run `coverage run -m pytest && coverage json` if coverage.py is available
/// 3. Parse coverage.json (preferred) or coverage.xml (fallback)
///
/// Returns a skipped result if no coverage tools are available.
pub fn collect_python_coverage(root: &Path, test_path: Option<&str>) -> CoverageResult {
    if !pytest_cov_available() && !coverage_available() {
        return CoverageResult::skipped();
    }

    let start = Instant::now();

    // Determine source directory for coverage
    // Default to "src" if it exists, otherwise use project root
    let source_dir = if root.join("src").is_dir() {
        "src"
    } else {
        "."
    };

    // Try pytest-cov first (preferred)
    if pytest_cov_available() {
        let result = run_pytest_cov(root, test_path, source_dir);
        if result.success {
            return result;
        }
        // Fall through to try coverage.py if pytest-cov failed
    }

    // Fallback to coverage.py
    if coverage_available() {
        return run_coverage_py(root, test_path, source_dir, start);
    }

    CoverageResult::failed(start.elapsed(), "no Python coverage tools available")
}

/// Run pytest with --cov flag and parse results.
fn run_pytest_cov(root: &Path, test_path: Option<&str>, source_dir: &str) -> CoverageResult {
    let start = Instant::now();

    // Clean up any existing coverage files first
    let _ = std::fs::remove_file(root.join("coverage.json"));
    let _ = std::fs::remove_file(root.join(".coverage"));

    let mut cmd = Command::new("pytest");
    cmd.arg(format!("--cov={source_dir}"));
    cmd.arg("--cov-report=json");
    cmd.arg("-q"); // Quiet mode for speed

    if let Some(path) = test_path {
        cmd.arg(path);
    }

    cmd.current_dir(root);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = match cmd.output() {
        Ok(out) => out,
        Err(e) => {
            return CoverageResult::failed(
                start.elapsed(),
                format!("failed to run pytest --cov: {e}"),
            );
        }
    };

    let duration = start.elapsed();

    // Even if tests fail, coverage may have been collected
    // Only report error if coverage file is missing
    let json_path = root.join("coverage.json");
    if json_path.exists() {
        let content = match std::fs::read_to_string(&json_path) {
            Ok(c) => c,
            Err(e) => {
                return CoverageResult::failed(
                    duration,
                    format!("failed to read coverage.json: {e}"),
                );
            }
        };

        // Clean up coverage file
        let _ = std::fs::remove_file(&json_path);
        let _ = std::fs::remove_file(root.join(".coverage"));

        return parse_coverage_json(&content, duration);
    }

    // No coverage file produced
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.lines().take(5).collect::<Vec<_>>().join("\n");
        return CoverageResult::failed(duration, format!("pytest --cov failed:\n{msg}"));
    }

    CoverageResult::failed(duration, "pytest --cov did not produce coverage.json")
}

/// Run coverage.py and parse results.
fn run_coverage_py(
    root: &Path,
    test_path: Option<&str>,
    source_dir: &str,
    start: Instant,
) -> CoverageResult {
    // Clean up any existing coverage files
    let _ = std::fs::remove_file(root.join("coverage.json"));
    let _ = std::fs::remove_file(root.join(".coverage"));

    // Run: coverage run --source=<source> -m pytest <path>
    let mut cmd = Command::new("coverage");
    cmd.args(["run", "--source", source_dir, "-m", "pytest", "-q"]);

    if let Some(path) = test_path {
        cmd.arg(path);
    }

    cmd.current_dir(root);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = match cmd.output() {
        Ok(out) => out,
        Err(e) => {
            return CoverageResult::failed(
                start.elapsed(),
                format!("failed to run coverage run: {e}"),
            );
        }
    };

    // Generate JSON report
    let mut json_cmd = Command::new("coverage");
    json_cmd.args(["json", "-o", "coverage.json"]);
    json_cmd.current_dir(root);
    json_cmd.stdout(Stdio::piped());
    json_cmd.stderr(Stdio::piped());

    if let Err(e) = json_cmd.output() {
        return CoverageResult::failed(
            start.elapsed(),
            format!("failed to run coverage json: {e}"),
        );
    }

    let duration = start.elapsed();

    // Read and parse coverage.json
    let json_path = root.join("coverage.json");
    if json_path.exists() {
        let content = match std::fs::read_to_string(&json_path) {
            Ok(c) => c,
            Err(e) => {
                return CoverageResult::failed(
                    duration,
                    format!("failed to read coverage.json: {e}"),
                );
            }
        };

        // Clean up coverage files
        let _ = std::fs::remove_file(&json_path);
        let _ = std::fs::remove_file(root.join(".coverage"));

        return parse_coverage_json(&content, duration);
    }

    // Fallback: try coverage.xml if JSON wasn't produced
    let xml_path = root.join("coverage.xml");
    if xml_path.exists() {
        let content = match std::fs::read_to_string(&xml_path) {
            Ok(c) => c,
            Err(e) => {
                return CoverageResult::failed(
                    duration,
                    format!("failed to read coverage.xml: {e}"),
                );
            }
        };

        let _ = std::fs::remove_file(&xml_path);
        let _ = std::fs::remove_file(root.join(".coverage"));

        return parse_cobertura_xml(&content, duration);
    }

    // No coverage file produced
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.lines().take(5).collect::<Vec<_>>().join("\n");
        return CoverageResult::failed(duration, format!("coverage run failed:\n{msg}"));
    }

    CoverageResult::failed(duration, "coverage.py did not produce coverage report")
}

// =============================================================================
// JSON Parsing (coverage.json format)
// =============================================================================

/// coverage.json top-level structure.
#[derive(Debug, Deserialize)]
struct CoverageJson {
    #[allow(dead_code)]
    meta: CoverageMeta,
    files: HashMap<String, FileData>,
    totals: TotalsData,
}

/// Metadata from coverage.json.
#[derive(Debug, Deserialize)]
struct CoverageMeta {
    #[allow(dead_code)]
    version: Option<String>,
    #[allow(dead_code)]
    branch_coverage: Option<bool>,
}

/// Per-file coverage data from coverage.json.
#[derive(Debug, Deserialize)]
struct FileData {
    #[allow(dead_code)]
    executed_lines: Vec<u32>,
    #[allow(dead_code)]
    missing_lines: Vec<u32>,
    #[allow(dead_code)]
    excluded_lines: Option<Vec<u32>>,
    summary: FileSummary,
}

/// File-level summary from coverage.json.
#[derive(Debug, Deserialize)]
struct FileSummary {
    #[allow(dead_code)]
    covered_lines: u64,
    #[allow(dead_code)]
    num_statements: u64,
    percent_covered: f64,
}

/// Total summary from coverage.json.
#[derive(Debug, Deserialize)]
struct TotalsData {
    #[allow(dead_code)]
    covered_lines: u64,
    #[allow(dead_code)]
    num_statements: u64,
    percent_covered: f64,
}

/// Parse coverage.json content.
pub(crate) fn parse_coverage_json(json: &str, duration: Duration) -> CoverageResult {
    let report: CoverageJson = match serde_json::from_str(json) {
        Ok(r) => r,
        Err(e) => {
            return CoverageResult::failed(duration, format!("failed to parse coverage.json: {e}"));
        }
    };

    if report.files.is_empty() {
        return CoverageResult {
            success: true,
            error: None,
            duration,
            line_coverage: Some(report.totals.percent_covered),
            files: HashMap::new(),
            packages: HashMap::new(),
        };
    }

    let mut files: HashMap<String, f64> = HashMap::new();
    let mut package_files: HashMap<String, Vec<f64>> = HashMap::new();

    for (file_path, file_data) in &report.files {
        let coverage = file_data.summary.percent_covered;
        let normalized_path = normalize_python_path(file_path);
        files.insert(normalized_path.clone(), coverage);

        let package = extract_python_package(&normalized_path);
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
        line_coverage: Some(report.totals.percent_covered),
        files,
        packages,
    }
}

// =============================================================================
// XML Parsing (Cobertura format)
// =============================================================================

/// Parse Cobertura XML coverage report.
///
/// Cobertura format:
/// ```xml
/// <coverage line-rate="0.8333">
///   <packages>
///     <package name="myproject">
///       <classes>
///         <class filename="src/myproject/math.py" line-rate="0.75">
///           <lines><line number="1" hits="1"/></lines>
///         </class>
///       </classes>
///     </package>
///   </packages>
/// </coverage>
/// ```
pub(crate) fn parse_cobertura_xml(xml: &str, duration: Duration) -> CoverageResult {
    // Parse using simple string matching (avoid adding XML dependency)
    // This is a minimal parser for Cobertura format

    // Extract overall line-rate from <coverage> element
    let line_coverage = extract_xml_attr(xml, "coverage", "line-rate")
        .and_then(|s| s.parse::<f64>().ok())
        .map(|rate| rate * 100.0);

    let mut files: HashMap<String, f64> = HashMap::new();
    let mut package_files: HashMap<String, Vec<f64>> = HashMap::new();

    // Parse class elements for per-file coverage
    for class_start in xml.match_indices("<class ").map(|(i, _)| i) {
        let class_end = xml[class_start..].find("/>").or_else(|| {
            xml[class_start..]
                .find("</class>")
                .map(|i| i + "</class>".len())
        });

        if let Some(end_offset) = class_end {
            let class_xml = &xml[class_start..class_start + end_offset];

            if let Some(filename) = extract_attr(class_xml, "filename")
                && let Some(line_rate_str) = extract_attr(class_xml, "line-rate")
                && let Ok(line_rate) = line_rate_str.parse::<f64>()
            {
                let coverage = line_rate * 100.0;
                let normalized_path = normalize_python_path(&filename);
                files.insert(normalized_path.clone(), coverage);

                let package = extract_python_package(&normalized_path);
                package_files.entry(package).or_default().push(coverage);
            }
        }
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
        line_coverage,
        files,
        packages,
    }
}

/// Extract an attribute value from an XML element string.
fn extract_attr(xml: &str, attr_name: &str) -> Option<String> {
    let pattern = format!("{attr_name}=\"");
    let start = xml.find(&pattern)? + pattern.len();
    let end = xml[start..].find('"')? + start;
    Some(xml[start..end].to_string())
}

/// Extract an attribute from the first occurrence of an element.
fn extract_xml_attr(xml: &str, element: &str, attr_name: &str) -> Option<String> {
    let element_start = xml.find(&format!("<{element} "))?;
    let element_end = xml[element_start..].find('>')?;
    let element_xml = &xml[element_start..element_start + element_end];
    extract_attr(element_xml, attr_name)
}

// =============================================================================
// Path Normalization
// =============================================================================

/// Normalize Python coverage paths to project-relative.
///
/// Python coverage reports absolute paths. We normalize to project-relative
/// paths starting from common markers like "src/" or the package name.
pub fn normalize_python_path(path: &str) -> String {
    // Look for src-layout: src/<package>/
    if let Some(idx) = path.find("/src/") {
        return path[idx + 1..].to_string();
    }

    // Look for tests directory
    if let Some(idx) = path.find("/tests/") {
        return path[idx + 1..].to_string();
    }

    // Look for common Python package markers in absolute paths
    // Try to find a reasonable start point after common prefixes
    for prefix in ["/site-packages/", "/lib/python"] {
        if path.contains(prefix) {
            // Skip site-packages files (external dependencies)
            return std::path::Path::new(path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string());
        }
    }

    // If path starts with a typical project structure marker, use it
    for marker in ["src/", "lib/", "app/"] {
        if let Some(idx) = path.find(marker) {
            return path[idx..].to_string();
        }
    }

    // Fallback: use filename only for absolute paths, keep relative paths as-is
    if path.starts_with('/') {
        std::path::Path::new(path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string())
    } else {
        path.to_string()
    }
}

/// Extract Python package name from normalized file path.
///
/// Heuristics:
/// - src-layout: src/<package>/... -> <package>
/// - flat-layout: <package>/... -> <package>
/// - Fallback: "root"
pub fn extract_python_package(path: &str) -> String {
    // src-layout: src/<package>/... -> <package>
    if let Some(rest) = path.strip_prefix("src/") {
        if let Some(end) = rest.find('/') {
            return rest[..end].to_string();
        }
        // Single file under src/
        return rest.to_string();
    }

    // tests directory: tests/... -> tests
    if path.starts_with("tests/") || path == "tests" {
        return "tests".to_string();
    }

    // flat-layout: <package>/... -> <package>
    if let Some(end) = path.find('/') {
        return path[..end].to_string();
    }

    // Single file at root
    "root".to_string()
}

#[cfg(test)]
#[path = "python_coverage_tests.rs"]
mod tests;
