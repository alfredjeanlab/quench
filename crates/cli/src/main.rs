// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Quench CLI entry point.

use clap::{CommandFactory, Parser};
use tracing_subscriber::{EnvFilter, fmt};

use quench::cli::{Cli, Command};
use quench::error::ExitCode;

mod cmd_check;
mod cmd_report;

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
        Some(Command::Check(args)) => cmd_check::run(&cli, args),
        Some(Command::Report(args)) => {
            cmd_report::run(&cli, args)?;
            Ok(ExitCode::Success)
        }
        Some(Command::Init(args)) => quench::cmd_init::run(args),
    }
}
