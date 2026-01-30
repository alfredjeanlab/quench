// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Check command implementation.

use std::sync::Arc;
use std::time::Instant;

use quench::adapter::{
    detect_all_languages, detect_language, patterns::correlation_exclude_defaults,
    project::apply_language_defaults, resolve_project_patterns,
};
use quench::baseline::Baseline;
use quench::cache::{self, CACHE_FILE_NAME, FileCache};
use quench::checks;
use quench::cli::{CheckArgs, CheckFilter, Cli, OutputFormat};
use quench::color::resolve_color;
use quench::config::{self, CheckLevel};
use quench::discovery;
use quench::error::ExitCode;
use quench::git::{
    detect_base_branch, find_ratchet_base, get_changed_files, get_commits_since, get_staged_files,
    is_git_repo, save_to_git_notes,
};
use quench::latest::{LatestMetrics, get_head_commit};
use quench::output::FormatOptions;
use quench::output::json::{self, JsonFormatter};
use quench::output::text::TextFormatter;
use quench::ratchet::{self, CurrentMetrics};
use quench::runner::{CheckRunner, RunnerConfig};
use quench::timing::{PhaseTiming, TimingInfo};
use quench::verbose::VerboseLogger;
use quench::walker::{FileWalker, WalkerConfig};

/// Check if debug files mode is enabled via QUENCH_DEBUG_FILES env var.
fn debug_files() -> bool {
    std::env::var("QUENCH_DEBUG_FILES").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Run the check command.
pub fn run(_cli: &Cli, args: &CheckArgs) -> anyhow::Result<ExitCode> {
    let total_start = Instant::now();

    // Validate flag combinations
    if args.dry_run && !args.fix {
        eprintln!("--dry-run only works with --fix");
        eprintln!(
            "  The --dry-run flag lets you preview what --fix would change without applying changes."
        );
        eprintln!("  Use: quench check --fix --dry-run");
        return Ok(ExitCode::ConfigError);
    }

    if args.staged && args.base.is_some() {
        eprintln!("--staged and --base cannot be used together");
        return Ok(ExitCode::ConfigError);
    }

    // Set up verbose logging (enabled by --ci, --verbose, or QUENCH_DEBUG)
    let verbose_enabled = args.ci
        || args.verbose
        || std::env::var("QUENCH_DEBUG").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let verbose = VerboseLogger::new(verbose_enabled);

    let cwd = std::env::current_dir()?;

    // Determine root directory
    let root = if args.paths.is_empty() {
        cwd.clone()
    } else {
        // Canonicalize the path to handle relative paths
        let path = &args.paths[0];
        if path.is_absolute() {
            path.clone()
        } else {
            cwd.join(path)
        }
    };

    // Resolve config from root directory
    let config_path = discovery::find_config(&root);

    let mut config = match &config_path {
        Some(path) => {
            tracing::debug!("loading config from {}", path.display());
            config::load_with_warnings(path)?
        }
        None => {
            tracing::debug!("no config found, using defaults");
            config::Config::default()
        }
    };

    tracing::trace!("check command starting");

    // Configure walker with language-specific exclude patterns and auto-detect packages
    let exclude_patterns = apply_language_defaults(&root, &mut config);

    // === Verbose: Configuration ===
    if verbose.is_enabled() {
        verbose.section("Configuration");
        match &config_path {
            Some(path) => {
                let display = path.strip_prefix(&root).unwrap_or(path);
                verbose.log(&format!("Config: {}", display.display()));
            }
            None => verbose.log("Config: (defaults)"),
        }
        let langs = detect_all_languages(&root);
        let lang_display: Vec<String> = langs.iter().map(|l| l.to_string()).collect();
        verbose.log(&format!("Language(s): {}", lang_display.join(", ")));

        let resolved = resolve_project_patterns(&root, &config);
        verbose_patterns(&verbose, "project.source", &resolved.source);
        verbose_patterns(&verbose, "project.tests", &resolved.test);
        verbose_patterns(&verbose, "project.exclude", &exclude_patterns);

        // Correlation config: only show source/test if user explicitly set them
        if !config.check.tests.commit.source_patterns.is_empty() {
            verbose_patterns(
                &verbose,
                "check.tests.commit.source_patterns",
                &config.check.tests.commit.source_patterns,
            );
        }
        if !config.check.tests.commit.test_patterns.is_empty() {
            verbose_patterns(
                &verbose,
                "check.tests.commit.test_patterns",
                &config.check.tests.commit.test_patterns,
            );
        }
        let lang = detect_language(&root);
        let corr_exclude = if config.check.tests.commit.exclude.is_empty() {
            correlation_exclude_defaults(lang)
        } else {
            config.check.tests.commit.exclude.clone()
        };
        verbose_patterns(&verbose, "check.tests.commit.exclude", &corr_exclude);
    }

    let walker_config = WalkerConfig {
        max_depth: Some(args.max_depth),
        exclude_patterns,
        ..Default::default()
    };

    // === Discovery Phase ===
    let discovery_start = Instant::now();

    let walker = FileWalker::new(walker_config);
    let (rx, handle) = walker.walk(&root);

    // Process files
    if debug_files() {
        // Debug mode: just list files
        for file in rx {
            // Make paths relative to root for cleaner output
            let display_path = file.path.strip_prefix(&root).unwrap_or(&file.path);
            println!("{}", display_path.display());
        }
        let stats = handle.join();
        if verbose.is_enabled() {
            eprintln!(
                "Scanned {} files, {} errors, {} symlink loops",
                stats.files_found, stats.errors, stats.symlink_loops
            );
        }
        return Ok(ExitCode::Success);
    }

    // Collect files for check
    let files: Vec<_> = rx.iter().collect();
    let stats = handle.join();

    let discovery_ms = discovery_start.elapsed().as_millis() as u64;

    // === Verbose: Discovery ===
    if verbose.is_enabled() {
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

    // Filter checks based on CLI flags
    let checks = checks::filter_checks(&args.enabled_checks(), &args.disabled_checks());

    // Determine base branch for CI mode
    let base_branch = if let Some(ref base) = args.base {
        Some(base.clone())
    } else if args.ci {
        // Auto-detect base branch in CI mode
        detect_base_branch(&root)
    } else {
        None
    };

    // Get changed files if --staged, --base is provided, or CI mode with detected base
    let changed_files = if args.staged {
        // Get staged files only
        match get_staged_files(&root) {
            Ok(files) => {
                if verbose.is_enabled() {
                    verbose.log(&format!("Checking staged files ({} files)", files.len()));
                }
                Some(files)
            }
            Err(e) => {
                eprintln!("quench: warning: could not get staged files: {}", e);
                None
            }
        }
    } else if let Some(ref base) = base_branch {
        match get_changed_files(&root, base) {
            Ok(files) => {
                if verbose.is_enabled() {
                    verbose.log(&format!(
                        "Comparing against base: {} ({} files changed)",
                        base,
                        files.len()
                    ));
                }
                Some(files)
            }
            Err(e) => {
                if args.base.is_some() {
                    // Only warn if --base was explicitly provided
                    eprintln!("quench: warning: could not get changed files: {}", e);
                }
                None
            }
        }
    } else {
        None
    };

    // Create runner
    // CI mode implicitly disables the violation limit
    let limit = if args.no_limit || args.ci {
        None
    } else {
        Some(args.limit)
    };
    // === Verbose: Test Suites ===
    if verbose.is_enabled() && !config.check.tests.suite.is_empty() {
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

    // === Verbose: Commits ===
    if verbose.is_enabled()
        && let Some(ref base) = base_branch
        && let Ok(commits) = get_commits_since(&root, base)
    {
        verbose.section("Commits");
        verbose.log(&format!("Commits since {} ({}):", base, commits.len()));
        for commit in &commits {
            verbose.log(&format!("  {} {}", commit.hash, commit.message));
        }
    }

    let mut runner = CheckRunner::new(RunnerConfig {
        limit,
        changed_files,
        fix: args.fix,
        dry_run: args.dry_run,
        ci_mode: args.ci,
        base_branch: base_branch.clone(),
        staged: args.staged,
        verbose: verbose_enabled,
    });

    // Set up caching (unless --no-cache)
    let cache_dir = root.join(".quench");
    let cache_path = cache_dir.join(CACHE_FILE_NAME);
    let config_hash = cache::hash_config(&config);

    let cache = if args.no_cache {
        None
    } else {
        match FileCache::from_persistent(&cache_path, config_hash) {
            Ok(cache) => {
                tracing::debug!("loaded cache from {}", cache_path.display());
                Some(Arc::new(cache))
            }
            Err(e) => {
                tracing::debug!("cache not loaded ({}), starting fresh", e);
                Some(Arc::new(FileCache::new(config_hash)))
            }
        }
    };

    if let Some(ref cache) = cache {
        runner = runner.with_cache(Arc::clone(cache));
    }

    // === Checking Phase ===
    let checking_start = Instant::now();

    // Run checks
    let check_results = runner.run(checks, &files, &config, &root);

    let checking_ms = checking_start.elapsed().as_millis() as u64;

    // Persist cache asynchronously (fire and forget for speed)
    // Cache write happens in background thread, doesn't block command exit.
    // In CI mode, we wait for completion to ensure cache is persisted for next job.
    let cache_handle = if let Some(cache) = &cache {
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            tracing::warn!("failed to create cache directory: {}", e);
            None
        } else {
            tracing::debug!("persisting cache to {} (async)", cache_path.display());
            Some(cache.persist_async(cache_path.clone()))
        }
    } else {
        None
    };

    // Report cache stats in verbose mode
    if verbose.is_enabled()
        && let Some(cache) = &cache
    {
        let stats = cache.stats();
        verbose.log(&format!(
            "Cache: {} hits, {} misses, {} entries",
            stats.hits, stats.misses, stats.entries
        ));
    }

    // Create output
    let output = json::create_output(check_results);
    let total_violations = output.total_violations();

    // Determine if we should use git notes for baseline
    let use_notes = config.git.uses_notes() && is_git_repo(&root);

    // === Verbose: Ratchet ===
    // Ratchet checking (cache baseline for potential --fix reuse)
    let (ratchet_result, baseline) = if config.ratchet.check != CheckLevel::Off {
        if verbose.is_enabled() {
            verbose.section("Ratchet");
            verbose.log(&format!(
                "Mode: {}",
                if use_notes { "git notes" } else { "file" }
            ));
            if let Some(ref base) = base_branch {
                verbose.log(&format!("Base branch: {}", base));
            }
        }
        if use_notes {
            // Git notes mode (default)
            match find_ratchet_base(&root, base_branch.as_deref()) {
                Ok(base_commit) => {
                    if verbose.is_enabled() {
                        verbose.log(&format!(
                            "Ratchet base: {}",
                            &base_commit[..7.min(base_commit.len())]
                        ));
                    }
                    match Baseline::load_from_notes(&root, &base_commit) {
                        Ok(Some(baseline)) => {
                            if verbose.is_enabled() {
                                verbose.log(&format!(
                                    "Baseline: loaded from git notes for {}",
                                    &base_commit[..7.min(base_commit.len())]
                                ));
                            }
                            if baseline.is_stale(config.ratchet.stale_days) {
                                eprintln!(
                                    "warning: baseline is {} days old. Consider refreshing with --fix.",
                                    baseline.age_days()
                                );
                            }
                            let current = CurrentMetrics::from_output(&output);
                            let result =
                                ratchet::compare(&current, &baseline.metrics, &config.ratchet);
                            (Some(result), Some(baseline))
                        }
                        Ok(None) => {
                            if verbose.is_enabled() {
                                verbose.log(&format!(
                                    "Baseline: not found (searched: refs/notes/quench for {})",
                                    &base_commit[..7.min(base_commit.len())]
                                ));
                            }
                            (None, None)
                        }
                        Err(e) => {
                            eprintln!("quench: warning: failed to load baseline from notes: {}", e);
                            (None, None)
                        }
                    }
                }
                Err(e) => {
                    if verbose.is_enabled() {
                        verbose.log(&format!("Ratchet base: not found ({})", e));
                    }
                    (None, None)
                }
            }
        } else if let Some(path) = config.git.baseline_path() {
            // File-based baseline mode
            let baseline_path = root.join(path);
            match Baseline::load(&baseline_path) {
                Ok(Some(baseline)) => {
                    if verbose.is_enabled() {
                        verbose.log(&format!(
                            "Baseline: loaded from {}",
                            baseline_path.display()
                        ));
                    }
                    if baseline.is_stale(config.ratchet.stale_days) {
                        eprintln!(
                            "warning: baseline is {} days old. Consider refreshing with --fix.",
                            baseline.age_days()
                        );
                    }
                    let current = CurrentMetrics::from_output(&output);
                    let result = ratchet::compare(&current, &baseline.metrics, &config.ratchet);
                    (Some(result), Some(baseline))
                }
                Ok(None) => {
                    if verbose.is_enabled() {
                        verbose.log(&format!(
                            "No baseline found at {}. Run with --fix to create.",
                            baseline_path.display()
                        ));
                    }
                    (None, None)
                }
                Err(e) => {
                    eprintln!("quench: warning: failed to load baseline: {}", e);
                    (None, None)
                }
            }
        } else {
            // Not in git repo and using notes mode - skip ratchet
            if verbose.is_enabled() {
                verbose.log("Ratchet check: off (not in git repo with notes mode)");
            }
            (None, None)
        }
    } else {
        if verbose.is_enabled() {
            verbose.section("Ratchet");
            verbose.log("Ratchet check: off");
        }
        (None, None)
    };

    // Handle --fix: update/sync baseline (reusing cached baseline)
    if args.fix {
        let current = CurrentMetrics::from_output(&output);

        // Use existing baseline or create new
        let mut baseline = baseline
            .map(|b| b.with_commit(&root))
            .unwrap_or_else(|| Baseline::new().with_commit(&root));

        ratchet::update_baseline(&mut baseline, &current);

        // Determine save target based on config and flags
        if use_notes {
            // Default: save to git notes
            let json = serde_json::to_string_pretty(&baseline)?;
            match save_to_git_notes(&root, &json) {
                Ok(()) => report_baseline_update(&ratchet_result, "git notes"),
                Err(e) => eprintln!("quench: warning: failed to save to git notes: {}", e),
            }
        }

        // Also save to file if explicitly configured (or when --no-notes is used)
        if let Some(path) = config.git.baseline_path() {
            let baseline_path = root.join(path);
            let baseline_existed = baseline_path.exists();
            if let Err(e) = baseline.save(&baseline_path) {
                eprintln!("quench: warning: failed to save baseline: {}", e);
            } else if !use_notes {
                // Only report file update if not using notes
                report_baseline_update_file(&ratchet_result, &baseline_path, baseline_existed);
            }
        }
    }

    // Always write latest.json for local caching
    let latest_path = root.join(".quench/latest.json");
    let latest = LatestMetrics {
        updated: chrono::Utc::now(),
        commit: get_head_commit(&root).ok(),
        output: output.clone(),
    };
    if let Err(e) = latest.save(&latest_path)
        && verbose.is_enabled()
    {
        verbose.log(&format!("Failed to write latest.json: {}", e));
    }

    // Resolve color mode
    let color_choice = resolve_color();

    // Set up formatter options
    // CI mode implicitly disables the violation limit
    let limit = if args.no_limit || args.ci {
        None
    } else {
        Some(args.limit)
    };
    let options = FormatOptions { limit };

    // === Build timing info before output ===
    let timing_info = if args.timing {
        let stats = cache.as_ref().map(|c| c.stats());
        Some(TimingInfo {
            phases: PhaseTiming {
                discovery_ms,
                checking_ms,
                output_ms: 0, // Updated after output
                total_ms: 0,  // Updated after output
            },
            files: files.len(),
            cache_hits: stats.as_ref().map(|s| s.hits).unwrap_or(0),
            checks: output
                .checks
                .iter()
                .filter_map(|r| r.duration_ms.map(|d| (r.name.clone(), d)))
                .collect(),
        })
    } else {
        None
    };

    // === Output Phase ===
    let output_start = Instant::now();

    // Format output
    match args.output {
        OutputFormat::Text | OutputFormat::Html | OutputFormat::Markdown => {
            // HTML/Markdown use text format for check command (specialized formats are for report only)
            let mut formatter = TextFormatter::new(color_choice, options);

            for result in &output.checks {
                formatter.write_check(result)?;
            }

            // Write ratchet results if applicable
            if let Some(ref result) = ratchet_result {
                formatter.write_ratchet(result, config.ratchet.check)?;
            }

            formatter.write_summary(&output)?;

            if formatter.was_truncated() {
                formatter.write_truncation_message(total_violations)?;
            }
        }
        OutputFormat::Json => {
            let mut formatter = JsonFormatter::new(std::io::stdout());
            formatter.write_with_timing(&output, ratchet_result.as_ref(), timing_info.as_ref())?;
        }
    }

    // Save metrics to file if requested
    if let Some(ref save_path) = args.save {
        if let Err(e) = save_metrics_to_file(save_path, &output) {
            eprintln!("quench: warning: failed to save metrics: {}", e);
        } else if verbose.is_enabled() {
            verbose.log(&format!("Saved metrics to {}", save_path.display()));
        }
    }

    let output_ms = output_start.elapsed().as_millis() as u64;
    let total_ms = total_start.elapsed().as_millis() as u64;

    // === Print timing to stderr (text mode only) ===
    if let Some(mut info) = timing_info {
        info.phases.output_ms = output_ms;
        info.phases.total_ms = total_ms;

        // Text output goes to stderr
        if !matches!(args.output, OutputFormat::Json) {
            eprintln!("{}", info.phases.format_text());
            // Per-check timing
            for result in &output.checks {
                if let Some(ms) = result.duration_ms {
                    eprintln!("{}: {}ms", result.name, ms);
                }
            }
            // File and cache stats
            eprintln!("files: {}", info.files);
            let misses = cache.as_ref().map(|c| c.stats().misses).unwrap_or(0);
            eprintln!("{}", info.format_cache(misses));
        }
    }

    // === Verbose: Summary ===
    if verbose.is_enabled() {
        verbose.section("Summary");
        let secs = total_ms as f64 / 1000.0;
        verbose.log(&format!("Total wall time: {:.2}s", secs));
    }

    // Wait for cache persistence to complete.
    // The cache write ran concurrently with output formatting, giving us overlap benefit.
    // We wait here to ensure the cache is fully persisted before process exit.
    if let Some(handle) = cache_handle
        && let Err(e) = handle.join().unwrap_or(Ok(()))
    {
        tracing::warn!("failed to persist cache: {}", e);
    }

    // Determine exit code considering ratchet result
    // Only fail if check level is Error; Warn level reports but exits 0
    let ratchet_failed = ratchet_result
        .as_ref()
        .is_some_and(|r| !r.passed && config.ratchet.check == CheckLevel::Error);
    // Dry-run always exits 0: preview is complete
    let exit_code = if args.dry_run {
        ExitCode::Success
    } else if !output.passed || ratchet_failed {
        ExitCode::CheckFailed
    } else {
        ExitCode::Success
    };

    Ok(exit_code)
}

/// Save metrics output to a JSON file.
fn save_metrics_to_file(
    path: &std::path::Path,
    output: &quench::check::CheckOutput,
) -> anyhow::Result<()> {
    // Create parent directories if needed
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    // Serialize and write
    let json = serde_json::to_string_pretty(output)?;
    std::fs::write(path, json)?;

    Ok(())
}

/// Report baseline update to stderr (for git notes mode).
fn report_baseline_update(ratchet_result: &Option<ratchet::RatchetResult>, target: &str) {
    if let Some(result) = ratchet_result {
        if result.improvements.is_empty() {
            eprintln!("ratchet: baseline synced ({})", target);
        } else {
            eprintln!("ratchet: updated baseline ({})", target);
            for improvement in &result.improvements {
                // Coverage uses "new floor" (ratchets UP), others use "new ceiling" (ratchet DOWN)
                let ratchet_label = if improvement.name.starts_with("coverage.") {
                    "new floor"
                } else {
                    "new ceiling"
                };
                eprintln!(
                    "  {}: {} -> {} ({})",
                    improvement.name,
                    improvement.format_value(improvement.old_value),
                    improvement.format_value(improvement.new_value),
                    ratchet_label,
                );
            }
        }
    } else {
        eprintln!("ratchet: created initial baseline ({})", target);
    }
}

/// Report baseline update to stderr (for file mode).
fn report_baseline_update_file(
    ratchet_result: &Option<ratchet::RatchetResult>,
    path: &std::path::Path,
    existed: bool,
) {
    if !existed {
        eprintln!("ratchet: created initial baseline at {}", path.display());
    } else if let Some(result) = ratchet_result {
        if result.improvements.is_empty() {
            eprintln!("ratchet: baseline synced");
        } else {
            eprintln!("ratchet: updated baseline at {}", path.display());
            for improvement in &result.improvements {
                let ratchet_label = if improvement.name.starts_with("coverage.") {
                    "new floor"
                } else {
                    "new ceiling"
                };
                eprintln!(
                    "  {}: {} -> {} ({})",
                    improvement.name,
                    improvement.format_value(improvement.old_value),
                    improvement.format_value(improvement.new_value),
                    ratchet_label,
                );
            }
        }
    } else {
        eprintln!("ratchet: baseline synced");
    }
}

/// Log a pattern list in verbose output (e.g. `project.source: a, b, c`).
fn verbose_patterns(verbose: &VerboseLogger, label: &str, patterns: &[String]) {
    let val = patterns.join(", ");
    if val.is_empty() {
        verbose.log(&format!("{label}:"));
    } else {
        verbose.log(&format!("{label}: {val}"));
    }
}
