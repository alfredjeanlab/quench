// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Cursor rule reconciliation logic.
//!
//! Verifies that `.mdc` cursor rules are consistent with CLAUDE.md / AGENTS.md
//! agent files. Supports both `alwaysApply` rules (reconciled against root
//! CLAUDE.md) and single-directory-scoped rules (reconciled against
//! directory-level agent files).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::check::Violation;

use super::mdc::{self, MdcRule, RuleScope, classify_scope, discover_mdc_files, parse_mdc};
use super::sync::{self, Section};

/// Direction for reconciliation checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileDirection {
    /// Check both directions (default).
    Bidirectional,
    /// Only check that cursor rules appear in agent files.
    CursorToClaude,
    /// Only check that agent file sections appear in cursor rules.
    ClaudeToCursor,
}

/// Derive reconciliation direction from sync configuration.
pub fn derive_direction_from_sync(
    sync_enabled: bool,
    sync_from: Option<&str>,
    _files: &[String],
) -> Option<ReconcileDirection> {
    // If sync is disabled, skip reconciliation
    if !sync_enabled {
        return None;
    }

    // Only use explicit sync_from if provided
    let Some(source) = sync_from else {
        // No sync_from specified → bidirectional
        return Some(ReconcileDirection::Bidirectional);
    };

    match source {
        // CLAUDE.md/AGENTS.md as source → sync cursor TO match
        s if s == "CLAUDE.md" || s == "AGENTS.md" => Some(ReconcileDirection::ClaudeToCursor),

        // .mdc as source → sync CLAUDE.md TO match cursor
        s if s.ends_with(".mdc") || s.contains(".cursor/rules/") => {
            Some(ReconcileDirection::CursorToClaude)
        }

        // Other files (.cursorrules, etc.) → bidirectional
        _ => Some(ReconcileDirection::Bidirectional),
    }
}

/// A reconciliation violation between cursor rules and agent files.
#[derive(Debug)]
pub struct ReconcileViolation {
    /// The `.mdc` file or agent file involved.
    pub file: String,
    /// Violation type identifier.
    pub violation_type: &'static str,
    /// Human-readable advice.
    pub advice: String,
    /// Section name involved (if applicable).
    pub section: Option<String>,
    /// Target agent file (for context).
    pub target: Option<String>,
}

/// Result of reconciliation for fix mode.
#[derive(Debug)]
pub struct ReconcileFix {
    /// Path to write.
    pub target_path: PathBuf,
    /// Content to write.
    pub content: String,
}

/// Context for a reconciliation run.
struct ReconcileCtx<'a> {
    root: &'a Path,
    agent_filename: &'a str,
    direction: &'a ReconcileDirection,
    fix: bool,
    dry_run: bool,
    violations: Vec<ReconcileViolation>,
    fixes: Vec<ReconcileFix>,
}

/// Run cursor reconciliation checks.
///
/// Returns violations found and any fixes applied.
pub fn check_cursor_reconciliation(
    root: &Path,
    agent_files: &[String],
    direction: &ReconcileDirection,
    fix: bool,
    dry_run: bool,
) -> (Vec<ReconcileViolation>, Vec<ReconcileFix>) {
    let agent_filename = determine_agent_file(agent_files);

    let mut ctx = ReconcileCtx {
        root,
        agent_filename: &agent_filename,
        direction,
        fix,
        dry_run,
        violations: Vec::new(),
        fixes: Vec::new(),
    };

    // Discover and parse all .mdc files
    let mdc_paths = discover_mdc_files(root);
    if mdc_paths.is_empty() {
        return (ctx.violations, ctx.fixes);
    }

    let mut rules: Vec<MdcRule> = Vec::new();
    for path in mdc_paths {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };

        match parse_mdc(&content, path.clone()) {
            Ok(rule) => rules.push(rule),
            Err(err) => {
                let rel_path =
                    path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string();
                ctx.violations.push(ReconcileViolation {
                    file: rel_path,
                    violation_type: "cursor_parse_error",
                    advice: format!("Malformed .mdc frontmatter: {}", err.message),
                    section: None,
                    target: None,
                });
            }
        }
    }

    // Reconcile alwaysApply rules against root agent file
    reconcile_always_apply(&mut ctx, &rules);

    // Reconcile directory-scoped rules
    reconcile_directory_scoped(&mut ctx, &rules);

    (ctx.violations, ctx.fixes)
}

/// Determine which agent file name to use for reconciliation.
fn determine_agent_file(agent_files: &[String]) -> String {
    // Prefer CLAUDE.md, then AGENTS.md
    for f in agent_files {
        if !f.contains('*') && !f.contains(".cursor") {
            return f.clone();
        }
    }
    "CLAUDE.md".to_string()
}

/// Reconcile all `alwaysApply: true` rules against the root agent file.
fn reconcile_always_apply(ctx: &mut ReconcileCtx, rules: &[MdcRule]) {
    let always_apply_rules: Vec<&MdcRule> =
        rules.iter().filter(|r| classify_scope(r) == RuleScope::AlwaysApply).collect();

    if always_apply_rules.is_empty() {
        return;
    }

    let agent_path = ctx.root.join(ctx.agent_filename);
    let agent_content = std::fs::read_to_string(&agent_path).unwrap_or_default();
    let agent_sections = sync::parse_sections(&agent_content);

    // Build union of all alwaysApply sections
    let mut cursor_sections: Vec<CursorSection> = Vec::new();
    for rule in &always_apply_rules {
        let body = mdc::strip_leading_header(&rule.body);
        let sections = sync::parse_sections(body);
        let rel_path =
            rule.path.strip_prefix(ctx.root).unwrap_or(&rule.path).to_string_lossy().to_string();

        for section in sections {
            cursor_sections.push(CursorSection { section, source_file: rel_path.clone() });
        }
    }

    // Forward check: cursor → claude
    if *ctx.direction != ReconcileDirection::ClaudeToCursor {
        check_cursor_to_agent(ctx, &cursor_sections, &agent_sections, &agent_path, &agent_content);
    }

    // Reverse check: claude → cursor
    if *ctx.direction != ReconcileDirection::CursorToClaude {
        check_agent_to_cursor(ctx, &cursor_sections, &agent_sections);
    }
}

/// A section from a cursor rule, tracking its source file.
struct CursorSection {
    section: Section,
    source_file: String,
}

/// Check that each cursor section exists in the agent file.
fn check_cursor_to_agent(
    ctx: &mut ReconcileCtx,
    cursor_sections: &[CursorSection],
    agent_sections: &[Section],
    agent_path: &Path,
    agent_content: &str,
) {
    let agent_names: HashSet<&str> = agent_sections.iter().map(|s| s.name.as_str()).collect();
    let mut missing_sections: Vec<&CursorSection> = Vec::new();

    for cs in cursor_sections {
        if !agent_names.contains(cs.section.name.as_str()) {
            // Section not found in agent file
            let section_display = section_name_display(&cs.section);

            ctx.violations.push(ReconcileViolation {
                file: cs.source_file.clone(),
                violation_type: "cursor_missing_in_claude",
                advice: format!(
                    "Section \"{}\" exists in {} (alwaysApply) but not in {}. Use --fix to add missing sections.",
                    section_display, cs.source_file, ctx.agent_filename
                ),
                section: Some(section_display),
                target: Some(ctx.agent_filename.to_string()),
            });

            missing_sections.push(cs);
        } else {
            // Section exists, check content matches
            if let Some(agent_section) = agent_sections.iter().find(|s| s.name == cs.section.name) {
                let cursor_normalized = normalize_content(&cs.section.content);
                let agent_normalized = normalize_content(&agent_section.content);

                if cursor_normalized != agent_normalized && !cs.section.name.is_empty() {
                    let section_display = cs.section.heading.clone();

                    ctx.violations.push(ReconcileViolation {
                        file: cs.source_file.clone(),
                        violation_type: "cursor_missing_in_claude",
                        advice: format!(
                            "Section \"{}\" content differs between {} and {}. Reconcile manually or use --fix.",
                            section_display, cs.source_file, ctx.agent_filename
                        ),
                        section: Some(section_display),
                        target: Some(ctx.agent_filename.to_string()),
                    });
                }
            }
        }
    }

    // Fix: append missing sections to agent file
    if ctx.fix && !missing_sections.is_empty() {
        let mut new_content = agent_content.to_string();
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }

        for cs in &missing_sections {
            if !cs.section.heading.is_empty() {
                new_content.push_str(&format!("\n## {}\n\n", cs.section.heading));
            }
            new_content.push_str(&cs.section.content);
            if !cs.section.content.ends_with('\n') {
                new_content.push('\n');
            }
        }

        if !ctx.dry_run && std::fs::write(agent_path, &new_content).is_err() {
            return;
        }
        ctx.fixes
            .push(ReconcileFix { target_path: agent_path.to_path_buf(), content: new_content });
    }
}

/// Check that each agent file section exists in some cursor rule.
fn check_agent_to_cursor(
    ctx: &mut ReconcileCtx,
    cursor_sections: &[CursorSection],
    agent_sections: &[Section],
) {
    let cursor_names: HashSet<&str> =
        cursor_sections.iter().map(|cs| cs.section.name.as_str()).collect();

    for section in agent_sections {
        if !cursor_names.contains(section.name.as_str()) {
            let section_display = section_name_display(section);

            ctx.violations.push(ReconcileViolation {
                file: ctx.agent_filename.to_string(),
                violation_type: "claude_missing_in_cursor",
                advice: format!(
                    "Section \"{}\" exists in {} but not in any alwaysApply cursor rule.",
                    section_display, ctx.agent_filename
                ),
                section: Some(section_display),
                target: None,
            });
        }
    }
}

/// Reconcile directory-scoped rules against per-directory agent files.
fn reconcile_directory_scoped(ctx: &mut ReconcileCtx, rules: &[MdcRule]) {
    for rule in rules {
        let scope = classify_scope(rule);
        let RuleScope::SingleDirectory(ref dir) = scope else {
            continue;
        };

        let rel_mdc =
            rule.path.strip_prefix(ctx.root).unwrap_or(&rule.path).to_string_lossy().to_string();

        let agent_path = ctx.root.join(dir).join(ctx.agent_filename);
        let dir_agent = format!("{}/{}", dir.display(), ctx.agent_filename);

        if !agent_path.exists() {
            // No agent file in the target directory
            ctx.violations.push(ReconcileViolation {
                file: rel_mdc.clone(),
                violation_type: "cursor_no_agent_file",
                advice: format!(
                    "Rule scoped to {}/ but no {} found there. Use --fix to create {} from rule content.",
                    dir.display(),
                    ctx.agent_filename,
                    dir_agent,
                ),
                section: None,
                target: Some(dir_agent),
            });

            // Fix: create agent file from rule body
            if ctx.fix {
                let body = mdc::strip_leading_header(&rule.body);
                let content = body.to_string();

                if !ctx.dry_run {
                    if let Some(parent) = agent_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if std::fs::write(&agent_path, &content).is_err() {
                        continue;
                    }
                }
                ctx.fixes.push(ReconcileFix { target_path: agent_path, content });
            }

            continue;
        }

        // Agent file exists - compare content
        let Ok(agent_content) = std::fs::read_to_string(&agent_path) else {
            continue;
        };

        let body = mdc::strip_leading_header(&rule.body);
        let cursor_sections = sync::parse_sections(body);
        let agent_sections = sync::parse_sections(&agent_content);

        // Forward check: cursor → agent
        if *ctx.direction != ReconcileDirection::ClaudeToCursor {
            let agent_names: HashSet<&str> =
                agent_sections.iter().map(|s| s.name.as_str()).collect();

            for section in &cursor_sections {
                if !agent_names.contains(section.name.as_str()) {
                    let section_display = section_name_display(section);

                    ctx.violations.push(ReconcileViolation {
                        file: rel_mdc.clone(),
                        violation_type: "cursor_dir_missing_in_agent",
                        advice: format!(
                            "Section \"{}\" in {} not found in {}.",
                            section_display, rel_mdc, dir_agent
                        ),
                        section: Some(section_display),
                        target: Some(dir_agent.clone()),
                    });
                }
            }
        }

        // Reverse check: agent → cursor
        if *ctx.direction != ReconcileDirection::CursorToClaude {
            let cursor_names: HashSet<&str> =
                cursor_sections.iter().map(|s| s.name.as_str()).collect();

            for section in &agent_sections {
                if !cursor_names.contains(section.name.as_str()) {
                    let section_display = section_name_display(section);

                    ctx.violations.push(ReconcileViolation {
                        file: dir_agent.clone(),
                        violation_type: "agent_dir_missing_in_cursor",
                        advice: format!(
                            "Section \"{}\" in {} not found in {}.",
                            section_display, dir_agent, rel_mdc
                        ),
                        section: Some(section_display),
                        target: Some(rel_mdc.clone()),
                    });
                }
            }
        }
    }
}

/// Display name for a section (heading or "(preamble)" for empty names).
fn section_name_display(section: &Section) -> String {
    if section.name.is_empty() { "(preamble)".to_string() } else { section.heading.clone() }
}

/// Normalize content for comparison (same as sync::normalize_content).
fn normalize_content(content: &str) -> String {
    content.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
}

/// Convert a `ReconcileViolation` to a `Violation`.
pub fn to_violation(rv: &ReconcileViolation) -> Violation {
    let mut v = Violation::file_only(&rv.file, rv.violation_type, rv.advice.clone());
    if let Some(ref section) = rv.section {
        v = v.with_section(section);
    }
    if let Some(ref target) = rv.target {
        v = v.with_target(target);
    }
    v
}

#[cfg(test)]
#[path = "reconcile_tests.rs"]
mod tests;
