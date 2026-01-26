// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::init::CursorMarker;

#[test]
fn parse_bare_invocation() {
    let cli = Cli::parse_from(["quench"]);
    assert!(cli.command.is_none());
}

#[test]
fn parse_check_command() {
    let cli = Cli::parse_from(["quench", "check"]);
    assert!(matches!(cli.command, Some(Command::Check(_))));
}

#[test]
fn parse_check_with_paths() {
    let cli = Cli::parse_from(["quench", "check", "src/", "lib/"]);
    if let Some(Command::Check(args)) = cli.command {
        assert_eq!(args.paths.len(), 2);
    } else {
        panic!("expected check command");
    }
}

#[test]
fn parse_check_with_output_format() {
    let cli = Cli::parse_from(["quench", "check", "-o", "json"]);
    if let Some(Command::Check(args)) = cli.command {
        assert!(matches!(args.output, OutputFormat::Json));
    } else {
        panic!("expected check command");
    }
}

#[test]
fn parse_report_command() {
    let cli = Cli::parse_from(["quench", "report"]);
    assert!(matches!(cli.command, Some(Command::Report(_))));
}

#[test]
fn parse_init_command() {
    let cli = Cli::parse_from(["quench", "init"]);
    assert!(matches!(cli.command, Some(Command::Init(_))));
}

#[test]
fn parse_init_with_force() {
    let cli = Cli::parse_from(["quench", "init", "--force"]);
    if let Some(Command::Init(args)) = cli.command {
        assert!(args.force);
    } else {
        panic!("expected init command");
    }
}

#[test]
fn default_template_contains_required_sections() {
    let template = default_template();
    assert!(template.contains("version = 1"));
    assert!(template.contains("[check.cloc]"));
    assert!(template.contains("[check.escapes]"));
    assert!(template.contains("[check.agents]"));
    assert!(template.contains("[check.docs]"));
    assert!(template.contains("[check.tests]"));
    assert!(template.contains("[check.license]"));
    assert!(template.contains("[git.commit]"));
    assert!(template.contains("# Supported Languages:"));
    assert!(template.contains("# [rust], [golang], [javascript], [shell]"));
}

#[test]
fn default_template_has_explicit_check_levels() {
    let template = default_template();
    // Enabled checks
    assert!(template.contains("[check.cloc]\ncheck = \"error\""));
    assert!(template.contains("[check.escapes]\ncheck = \"error\""));
    assert!(template.contains("[check.agents]\ncheck = \"error\""));
    assert!(template.contains("[check.docs]\ncheck = \"error\""));
    // Disabled checks with stub comments
    assert!(template.contains("check = \"off\"  # stub in quench v0.3.0"));
}

// =============================================================================
// agents_section() Tests
// =============================================================================

use crate::init::DetectedAgent;

#[test]
fn agents_section_empty_has_no_required() {
    let section = agents_section(&[]);
    assert!(section.contains("[check.agents]"));
    assert!(section.contains("check = \"error\""));
    assert!(!section.contains("required"));
}

#[test]
fn agents_section_claude_requires_claude_md() {
    let section = agents_section(&[DetectedAgent::Claude]);
    assert!(section.contains("[check.agents]"));
    assert!(section.contains("check = \"error\""));
    assert!(section.contains("required"));
    assert!(section.contains("CLAUDE.md"));
}

#[test]
fn agents_section_cursor_requires_cursorrules() {
    let section = agents_section(&[DetectedAgent::Cursor(CursorMarker::Cursorrules)]);
    assert!(section.contains("[check.agents]"));
    assert!(section.contains("required"));
    assert!(section.contains(".cursorrules"));
}

#[test]
fn agents_section_cursor_rules_dir() {
    let section = agents_section(&[DetectedAgent::Cursor(CursorMarker::CursorRulesDir)]);
    assert!(section.contains("[check.agents]"));
    assert!(section.contains("required"));
    assert!(section.contains(".cursor/rules/*.mdc"));
}

#[test]
fn agents_section_both_merges_required() {
    let section = agents_section(&[
        DetectedAgent::Claude,
        DetectedAgent::Cursor(CursorMarker::Cursorrules),
    ]);
    assert!(section.contains("required"));
    assert!(section.contains("CLAUDE.md"));
    assert!(section.contains(".cursorrules"));
    // Should be a single array with both
    assert_eq!(section.matches("required").count(), 1);
}

#[test]
fn default_template_no_duplicate_agents_section() {
    let template = default_template();
    let count = template.matches("[check.agents]").count();
    assert_eq!(count, 1, "should have exactly one [check.agents] section");
}
