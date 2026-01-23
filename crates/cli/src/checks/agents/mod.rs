// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Agents check for AI agent context files.
//!
//! Validates CLAUDE.md, .cursorrules, and other agent context files.
//! This phase implements:
//! - File detection at root, package, and module scopes
//! - Required/optional/forbid file validation
//! - Basic metrics output

pub mod config;
mod detection;
mod sync;

use serde_json::json;

use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::CheckLevel;

pub use config::AgentsConfig;
use detection::{DetectedFile, Scope, detect_agent_files, file_exists_at_root};
use sync::{DiffType, compare_files};

/// The agents check validates AI agent context files.
pub struct AgentsCheck;

impl Check for AgentsCheck {
    fn name(&self) -> &'static str {
        "agents"
    }

    fn description(&self) -> &'static str {
        "Agent file validation"
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        let config = &ctx.config.check.agents;

        // Skip if disabled
        if config.check == CheckLevel::Off {
            return CheckResult::passed(self.name());
        }

        let packages = &ctx.config.workspace.packages;

        // Detect all agent files
        let detected = detect_agent_files(ctx.root, packages, &config.files);

        let mut violations = Vec::new();
        let mut files_missing: Vec<String> = Vec::new();

        // Check required files exist at root
        check_required_files(ctx, config, &mut violations, &mut files_missing);

        // Check forbidden files don't exist at root
        check_forbidden_files(ctx, config, &detected, &mut violations);

        // Check sync if enabled
        let in_sync = if config.sync {
            check_sync(ctx, config, &detected, &mut violations)
        } else {
            true
        };

        // Build metrics
        let files_found: Vec<String> = detected
            .iter()
            .map(|f| {
                f.path
                    .strip_prefix(ctx.root)
                    .unwrap_or(&f.path)
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        let metrics = json!({
            "files_found": files_found,
            "files_missing": files_missing,
            "in_sync": in_sync,
        });

        let result = if violations.is_empty() {
            CheckResult::passed(self.name())
        } else {
            CheckResult::failed(self.name(), violations)
        };

        result.with_metrics(metrics)
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

/// Check that required files exist.
fn check_required_files(
    ctx: &CheckContext,
    config: &AgentsConfig,
    violations: &mut Vec<Violation>,
    files_missing: &mut Vec<String>,
) {
    // Get effective requirements for root scope
    let required = if let Some(ref root) = config.root {
        &root.required
    } else {
        &config.required
    };

    for filename in required {
        if !file_exists_at_root(ctx.root, filename) {
            files_missing.push(filename.clone());
            violations.push(Violation::file_only(
                filename,
                "missing_file",
                format!(
                    "Required agent file '{}' not found at project root",
                    filename
                ),
            ));
        }
    }
}

/// Check that forbidden files don't exist.
fn check_forbidden_files(
    ctx: &CheckContext,
    config: &AgentsConfig,
    detected: &[detection::DetectedFile],
    violations: &mut Vec<Violation>,
) {
    // Get effective forbid list for root scope
    let forbid = if let Some(ref root) = config.root {
        &root.forbid
    } else {
        &config.forbid
    };

    for filename in forbid {
        // Check if this forbidden file was detected at root scope
        let found_at_root = detected.iter().any(|f| {
            f.scope == Scope::Root
                && f.path
                    .file_name()
                    .map(|n| n.to_string_lossy() == *filename)
                    .unwrap_or(false)
        });

        // Also do a direct check in case it wasn't in the detection patterns
        let exists_at_root = file_exists_at_root(ctx.root, filename);

        if found_at_root || exists_at_root {
            violations.push(Violation::file_only(
                filename,
                "forbidden_file",
                format!("Forbidden agent file '{}' exists at project root", filename),
            ));
        }
    }
}

/// Check synchronization between agent files.
fn check_sync(
    ctx: &CheckContext,
    config: &AgentsConfig,
    detected: &[DetectedFile],
    violations: &mut Vec<Violation>,
) -> bool {
    // Get root-scope files only
    let root_files: Vec<_> = detected.iter().filter(|f| f.scope == Scope::Root).collect();

    if root_files.len() < 2 {
        // Nothing to sync
        return true;
    }

    // Determine sync source (first in files list, or explicit sync_source)
    let source_name = config
        .sync_source
        .as_ref()
        .or_else(|| config.files.first())
        .map(|s| s.as_str());

    let Some(source_name) = source_name else {
        return true;
    };

    // Find source file in detected
    let source_file = root_files.iter().find(|f| {
        f.path
            .file_name()
            .map(|n| n.to_string_lossy() == source_name)
            .unwrap_or(false)
    });

    let Some(source_file) = source_file else {
        return true; // Source not present, nothing to sync
    };

    // Read source content
    let Ok(source_content) = std::fs::read_to_string(&source_file.path) else {
        return true; // Can't read source
    };

    let mut all_in_sync = true;

    // Compare against each other root file
    for target_file in &root_files {
        if target_file.path == source_file.path {
            continue;
        }

        let Ok(target_content) = std::fs::read_to_string(&target_file.path) else {
            continue;
        };

        let comparison = compare_files(&source_content, &target_content);

        if !comparison.in_sync {
            // If fix mode is enabled, sync the target file from source
            if ctx.fix && std::fs::write(&target_file.path, &source_content).is_ok() {
                // File was fixed, no violation needed
                continue;
            }

            all_in_sync = false;

            let target_name = target_file
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| target_file.path.display().to_string());

            for diff in comparison.differences {
                let advice = match diff.diff_type {
                    DiffType::ContentDiffers => format!(
                        "Section \"{}\" differs. Use --fix to sync from {}, or reconcile manually.",
                        diff.source_heading.as_deref().unwrap_or(&diff.section),
                        source_name
                    ),
                    DiffType::MissingInTarget => format!(
                        "Section \"{}\" missing in {}. Use --fix to sync from {}.",
                        diff.source_heading.as_deref().unwrap_or(&diff.section),
                        target_name,
                        source_name
                    ),
                    DiffType::ExtraInTarget => format!(
                        "Section \"{}\" exists in {} but not in {}. Remove or add to source.",
                        diff.target_heading.as_deref().unwrap_or(&diff.section),
                        target_name,
                        source_name
                    ),
                };

                let violation = Violation::file_only(&target_name, "out_of_sync", advice)
                    .with_sync(
                        source_name,
                        if diff.section.is_empty() {
                            "(preamble)"
                        } else {
                            &diff.section
                        },
                    );

                violations.push(violation);
            }
        }
    }

    all_in_sync
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
