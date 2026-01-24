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
mod content;
mod detection;
mod sections;
mod sync;

use serde_json::json;

use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::CheckLevel;

pub use config::AgentsConfig;
use config::ContentRule;
use content::{
    check_line_count, check_token_count, detect_box_diagrams, detect_mermaid_blocks, detect_tables,
};
use detection::{DetectedFile, Scope, detect_agent_files, file_exists_at_root};
use sections::validate_sections;
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
        let mut fixes = FixSummary::default();

        // Check required files exist at root
        check_required_files(ctx, config, &detected, &mut violations, &mut files_missing);

        // Check forbidden files don't exist at root
        check_forbidden_files(ctx, config, &detected, &mut violations);

        // Check sync if enabled
        let in_sync = if config.sync {
            check_sync(ctx, config, &detected, &mut violations, &mut fixes)
        } else {
            true
        };

        // Check sections in each detected file
        check_sections(ctx, config, &detected, &mut violations);

        // Check content rules (tables, diagrams, size limits)
        check_content(config, &detected, &mut violations);

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

        // Update in_sync metric based on whether we fixed things
        let final_in_sync = in_sync || !fixes.is_empty();

        let metrics = json!({
            "files_found": files_found,
            "files_missing": files_missing,
            "in_sync": final_in_sync,
        });

        // Determine result based on violations and fixes
        let result = if violations.is_empty() {
            if !fixes.is_empty() {
                // Fixes were applied and no remaining violations
                CheckResult::fixed(self.name(), fixes.to_json())
            } else {
                CheckResult::passed(self.name())
            }
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
    detected: &[DetectedFile],
    violations: &mut Vec<Violation>,
    files_missing: &mut Vec<String>,
) {
    // Get effective requirements for root scope
    let required = if let Some(ref root) = config.root {
        &root.required
    } else {
        &config.required
    };

    // Count root-scope detected files
    let root_files: Vec<_> = detected.iter().filter(|f| f.scope == Scope::Root).collect();

    for filename in required {
        if filename == "*" {
            // Wildcard: at least one agent file must exist at root
            if root_files.is_empty() {
                files_missing.push("(any agent file)".to_string());
                violations.push(Violation::file_only(
                    "(project root)",
                    "missing_file",
                    "No agent context file found. Create CLAUDE.md or .cursorrules at project root."
                        .to_string(),
                ));
            }
        } else if !file_exists_at_root(ctx.root, filename) {
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

/// Track fixes applied during check execution.
///
/// Uses separate types for real syncs vs previews because:
/// - They serve different purposes (actual changes vs proposed changes)
/// - They're mutually exclusive (fix mode populates `files_synced`, dry-run populates `previews`)
/// - SyncPreview needs content for diff display, SyncedFile doesn't
/// - Separate JSON arrays (`files_synced` vs `previews`) match their semantic purpose
#[derive(Debug, Default)]
struct FixSummary {
    files_synced: Vec<SyncedFile>,
    previews: Vec<SyncPreview>,
}

/// A file that was synced during fix mode.
#[derive(Debug)]
struct SyncedFile {
    file: String,
    source: String,
    sections: usize,
}

/// A preview of what would be synced in dry-run mode.
#[derive(Debug)]
struct SyncPreview {
    file: String,
    source: String,
    old_content: String,
    new_content: String,
    sections: usize,
}

impl FixSummary {
    fn add_sync(&mut self, file: String, source: String, sections: usize) {
        self.files_synced.push(SyncedFile {
            file,
            source,
            sections,
        });
    }

    fn add_preview(
        &mut self,
        file: String,
        source: String,
        old_content: String,
        new_content: String,
        sections: usize,
    ) {
        self.previews.push(SyncPreview {
            file,
            source,
            old_content,
            new_content,
            sections,
        });
    }

    fn is_empty(&self) -> bool {
        self.files_synced.is_empty() && self.previews.is_empty()
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "files_synced": self.files_synced.iter().map(|f| {
                json!({
                    "file": f.file,
                    "source": f.source,
                    "sections": f.sections,
                })
            }).collect::<Vec<_>>(),
            "previews": self.previews.iter().map(|p| {
                json!({
                    "file": p.file,
                    "source": p.source,
                    "old_content": p.old_content,
                    "new_content": p.new_content,
                    "sections": p.sections,
                })
            }).collect::<Vec<_>>()
        })
    }
}

/// Check synchronization between agent files.
fn check_sync(
    ctx: &CheckContext,
    config: &AgentsConfig,
    detected: &[DetectedFile],
    violations: &mut Vec<Violation>,
    fixes: &mut FixSummary,
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
            let target_name = target_file
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| target_file.path.display().to_string());

            // If fix mode is enabled, sync the target file from source
            if ctx.fix {
                let section_count = comparison.differences.len();

                if ctx.dry_run {
                    // Preview only: collect diff data without writing
                    fixes.add_preview(
                        target_name.clone(),
                        source_name.to_string(),
                        target_content.clone(),
                        source_content.clone(),
                        section_count,
                    );
                    continue;
                } else if std::fs::write(&target_file.path, &source_content).is_ok() {
                    // Actual fix: write and track
                    fixes.add_sync(target_name, source_name.to_string(), section_count);
                    continue;
                }
            }

            all_in_sync = false;

            for diff in comparison.differences {
                // Use heading if available, otherwise section name, "(preamble)" for empty
                let section_display = |heading: Option<&str>, section: &str| -> String {
                    if let Some(h) = heading {
                        if h.is_empty() {
                            "(preamble)".to_string()
                        } else {
                            h.to_string()
                        }
                    } else if section.is_empty() {
                        "(preamble)".to_string()
                    } else {
                        section.to_string()
                    }
                };

                let advice = match diff.diff_type {
                    DiffType::ContentDiffers => format!(
                        "{} differs. Use --fix to sync from {}, or reconcile manually.",
                        section_display(diff.source_heading.as_deref(), &diff.section),
                        source_name
                    ),
                    DiffType::MissingInTarget => format!(
                        "{} missing in {}. Use --fix to sync from {}.",
                        section_display(diff.source_heading.as_deref(), &diff.section),
                        target_name,
                        source_name
                    ),
                    DiffType::ExtraInTarget => format!(
                        "{} exists in {} but not in {}. Remove or add to source.",
                        section_display(diff.target_heading.as_deref(), &diff.section),
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

/// Check section requirements in agent files.
fn check_sections(
    _ctx: &CheckContext,
    config: &AgentsConfig,
    detected: &[DetectedFile],
    violations: &mut Vec<Violation>,
) {
    // Skip if no section requirements configured
    if config.sections.required.is_empty() && config.sections.forbid.is_empty() {
        return;
    }

    // Only check files at root scope for now
    let root_files: Vec<_> = detected.iter().filter(|f| f.scope == Scope::Root).collect();

    for file in root_files {
        let Ok(content) = std::fs::read_to_string(&file.path) else {
            continue;
        };

        let validation = validate_sections(&content, &config.sections);
        let filename = file
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Generate violations for missing required sections
        for missing in validation.missing {
            let advice = if let Some(ref section_advice) = missing.advice {
                format!("Add a \"## {}\" section: {}", missing.name, section_advice)
            } else {
                format!("Add a \"## {}\" section", missing.name)
            };

            violations.push(Violation::file_only(&filename, "missing_section", advice));
        }

        // Generate violations for forbidden sections
        for forbidden in validation.forbidden {
            let advice = format!(
                "Remove or rename the \"{}\" section (matches forbidden pattern \"{}\")",
                forbidden.heading, forbidden.matched_pattern
            );

            violations.push(Violation::file(
                &filename,
                forbidden.line,
                "forbidden_section",
                advice,
            ));
        }
    }
}

/// Check content rules in agent files.
fn check_content(
    config: &AgentsConfig,
    detected: &[DetectedFile],
    violations: &mut Vec<Violation>,
) {
    for file in detected {
        let Ok(content) = std::fs::read_to_string(&file.path) else {
            continue;
        };

        let filename = file
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Get effective limits for this scope
        let (max_lines, max_tokens) = get_scope_limits(config, &file.scope);

        // Check content rules
        if config.tables == ContentRule::Forbid {
            for issue in detect_tables(&content) {
                violations.push(Violation::file(
                    &filename,
                    issue.line,
                    issue.content_type.violation_type(),
                    issue.content_type.advice(),
                ));
            }
        }

        if config.box_diagrams == ContentRule::Forbid {
            for issue in detect_box_diagrams(&content) {
                violations.push(Violation::file(
                    &filename,
                    issue.line,
                    issue.content_type.violation_type(),
                    issue.content_type.advice(),
                ));
            }
        }

        if config.mermaid == ContentRule::Forbid {
            for issue in detect_mermaid_blocks(&content) {
                violations.push(Violation::file(
                    &filename,
                    issue.line,
                    issue.content_type.violation_type(),
                    issue.content_type.advice(),
                ));
            }
        }

        // Check size limits
        if let Some(limit) = max_lines
            && let Some(violation) = check_line_count(&content, limit)
        {
            violations.push(
                Violation::file_only(
                    &filename,
                    "file_too_large",
                    violation
                        .limit_type
                        .advice(violation.value, violation.threshold),
                )
                .with_threshold(violation.value as i64, violation.threshold as i64),
            );
        }

        if let Some(limit) = max_tokens
            && let Some(violation) = check_token_count(&content, limit)
        {
            violations.push(
                Violation::file_only(
                    &filename,
                    "file_too_large",
                    violation
                        .limit_type
                        .advice(violation.value, violation.threshold),
                )
                .with_threshold(violation.value as i64, violation.threshold as i64),
            );
        }
    }
}

/// Get effective size limits for a scope, with inheritance.
fn get_scope_limits(config: &AgentsConfig, scope: &Scope) -> (Option<usize>, Option<usize>) {
    let scope_config = match scope {
        Scope::Root => config.root.as_ref(),
        Scope::Package(_) => config.package.as_ref(),
        Scope::Module => config.module.as_ref(),
    };

    // Scope config overrides top-level, top-level provides defaults
    let max_lines = scope_config.and_then(|s| s.max_lines).or(config.max_lines);

    let max_tokens = scope_config
        .and_then(|s| s.max_tokens)
        .or(config.max_tokens);

    (max_lines, max_tokens)
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
