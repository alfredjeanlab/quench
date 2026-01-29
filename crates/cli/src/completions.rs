// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shell completion installation for quench.
//!
//! Installs shell completion scripts and adds sourcing lines to shell RC files.
//! Follows the marker-based pattern for safe, idempotent installation.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::Cli;
use crate::error::{Error, Result};

/// Marker comment to identify quench completion blocks.
const QUENCH_COMPLETION_MARKER: &str = "# quench-shell-completion";

/// Get the user's home directory.
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Get the local data directory (XDG_DATA_HOME or ~/.local/share).
fn data_local_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".local/share")))
}

/// Get the config directory (XDG_CONFIG_HOME or ~/.config).
fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".config")))
}

/// Supported shells for completion installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Bash,
    Zsh,
    Fish,
}

impl ShellKind {
    /// Get the shell's RC file path, if it exists.
    pub fn rc_file(&self) -> Option<PathBuf> {
        let home = home_dir()?;
        match self {
            ShellKind::Bash => {
                // Prefer .bashrc, fall back to .bash_profile (macOS)
                let bashrc = home.join(".bashrc");
                let bash_profile = home.join(".bash_profile");
                if bashrc.exists() {
                    Some(bashrc)
                } else if bash_profile.exists() {
                    Some(bash_profile)
                } else {
                    None
                }
            }
            ShellKind::Zsh => {
                let zshrc = home.join(".zshrc");
                if zshrc.exists() { Some(zshrc) } else { None }
            }
            ShellKind::Fish => {
                // Fish uses XDG config
                let fish_config = config_dir()?.join("fish/config.fish");
                if fish_config.exists() {
                    Some(fish_config)
                } else {
                    None
                }
            }
        }
    }

    /// Get the clap_complete shell type.
    fn clap_shell(&self) -> clap_complete::Shell {
        match self {
            ShellKind::Bash => clap_complete::Shell::Bash,
            ShellKind::Zsh => clap_complete::Shell::Zsh,
            ShellKind::Fish => clap_complete::Shell::Fish,
        }
    }

    /// Get the completion script filename.
    fn script_filename(&self) -> &'static str {
        match self {
            ShellKind::Bash => "quench.bash",
            ShellKind::Zsh => "_quench",
            ShellKind::Fish => "quench.fish",
        }
    }
}

/// Detect which shells are installed on the system.
pub fn detect_shells() -> Vec<ShellKind> {
    let mut shells = Vec::new();

    if shell_exists("bash") {
        shells.push(ShellKind::Bash);
    }
    if shell_exists("zsh") {
        shells.push(ShellKind::Zsh);
    }
    if shell_exists("fish") {
        shells.push(ShellKind::Fish);
    }

    shells
}

/// Check if a shell binary exists.
fn shell_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the directory for storing completion scripts.
fn completions_dir() -> Option<PathBuf> {
    // Use ~/.local/share/quench/completions/
    data_local_dir().map(|d| d.join("quench/completions"))
}

/// Generate and write completion script for a shell.
fn write_completion_script(shell: ShellKind) -> Result<PathBuf> {
    let dir = completions_dir().ok_or_else(|| Error::Config {
        message: "Cannot determine data directory".into(),
        path: None,
    })?;

    fs::create_dir_all(&dir).map_err(|e| Error::Io {
        path: dir.clone(),
        source: e,
    })?;

    let path = dir.join(shell.script_filename());
    let mut file = fs::File::create(&path).map_err(|e| Error::Io {
        path: path.clone(),
        source: e,
    })?;

    let mut cmd = Cli::command();
    generate(shell.clap_shell(), &mut cmd, "quench", &mut file);

    Ok(path)
}

/// Install completion sourcing in shell RC file.
fn install_completion_source(shell: ShellKind, script_path: &Path) -> Result<()> {
    let rc_path = shell.rc_file().ok_or_else(|| Error::Config {
        message: format!("No RC file found for {:?}", shell),
        path: None,
    })?;

    let existing = fs::read_to_string(&rc_path).unwrap_or_default();

    // Already installed - skip
    if existing.contains(QUENCH_COMPLETION_MARKER) {
        return Ok(());
    }

    let source_line = match shell {
        ShellKind::Bash | ShellKind::Zsh => format!(
            "\n{}\n[ -f \"{}\" ] && source \"{}\"\n",
            QUENCH_COMPLETION_MARKER,
            script_path.display(),
            script_path.display()
        ),
        ShellKind::Fish => format!(
            "\n{}\ntest -f \"{}\" && source \"{}\"\n",
            QUENCH_COMPLETION_MARKER,
            script_path.display(),
            script_path.display()
        ),
    };

    // Append to RC file
    let mut file = OpenOptions::new()
        .append(true)
        .open(&rc_path)
        .map_err(|e| Error::Io {
            path: rc_path.clone(),
            source: e,
        })?;
    file.write_all(source_line.as_bytes())
        .map_err(|e| Error::Io {
            path: rc_path.clone(),
            source: e,
        })?;

    Ok(())
}

/// Install completions for a single shell.
fn install_for_shell(shell: ShellKind) -> Result<()> {
    // Fish has a native completions directory - use it directly
    if shell == ShellKind::Fish {
        return install_fish_completions();
    }

    // For bash/zsh: write script and add to RC
    let script_path = write_completion_script(shell)?;
    install_completion_source(shell, &script_path)?;
    Ok(())
}

/// Install Fish completions to the native completions directory.
fn install_fish_completions() -> Result<()> {
    // Fish auto-loads from ~/.config/fish/completions/
    let fish_completions = config_dir()
        .ok_or_else(|| Error::Config {
            message: "Cannot determine config directory".into(),
            path: None,
        })?
        .join("fish/completions");

    fs::create_dir_all(&fish_completions).map_err(|e| Error::Io {
        path: fish_completions.clone(),
        source: e,
    })?;

    let path = fish_completions.join("quench.fish");
    let mut file = fs::File::create(&path).map_err(|e| Error::Io {
        path: path.clone(),
        source: e,
    })?;

    let mut cmd = Cli::command();
    generate(clap_complete::Shell::Fish, &mut cmd, "quench", &mut file);

    Ok(())
}

/// Install shell completions for all detected shells.
///
/// This function:
/// 1. Detects which shells are installed
/// 2. Generates completion scripts for each
/// 3. Adds sourcing lines to shell RC files (idempotently)
///
/// For Fish, completions are installed to the native completions directory
/// instead of modifying config.fish.
pub fn install_all() -> Result<()> {
    let shells = detect_shells();
    let mut any_success = false;
    let mut errors = Vec::new();

    for shell in shells {
        // Only install for shells that have RC files (except Fish which uses native dir)
        if shell != ShellKind::Fish && shell.rc_file().is_none() {
            continue;
        }

        match install_for_shell(shell) {
            Ok(()) => any_success = true,
            Err(e) => errors.push((shell, e)),
        }
    }

    // Log warnings for failures but don't fail overall
    for (shell, error) in &errors {
        eprintln!(
            "Warning: could not install {:?} completions: {}",
            shell, error
        );
    }

    if any_success || errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Config {
            message: "Failed to install any shell completions".into(),
            path: None,
        })
    }
}

#[cfg(test)]
#[path = "completions_tests.rs"]
mod tests;
