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

#[path = "specs/cli/init.rs"]
mod cli_init;

#[path = "specs/cli/commands.rs"]
mod cli_commands;

#[path = "specs/cli/flags.rs"]
mod cli_flags;

#[path = "specs/cli/toggles.rs"]
mod cli_toggles;

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

#[path = "specs/checks/agents/mod.rs"]
mod checks_agents;

#[path = "specs/checks/docs/mod.rs"]
mod checks_docs;

#[path = "specs/checks/tests/mod.rs"]
mod checks_tests;

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

// adapters/
#[path = "specs/adapters/mod.rs"]
mod adapters;
