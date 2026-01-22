//! Quench CLI entry point.

use std::io::BufRead;

use clap::{CommandFactory, Parser};
use termcolor::ColorChoice;
use tracing_subscriber::{EnvFilter, fmt};

use quench::check::{CheckResult, Violation};
use quench::cli::{CheckArgs, Cli, Command, InitArgs, OutputFormat, ReportArgs};
use quench::color::is_no_color_env;
use quench::config::{self, CheckLevel};
use quench::discovery;
use quench::error::ExitCode;
use quench::output::FormatOptions;
use quench::output::json::{self, JsonFormatter};
use quench::output::text::TextFormatter;
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

    // Run checks
    let mut check_results = Vec::new();

    // Run cloc check if enabled
    if config.check.cloc.check != CheckLevel::Off {
        let cloc_result = run_cloc_check(&config, &files, &root);
        check_results.push(cloc_result);
    }

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

/// Run the cloc (count lines of code) check.
fn run_cloc_check(
    config: &config::Config,
    files: &[quench::walker::WalkedFile],
    root: &std::path::Path,
) -> CheckResult {
    let cloc_config = &config.check.cloc;
    let mut violations = Vec::new();

    for file in files {
        // Skip non-text files (binary files, etc.)
        if !is_text_file(&file.path) {
            continue;
        }

        // Count lines
        match count_lines(&file.path) {
            Ok(line_count) => {
                let is_test = is_test_file(&file.path);
                let max_lines = if is_test {
                    cloc_config.max_lines_test
                } else {
                    cloc_config.max_lines
                };

                if line_count > max_lines {
                    // Make path relative for output
                    let display_path = file.path.strip_prefix(root).unwrap_or(&file.path);

                    violations.push(
                        Violation::file_only(
                            display_path,
                            "file_too_large",
                            format!(
                                "Split into smaller modules. {} lines exceeds {} line limit.",
                                line_count, max_lines
                            ),
                        )
                        .with_threshold(line_count as i64, max_lines as i64),
                    );
                }
            }
            Err(e) => {
                tracing::warn!("failed to count lines in {}: {}", file.path.display(), e);
            }
        }
    }

    if violations.is_empty() {
        CheckResult::passed("cloc")
    } else {
        CheckResult::failed("cloc", violations)
    }
}

/// Check if a file appears to be a text file (not binary).
fn is_text_file(path: &std::path::Path) -> bool {
    // Check extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Common source code extensions
    matches!(
        ext.as_str(),
        "rs" | "py"
            | "js"
            | "ts"
            | "jsx"
            | "tsx"
            | "go"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "java"
            | "kt"
            | "scala"
            | "rb"
            | "php"
            | "cs"
            | "swift"
            | "m"
            | "mm"
            | "sh"
            | "bash"
            | "zsh"
            | "fish"
            | "ps1"
            | "bat"
            | "cmd"
            | "lua"
            | "pl"
            | "pm"
            | "r"
            | "sql"
            | "md"
            | "txt"
            | "toml"
            | "yaml"
            | "yml"
            | "json"
            | "xml"
            | "html"
            | "css"
            | "scss"
            | "sass"
            | "less"
            | "vue"
            | "svelte"
    )
}

/// Check if a file is a test file based on its filename.
/// Only uses filename patterns to avoid false positives from parent directory names.
fn is_test_file(path: &std::path::Path) -> bool {
    // Get just the file name
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // File name patterns that indicate test files
    file_name.contains("_test.")
        || file_name.contains("_tests.")
        || file_name.contains(".test.")
        || file_name.contains(".spec.")
        || file_name.ends_with("_test.rs")
        || file_name.ends_with("_tests.rs")
        || file_name.ends_with("_test.go")
        || file_name.ends_with("_test.py")
        || file_name.ends_with(".test.js")
        || file_name.ends_with(".test.ts")
        || file_name.ends_with(".test.tsx")
        || file_name.ends_with(".spec.js")
        || file_name.ends_with(".spec.ts")
        || file_name.ends_with(".spec.tsx")
}

/// Count the number of lines in a file.
fn count_lines(path: &std::path::Path) -> std::io::Result<usize> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    Ok(reader.lines().count())
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
