//! Quench CLI entry point.

use clap::{CommandFactory, Parser};
use termcolor::ColorChoice;
use tracing_subscriber::{EnvFilter, fmt};

use quench::checks;
use quench::cli::{CheckArgs, Cli, Command, InitArgs, OutputFormat, ReportArgs};
use quench::color::is_no_color_env;
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

    let config = match &config_path {
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

    // Configure walker
    let walker_config = WalkerConfig {
        max_depth: Some(args.max_depth),
        ignore_patterns: config.project.ignore.patterns.clone(),
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

    // Create runner
    let limit = if args.no_limit {
        None
    } else {
        Some(args.limit)
    };
    let runner = CheckRunner::new(RunnerConfig { limit });

    // Run checks
    let check_results = runner.run(checks, &files, &config, &root);

    // Create output
    let output = json::create_output(check_results);
    let total_violations = output.total_violations();

    // Resolve color mode
    let color_choice = if args.no_color || is_no_color_env() {
        ColorChoice::Never
    } else {
        args.color.resolve()
    };

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

fn run_report(_cli: &Cli, args: &ReportArgs) -> anyhow::Result<()> {
    match args.output {
        OutputFormat::Text => println!("No metrics collected yet."),
        OutputFormat::Json => println!(r#"{{"metrics": {{}}}}"#),
    }
    Ok(())
}

fn run_init(_cli: &Cli, args: &InitArgs) -> anyhow::Result<ExitCode> {
    let cwd = std::env::current_dir()?;
    let config_path = cwd.join("quench.toml");

    if config_path.exists() && !args.force {
        eprintln!("quench.toml already exists. Use --force to overwrite.");
        return Ok(ExitCode::ConfigError);
    }

    std::fs::write(&config_path, "version = 1\n")?;
    println!("Created quench.toml");
    Ok(ExitCode::Success)
}
