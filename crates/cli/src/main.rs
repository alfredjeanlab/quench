//! Quench CLI entry point.

use std::sync::Arc;

use clap::{CommandFactory, Parser};
use tracing_subscriber::{EnvFilter, fmt};

use quench::adapter::{ProjectLanguage, detect_language, rust::CargoWorkspace};
use quench::cache::{self, CACHE_FILE_NAME, FileCache};
use quench::checks;
use quench::cli::{CheckArgs, Cli, Command, InitArgs, OutputFormat, ReportArgs};
use quench::color::{is_no_color_env, resolve_color};
use quench::config;
use quench::discovery;
use quench::error::ExitCode;
use quench::output::FormatOptions;
use quench::output::json::{self, JsonFormatter};
use quench::output::text::TextFormatter;
use quench::runner::{CheckRunner, RunnerConfig};
use quench::walker::{FileWalker, WalkerConfig};

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
            if config.workspace.packages.is_empty() {
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
                                                    .workspace
                                                    .package_names
                                                    .insert(rel_path.clone(), name.to_string());
                                            }
                                            config.workspace.packages.push(rel_path);
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
                                    .workspace
                                    .package_names
                                    .insert(pattern.clone(), name.to_string());
                            }
                            config.workspace.packages.push(pattern.clone());
                        }
                    }
                    config.workspace.packages.sort();
                    tracing::debug!(
                        "auto-detected workspace packages: {:?}",
                        config.workspace.packages
                    );
                    tracing::debug!("package names: {:?}", config.workspace.package_names);
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

    // Get changed files if --base is provided
    let changed_files = if let Some(ref base) = args.base {
        match get_changed_files(&root, base) {
            Ok(files) => {
                if args.verbose {
                    eprintln!("Comparing against base: {}", base);
                    eprintln!("{} files changed", files.len());
                }
                Some(files)
            }
            Err(e) => {
                eprintln!("quench: warning: could not get changed files: {}", e);
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
        OutputFormat::Text => {
            let mut formatter = TextFormatter::new(color_choice, options);

            for result in &output.checks {
                formatter.write_check(result)?;
            }

            formatter.write_summary(&output)?;

            if formatter.was_truncated() {
                formatter.write_truncation_message(total_violations)?;
            }
        }
        OutputFormat::Json => {
            let mut formatter = JsonFormatter::new(std::io::stdout());
            formatter.write(&output)?;
        }
    }

    // Determine exit code
    let exit_code = if !output.passed {
        ExitCode::CheckFailed
    } else {
        ExitCode::Success
    };

    Ok(exit_code)
}

/// Get list of changed files compared to a git base ref.
fn get_changed_files(
    root: &std::path::Path,
    base: &str,
) -> anyhow::Result<Vec<std::path::PathBuf>> {
    use std::process::Command;

    // Get staged/unstaged changes (diffstat against base)
    let output = Command::new("git")
        .args(["diff", "--name-only", base])
        .current_dir(root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git diff failed: {}", stderr.trim());
    }

    // Also get staged changes
    let staged_output = Command::new("git")
        .args(["diff", "--name-only", "--cached", base])
        .current_dir(root)
        .output()?;

    let mut files: std::collections::HashSet<std::path::PathBuf> = std::collections::HashSet::new();

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if !line.is_empty() {
            files.insert(root.join(line));
        }
    }

    if staged_output.status.success() {
        for line in String::from_utf8_lossy(&staged_output.stdout).lines() {
            if !line.is_empty() {
                files.insert(root.join(line));
            }
        }
    }

    Ok(files.into_iter().collect())
}

fn run_report(_cli: &Cli, args: &ReportArgs) -> anyhow::Result<()> {
    match args.output {
        OutputFormat::Text => println!("No metrics collected yet."),
        OutputFormat::Json => println!(r#"{{"metrics": {{}}}}"#),
    }
    Ok(())
}

fn run_init(_cli: &Cli, args: &InitArgs) -> anyhow::Result<ExitCode> {
    use quench::cli::rust_profile_defaults;

    let cwd = std::env::current_dir()?;
    let config_path = cwd.join("quench.toml");

    if config_path.exists() && !args.force {
        eprintln!("quench.toml already exists. Use --force to overwrite.");
        return Ok(ExitCode::ConfigError);
    }

    // Build config based on profiles
    let mut config = String::from("version = 1\n");

    for profile in &args.profile {
        match profile.as_str() {
            "rust" => {
                config.push('\n');
                config.push_str(&rust_profile_defaults());
            }
            other => {
                eprintln!("quench: warning: unknown profile '{}', skipping", other);
            }
        }
    }

    std::fs::write(&config_path, config)?;
    if args.profile.is_empty() {
        println!("Created quench.toml");
    } else {
        println!(
            "Created quench.toml with profile(s): {}",
            args.profile.join(", ")
        );
    }
    Ok(ExitCode::Success)
}
