// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

// =============================================================================
// SECTION PARSING TESTS
// =============================================================================

#[test]
fn parse_sections_empty() {
    let sections = parse_sections("");
    assert!(sections.is_empty());
}

#[test]
fn parse_sections_preamble_only() {
    let content = "# Title\n\nSome intro text.\n";
    let sections = parse_sections(content);

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].name, "");
    assert_eq!(sections[0].heading, "");
    assert!(sections[0].content.contains("# Title"));
    assert!(sections[0].content.contains("Some intro text."));
    assert_eq!(sections[0].line, 1);
}

#[test]
fn parse_sections_single_section() {
    let content = "## Code Style\n\nUse 4 spaces.\n";
    let sections = parse_sections(content);

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].name, "code style");
    assert_eq!(sections[0].heading, "Code Style");
    assert!(sections[0].content.contains("Use 4 spaces."));
    assert_eq!(sections[0].line, 1);
}

#[test]
fn parse_sections_multiple_sections() {
    let content = "## Code Style\n\nUse 4 spaces.\n\n## Testing\n\nRun cargo test.\n";
    let sections = parse_sections(content);

    assert_eq!(sections.len(), 2);

    assert_eq!(sections[0].name, "code style");
    assert_eq!(sections[0].heading, "Code Style");
    assert!(sections[0].content.contains("Use 4 spaces."));

    assert_eq!(sections[1].name, "testing");
    assert_eq!(sections[1].heading, "Testing");
    assert!(sections[1].content.contains("Run cargo test."));
}

#[test]
fn parse_sections_with_preamble() {
    let content = "# Project\n\nOverview.\n\n## Code Style\n\nUse 4 spaces.\n";
    let sections = parse_sections(content);

    assert_eq!(sections.len(), 2);

    // Preamble
    assert_eq!(sections[0].name, "");
    assert!(sections[0].content.contains("# Project"));

    // Section
    assert_eq!(sections[1].name, "code style");
}

#[test]
fn parse_sections_ignores_h3_headings() {
    let content = "## Code Style\n\n### Indentation\n\nUse 4 spaces.\n";
    let sections = parse_sections(content);

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].name, "code style");
    assert!(sections[0].content.contains("### Indentation"));
    assert!(sections[0].content.contains("Use 4 spaces."));
}

#[test]
fn parse_sections_normalizes_names() {
    let content = "##   Code Style   \n\nContent.\n";
    let sections = parse_sections(content);

    assert_eq!(sections[0].name, "code style");
    assert_eq!(sections[0].heading, "Code Style");
}

#[test]
fn parse_sections_tracks_line_numbers() {
    let content = "# Preamble\n\n## First\n\nContent.\n\n## Second\n\nMore.\n";
    let sections = parse_sections(content);

    assert_eq!(sections.len(), 3);
    assert_eq!(sections[0].line, 1); // Preamble
    assert_eq!(sections[1].line, 3); // ## First
    assert_eq!(sections[2].line, 7); // ## Second
}

// =============================================================================
// COMPARISON TESTS
// =============================================================================

#[test]
fn compare_identical_files() {
    let content = "## Code Style\n\nUse 4 spaces.\n";
    let comparison = compare_files(content, content);

    assert!(comparison.in_sync);
    assert!(comparison.differences.is_empty());
}

#[test]
fn compare_content_differs() {
    let source = "## Code Style\n\nUse 4 spaces.\n";
    let target = "## Code Style\n\nUse 2 spaces.\n";
    let comparison = compare_files(source, target);

    assert!(!comparison.in_sync);
    assert_eq!(comparison.differences.len(), 1);
    assert_eq!(comparison.differences[0].section, "code style");
    assert_eq!(comparison.differences[0].diff_type, DiffType::ContentDiffers);
}

#[test]
fn compare_missing_in_target() {
    let source = "## Code Style\n\nUse 4 spaces.\n\n## Testing\n\nRun tests.\n";
    let target = "## Code Style\n\nUse 4 spaces.\n";
    let comparison = compare_files(source, target);

    assert!(!comparison.in_sync);
    assert_eq!(comparison.differences.len(), 1);
    assert_eq!(comparison.differences[0].section, "testing");
    assert_eq!(comparison.differences[0].diff_type, DiffType::MissingInTarget);
}

#[test]
fn compare_extra_in_target() {
    let source = "## Code Style\n\nUse 4 spaces.\n";
    let target = "## Code Style\n\nUse 4 spaces.\n\n## Extra\n\nBonus content.\n";
    let comparison = compare_files(source, target);

    assert!(!comparison.in_sync);
    assert_eq!(comparison.differences.len(), 1);
    assert_eq!(comparison.differences[0].section, "extra");
    assert_eq!(comparison.differences[0].diff_type, DiffType::ExtraInTarget);
}

#[test]
fn compare_whitespace_normalized() {
    let source = "## Code Style\n\nUse 4 spaces.\n";
    let target = "## Code Style\n\n  Use 4 spaces.  \n\n\n";
    let comparison = compare_files(source, target);

    assert!(comparison.in_sync);
}

#[test]
fn compare_case_insensitive_section_names() {
    let source = "## Code Style\n\nContent.\n";
    let target = "## code style\n\nContent.\n";
    let comparison = compare_files(source, target);

    assert!(comparison.in_sync);
}

#[test]
fn compare_preamble_differs() {
    let source = "# Project A\n\nIntro.\n";
    let target = "# Project B\n\nDifferent intro.\n";
    let comparison = compare_files(source, target);

    assert!(!comparison.in_sync);
    assert_eq!(comparison.differences.len(), 1);
    assert_eq!(comparison.differences[0].section, ""); // Preamble
    assert_eq!(comparison.differences[0].diff_type, DiffType::ContentDiffers);
}

#[test]
fn compare_multiple_differences() {
    let source = "## A\n\nA content.\n\n## B\n\nB content.\n";
    let target = "## A\n\nDifferent.\n\n## C\n\nC content.\n";
    let comparison = compare_files(source, target);

    assert!(!comparison.in_sync);
    assert_eq!(comparison.differences.len(), 3);

    // A differs
    assert!(
        comparison
            .differences
            .iter()
            .any(|d| d.section == "a" && d.diff_type == DiffType::ContentDiffers)
    );

    // B missing in target
    assert!(
        comparison
            .differences
            .iter()
            .any(|d| d.section == "b" && d.diff_type == DiffType::MissingInTarget)
    );

    // C extra in target
    assert!(
        comparison
            .differences
            .iter()
            .any(|d| d.section == "c" && d.diff_type == DiffType::ExtraInTarget)
    );
}
