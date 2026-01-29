// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

pub mod adapter;
pub mod baseline;
pub mod cache;
pub mod check;
pub mod checks;
pub mod cli;
pub mod cloc;
pub mod cmd_init;
pub mod color;
pub mod completions;
pub mod config;
pub mod discovery;
pub mod error;
pub mod file_reader;
pub mod file_size;
pub mod git;
pub mod help;
pub mod init;
pub mod latest;
pub mod output;
pub mod pattern;
pub mod profiles;
pub mod ratchet;
pub mod report;
pub mod runner;
pub mod timing;
pub mod tolerance;
pub mod verbose;
pub mod walker;

pub use baseline::Baseline;
pub use cli::{Cli, Command};
pub use error::{Error, ExitCode};

#[cfg(test)]
pub mod test_utils;
