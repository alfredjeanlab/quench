// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! `quench config` command implementation.

use std::io::Write;

use anyhow::{Context, Result, bail};
use clap::CommandFactory;

use quench::cli::{Cli, ConfigArgs};
use quench::color;
use quench::error::ExitCode;
use quench::help::format_help;

/// Template files are embedded at compile time
const TEMPLATES: &[(&str, &str)] = &[
    (
        "agents",
        include_str!("../../../docs/specs/templates/guide.agents.md"),
    ),
    (
        "build",
        include_str!("../../../docs/specs/templates/guide.build.md"),
    ),
    (
        "cloc",
        include_str!("../../../docs/specs/templates/guide.cloc.md"),
    ),
    (
        "docs",
        include_str!("../../../docs/specs/templates/guide.docs.md"),
    ),
    (
        "escapes",
        include_str!("../../../docs/specs/templates/guide.escapes.md"),
    ),
    (
        "git",
        include_str!("../../../docs/specs/templates/guide.git.md"),
    ),
    (
        "golang",
        include_str!("../../../docs/specs/templates/guide.golang.md"),
    ),
    (
        "go",
        include_str!("../../../docs/specs/templates/guide.golang.md"),
    ),
    (
        "javascript",
        include_str!("../../../docs/specs/templates/guide.javascript.md"),
    ),
    (
        "js",
        include_str!("../../../docs/specs/templates/guide.javascript.md"),
    ),
    (
        "typescript",
        include_str!("../../../docs/specs/templates/guide.javascript.md"),
    ),
    (
        "ts",
        include_str!("../../../docs/specs/templates/guide.javascript.md"),
    ),
    (
        "license",
        include_str!("../../../docs/specs/templates/guide.license.md"),
    ),
    (
        "python",
        include_str!("../../../docs/specs/templates/guide.python.md"),
    ),
    (
        "py",
        include_str!("../../../docs/specs/templates/guide.python.md"),
    ),
    (
        "ruby",
        include_str!("../../../docs/specs/templates/guide.ruby.md"),
    ),
    (
        "rb",
        include_str!("../../../docs/specs/templates/guide.ruby.md"),
    ),
    (
        "rust",
        include_str!("../../../docs/specs/templates/guide.rust.md"),
    ),
    (
        "rs",
        include_str!("../../../docs/specs/templates/guide.rust.md"),
    ),
    (
        "shell",
        include_str!("../../../docs/specs/templates/guide.shell.md"),
    ),
    (
        "sh",
        include_str!("../../../docs/specs/templates/guide.shell.md"),
    ),
    (
        "bash",
        include_str!("../../../docs/specs/templates/guide.shell.md"),
    ),
    (
        "tests",
        include_str!("../../../docs/specs/templates/guide.tests.md"),
    ),
];

pub fn run(args: &ConfigArgs) -> Result<ExitCode> {
    let feature = match &args.feature {
        Some(f) => f.to_lowercase(),
        None => {
            let mut cmd = Cli::command();
            if let Some(subcmd) = cmd.find_subcommand_mut("config") {
                print!("{}", format_help(subcmd));
                println!();
            }
            println!();
            println!("{}", color::header("Available features:"));
            println!("  Checks:    {}", color::literal("agents, build, cloc, docs, escapes, git, license, tests"));
            println!("  Languages: {}", color::literal("golang (go), javascript (js/ts), python (py), ruby (rb),"));
            println!("             {}", color::literal("rust (rs), shell (sh/bash)"));
            return Ok(ExitCode::Success);
        }
    };

    // Find the template
    let template = TEMPLATES
        .iter()
        .find(|(name, _)| *name == feature)
        .map(|(_, content)| *content);

    match template {
        Some(content) => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            let colored = color::guide(content);
            handle
                .write_all(colored.as_bytes())
                .context("Failed to write template to stdout")?;
            Ok(ExitCode::Success)
        }
        None => {
            bail!(
                "Unknown feature '{}'\n\n\
                Available features:\n\
                  Checks:  agents, build, cloc, docs, escapes, git, license, tests\n\
                  Languages: golang (go), javascript (js/ts/typescript), python (py), ruby (rb), rust (rs), shell (sh/bash)",
                feature
            );
        }
    }
}
