// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Coverage target resolution.
//!
//! Resolves target strings to concrete coverage collection strategies:
//! - Build target names (no glob characters) → Rust binary via llvm-cov
//! - Glob patterns → Shell scripts via kcov

use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher};
use serde::Deserialize;

use crate::config::Config;

/// Resolved coverage target with collection strategy.
#[derive(Debug, Clone)]
pub enum ResolvedTarget {
    /// Rust binary target, collected via instrumented build + llvm-cov.
    RustBinary {
        /// Binary target name.
        name: String,
        /// Path to binary (determined during build).
        binary_path: Option<PathBuf>,
    },
    /// Shell scripts, collected via kcov.
    ShellScripts {
        /// Original glob pattern.
        pattern: String,
        /// Resolved file paths.
        files: Vec<PathBuf>,
    },
}

/// Error returned when target resolution fails.
#[derive(Debug, Clone)]
pub struct TargetResolutionError {
    /// The target string that failed to resolve.
    pub target: String,
    /// Error message.
    pub message: String,
}

impl std::fmt::Display for TargetResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.target, self.message)
    }
}

/// Resolve a target string to a concrete coverage target.
///
/// Target resolution rules:
/// 1. Build target name (no glob characters): Look up in Cargo.toml `[[bin]]` entries
/// 2. Glob pattern (contains `*`, `?`, `[`): Match against shell source patterns
pub fn resolve_target(
    target: &str,
    config: &Config,
    root: &Path,
) -> Result<ResolvedTarget, TargetResolutionError> {
    if is_glob_pattern(target) {
        resolve_shell_pattern(target, config, root)
    } else {
        resolve_rust_binary(target, root)
    }
}

/// Resolve multiple targets.
pub fn resolve_targets(
    targets: &[String],
    config: &Config,
    root: &Path,
) -> Result<Vec<ResolvedTarget>, TargetResolutionError> {
    targets.iter().map(|t| resolve_target(t, config, root)).collect()
}

/// Check if a string contains glob pattern characters.
pub fn is_glob_pattern(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

/// Resolve a Rust binary target.
///
/// Resolution order:
/// 1. Parse Cargo.toml for `[[bin]]` entries
/// 2. Check default binary (package name)
fn resolve_rust_binary(name: &str, root: &Path) -> Result<ResolvedTarget, TargetResolutionError> {
    let cargo_path = root.join("Cargo.toml");

    if cargo_path.exists() {
        match std::fs::read_to_string(&cargo_path) {
            Ok(content) => {
                if let Ok(cargo) = toml::from_str::<CargoToml>(&content) {
                    // Check [[bin]] entries
                    if let Some(bins) = &cargo.bin
                        && bins.iter().any(|b| b.name == name)
                    {
                        return Ok(ResolvedTarget::RustBinary {
                            name: name.to_string(),
                            binary_path: None,
                        });
                    }

                    // Check package name (default binary)
                    if let Some(pkg) = &cargo.package
                        && pkg.name == name
                    {
                        return Ok(ResolvedTarget::RustBinary {
                            name: name.to_string(),
                            binary_path: None,
                        });
                    }
                }
            }
            Err(_) => {
                // Cargo.toml exists but couldn't be read
            }
        }
    }

    Err(TargetResolutionError {
        target: name.to_string(),
        message: "not a Rust binary or glob pattern".to_string(),
    })
}

/// Resolve a shell script glob pattern.
fn resolve_shell_pattern(
    pattern: &str,
    config: &Config,
    root: &Path,
) -> Result<ResolvedTarget, TargetResolutionError> {
    // Get shell source patterns
    let source_patterns = &config.shell.source;

    // Build glob matcher for the target pattern
    let target_glob = Glob::new(pattern).map_err(|e| TargetResolutionError {
        target: pattern.to_string(),
        message: format!("invalid glob pattern: {e}"),
    })?;
    let target_matcher = target_glob.compile_matcher();

    // Build matchers for shell source patterns
    let source_matchers: Vec<GlobMatcher> = source_patterns
        .iter()
        .filter_map(|p| Glob::new(p).ok().map(|g| g.compile_matcher()))
        .collect();

    // Walk the directory and find matching files
    let files = find_matching_files(root, &target_matcher, &source_matchers);

    if files.is_empty() {
        return Err(TargetResolutionError {
            target: pattern.to_string(),
            message: "no shell scripts match pattern".to_string(),
        });
    }

    Ok(ResolvedTarget::ShellScripts { pattern: pattern.to_string(), files })
}

/// Find files matching both the target pattern and shell source patterns.
fn find_matching_files(
    root: &Path,
    target_matcher: &GlobMatcher,
    source_matchers: &[GlobMatcher],
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_dir(root, &mut |path| {
        let rel_path = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel_path.to_string_lossy();

        // Must match target pattern
        if !target_matcher.is_match(&*rel_str) {
            return;
        }

        // Must match at least one shell source pattern
        if source_matchers.iter().any(|m| m.is_match(&*rel_str)) {
            files.push(path.to_path_buf());
        }
    });
    files
}

/// Simple recursive directory walker.
fn walk_dir(dir: &Path, callback: &mut impl FnMut(&Path)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip hidden directories and common ignored directories
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }
            walk_dir(&path, callback);
        } else if path.is_file() {
            callback(&path);
        }
    }
}

/// Get all Rust binary target names from resolved targets.
pub fn rust_binary_names(targets: &[ResolvedTarget]) -> Vec<String> {
    targets
        .iter()
        .filter_map(|t| match t {
            ResolvedTarget::RustBinary { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

/// Get all shell script files from resolved targets.
pub fn shell_script_files(targets: &[ResolvedTarget]) -> Vec<PathBuf> {
    targets
        .iter()
        .filter_map(|t| match t {
            ResolvedTarget::ShellScripts { files, .. } => Some(files.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

// =============================================================================
// Cargo.toml Parsing
// =============================================================================

/// Minimal Cargo.toml parsing for binary detection.
#[derive(Debug, Deserialize)]
struct CargoToml {
    package: Option<CargoPackage>,
    bin: Option<Vec<CargoBin>>,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CargoBin {
    name: String,
}

#[cfg(test)]
#[path = "targets_tests.rs"]
mod tests;
