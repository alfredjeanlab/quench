// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! kcov integration for shell script coverage.
//!
//! Executes tests wrapped by kcov and parses Cobertura XML output.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::coverage::CoverageResult;

/// Check if kcov is available.
pub fn kcov_available() -> bool {
    Command::new("kcov")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Collect shell script coverage via kcov.
///
/// Wraps the test command with kcov to collect coverage for the specified scripts.
pub fn collect_shell_coverage(
    scripts: &[PathBuf],
    test_command: &[String],
    root: &Path,
) -> CoverageResult {
    if !kcov_available() {
        return CoverageResult::skipped();
    }

    if scripts.is_empty() {
        return CoverageResult::skipped();
    }

    let start = Instant::now();
    let output_dir = root.join("target").join("kcov");

    // Clean previous output
    if output_dir.exists() {
        let _ = std::fs::remove_dir_all(&output_dir);
    }
    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        return CoverageResult::failed(start.elapsed(), format!("failed to create kcov dir: {e}"));
    }

    // Build include paths from script directories
    let include_paths: HashSet<PathBuf> =
        scripts.iter().filter_map(|p| p.parent().map(|d| d.to_path_buf())).collect();

    let include_arg =
        include_paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>().join(",");

    // Run kcov wrapping the test command
    let mut cmd = Command::new("kcov");
    cmd.arg("--include-path").arg(&include_arg);
    cmd.arg(&output_dir);
    cmd.args(test_command);
    cmd.current_dir(root);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = match cmd.output() {
        Ok(out) => out,
        Err(e) => {
            return CoverageResult::failed(start.elapsed(), format!("failed to run kcov: {e}"));
        }
    };

    let duration = start.elapsed();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let truncated = stderr.lines().take(10).collect::<Vec<_>>().join("\n");
        return CoverageResult::failed(duration, format!("kcov failed:\n{truncated}"));
    }

    // Parse kcov output
    parse_kcov_output(&output_dir, duration, root)
}

/// Parse kcov Cobertura XML output.
fn parse_kcov_output(output_dir: &Path, duration: Duration, root: &Path) -> CoverageResult {
    // kcov creates a subdirectory with the executable name
    // Look for cobertura.xml in any subdirectory
    let xml_path = find_cobertura_xml(output_dir);

    let xml_path = match xml_path {
        Some(p) => p,
        None => {
            return CoverageResult::failed(duration, "kcov output not found");
        }
    };

    let xml_content = match std::fs::read_to_string(&xml_path) {
        Ok(c) => c,
        Err(e) => {
            return CoverageResult::failed(duration, format!("failed to read kcov output: {e}"));
        }
    };

    parse_cobertura_xml(&xml_content, duration, root)
}

/// Find cobertura.xml in kcov output directory.
fn find_cobertura_xml(output_dir: &Path) -> Option<PathBuf> {
    // Direct path
    let direct = output_dir.join("cobertura.xml");
    if direct.exists() {
        return Some(direct);
    }

    // Search subdirectories
    if let Ok(entries) = std::fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                let xml = entry.path().join("cobertura.xml");
                if xml.exists() {
                    return Some(xml);
                }
            }
        }
    }

    None
}

/// Parse Cobertura XML format.
fn parse_cobertura_xml(xml: &str, duration: Duration, root: &Path) -> CoverageResult {
    // Simple XML parsing for Cobertura format
    // <coverage line-rate="0.75" ...>
    //   <packages>
    //     <package>
    //       <classes>
    //         <class filename="path" line-rate="0.80">
    //           ...
    //         </class>
    //       </classes>
    //     </package>
    //   </packages>
    // </coverage>

    let mut files = HashMap::new();
    let mut overall_coverage = None;

    // Extract overall line-rate from coverage element
    if let Some(rate) = extract_line_rate(xml, "coverage") {
        overall_coverage = Some(rate * 100.0);
    }

    // Extract per-file coverage from class elements
    for class_content in extract_elements(xml, "class") {
        if let Some(filename) = extract_attribute(&class_content, "filename")
            && let Some(rate) =
                extract_attribute(&class_content, "line-rate").and_then(|r| r.parse::<f64>().ok())
        {
            let normalized = normalize_path(&filename, root);
            files.insert(normalized, rate * 100.0);
        }
    }

    // If we got file data but no overall, compute from files
    if overall_coverage.is_none() && !files.is_empty() {
        let sum: f64 = files.values().sum();
        overall_coverage = Some(sum / files.len() as f64);
    }

    CoverageResult {
        success: true,
        error: None,
        duration,
        line_coverage: overall_coverage,
        files,
        packages: std::collections::HashMap::new(),
    }
}

/// Extract line-rate from an element.
fn extract_line_rate(xml: &str, element: &str) -> Option<f64> {
    let start_tag = format!("<{} ", element);
    if let Some(start) = xml.find(&start_tag) {
        let end = xml[start..].find('>')?;
        let tag_content = &xml[start..start + end];
        extract_attribute(tag_content, "line-rate").and_then(|r| r.parse().ok())
    } else {
        None
    }
}

/// Extract all elements with the given tag name.
fn extract_elements(xml: &str, tag: &str) -> Vec<String> {
    let mut elements = Vec::new();
    let start_tag = format!("<{} ", tag);
    let mut search_start = 0;

    while let Some(start) = xml[search_start..].find(&start_tag) {
        let abs_start = search_start + start;
        // Find the end of this opening tag
        if let Some(tag_end) = xml[abs_start..].find('>') {
            let tag_content = &xml[abs_start..abs_start + tag_end + 1];
            elements.push(tag_content.to_string());
            search_start = abs_start + tag_end + 1;
        } else {
            break;
        }
    }

    elements
}

/// Extract an attribute value from a tag string.
fn extract_attribute(tag: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    let start = tag.find(&pattern)?;
    let value_start = start + pattern.len();
    let value_end = tag[value_start..].find('"')?;
    Some(tag[value_start..value_start + value_end].to_string())
}

/// Normalize path to workspace-relative.
fn normalize_path(path: &str, root: &Path) -> String {
    let path = Path::new(path);

    // Try to make relative to root
    if let Ok(rel) = path.strip_prefix(root) {
        return rel.to_string_lossy().to_string();
    }

    // Look for common markers
    let path_str = path.to_string_lossy();
    for marker in ["scripts/", "src/", "bin/", "lib/"] {
        if let Some(idx) = path_str.find(marker) {
            return path_str[idx..].to_string();
        }
    }

    // Fallback to filename
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path_str.to_string())
}

#[cfg(test)]
#[path = "kcov_tests.rs"]
mod tests;
