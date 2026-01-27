// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Check command implementation.

use std::sync::Arc;
use std::time::Instant;

use quench::adapter::{JsWorkspace, ProjectLanguage, detect_language, rust::CargoWorkspace};
use quench::baseline::Baseline;
use quench::cache::{self, CACHE_FILE_NAME, FileCache};
use quench::checks;
use quench::cli::{CheckArgs, CheckFilter, Cli, OutputFormat};
use quench::color::resolve_color;
use quench::config::{self, CheckLevel};
use quench::discovery;
use quench::error::ExitCode;
use quench::git::{
    detect_base_branch, get_changed_files, get_staged_files, is_git_repo, save_to_git_notes,
};
use quench::output::FormatOptions;
use quench::output::json::{self, JsonFormatter};
use quench::output::text::TextFormatter;
use quench::ratchet::{self, CurrentMetrics};
use quench::runner::{CheckRunner, RunnerConfig};
use quench::timing::{PhaseTiming, TimingInfo};
use quench::walker::{FileWalker, WalkerConfig};

/// Check if debug logging is enabled via QUENCH_DEBUG env var.
fn debug_logging() -> bool {
    std::env::var("QUENCH_DEBUG").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

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

    // Configure walker with language-specific ignore patterns
    let mut ignore_patterns = config.project.ignore.patterns.clone();

    // Add language-specific patterns and auto-detect workspace packages
    match detect_language(&root) {
        ProjectLanguage::Rust => {
            // Ignore target/ directory for Rust projects
            if !ignore_patterns.iter().any(|p| p.contains("target")) {
                ignore_patterns.push("target".to_string());
            }

            // Auto-detect workspace packages if not configured
            if config.project.packages.is_empty() {
                let workspace = CargoWorkspace::from_root(&root);
                if workspace.is_workspace {
                    // For workspaces, expand member patterns to get both paths and names
                    for pattern in &workspace.member_patterns {
                        if pattern.contains('*') {
                            // Expand glob patterns
                            if let Some(base) = pattern.strip_suffix("/*") {
                                let dir = root.join(base);
                                if let Ok(entries) = std::fs::read_dir(&dir) {
                                    for entry in entries.flatten() {
                                        if entry.path().is_dir() {
                                            let rel_path = format!(
                                                "{}/{}",
                                                base,
                                                entry.file_name().to_string_lossy()
                                            );
                                            // Read package name from Cargo.toml
                                            let cargo_toml = entry.path().join("Cargo.toml");
                                            if let Ok(content) =
                                                std::fs::read_to_string(&cargo_toml)
                                                && let Ok(value) = content.parse::<toml::Value>()
                                                && let Some(name) = value
                                                    .get("package")
                                                    .and_then(|p| p.get("name"))
                                                    .and_then(|n| n.as_str())
                                            {
                                                config
                                                    .project
                                                    .package_names
                                                    .insert(rel_path.clone(), name.to_string());
                                            }
                                            config.project.packages.push(rel_path);
                                        }
                                    }
                                }
                            }
                        } else {
                            // Direct path to package
                            let pkg_dir = root.join(pattern);
                            let cargo_toml = pkg_dir.join("Cargo.toml");
                            if let Ok(content) = std::fs::read_to_string(&cargo_toml)
                                && let Ok(value) = content.parse::<toml::Value>()
                                && let Some(name) = value
                                    .get("package")
                                    .and_then(|p| p.get("name"))
                                    .and_then(|n| n.as_str())
                            {
                                config
                                    .project
                                    .package_names
                                    .insert(pattern.clone(), name.to_string());
                            }
                            config.project.packages.push(pattern.clone());
                        }
                    }
                    config.project.packages.sort();
                    tracing::debug!(
                        "auto-detected workspace packages: {:?}",
                        config.project.packages
                    );
                    tracing::debug!("package names: {:?}", config.project.package_names);
                }
            }
        }
        ProjectLanguage::Go => {
            // Ignore vendor/ directory for Go projects
            if !ignore_patterns.iter().any(|p| p.contains("vendor")) {
                ignore_patterns.push("vendor".to_string());
            }
        }
        ProjectLanguage::Shell => {
            // No special ignore patterns for Shell projects
        }
        ProjectLanguage::JavaScript => {
            // Ignore node_modules, dist, build for JS projects
            for pattern in ["node_modules", "dist", "build", ".next", "coverage"] {
                if !ignore_patterns.iter().any(|p| p.contains(pattern)) {
                    ignore_patterns.push(pattern.to_string());
                }
            }

            // Auto-detect workspace packages if not configured
            if config.project.packages.is_empty() {
                let workspace = JsWorkspace::from_root(&root);
                if workspace.is_workspace {
                    for path in &workspace.package_paths {
                        config.project.packages.push(path.clone());
                    }
                    config.project.package_names = workspace.package_names.clone();
                    tracing::debug!(
                        "auto-detected JS workspace packages: {:?}",
                        config.project.packages
                    );
                    tracing::debug!("package names: {:?}", config.project.package_names);
                }
            }
        }
        ProjectLanguage::Ruby => {
            // Ignore vendor, tmp, log, coverage for Ruby projects
            for pattern in ["vendor", "tmp", "log", "coverage"] {
                if !ignore_patterns.iter().any(|p| p.contains(pattern)) {
                    ignore_patterns.push(pattern.to_string());
                }
            }
        }
        ProjectLanguage::Generic => {}
    }

    let walker_config = WalkerConfig {
        max_depth: Some(args.max_depth),
        ignore_patterns,
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
        if debug_logging() {
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

    // Report stats in verbose mode
    if debug_logging() {
        eprintln!("Max depth limit: {}", args.max_depth);
        if stats.symlink_loops > 0 {
            eprintln!("Warning: {} symlink loop(s) detected", stats.symlink_loops);
        }
        if stats.errors > 0 {
            eprintln!("Warning: {} walk error(s)", stats.errors);
        }
        if stats.files_skipped_size > 0 {
            eprintln!("{} file(s) skipped (>10MB limit)", stats.files_skipped_size);
        }
        eprintln!("Scanned {} files", files.len());
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
                if debug_logging() {
                    eprintln!("Checking staged files");
                    eprintln!("{} files staged", files.len());
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
                if debug_logging() {
                    eprintln!("Comparing against base: {}", base);
                    eprintln!("{} files changed", files.len());
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
    let mut runner = CheckRunner::new(RunnerConfig {
        limit,
        changed_files,
        fix: args.fix,
        dry_run: args.dry_run,
        ci_mode: args.ci,
        base_branch,
        staged: args.staged,
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
    if debug_logging()
        && let Some(cache) = &cache
    {
        let stats = cache.stats();
        eprintln!(
            "Cache: {} hits, {} misses, {} entries",
            stats.hits, stats.misses, stats.entries
        );
    }

    // Create output
    let output = json::create_output(check_results);
    let total_violations = output.total_violations();

    // Ratchet checking (cache baseline for potential --fix reuse)
    let baseline_path = root.join(&config.git.baseline);
    let (ratchet_result, baseline) = if config.ratchet.check != CheckLevel::Off {
        match Baseline::load(&baseline_path) {
            Ok(Some(baseline)) => {
                // Warn if baseline is stale
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
                // No baseline yet - pass but suggest creating one
                if debug_logging() {
                    eprintln!(
                        "No baseline found at {}. Run with --fix to create.",
                        baseline_path.display()
                    );
                }
                (None, None)
            }
            Err(e) => {
                eprintln!("quench: warning: failed to load baseline: {}", e);
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    // Handle --fix: update/sync baseline (reusing cached baseline)
    if args.fix {
        let baseline_existed = baseline_path.exists();
        let current = CurrentMetrics::from_output(&output);

        // Use existing baseline or create new
        let mut baseline = baseline
            .map(|b| b.with_commit(&root))
            .unwrap_or_else(|| Baseline::new().with_commit(&root));

        ratchet::update_baseline(&mut baseline, &current);

        if let Err(e) = baseline.save(&baseline_path) {
            eprintln!("quench: warning: failed to save baseline: {}", e);
        } else {
            // Report what happened (to stderr to not interfere with JSON output)
            if !baseline_existed {
                eprintln!(
                    "ratchet: created initial baseline at {}",
                    baseline_path.display()
                );
            } else if let Some(ref result) = ratchet_result {
                if result.improvements.is_empty() {
                    eprintln!("ratchet: baseline synced");
                } else {
                    eprintln!("ratchet: updated baseline");
                    for improvement in &result.improvements {
                        eprintln!(
                            "  {}: {} -> {} (new ceiling)",
                            improvement.name,
                            improvement.format_value(improvement.old_value),
                            improvement.format_value(improvement.new_value),
                        );
                    }
                }
            } else {
                // No comparison was done (ratchet disabled or initial creation)
                eprintln!("ratchet: baseline synced");
            }
        }
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
        } else if debug_logging() {
            eprintln!("Saved metrics to {}", save_path.display());
        }
    }

    // Save metrics to git notes if requested
    if args.save_notes {
        if !is_git_repo(&root) {
            eprintln!("quench: error: not a git repository");
            return Ok(ExitCode::ConfigError);
        }

        let json = serde_json::to_string(&output)?;
        if let Err(e) = save_to_git_notes(&root, &json) {
            eprintln!("quench: warning: failed to save to git notes: {}", e);
        } else if debug_logging() {
            eprintln!("Saved metrics to git notes (refs/notes/quench)");
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
