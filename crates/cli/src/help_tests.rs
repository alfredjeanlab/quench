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

    // Strip ANSI for content verification
    let stripped = strip_ansi(&help);

    // Verify consolidation happened for limit
    assert!(
        stripped.contains("--[no-]limit"),
        "check --help should have --[no-]limit: {stripped}"
    );

    // Verify standalone --no-cache preserved
    assert!(
        stripped.contains("--no-cache"),
        "check --help should have --no-cache: {stripped}"
    );
    assert!(
        !stripped.contains("--[no-]cache"),
        "Should not consolidate --no-cache: {stripped}"
    );

    // Verify all check toggles consolidated
    for check in [
        "cloc", "escapes", "agents", "docs", "tests", "git", "build", "license",
    ] {
        assert!(
            stripped.contains(&format!("--[no-]{check}")),
            "check --help should have --[no-]{check}: {stripped}"
        );
    }
}

// =============================================================================
// Colorization tests
// =============================================================================

/// Strip all ANSI escape sequences from a string (for testing)
fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until 'm'
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[test]
fn colorize_help_preserves_structure() {
    let input = "Usage: quench [OPTIONS]\n\nCommands:\n  check  Run checks\n\nOptions:\n  -h, --help  Print help";
    let result = colorize_help(input);
    let stripped = strip_ansi(&result);
    assert_eq!(stripped, input);
}

#[test]
fn is_section_header_matches_known_headers() {
    // Usage: is handled by colorize_usage_line, not is_section_header
    assert!(is_section_header("Commands:"));
    assert!(is_section_header("Options:"));
    assert!(is_section_header("Arguments:"));
    assert!(is_section_header("  Options:")); // with leading whitespace
}

#[test]
fn is_section_header_rejects_non_headers() {
    assert!(!is_section_header("Description:"));
    assert!(!is_section_header("Examples:"));
    assert!(!is_section_header("Usage:")); // Handled by colorize_usage_line
    assert!(!is_section_header("Usage: quench"));
    assert!(!is_section_header("--help"));
}

#[test]
fn colorize_usage_line_with_pattern() {
    let result = colorize_usage_line("Usage: quench [OPTIONS] [COMMAND]");
    assert!(result.is_some());
    let stripped = strip_ansi(&result.unwrap());
    assert_eq!(stripped, "Usage: quench [OPTIONS] [COMMAND]");
}

#[test]
fn colorize_usage_line_standalone() {
    let result = colorize_usage_line("Usage:");
    assert!(result.is_some());
    let stripped = strip_ansi(&result.unwrap());
    assert_eq!(stripped, "Usage:");
}

#[test]
fn colorize_usage_line_rejects_non_usage() {
    assert!(colorize_usage_line("Commands:").is_none());
    assert!(colorize_usage_line("  check   Run checks").is_none());
}

#[test]
fn colorize_command_line_simple() {
    let result = colorize_command_line("  check   Run checks");
    assert!(result.is_some());
    let stripped = strip_ansi(&result.unwrap());
    assert_eq!(stripped, "  check   Run checks");
}

#[test]
fn colorize_command_line_rejects_options() {
    assert!(colorize_command_line("  -h, --help   Print help").is_none());
    assert!(colorize_command_line("  --verbose   Enable verbose").is_none());
}

#[test]
fn colorize_command_line_rejects_wrong_indent() {
    assert!(colorize_command_line("check   Run checks").is_none()); // no indent
    assert!(colorize_command_line("   check   Run checks").is_none()); // 3 spaces
}

#[test]
fn colorize_option_line_simple() {
    let result = colorize_option_line("  -h, --help  Print help");
    assert!(result.is_some());
    let stripped = strip_ansi(&result.unwrap());
    assert_eq!(stripped, "  -h, --help  Print help");
}

#[test]
fn colorize_option_line_with_value() {
    let result = colorize_option_line("  -o, --output <FORMAT>  Output format");
    assert!(result.is_some());
    let stripped = strip_ansi(&result.unwrap());
    assert_eq!(stripped, "  -o, --output <FORMAT>  Output format");
}

#[test]
fn colorize_option_line_with_no_prefix() {
    let result = colorize_option_line("      --[no-]limit [N]  Set limit");
    assert!(result.is_some());
    let stripped = strip_ansi(&result.unwrap());
    assert_eq!(stripped, "      --[no-]limit [N]  Set limit");
}

#[test]
fn colorize_option_line_long_only() {
    let result = colorize_option_line("      --verbose  Enable verbose");
    assert!(result.is_some());
    let stripped = strip_ansi(&result.unwrap());
    assert_eq!(stripped, "      --verbose  Enable verbose");
}
