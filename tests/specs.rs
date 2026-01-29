// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specifications for quench CLI.
//!
//! These tests are black-box: they invoke the CLI binary and verify
//! stdout, stderr, and exit codes. See CLAUDE.md for conventions.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "specs/prelude.rs"]
mod prelude;

// cli/
#[path = "specs/cli/dry_run.rs"]
mod cli_dry_run;

#[path = "specs/cli/init_tests/mod.rs"]
mod cli_init;

#[path = "specs/cli/commands.rs"]
mod cli_commands;

#[path = "specs/cli/flags.rs"]
mod cli_flags;

#[path = "specs/cli/toggles.rs"]
mod cli_toggles;

#[path = "specs/cli/report.rs"]
mod cli_report;

#[path = "specs/cli/timing.rs"]
mod cli_timing;

#[path = "specs/cli/ci_mode.rs"]
mod cli_ci_mode;

#[path = "specs/cli/help.rs"]
mod cli_help;

// config/
#[path = "specs/config/mod.rs"]
mod config;

// checks/
#[path = "specs/checks/cloc.rs"]
mod checks_cloc;

#[path = "specs/checks/cloc_lang.rs"]
mod checks_cloc_lang;

#[path = "specs/checks/escapes.rs"]
mod checks_escapes;

#[path = "specs/checks/policy_lang.rs"]
mod checks_policy_lang;

#[path = "specs/checks/placeholders.rs"]
mod checks_placeholders;

#[path = "specs/checks/agents/mod.rs"]
mod checks_agents;

#[path = "specs/checks/docs/mod.rs"]
mod checks_docs;

#[path = "specs/checks/tests/mod.rs"]
mod checks_tests;

#[path = "specs/checks/git.rs"]
mod checks_git;

#[path = "specs/checks/build/mod.rs"]
mod checks_build;

#[path = "specs/checks/license.rs"]
mod checks_license;

// output/
#[path = "specs/output/format.rs"]
mod output_format;

// modes/
#[path = "specs/modes/cache.rs"]
mod modes_cache;

#[path = "specs/modes/file_walking.rs"]
mod modes_file_walking;

#[path = "specs/modes/ratchet.rs"]
mod modes_ratchet;

#[path = "specs/modes/large_files.rs"]
mod modes_large_files;

// adapters/
#[path = "specs/adapters/mod.rs"]
mod adapters;

// verbose output/
#[path = "specs/verbose.rs"]
mod verbose;
