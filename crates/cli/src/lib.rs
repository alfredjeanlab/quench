// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

pub mod adapter;
pub mod baseline;
pub mod cache;
pub mod check;
pub mod checks;
pub mod cli;
pub mod cmd_init;
pub mod color;
pub mod config;
pub mod discovery;
pub mod error;
pub mod git;
pub mod init;
pub mod output;
pub mod pattern;
pub mod profiles;
pub mod ratchet;
pub mod report;
pub mod runner;
pub mod timing;
pub mod tolerance;
pub mod walker;

pub use baseline::{Baseline, BaselineError, BaselineMetrics};
pub use cache::{CacheStats, FileCache};
pub use check::{Check, CheckContext, CheckOutput, CheckResult, Violation};
pub use cli::{CheckArgs, CheckFilter, Cli, Command, InitArgs, OutputFormat, ReportArgs};
pub use color::{is_no_color_env, resolve_color};
pub use config::IgnoreConfig;
pub use error::{Error, ExitCode, Result};
pub use ratchet::{CurrentMetrics, RatchetResult};
pub use walker::{FileWalker, WalkStats, WalkedFile, WalkerConfig};

#[cfg(test)]
pub mod test_utils;
