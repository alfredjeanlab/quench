// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Specs directory validation.
//!
//! Validates that specs directories have proper index files.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::check::{CheckContext, Violation};

/// Index file detection candidates in priority order.
const INDEX_CANDIDATES: &[IndexCandidate] = &[
    IndexCandidate::InPath("CLAUDE.md"),
    IndexCandidate::Fixed("docs/CLAUDE.md"),
    IndexCandidate::InPath("00-overview.md"),
    IndexCandidate::InPath("overview.md"),
    IndexCandidate::InPath("00-summary.md"),
    IndexCandidate::InPath("summary.md"),
    IndexCandidate::InPath("00-index.md"),
    IndexCandidate::InPath("index.md"),
    IndexCandidate::Fixed("docs/SPECIFICATIONS.md"),
    IndexCandidate::Fixed("docs/SPECS.md"),
];

/// Index file candidate type.
enum IndexCandidate {
    /// Relative to configured path (e.g., "{path}/CLAUDE.md").
    InPath(&'static str),
    /// Fixed path from project root.
    Fixed(&'static str),
}

/// Detect index file using priority order.
fn detect_index_file(root: &Path, specs_path: &str) -> Option<PathBuf> {
    for candidate in INDEX_CANDIDATES {
        let path = match candidate {
            IndexCandidate::InPath(name) => root.join(specs_path).join(name),
            IndexCandidate::Fixed(path) => root.join(path),
        };
        if path.exists() && path.is_file() {
            return Some(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    None
}

/// Check if a file matches the configured extension.
// KEEP UNTIL: Phase 616+ uses for spec file filtering
#[allow(dead_code)]
fn matches_extension(path: &Path, extension: &str) -> bool {
    // Handle both ".md" and "md" formats
    let ext = extension.trim_start_matches('.');
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(ext))
        .unwrap_or(false)
}

/// Count spec files in the directory.
// KEEP UNTIL: Phase 616+ uses for metrics reporting
#[allow(dead_code)]
fn count_spec_files(root: &Path, specs_path: &str, extension: &str) -> usize {
    let specs_dir = root.join(specs_path);
    if !specs_dir.exists() || !specs_dir.is_dir() {
        return 0;
    }

    ignore::WalkBuilder::new(&specs_dir)
        .build()
        .flatten()
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .filter(|e| matches_extension(e.path(), extension))
        .count()
}

/// Validate specs directory.
pub fn validate_specs(ctx: &CheckContext, violations: &mut Vec<Violation>) {
    let config = &ctx.config.check.docs.specs;

    // Check if specs validation is disabled
    let check_level = config
        .check
        .as_deref()
        .or(ctx.config.check.docs.check.as_deref())
        .unwrap_or("error");
    if check_level == "off" {
        return;
    }

    let specs_path = &config.path;
    let specs_dir = ctx.root.join(specs_path);

    // Skip if specs directory doesn't exist (not an error - project may not use specs)
    if !specs_dir.exists() || !specs_dir.is_dir() {
        return;
    }

    // Detect or use configured index file
    let index_file = config
        .index_file
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| detect_index_file(ctx.root, specs_path));

    // Validate based on mode
    match config.index.as_str() {
        "exists" | "auto" => {
            // Check that index file exists
            if index_file.is_none() {
                violations.push(Violation::file_only(
                    specs_path,
                    "missing_index",
                    "Specs directory has no index file.\n\
                     Create CLAUDE.md, overview.md, or index.md in the specs directory.",
                ));
            }
        }
        "toc" | "linked" => {
            // Future phases - fall back to exists mode for now
            if index_file.is_none() {
                violations.push(Violation::file_only(
                    specs_path,
                    "missing_index",
                    "Specs directory has no index file.\n\
                     Create CLAUDE.md, overview.md, or index.md in the specs directory.",
                ));
            }
        }
        _ => {
            // Unknown mode - treat as exists
            if index_file.is_none() {
                violations.push(Violation::file_only(
                    specs_path,
                    "missing_index",
                    "Specs directory has no index file.\n\
                     Create CLAUDE.md, overview.md, or index.md in the specs directory.",
                ));
            }
        }
    }
}

/// Specs metrics for reporting.
// KEEP UNTIL: Phase 616+ adds metrics to JSON output
#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct SpecsMetrics {
    /// Detected or configured index file path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_file: Option<String>,
    /// Number of spec files found.
    pub spec_files: usize,
}

/// Collect specs metrics for reporting.
// KEEP UNTIL: Phase 616+ adds metrics to JSON output
#[allow(dead_code)]
pub fn collect_metrics(ctx: &CheckContext) -> Option<SpecsMetrics> {
    let config = &ctx.config.check.docs.specs;
    let specs_dir = ctx.root.join(&config.path);

    if !specs_dir.exists() {
        return None;
    }

    let index_file = config
        .index_file
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| detect_index_file(ctx.root, &config.path));

    Some(SpecsMetrics {
        index_file: index_file.map(|p| p.to_string_lossy().to_string()),
        spec_files: count_spec_files(ctx.root, &config.path, &config.extension),
    })
}

#[cfg(test)]
#[path = "specs_tests.rs"]
mod tests;
