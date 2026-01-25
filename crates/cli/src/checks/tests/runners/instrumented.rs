// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Instrumented binary building for coverage collection.
//!
//! Builds Rust binaries with LLVM coverage instrumentation for external test execution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::coverage::CoverageResult;

/// Build context for instrumented binaries.
#[derive(Debug, Clone)]
pub struct InstrumentedBuild {
    /// Directory for coverage profiles.
    pub profile_dir: PathBuf,
    /// Built binary paths by target name.
    pub binaries: HashMap<String, PathBuf>,
}

/// Build Rust binaries with coverage instrumentation.
///
/// This uses `cargo build` with `RUSTFLAGS="-C instrument-coverage"` to create
/// instrumented binaries that generate `.profraw` files when executed.
pub fn build_instrumented(targets: &[String], root: &Path) -> Result<InstrumentedBuild, String> {
    if targets.is_empty() {
        return Err("no targets specified".to_string());
    }

    let profile_dir = root.join("target").join("quench-coverage");
    std::fs::create_dir_all(&profile_dir)
        .map_err(|e| format!("failed to create profile dir: {e}"))?;

    // Build with instrumentation
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    for target in targets {
        cmd.args(["--bin", target]);
    }
    cmd.env("RUSTFLAGS", "-C instrument-coverage")
        .env(
            "LLVM_PROFILE_FILE",
            profile_dir
                .join("%p-%m.profraw")
                .to_string_lossy()
                .to_string(),
        )
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd
        .output()
        .map_err(|e| format!("failed to build instrumented binaries: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let truncated = truncate_lines(&stderr, 10);
        return Err(format!("instrumented build failed:\n{truncated}"));
    }

    // Locate built binaries
    let mut binaries = HashMap::new();
    for target in targets {
        let binary_path = root.join("target").join("debug").join(target);
        if binary_path.exists() {
            binaries.insert(target.clone(), binary_path);
        }
    }

    Ok(InstrumentedBuild {
        profile_dir,
        binaries,
    })
}

/// Get environment variables needed to run an instrumented binary.
///
/// These should be set when executing tests that use the instrumented binary
/// so that coverage profiles are written to the correct location.
pub fn coverage_env(build: &InstrumentedBuild) -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert(
        "LLVM_PROFILE_FILE".to_string(),
        build
            .profile_dir
            .join("%p-%m.profraw")
            .to_string_lossy()
            .to_string(),
    );
    env
}

/// Collect coverage from instrumented builds.
///
/// This merges `.profraw` files and generates a coverage report using llvm-cov.
pub fn collect_instrumented_coverage(build: &InstrumentedBuild, root: &Path) -> CoverageResult {
    let start = std::time::Instant::now();

    // Find all .profraw files
    let profraw_files: Vec<PathBuf> = std::fs::read_dir(&build.profile_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| {
                    let path = e.path();
                    if path.extension().is_some_and(|ext| ext == "profraw") {
                        Some(path)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    if profraw_files.is_empty() {
        return CoverageResult::failed(start.elapsed(), "no coverage profiles found");
    }

    // Merge profiles with llvm-profdata
    let merged_profdata = build.profile_dir.join("merged.profdata");
    let merge_result = merge_profiles(&profraw_files, &merged_profdata);
    if let Err(e) = merge_result {
        return CoverageResult::failed(start.elapsed(), e);
    }

    // Generate coverage report using llvm-cov
    // We use cargo llvm-cov report with the merged profdata
    let binaries: Vec<_> = build.binaries.values().collect();
    if binaries.is_empty() {
        return CoverageResult::failed(start.elapsed(), "no binaries to analyze");
    }

    // Use llvm-cov export to get JSON output
    let coverage_result = export_coverage(&merged_profdata, &binaries, root);
    let duration = start.elapsed();

    match coverage_result {
        Ok((line_coverage, files)) => CoverageResult {
            success: true,
            error: None,
            duration,
            line_coverage: Some(line_coverage),
            files,
            packages: std::collections::HashMap::new(),
        },
        Err(e) => CoverageResult::failed(duration, e),
    }
}

/// Merge .profraw files into a single .profdata file.
fn merge_profiles(profraw_files: &[PathBuf], output: &Path) -> Result<(), String> {
    // First try using cargo-llvm-cov's bundled llvm-profdata
    let mut cmd = Command::new("cargo");
    cmd.args(["llvm-cov", "show", "--help"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // If cargo llvm-cov is available, use its profdata merge
    let cargo_llvm_cov = cmd.status().is_ok_and(|s| s.success());

    if cargo_llvm_cov {
        // cargo llvm-cov doesn't expose profdata directly, so try llvm-profdata
        let mut merge_cmd = Command::new("llvm-profdata");
        merge_cmd.args(["merge", "-sparse"]);
        for file in profraw_files {
            merge_cmd.arg(file);
        }
        merge_cmd.args(["-o", &output.to_string_lossy()]);
        merge_cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output_result = merge_cmd.output();
        if let Ok(out) = output_result
            && out.status.success()
        {
            return Ok(());
        }
    }

    // Fallback: try system llvm-profdata
    let mut merge_cmd = Command::new("llvm-profdata");
    merge_cmd.args(["merge", "-sparse"]);
    for file in profraw_files {
        merge_cmd.arg(file);
    }
    merge_cmd.args(["-o", &output.to_string_lossy()]);
    merge_cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let output_result = merge_cmd
        .output()
        .map_err(|e| format!("failed to run llvm-profdata: {e}"))?;

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        return Err(format!(
            "llvm-profdata merge failed: {}",
            truncate_lines(&stderr, 5)
        ));
    }

    Ok(())
}

/// Export coverage data as JSON using llvm-cov.
fn export_coverage(
    profdata: &Path,
    binaries: &[&PathBuf],
    root: &Path,
) -> Result<(f64, HashMap<String, f64>), String> {
    // Try llvm-cov export
    let mut cmd = Command::new("llvm-cov");
    cmd.args(["export", "-format=text", "-summary-only"]);
    cmd.args(["-instr-profile", &profdata.to_string_lossy()]);

    // Add all binaries as objects
    for (i, binary) in binaries.iter().enumerate() {
        if i == 0 {
            cmd.arg(binary.to_string_lossy().to_string());
        } else {
            cmd.args(["-object", &*binary.to_string_lossy()]);
        }
    }

    cmd.current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run llvm-cov: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "llvm-cov export failed: {}",
            truncate_lines(&stderr, 5)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_llvm_cov_export(&stdout)
}

/// Parse llvm-cov export JSON output.
fn parse_llvm_cov_export(json: &str) -> Result<(f64, HashMap<String, f64>), String> {
    #[derive(serde::Deserialize)]
    struct LlvmCovExport {
        data: Vec<LlvmCovData>,
    }

    #[derive(serde::Deserialize)]
    struct LlvmCovData {
        totals: LlvmCovTotals,
        files: Option<Vec<LlvmCovFile>>,
    }

    #[derive(serde::Deserialize)]
    struct LlvmCovTotals {
        lines: LlvmCovLines,
    }

    #[derive(serde::Deserialize)]
    struct LlvmCovLines {
        percent: f64,
    }

    #[derive(serde::Deserialize)]
    struct LlvmCovFile {
        filename: String,
        summary: LlvmCovTotals,
    }

    let export: LlvmCovExport =
        serde_json::from_str(json).map_err(|e| format!("failed to parse llvm-cov output: {e}"))?;

    let data = export
        .data
        .first()
        .ok_or_else(|| "no coverage data in report".to_string())?;

    let line_coverage = data.totals.lines.percent;

    let mut files = HashMap::new();
    if let Some(file_data) = &data.files {
        for f in file_data {
            // Normalize path
            let path = normalize_path(&f.filename);
            files.insert(path, f.summary.lines.percent);
        }
    }

    Ok((line_coverage, files))
}

/// Normalize coverage paths to workspace-relative.
fn normalize_path(path: &str) -> String {
    // llvm-cov reports absolute paths; extract relative portion
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

/// Clean up coverage profiles after report generation.
///
/// Available for explicit cleanup after coverage collection if needed.
#[allow(dead_code)] // Utility for manual cleanup
pub(crate) fn cleanup_coverage_profiles(profile_dir: &Path) {
    if profile_dir.exists() {
        let _ = std::fs::remove_dir_all(profile_dir);
    }
}

/// Truncate text to first N lines.
fn truncate_lines(text: &str, max_lines: usize) -> String {
    text.lines().take(max_lines).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
#[path = "instrumented_tests.rs"]
mod tests;
