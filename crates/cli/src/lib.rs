// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

pub mod adapter;
pub mod cache;
pub mod check;
pub mod checks;
pub mod cli;
pub mod color;
pub mod config;
pub mod discovery;
pub mod error;
pub mod output;
pub mod pattern;
pub mod runner;
pub mod walker;

pub use cache::{CacheStats, FileCache};
pub use check::{Check, CheckContext, CheckOutput, CheckResult, Violation};
pub use cli::{CheckArgs, Cli, Command, InitArgs, OutputFormat, ReportArgs};
pub use color::{is_no_color_env, resolve_color};
pub use config::IgnoreConfig;
pub use error::{Error, ExitCode, Result};
pub use walker::{FileWalker, WalkStats, WalkedFile, WalkerConfig};

#[cfg(test)]
pub mod test_utils;
