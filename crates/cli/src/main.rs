// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Quench CLI entry point.

use std::io;

use clap::{CommandFactory, Parser, error::ErrorKind};
use clap_complete::generate;
use tracing_subscriber::{EnvFilter, fmt};

use quench::cli::{Cli, Command};
use quench::error::ExitCode;
use quench::help::format_help;

mod cmd_check;
mod cmd_cloc;
mod cmd_config;
mod cmd_report;

fn init_logging() {
    let filter = EnvFilter::try_from_env(quench::env::quench_log_var())
        .unwrap_or_else(|_| EnvFilter::new("off"));

    fmt().with_env_filter(filter).with_writer(std::io::stderr).with_target(false).init();
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
    // Use try_parse to intercept help display
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            return match e.kind() {
                ErrorKind::DisplayHelp => {
                    // Use custom help formatter
                    print_custom_help(&std::env::args().collect::<Vec<_>>());
                    Ok(ExitCode::Success)
                }
                ErrorKind::DisplayVersion => {
                    // Let clap handle version display
                    e.print()?;
                    Ok(ExitCode::Success)
                }
                _ => {
                    // Let clap handle other errors (including DisplayHelpOnMissingArgumentOrSubcommand)
                    e.exit();
                }
            };
        }
    };

    match &cli.command {
        None => {
            // Show help for bare invocation
            print!("{}", format_help(&mut Cli::command()));
            println!();
            Ok(ExitCode::Success)
        }
        Some(Command::Check(args)) => cmd_check::run(&cli, args),
        Some(Command::Cloc(args)) => cmd_cloc::run(args),
        Some(Command::Report(args)) => {
            cmd_report::run(&cli, args)?;
            Ok(ExitCode::Success)
        }
        Some(Command::Init(args)) => quench::cmd_init::run(args),
        Some(Command::Config(args)) => cmd_config::run(args),
        Some(Command::Completions(args)) => {
            let mut cmd = Cli::command();
            generate(args.shell, &mut cmd, "quench", &mut io::stdout());
            Ok(ExitCode::Success)
        }
    }
}

/// Print custom help based on the subcommand in args.
fn print_custom_help(args: &[String]) {
    let mut cmd = Cli::command();

    // Check if a subcommand was specified
    let subcommand = args.iter().skip(1).find(|arg| !arg.starts_with('-'));

    match subcommand.map(|s| s.as_str()) {
        Some("check") => {
            if let Some(subcmd) = cmd.find_subcommand_mut("check") {
                print!("{}", format_help(subcmd));
            }
        }
        Some("cloc") => {
            if let Some(subcmd) = cmd.find_subcommand_mut("cloc") {
                print!("{}", format_help(subcmd));
            }
        }
        Some("report") => {
            if let Some(subcmd) = cmd.find_subcommand_mut("report") {
                print!("{}", format_help(subcmd));
            }
        }
        Some("init") => {
            if let Some(subcmd) = cmd.find_subcommand_mut("init") {
                print!("{}", format_help(subcmd));
            }
        }
        Some("config") => {
            if let Some(subcmd) = cmd.find_subcommand_mut("config") {
                print!("{}", format_help(subcmd));
            }
        }
        Some("completions") => {
            if let Some(subcmd) = cmd.find_subcommand_mut("completions") {
                print!("{}", format_help(subcmd));
            }
        }
        Some("help") => {
            // Handle `quench help <subcommand>`
            let next_arg = args.iter().skip(2).find(|arg| !arg.starts_with('-'));
            match next_arg.map(|s| s.as_str()) {
                Some("check") => {
                    if let Some(subcmd) = cmd.find_subcommand_mut("check") {
                        print!("{}", format_help(subcmd));
                    }
                }
                Some("report") => {
                    if let Some(subcmd) = cmd.find_subcommand_mut("report") {
                        print!("{}", format_help(subcmd));
                    }
                }
                Some("init") => {
                    if let Some(subcmd) = cmd.find_subcommand_mut("init") {
                        print!("{}", format_help(subcmd));
                    }
                }
                Some("config") => {
                    if let Some(subcmd) = cmd.find_subcommand_mut("config") {
                        print!("{}", format_help(subcmd));
                    }
                }
                Some("completions") => {
                    if let Some(subcmd) = cmd.find_subcommand_mut("completions") {
                        print!("{}", format_help(subcmd));
                    }
                }
                _ => {
                    print!("{}", format_help(&mut cmd));
                }
            }
        }
        _ => {
            print!("{}", format_help(&mut cmd));
        }
    }
    println!();
}
