// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TOC (directory tree) validation.
//!
//! Validates that directory trees in markdown files reference existing files.

mod detect;
mod parse;
mod resolve;

use std::path::Path;

use crate::check::{CheckContext, Violation};

// Re-export functions used by specs.rs
pub(super) use detect::{is_valid_tree_format, looks_like_tree};
pub(super) use parse::{extract_fenced_blocks, parse_tree_block};

use detect::TOC_LANGUAGE;

use parse::TreeEntry;
use resolve::{try_resolve, try_resolve_block};

// Re-exports for toc_tests.rs which uses `use super::*`
// Required: tests use these types/functions directly
#[allow(unused_imports)]
use detect::looks_like_error_output;
// Required: tests construct FencedBlock for unit testing
#[allow(unused_imports)]
use parse::FencedBlock;
// Required: tests verify resolution strategies and glob patterns
#[allow(unused_imports)]
use resolve::{ResolutionStrategy, is_glob_pattern};

/// Validate TOC entries in all markdown files (parallel version).
pub fn validate_toc_parallel(ctx: &CheckContext, path_cache: &super::PathCache) -> Vec<Violation> {
    let config = &ctx.config.check.docs.toc;

    // Check if TOC validation is disabled
    if !super::is_check_enabled(
        config.check.as_deref(),
        ctx.config.check.docs.check.as_deref(),
    ) {
        return Vec::new();
    }

    super::process_markdown_files_parallel(
        ctx,
        &config.include,
        &config.exclude,
        path_cache,
        validate_file_toc_cached,
    )
}

/// Validate TOC entries in a single file (uses path cache).
fn validate_file_toc_cached(
    ctx: &CheckContext,
    relative_path: &Path,
    content: &str,
    _path_cache: &super::PathCache,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let blocks = extract_fenced_blocks(content);
    let strategies = [
        ResolutionStrategy::RelativeToFile,
        ResolutionStrategy::RelativeToRoot,
        ResolutionStrategy::StripParentDirName,
    ];

    for block in blocks {
        // For explicit toc tag, validate format
        if block.language.as_deref() == Some(TOC_LANGUAGE) && !is_valid_tree_format(&block) {
            violations.push(Violation::file(
                relative_path,
                block.start_line,
                "invalid_toc_format",
                "Code block marked as `toc` doesn't match box-drawing or indentation format.\n\
                 Use box-drawing (├──, └──, │) or consistent indentation.",
            ));
            continue;
        }

        // Skip blocks that don't look like directory trees
        if !looks_like_tree(&block) {
            continue;
        }

        let entries = parse_tree_block(&block);
        let abs_file = ctx.root.join(relative_path);
        let file_entries: Vec<_> = entries.iter().filter(|e| !e.is_dir).collect();

        // Find entries that fail ALL strategies (truly broken)
        let resolves_any = |e: &&TreeEntry| {
            strategies
                .iter()
                .any(|&s| try_resolve(ctx.root, &abs_file, &e.path, s))
        };
        let failed_all: Vec<_> = file_entries.iter().filter(|e| !resolves_any(e)).collect();

        let total = file_entries.len();
        let (to_report, advice) = if !failed_all.is_empty() {
            let tried: Vec<_> = strategies.iter().map(|s| s.description()).collect();
            let (valid, failed) = (total - failed_all.len(), failed_all.len());
            (
                failed_all.into_iter().copied().collect(),
                format!(
                    "File does not exist ({valid} of {total} paths valid, {failed} failed).\n\
                     This check ensures directory trees in documentation stay up-to-date.\n\
                     Update the table of contents or directory tree to match actual files.\n\
                     If this is illustrative, add a ```no-toc language tag.\n\nTried: {}",
                    tried.join(", ")
                ),
            )
        } else {
            // Check if any single strategy resolves all entries
            let mut best: Option<(ResolutionStrategy, Vec<&TreeEntry>)> = None;
            for strategy in strategies {
                match try_resolve_block(ctx.root, &abs_file, &entries, strategy) {
                    None => {
                        best = None;
                        break;
                    }
                    Some(f) if best.as_ref().is_none_or(|(_, b)| f.len() < b.len()) => {
                        best = Some((strategy, f));
                    }
                    _ => {}
                }
            }
            let Some((strategy, failed)) = best else {
                continue;
            };
            let (valid, failed_count) = (total - failed.len(), failed.len());
            (
                failed,
                format!(
                    "File does not exist ({valid} of {total} paths valid, {failed_count} failed).\n\
                     This check ensures directory trees in documentation stay up-to-date.\n\
                     TOC entries should use a consistent path style (resolving {}).\n\
                     If this is illustrative, add a ```no-toc language tag.",
                    strategy.description()
                ),
            )
        };

        for entry in to_report {
            violations.push(
                Violation::file(
                    relative_path,
                    block.start_line + entry.line_offset,
                    "broken_toc",
                    &advice,
                )
                .with_path(entry.path.clone()),
            );
        }
    }
    violations
}

#[cfg(test)]
#[path = "../toc_tests.rs"]
mod tests;
