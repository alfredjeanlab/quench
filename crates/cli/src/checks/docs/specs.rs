// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Specs directory validation.
//!
//! Validates that specs directories have proper index files.

use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{content, links, toc};
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
    collect_spec_files(root, specs_path, extension).len()
}

/// Collect all spec files in the specs directory.
///
/// Returns canonicalized paths for consistent comparison.
fn collect_spec_files(root: &Path, specs_path: &str, extension: &str) -> HashSet<PathBuf> {
    let specs_dir = root.join(specs_path);
    if !specs_dir.exists() || !specs_dir.is_dir() {
        return HashSet::new();
    }

    ignore::WalkBuilder::new(&specs_dir)
        .build()
        .flatten()
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .filter(|e| matches_extension(e.path(), extension))
        .filter_map(|e| e.into_path().canonicalize().ok())
        .collect()
}

/// Validate specs using TOC mode.
///
/// Parses directory trees in the index file and verifies all spec files
/// are referenced in a tree block.
fn validate_toc_mode(
    root: &Path,
    index_file: &Path,
    specs_dir: &Path,
    all_specs: &HashSet<PathBuf>,
    violations: &mut Vec<Violation>,
    limit: Option<usize>,
) {
    // Canonicalize root for path comparison
    let canonical_root = match root.canonicalize() {
        Ok(r) => r,
        Err(_) => return,
    };

    // Read index file content
    let abs_index = root.join(index_file);
    let content = match fs::read_to_string(&abs_index) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Extract tree blocks and collect referenced paths
    let blocks = toc::extract_fenced_blocks(&content);
    let mut reachable: HashSet<PathBuf> = HashSet::new();

    for block in &blocks {
        if !toc::looks_like_tree(block) {
            continue;
        }

        let entries = toc::parse_tree_block(block);
        for entry in entries {
            if entry.is_dir {
                continue;
            }

            // Tree entries are typically relative to the specs directory
            // e.g., "docs/specs/00-overview.md" or just "00-overview.md"
            let resolved = if entry.path.starts_with("docs/specs/") {
                // Full path from root
                root.join(&entry.path)
            } else if entry.path.contains('/') {
                // Might be relative to project root
                let from_root = root.join(&entry.path);
                if from_root.exists() {
                    from_root
                } else {
                    specs_dir.join(&entry.path)
                }
            } else {
                // Just filename, relative to specs dir
                specs_dir.join(&entry.path)
            };

            if resolved.exists()
                && let Ok(canonical) = resolved.canonicalize()
                && all_specs.contains(&canonical)
            {
                reachable.insert(canonical);
            }
        }
    }

    // Generate violations for unreachable specs
    for spec in all_specs.difference(&reachable) {
        if limit.is_some_and(|l| violations.len() >= l) {
            break;
        }
        let rel_path = spec.strip_prefix(&canonical_root).unwrap_or(spec);
        violations.push(Violation::file_only(
            rel_path,
            "unreachable_spec",
            "Spec file not referenced in index directory tree.\n\
             Add this file to the TOC in the index file.",
        ));
    }
}

/// Validate specs using linked mode.
///
/// Traces markdown links starting from the index file using BFS to find
/// all reachable spec files.
fn validate_linked_mode(
    root: &Path,
    index_file: &Path,
    specs_dir: &Path,
    all_specs: &HashSet<PathBuf>,
    violations: &mut Vec<Violation>,
    limit: Option<usize>,
) {
    // Canonicalize root and specs_dir for path comparison
    let canonical_root = match root.canonicalize() {
        Ok(r) => r,
        Err(_) => return,
    };
    let canonical_specs_dir = match specs_dir.canonicalize() {
        Ok(s) => s,
        Err(_) => return,
    };
    let abs_index = match root.join(index_file).canonicalize() {
        Ok(i) => i,
        Err(_) => return,
    };

    // BFS from index file
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    let mut reachable: HashSet<PathBuf> = HashSet::new();

    queue.push_back(abs_index.clone());
    visited.insert(abs_index);

    while let Some(current) = queue.pop_front() {
        let content = match fs::read_to_string(&current) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Extract links
        let extracted = links::extract_links(&content);
        for link in extracted {
            // Skip external links
            if !links::is_local_link(&link.target) {
                continue;
            }

            // Resolve relative to current file
            let resolved = links::resolve_link(&current, &link.target);

            // Canonicalize to handle .. and symlinks
            let canonical = match resolved.canonicalize() {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Track if it's a spec file
            if all_specs.contains(&canonical) {
                reachable.insert(canonical.clone());
            }

            // Queue markdown files for traversal if not yet visited
            // Only follow links within the specs directory
            if canonical.extension().is_some_and(|e| e == "md")
                && canonical.starts_with(&canonical_specs_dir)
                && visited.insert(canonical.clone())
            {
                queue.push_back(canonical);
            }
        }
    }

    // Generate violations for unreachable specs
    for spec in all_specs.difference(&reachable) {
        if limit.is_some_and(|l| violations.len() >= l) {
            break;
        }
        let rel_path = spec.strip_prefix(&canonical_root).unwrap_or(spec);
        violations.push(Violation::file_only(
            rel_path,
            "unreachable_spec",
            "Spec file unreachable from index via markdown links.\n\
             Add a link to this file from the index or another linked spec.",
        ));
    }
}

/// Validate specs using auto mode.
///
/// Tries TOC mode first if tree blocks exist, otherwise falls back to linked mode.
fn validate_auto_mode(
    root: &Path,
    index_file: &Path,
    specs_dir: &Path,
    all_specs: &HashSet<PathBuf>,
    violations: &mut Vec<Violation>,
    limit: Option<usize>,
) {
    // Read index file to check for tree blocks
    let abs_index = root.join(index_file);
    let content = match fs::read_to_string(&abs_index) {
        Ok(c) => c,
        Err(_) => return,
    };

    let blocks = toc::extract_fenced_blocks(&content);
    let has_trees = blocks.iter().any(toc::looks_like_tree);

    if has_trees {
        validate_toc_mode(root, index_file, specs_dir, all_specs, violations, limit);
    } else {
        validate_linked_mode(root, index_file, specs_dir, all_specs, violations, limit);
    }
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

    // Check for missing index file (required for all modes)
    let Some(index_file) = index_file else {
        violations.push(Violation::file_only(
            specs_path,
            "missing_index",
            "Specs directory has no index file.\n\
             Create CLAUDE.md, overview.md, or index.md in the specs directory.",
        ));
        return;
    };

    // Validate based on mode
    match config.index.as_str() {
        "exists" => {
            // Index exists - nothing more to check
        }
        "toc" => {
            let mut all_specs = collect_spec_files(ctx.root, specs_path, &config.extension);
            // Exclude the index file from spec files (it's the starting point, not a spec to validate)
            if let Ok(canonical_index) = ctx.root.join(&index_file).canonicalize() {
                all_specs.remove(&canonical_index);
            }
            validate_toc_mode(
                ctx.root,
                &index_file,
                &specs_dir,
                &all_specs,
                violations,
                ctx.limit,
            );
        }
        "linked" => {
            let mut all_specs = collect_spec_files(ctx.root, specs_path, &config.extension);
            // Exclude the index file from spec files (it's the starting point, not a spec to validate)
            if let Ok(canonical_index) = ctx.root.join(&index_file).canonicalize() {
                all_specs.remove(&canonical_index);
            }
            validate_linked_mode(
                ctx.root,
                &index_file,
                &specs_dir,
                &all_specs,
                violations,
                ctx.limit,
            );
        }
        "auto" => {
            let mut all_specs = collect_spec_files(ctx.root, specs_path, &config.extension);
            // Exclude the index file from spec files (it's the starting point, not a spec to validate)
            if let Ok(canonical_index) = ctx.root.join(&index_file).canonicalize() {
                all_specs.remove(&canonical_index);
            }
            validate_auto_mode(
                ctx.root,
                &index_file,
                &specs_dir,
                &all_specs,
                violations,
                ctx.limit,
            );
        }
        _ => {
            // Unknown mode - treat as exists (no additional validation)
        }
    }

    // Content validation for all spec files
    let all_specs = collect_spec_files(ctx.root, specs_path, &config.extension);
    if !all_specs.is_empty() {
        validate_specs_content(ctx, &all_specs, violations);
    }
}

/// Validate content of all spec files.
fn validate_specs_content(
    ctx: &CheckContext,
    specs: &HashSet<PathBuf>,
    violations: &mut Vec<Violation>,
) {
    let config = &ctx.config.check.docs.specs;
    let canonical_root = match ctx.root.canonicalize() {
        Ok(r) => r,
        Err(_) => return,
    };

    for spec_path in specs {
        if ctx.limit.is_some_and(|l| violations.len() >= l) {
            break;
        }

        let file_content = match fs::read_to_string(spec_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let rel_path = spec_path.strip_prefix(&canonical_root).unwrap_or(spec_path);
        let file_violations = content::validate_spec_content(rel_path, &file_content, config);

        for v in file_violations {
            if ctx.limit.is_some_and(|l| violations.len() >= l) {
                break;
            }
            violations.push(v);
        }
    }
}

/// Specs metrics for reporting.
#[derive(Debug, Serialize)]
pub struct SpecsMetrics {
    /// Detected or configured index file path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_file: Option<String>,
    /// Number of spec files found.
    pub spec_files: usize,
}

/// Collect specs metrics for reporting.
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
