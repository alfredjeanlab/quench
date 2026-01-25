// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Git check implementation.
//!
//! Validates commit message format and git-related conventions.
//! Skips if not in a git repository.

use std::path::Path;

use git2::Repository;

use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::GitCommitConfig;
use crate::git::{Commit, get_all_branch_commits, get_commits_since, is_git_repo};

pub mod docs;
pub mod parse;
mod template;

use template::{TEMPLATE_PATH, generate_template};

use docs::{DocsResult, check_commit_docs, primary_agent_file};
pub use parse::{
    DEFAULT_TYPES, ParseResult, ParsedCommit, is_merge_commit, parse_conventional_commit,
};

/// The git check validates commit message format.
pub struct GitCheck;

impl Check for GitCheck {
    fn name(&self) -> &'static str {
        "git"
    }

    fn description(&self) -> &'static str {
        "Commit message format"
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        // Check if we're in a git repository
        if !is_git_repo(ctx.root) {
            return CheckResult::skipped(self.name(), "Not a git repository");
        }

        // Get check configuration
        let config = &ctx.config.git.commit;

        // Skip if check is disabled
        if config.check.as_deref() == Some("off") {
            return CheckResult::skipped(self.name(), "Check disabled");
        }

        // Skip format validation if format = "none"
        if config.effective_format() == "none" {
            return CheckResult::passed(self.name());
        }

        let mut violations = Vec::new();

        // Check agent documentation (if enabled)
        if config.agents {
            check_agent_docs(ctx.root, &mut violations);
        }

        // Get commits to validate
        let commits = match get_commits_to_check(ctx) {
            Ok(commits) => commits,
            Err(e) => return CheckResult::skipped(self.name(), e.to_string()),
        };

        // Validate each commit (if any)
        let mut validated_count = 0;
        for commit in &commits {
            if validate_commit(commit, config, &mut violations) {
                validated_count += 1;
            }
        }

        // Handle --fix for template creation
        let fix_summary = if ctx.fix && config.template {
            fix_template(ctx.root, config, ctx.dry_run)
        } else {
            None
        };

        // Build metrics (only if commits were validated)
        let metrics = if validated_count > 0 {
            // Count commits with violations
            let commits_with_violations = violations
                .iter()
                .filter_map(|v| v.commit.as_ref())
                .collect::<std::collections::HashSet<_>>()
                .len();

            Some(serde_json::json!({
                "commits_checked": validated_count,
                "commits_valid": validated_count - commits_with_violations,
                "commits_skipped": commits.len() - validated_count,
            }))
        } else {
            None
        };

        let mut result = if violations.is_empty() {
            if let Some(summary) = fix_summary {
                CheckResult::fixed(self.name(), summary)
            } else {
                CheckResult::passed(self.name())
            }
        } else {
            CheckResult::failed(self.name(), violations)
        };

        if let Some(m) = metrics {
            result = result.with_metrics(m);
        }

        result
    }

    fn default_enabled(&self) -> bool {
        false
    }
}

/// Check that commit format is documented in agent files.
fn check_agent_docs(root: &Path, violations: &mut Vec<Violation>) {
    match check_commit_docs(root) {
        DocsResult::Found(_) => {
            // Documentation found, nothing to do
        }
        DocsResult::NotFound(_checked) => {
            // Files exist but lack documentation
            let file = primary_agent_file(root);
            violations.push(Violation::file_only(
                file,
                "missing_docs",
                "Add a Commits section describing the format, e.g.:\n\n\
                ## Commits\n\n\
                Use conventional commit format: `type(scope): description`\n\
                Types: feat, fix, chore, docs, test, refactor",
            ));
        }
        DocsResult::NoAgentFiles => {
            // No agent files to check - this is handled by agents check
            // or the user may not want agent files at all
        }
    }
}

/// Get commits to validate based on context.
fn get_commits_to_check(ctx: &CheckContext) -> anyhow::Result<Vec<Commit>> {
    // Staged mode: no commit message to check yet
    if ctx.staged {
        return Ok(Vec::new());
    }

    // CI mode or explicit base: check commits on branch
    if ctx.ci_mode {
        get_all_branch_commits(ctx.root)
    } else if let Some(base) = ctx.base_branch {
        get_commits_since(ctx.root, base)
    } else {
        // No base specified, no commits to check
        Ok(Vec::new())
    }
}

/// Validate a single commit and add violations if invalid.
///
/// Returns `true` if the commit was validated, `false` if skipped.
pub fn validate_commit(
    commit: &Commit,
    config: &GitCommitConfig,
    violations: &mut Vec<Violation>,
) -> bool {
    // Skip merge commits if configured
    if config.skip_merge && is_merge_commit(&commit.message) {
        return false; // Skipped
    }

    match parse_conventional_commit(&commit.message) {
        ParseResult::NonConventional => {
            violations.push(Violation::commit_violation(
                &commit.hash,
                &commit.message,
                "invalid_format",
                "Expected: <type>(<scope>): <description>",
            ));
        }
        ParseResult::Conventional(parsed) => {
            // Check type
            let allowed_types = config.types.as_deref();
            if !parsed.is_type_allowed(allowed_types) {
                let advice = format_type_advice(allowed_types);
                let mut violation = Violation::commit_violation(
                    &commit.hash,
                    &commit.message,
                    "invalid_type",
                    advice,
                );
                if let Some(ref scope) = parsed.scope {
                    violation = violation.with_scope(scope.clone());
                }
                violations.push(violation);
            }

            // Check scope (only if scopes are configured)
            if let Some(scopes) = config.scopes.as_ref()
                && !parsed.is_scope_allowed(Some(scopes))
            {
                let advice = format!("Allowed scopes: {}", scopes.join(", "));
                let mut violation = Violation::commit_violation(
                    &commit.hash,
                    &commit.message,
                    "invalid_scope",
                    advice,
                );
                if let Some(ref scope) = parsed.scope {
                    violation = violation.with_scope(scope.clone());
                }
                violations.push(violation);
            }
        }
    }

    true // Validated
}

/// Format advice for invalid type violations.
fn format_type_advice(allowed_types: Option<&[String]>) -> String {
    match allowed_types {
        None => format!("Allowed types: {}", DEFAULT_TYPES.join(", ")),
        Some([]) => "Any type allowed (check format only)".to_string(),
        Some(types) => format!("Allowed types: {}", types.join(", ")),
    }
}

/// Fix template and git config if needed.
///
/// Returns fix summary if anything was fixed, None otherwise.
fn fix_template(root: &Path, config: &GitCommitConfig, dry_run: bool) -> Option<serde_json::Value> {
    let template_path = root.join(TEMPLATE_PATH);
    let mut actions = Vec::new();

    // Create .gitmessage if missing
    if !template_path.exists() {
        let content = generate_template(config);
        if !dry_run {
            if let Err(e) = std::fs::write(&template_path, &content) {
                // Log error but continue - this is a best-effort fix
                eprintln!("Warning: Failed to create {}: {}", TEMPLATE_PATH, e);
            } else {
                actions.push(format!("Created {} (commit template)", TEMPLATE_PATH));
            }
        } else {
            actions.push(format!("Would create {} (commit template)", TEMPLATE_PATH));
        }
    }

    // Configure git commit.template if not set
    if !is_template_configured(root) {
        if !dry_run {
            if configure_git_template(root) {
                actions.push("Configured git commit.template".to_string());
            }
        } else {
            actions.push("Would configure git commit.template".to_string());
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(serde_json::json!({
            "actions": actions
        }))
    }
}

/// Check if commit.template is already configured.
fn is_template_configured(root: &Path) -> bool {
    Repository::discover(root)
        .and_then(|repo| repo.config())
        .and_then(|config| config.get_string("commit.template"))
        .is_ok()
}

/// Configure git commit.template to use .gitmessage.
fn configure_git_template(root: &Path) -> bool {
    let Ok(repo) = Repository::discover(root) else {
        return false;
    };
    let Ok(mut config) = repo.config() else {
        return false;
    };

    config.set_str("commit.template", TEMPLATE_PATH).is_ok()
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
