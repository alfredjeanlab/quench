// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Verbose logging helpers for the check command.

use std::sync::Arc;

use quench::adapter::{
    detect_all_languages, detect_language, patterns::correlation_exclude_defaults,
    resolve_project_patterns,
};
use quench::cache::FileCache;
use quench::cli::CheckArgs;
use quench::config;
use quench::git::get_commits_since;
use quench::verbose::VerboseLogger;

pub(super) fn config(
    verbose: &VerboseLogger,
    root: &std::path::Path,
    config: &config::Config,
    config_path: &Option<std::path::PathBuf>,
    exclude_patterns: &[String],
) {
    if !verbose.is_enabled() {
        return;
    }
    verbose.section("Configuration");
    match config_path {
        Some(path) => {
            let display = path.strip_prefix(root).unwrap_or(path);
            verbose.log(&format!("Config: {}", display.display()));
        }
        None => verbose.log("Config: (defaults)"),
    }
    let langs = detect_all_languages(root);
    let lang_display: Vec<String> = langs.iter().map(|l| l.to_string()).collect();
    verbose.log(&format!("Language(s): {}", lang_display.join(", ")));

    let resolved = resolve_project_patterns(root, config);
    patterns(verbose, "project.source", &resolved.source);
    patterns(verbose, "project.tests", &resolved.test);
    patterns(verbose, "project.exclude", exclude_patterns);

    let lang = detect_language(root);
    let corr_exclude = if config.check.tests.commit.exclude.is_empty() {
        correlation_exclude_defaults(lang)
    } else {
        config.check.tests.commit.exclude.clone()
    };
    patterns(verbose, "check.tests.commit.exclude", &corr_exclude);
}

pub(super) fn discovery(
    verbose: &VerboseLogger,
    args: &CheckArgs,
    files: &[quench::walker::WalkedFile],
    stats: &quench::walker::WalkStats,
) {
    if !verbose.is_enabled() {
        return;
    }
    verbose.section("Discovery");
    verbose.log(&format!("Max depth limit: {}", args.max_depth));
    verbose.log(&format!(
        "Scanned {} files ({} errors, {} symlink loops, {} skipped >10MB)",
        files.len(),
        stats.errors,
        stats.symlink_loops,
        stats.files_skipped_size,
    ));
}

pub(super) fn suites(verbose: &VerboseLogger, config: &config::Config) {
    if !verbose.is_enabled() || config.check.tests.suite.is_empty() {
        return;
    }
    verbose.section("Test Suites");
    let suite_names: Vec<String> = config
        .check
        .tests
        .suite
        .iter()
        .map(|s| {
            let name = s.name.clone().unwrap_or_else(|| s.runner.clone());
            format!("{} ({})", name, s.runner)
        })
        .collect();
    verbose.log(&format!("Configured suites: {}", suite_names.join(", ")));
}

pub(super) fn commits(
    verbose: &VerboseLogger,
    root: &std::path::Path,
    base_branch: &Option<String>,
) {
    if !verbose.is_enabled() {
        return;
    }
    if let Some(base) = base_branch
        && let Ok(commits) = get_commits_since(root, base)
    {
        verbose.section("Commits");
        verbose.log(&format!("Commits since {} ({}):", base, commits.len()));
        for commit in &commits {
            verbose.log(&format!("  {} {}", commit.hash, commit.message));
        }
    }
}

pub(super) fn cache(verbose: &VerboseLogger, cache: &Option<Arc<FileCache>>) {
    if !verbose.is_enabled() {
        return;
    }
    if let Some(cache) = cache {
        let stats = cache.stats();
        verbose.log(&format!(
            "Cache: {} hits, {} misses, {} entries",
            stats.hits, stats.misses, stats.entries
        ));
    }
}

pub(super) fn summary(verbose: &VerboseLogger, total_ms: u64) {
    if verbose.is_enabled() {
        verbose.section("Summary");
        let secs = total_ms as f64 / 1000.0;
        verbose.log(&format!("Total wall time: {:.2}s", secs));
    }
}

/// Log a pattern list in verbose output (e.g. `project.source: a, b, c`).
pub(super) fn patterns(verbose: &VerboseLogger, label: &str, patterns: &[String]) {
    let val = patterns.join(", ");
    if val.is_empty() {
        verbose.log(&format!("{label}:"));
    } else {
        verbose.log(&format!("{label}: {val}"));
    }
}
