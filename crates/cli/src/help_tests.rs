#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn consolidates_simple_pair() {
    let input = r#"Options:
      --color       Force color output
      --no-color    Disable color output"#;

    let result = consolidate_negatable_flags(input);

    assert!(
        result.contains("--[no-]color"),
        "Expected consolidated flag: {result}"
    );
    assert!(
        !result.lines().any(|l| l.trim().starts_with("--no-color")),
        "Should not have separate --no-color line: {result}"
    );
}

#[test]
fn preserves_standalone_no_flag() {
    let input = r#"Options:
      --no-cache    Bypass the cache"#;

    let result = consolidate_negatable_flags(input);

    assert!(
        result.contains("--no-cache"),
        "Should preserve --no-cache: {result}"
    );
    assert!(
        !result.contains("--[no-]cache"),
        "Should not consolidate without --cache: {result}"
    );
}

#[test]
fn handles_flag_with_value() {
    let input = r#"Options:
      --limit <N>   Maximum violations to display
      --no-limit    Show all violations"#;

    let result = consolidate_negatable_flags(input);

    assert!(
        result.contains("--[no-]limit"),
        "Expected consolidated flag: {result}"
    );
    assert!(
        result.contains("[N]"),
        "Value should become optional: {result}"
    );
}

#[test]
fn preserves_short_option() {
    let input = r#"Options:
  -c, --color       Force color output
      --no-color    Disable color output"#;

    let result = consolidate_negatable_flags(input);

    assert!(
        result.contains("-c"),
        "Should preserve short option: {result}"
    );
    assert!(
        result.contains("--[no-]color"),
        "Should consolidate: {result}"
    );
}

#[test]
fn handles_multiple_pairs() {
    let input = r#"Options:
      --color       Force color
      --no-color    Disable color
      --limit <N>   Set limit
      --no-limit    No limit
      --verbose     Verbose mode"#;

    let result = consolidate_negatable_flags(input);

    assert!(
        result.contains("--[no-]color"),
        "Expected --[no-]color: {result}"
    );
    assert!(
        result.contains("--[no-]limit"),
        "Expected --[no-]limit: {result}"
    );
    assert!(
        result.contains("--verbose"),
        "Should preserve --verbose: {result}"
    );

    // Count lines - should have fewer after consolidation
    let original_option_lines = input.lines().filter(|l| l.contains("--")).count();
    let result_option_lines = result.lines().filter(|l| l.contains("--")).count();
    assert!(
        result_option_lines < original_option_lines,
        "Should have fewer lines after consolidation"
    );
}

#[test]
fn preserves_non_option_lines() {
    let input = r#"My CLI Tool

Usage: mycli [OPTIONS]

Options:
      --color       Force color
      --no-color    Disable color

Examples:
  mycli --color"#;

    let result = consolidate_negatable_flags(input);

    assert!(result.contains("My CLI Tool"), "Should preserve header");
    assert!(result.contains("Usage:"), "Should preserve usage");
    assert!(result.contains("Examples:"), "Should preserve examples");
    assert!(result.contains("--[no-]color"), "Should consolidate flag");
}

#[test]
fn handles_check_toggles() {
    let input = r#"Options:
      --cloc          Run only cloc check
      --no-cloc       Skip cloc check
      --escapes       Run only escapes check
      --no-escapes    Skip escapes check"#;

    let result = consolidate_negatable_flags(input);

    assert!(
        result.contains("--[no-]cloc"),
        "Expected --[no-]cloc: {result}"
    );
    assert!(
        result.contains("--[no-]escapes"),
        "Expected --[no-]escapes: {result}"
    );
}

#[test]
fn format_help_with_real_command() {
    use crate::cli::Cli;
    use clap::CommandFactory;

    let mut cmd = Cli::command();
    let check_cmd = cmd.find_subcommand_mut("check").unwrap();
    let help = format_help(check_cmd);

    // Verify consolidation happened for limit
    assert!(
        help.contains("--[no-]limit"),
        "check --help should have --[no-]limit: {help}"
    );

    // Verify standalone --no-cache preserved
    assert!(
        help.contains("--no-cache"),
        "check --help should have --no-cache: {help}"
    );
    assert!(
        !help.contains("--[no-]cache"),
        "Should not consolidate --no-cache: {help}"
    );

    // Verify all check toggles consolidated
    for check in [
        "cloc", "escapes", "agents", "docs", "tests", "git", "build", "license",
    ] {
        assert!(
            help.contains(&format!("--[no-]{check}")),
            "check --help should have --[no-]{check}: {help}"
        );
    }
}
