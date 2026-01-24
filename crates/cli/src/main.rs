// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Quench CLI entry point.

mod git;

use std::sync::Arc;

use clap::{CommandFactory, Parser};
use tracing_subscriber::{EnvFilter, fmt};

use quench::adapter::{JsWorkspace, ProjectLanguage, detect_language, rust::CargoWorkspace};
use quench::baseline::Baseline;
use quench::cache::{self, CACHE_FILE_NAME, FileCache};
use quench::checks;
use quench::cli::{CheckArgs, CheckFilter, Cli, Command, InitArgs, OutputFormat, ReportArgs};
use quench::color::{is_no_color_env, resolve_color};
use quench::config::{self, CheckLevel};
use quench::discovery;
use quench::error::ExitCode;
use quench::init::{DetectedAgent, DetectedLanguage, detect_agents, detect_languages};
use quench::output::FormatOptions;
use quench::output::json::{self, JsonFormatter};
use quench::output::text::TextFormatter;
use quench::ratchet::{self, CurrentMetrics};
use quench::runner::{CheckRunner, RunnerConfig};
use quench::walker::{FileWalker, WalkerConfig};

use git::{detect_base_branch, get_changed_files, get_staged_files};
use quench::report;

fn init_logging() {
    let filter = EnvFilter::try_from_env("QUENCH_LOG").unwrap_or_else(|_| EnvFilter::new("off"));

    fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();
}

fn main() {
    init_logging();

    let exit_code = match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("quench: {}", e);
            match e.downcast_ref::<quench::Error>() {
                Some(err) => ExitCode::from(err),
                None => ExitCode::InternalError,
            }
        }
    };

    std::process::exit(exit_code as i32);
}

fn run() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();

    match &cli.command {
        None => {
            // Show help for bare invocation
            Cli::command().print_help()?;
            println!();
            Ok(ExitCode::Success)
        }
        Some(Command::Check(args)) => run_check(&cli, args),
        Some(Command::Report(args)) => {
            run_report(&cli, args)?;
            Ok(ExitCode::Success)
        }
        Some(Command::Init(args)) => run_init(&cli, args),
    }
}

fn run_check(cli: &Cli, args: &CheckArgs) -> anyhow::Result<ExitCode> {
    // Validate flag combinations
    if args.dry_run && !args.fix {
        eprintln!("--dry-run requires --fix");
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

    // Resolve config from root directory (not cwd)
    let config_path = if cli.config.is_some() {
        discovery::resolve_config(cli.config.as_deref(), &cwd)?
    } else {
        discovery::find_config(&root)
    };

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

    // Config-only mode: validate and exit
    if args.config_only {
        return Ok(ExitCode::Success);
    }

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
        ProjectLanguage::Generic => {}
    }

    let walker_config = WalkerConfig {
        max_depth: Some(args.max_depth),
        ignore_patterns,
        ..Default::default()
    };

    let walker = FileWalker::new(walker_config);
    let (rx, handle) = walker.walk(&root);

    // Process files
    if args.debug_files {
        // Debug mode: just list files
        for file in rx {
            // Make paths relative to root for cleaner output
            let display_path = file.path.strip_prefix(&root).unwrap_or(&file.path);
            println!("{}", display_path.display());
        }
        let stats = handle.join();
        if args.verbose {
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

    // Report stats in verbose mode
    if args.verbose {
        eprintln!("Max depth limit: {}", args.max_depth);
        if stats.symlink_loops > 0 {
            eprintln!("Warning: {} symlink loop(s) detected", stats.symlink_loops);
        }
        if stats.errors > 0 {
            eprintln!("Warning: {} walk error(s)", stats.errors);
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
                if args.verbose {
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
                if args.verbose {
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
    let limit = if args.no_limit {
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

    // Run checks
    let check_results = runner.run(checks, &files, &config, &root);

    // Persist cache (best effort)
    if let Some(cache) = &cache {
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            tracing::warn!("failed to create cache directory: {}", e);
        } else if let Err(e) = cache.persist(&cache_path) {
            tracing::warn!("failed to persist cache: {}", e);
        } else {
            tracing::debug!("persisted cache to {}", cache_path.display());
        }

        // Report cache stats in verbose mode
        if args.verbose {
            let stats = cache.stats();
            eprintln!(
                "Cache: {} hits, {} misses, {} entries",
                stats.hits, stats.misses, stats.entries
            );
        }
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
                if args.verbose {
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
    let color_choice = resolve_color(args.color, args.no_color || is_no_color_env());

    // Set up formatter options
    let limit = if args.no_limit {
        None
    } else {
        Some(args.limit)
    };
    let options = FormatOptions { limit };

    // Format output
    match args.output {
        OutputFormat::Text | OutputFormat::Html => {
            // HTML uses text format for check command (HTML is for report only)
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
            formatter.write_with_ratchet(&output, ratchet_result.as_ref())?;
        }
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

fn run_report(cli: &Cli, args: &ReportArgs) -> anyhow::Result<()> {
    use std::io::Write;

    let cwd = std::env::current_dir()?;

    // Find and load config
    let config = if let Some(ref path) = cli.config {
        config::load_with_warnings(path)?
    } else if let Some(path) = discovery::find_config(&cwd) {
        config::load_with_warnings(&path)?
    } else {
        config::Config::default()
    };

    // Determine baseline path
    let baseline_path = cwd.join(&config.git.baseline);

    // Parse output target (format and optional file path)
    let (format, file_path) = args.output_target();

    // Validate --compact flag (only applies to JSON)
    if args.compact && !matches!(format, OutputFormat::Json) {
        eprintln!("warning: --compact only applies to JSON output, ignoring");
    }

    // Load baseline
    let baseline = Baseline::load(&baseline_path)?;

    // Write output using streaming when possible
    match file_path {
        Some(path) => {
            // File output: use buffered writer for efficiency
            let file = std::fs::File::create(&path)?;
            let mut writer = std::io::BufWriter::new(file);
            report::format_report_to(&mut writer, format, baseline.as_ref(), args, args.compact)?;
            writer.flush()?;
        }
        None => {
            // Stdout: use stdout lock for efficiency
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            report::format_report_to(&mut handle, format, baseline.as_ref(), args, args.compact)?;
            // Add trailing newline for JSON output
            if matches!(format, OutputFormat::Json) {
                writeln!(handle)?;
            }
        }
    }
    Ok(())
}

fn run_init(_cli: &Cli, args: &InitArgs) -> anyhow::Result<ExitCode> {
    use quench::profiles::{
        ProfileRegistry, agents_section, default_template_base, default_template_suffix,
        golang_detected_section, javascript_detected_section, rust_detected_section,
        shell_detected_section,
    };

    let cwd = std::env::current_dir()?;
    let config_path = cwd.join("quench.toml");

    if config_path.exists() && !args.force {
        eprintln!("quench.toml already exists. Use --force to overwrite.");
        return Ok(ExitCode::ConfigError);
    }

    // Determine what to include
    let (config, message) = if !args.with_profiles.is_empty() {
        // --with specified: use full profiles, skip detection
        // Separate agent profiles from language profiles since agents replace agents section
        let mut agent_required: Vec<&str> = Vec::new();
        let mut lang_config = String::new();

        for profile in &args.with_profiles {
            if ProfileRegistry::is_agent_profile(profile) {
                // Agent profile: collect required files
                match profile.to_lowercase().as_str() {
                    "claude" => {
                        if !agent_required.contains(&"CLAUDE.md") {
                            agent_required.push("CLAUDE.md");
                        }
                    }
                    "cursor" => {
                        if !agent_required.contains(&".cursorrules") {
                            agent_required.push(".cursorrules");
                        }
                    }
                    _ => {}
                }
            } else if let Some(content) = ProfileRegistry::get(profile) {
                // Language profile: append to config
                lang_config.push('\n');
                lang_config.push_str(&content);
            } else {
                // Unknown profile: warn and suggest
                if let Some(suggestion) = ProfileRegistry::suggest(profile) {
                    eprintln!(
                        "quench: warning: unknown profile '{}', did you mean '{}'?",
                        profile, suggestion
                    );
                } else {
                    eprintln!("quench: warning: unknown profile '{}', skipping", profile);
                }
            }
        }

        // Build final config
        let mut cfg = default_template_base().to_string();
        if !agent_required.is_empty() {
            cfg.push_str(&format!(
                "[check.agents]\ncheck = \"error\"\nrequired = {:?}\n",
                agent_required
            ));
        } else {
            cfg.push_str(&agents_section(&[]));
        }
        cfg.push_str(default_template_suffix());
        cfg.push_str(&lang_config);

        let msg = format!(
            "Created quench.toml with profile(s): {}",
            args.with_profiles.join(", ")
        );
        (cfg, msg)
    } else {
        // No --with: run auto-detection for both languages and agents
        let detected_langs = detect_languages(&cwd);
        let detected_agents = detect_agents(&cwd);

        // Build config with proper agents section placement
        let mut cfg = default_template_base().to_string();
        cfg.push_str(&agents_section(&detected_agents));
        cfg.push_str(default_template_suffix());

        // Add language sections (after # Supported Languages:)
        for lang in &detected_langs {
            cfg.push('\n');
            match lang {
                DetectedLanguage::Rust => cfg.push_str(rust_detected_section()),
                DetectedLanguage::Golang => cfg.push_str(golang_detected_section()),
                DetectedLanguage::JavaScript => cfg.push_str(javascript_detected_section()),
                DetectedLanguage::Shell => cfg.push_str(shell_detected_section()),
            }
        }

        // Build message listing detected items
        let mut detected_names = Vec::new();
        for lang in &detected_langs {
            detected_names.push(match lang {
                DetectedLanguage::Rust => "rust",
                DetectedLanguage::Golang => "golang",
                DetectedLanguage::JavaScript => "javascript",
                DetectedLanguage::Shell => "shell",
            });
        }
        for agent in &detected_agents {
            detected_names.push(match agent {
                DetectedAgent::Claude => "claude",
                DetectedAgent::Cursor(_) => "cursor",
            });
        }

        let msg = if detected_names.is_empty() {
            "Created quench.toml".to_string()
        } else {
            format!(
                "Created quench.toml (detected: {})",
                detected_names.join(", ")
            )
        };
        (cfg, msg)
    };

    std::fs::write(&config_path, config)?;
    println!("{}", message);
    Ok(ExitCode::Success)
}
