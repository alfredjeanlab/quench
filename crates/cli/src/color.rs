// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Color detection and terminal styling.
//!
//! Detection logic per docs/specs/03-output.md#colorization:
//! 1. --no-color → no color
//! 2. --color → use color
//! 3. default:
//!    - If not stdout.is_tty() → no color
//!    - If CLAUDE_CODE, CODEX, CI, or CURSOR env var set → no color
//!    - Else → use color

use std::io::IsTerminal;
use termcolor::ColorChoice;

/// Resolve color choice from CLI flags.
///
/// Priority: no_color > force_color > auto-detect
pub fn resolve_color(force_color: bool, no_color: bool) -> ColorChoice {
    if no_color {
        return ColorChoice::Never;
    }
    if force_color {
        return ColorChoice::Always;
    }
    // Auto-detect
    if !std::io::stdout().is_terminal() {
        return ColorChoice::Never;
    }
    if is_agent_environment() {
        return ColorChoice::Never;
    }
    ColorChoice::Auto
}

/// Check if running in an AI agent environment.
fn is_agent_environment() -> bool {
    std::env::var_os("CLAUDE_CODE").is_some()
        || std::env::var_os("CODEX").is_some()
        || std::env::var_os("CURSOR").is_some()
        || std::env::var_os("CI").is_some()
}

/// Check if QUENCH_NO_COLOR environment variable is set.
pub fn is_no_color_env() -> bool {
    std::env::var_os("QUENCH_NO_COLOR").is_some()
}

/// Color scheme for output per spec.
pub mod scheme {
    use termcolor::{Color, ColorSpec};

    /// Bold check name (e.g., "cloc").
    pub fn check_name() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_bold(true);
        spec
    }

    /// Red "FAIL" indicator.
    pub fn fail() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Red)).set_bold(true);
        spec
    }

    /// Green "PASS" indicator.
    pub fn pass() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Green)).set_bold(true);
        spec
    }

    /// Green "FIXED" indicator.
    pub fn fixed() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Green)).set_bold(true);
        spec
    }

    /// Yellow "SKIP" indicator.
    pub fn skip() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Yellow)).set_bold(true);
        spec
    }

    /// Yellow "WARN" indicator.
    pub fn warn() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Yellow)).set_bold(true);
        spec
    }

    /// Cyan file path.
    pub fn path() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Cyan));
        spec
    }

    /// Yellow line number.
    pub fn line_number() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Yellow));
        spec
    }

    /// Default (no color) for advice text.
    pub fn advice() -> ColorSpec {
        ColorSpec::new()
    }

    /// Red for diff removed lines.
    pub fn diff_remove() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Red));
        spec
    }

    /// Green for diff added lines.
    pub fn diff_add() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Green));
        spec
    }
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod tests;
