// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use termcolor::ColorChoice;

use super::{FormatOptions, TextFormatter};
use crate::check::{CheckResult, Violation};

#[test]
fn text_formatter_creates_successfully() {
    let _formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
}

#[test]
fn text_formatter_silent_on_pass() {
    let mut formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let result = CheckResult::passed("cloc");
    let truncated = formatter.write_check(&result).unwrap();
    assert!(!truncated);
}

#[test]
fn text_formatter_tracks_violations_shown() {
    let mut formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let violations = vec![
        Violation::file("src/main.rs", 42, "file_too_large", "Split into modules."),
        Violation::file("src/lib.rs", 100, "file_too_large", "Split into modules."),
    ];
    let result = CheckResult::failed("cloc", violations);
    formatter.write_check(&result).unwrap();
    assert_eq!(formatter.violations_shown(), 2);
}

#[test]
fn text_formatter_respects_limit() {
    let options = FormatOptions::with_limit(1);
    let mut formatter = TextFormatter::new(ColorChoice::Never, options);
    let violations = vec![
        Violation::file("src/main.rs", 42, "file_too_large", "Split into modules."),
        Violation::file("src/lib.rs", 100, "file_too_large", "Split into modules."),
    ];
    let result = CheckResult::failed("cloc", violations);
    let truncated = formatter.write_check(&result).unwrap();
    assert!(truncated);
    assert!(formatter.was_truncated());
    assert_eq!(formatter.violations_shown(), 1);
}

#[test]
fn text_formatter_no_truncation_without_limit() {
    let options = FormatOptions::no_limit();
    let mut formatter = TextFormatter::new(ColorChoice::Never, options);
    let violations = vec![
        Violation::file("src/main.rs", 42, "file_too_large", "Split into modules."),
        Violation::file("src/lib.rs", 100, "file_too_large", "Split into modules."),
    ];
    let result = CheckResult::failed("cloc", violations);
    let truncated = formatter.write_check(&result).unwrap();
    assert!(!truncated);
    assert!(!formatter.was_truncated());
    assert_eq!(formatter.violations_shown(), 2);
}

// =============================================================================
// AGENTS VIOLATION DESCRIPTION TESTS
// =============================================================================

#[test]
fn agents_missing_file_description() {
    let formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let violation = Violation::file_only("CLAUDE.md", "missing_file", "Required file missing");
    let desc = formatter.format_violation_desc(&violation);
    assert_eq!(desc, "missing required file");
}

#[test]
fn agents_forbidden_file_description() {
    let formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let violation = Violation::file_only(".aider", "forbidden_file", "Forbidden file exists");
    let desc = formatter.format_violation_desc(&violation);
    assert_eq!(desc, "forbidden file exists");
}

#[test]
fn agents_out_of_sync_description_with_other_file() {
    let formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let violation = Violation::file_only(".cursorrules", "out_of_sync", "Section differs")
        .with_sync("CLAUDE.md", "Code Style");
    let desc = formatter.format_violation_desc(&violation);
    assert_eq!(desc, "out of sync with CLAUDE.md");
}

#[test]
fn agents_out_of_sync_description_without_other_file() {
    let formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let violation = Violation::file_only(".cursorrules", "out_of_sync", "Files differ");
    let desc = formatter.format_violation_desc(&violation);
    assert_eq!(desc, "out of sync");
}

#[test]
fn agents_missing_section_description() {
    let formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let violation = Violation::file_only("CLAUDE.md", "missing_section", "Add a section");
    let desc = formatter.format_violation_desc(&violation);
    assert_eq!(desc, "missing required section");
}

#[test]
fn agents_forbidden_section_description() {
    let formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let violation = Violation::file("CLAUDE.md", 10, "forbidden_section", "Remove this section");
    let desc = formatter.format_violation_desc(&violation);
    assert_eq!(desc, "forbidden section found");
}

#[test]
fn agents_forbidden_table_description() {
    let formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let violation = Violation::file("CLAUDE.md", 45, "forbidden_table", "Convert to list");
    let desc = formatter.format_violation_desc(&violation);
    assert_eq!(desc, "forbidden table");
}

#[test]
fn agents_forbidden_diagram_description() {
    let formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let violation = Violation::file("CLAUDE.md", 20, "forbidden_diagram", "Remove diagram");
    let desc = formatter.format_violation_desc(&violation);
    assert_eq!(desc, "forbidden box diagram");
}

#[test]
fn agents_forbidden_mermaid_description() {
    let formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let violation = Violation::file("CLAUDE.md", 30, "forbidden_mermaid", "Remove mermaid block");
    let desc = formatter.format_violation_desc(&violation);
    assert_eq!(desc, "forbidden mermaid block");
}

#[test]
fn agents_file_too_large_description() {
    let formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let violation = Violation::file_only("CLAUDE.md", "file_too_large", "File exceeds limit")
        .with_threshold(502, 500);
    let desc = formatter.format_violation_desc(&violation);
    assert_eq!(desc, "file too large (502 vs 500)");
}

#[test]
fn cloc_file_too_large_uses_lines_label() {
    // Cloc violations set lines/nonblank - should use default format with "lines:" prefix
    let formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let violation = Violation::file("src/main.rs", 1, "file_too_large", "Split file")
        .with_threshold(800, 750)
        .with_line_counts(800, 700);
    let desc = formatter.format_violation_desc(&violation);
    assert_eq!(desc, "file_too_large (lines: 800 vs 750)");
}

// =============================================================================
// FIXED STATUS TESTS
// =============================================================================

#[test]
fn text_formatter_shows_fixed_status() {
    let mut formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let fix_summary = serde_json::json!({
        "files_synced": [
            {"file": ".cursorrules", "source": "CLAUDE.md", "sections": 3}
        ]
    });
    let result = CheckResult::fixed("agents", fix_summary);
    let truncated = formatter.write_check(&result).unwrap();
    assert!(!truncated);
}

#[test]
fn text_formatter_silent_on_fixed() {
    // Fixed results should still produce output (showing what was fixed)
    let mut formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let fix_summary = serde_json::json!({
        "files_synced": []
    });
    let result = CheckResult::fixed("agents", fix_summary);
    let truncated = formatter.write_check(&result).unwrap();
    assert!(!truncated);
}
