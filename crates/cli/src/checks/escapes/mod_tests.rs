// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

// Unit tests for escapes check internals
// Behavioral tests are in tests/specs/checks/escapes.rs

use super::*;
use yare::parameterized;

use comment::{is_comment_line, is_match_in_comment, strip_comment_markers};

#[parameterized(
    same_line = { "unsafe { code } // SAFETY: reason", 1, true },
    preceding_line = { "// SAFETY: reason\nunsafe { code }", 2, true },
    through_blank_lines = { "// SAFETY: reason\n\nunsafe { code }", 3, true },
    through_other_comments = { "// SAFETY: reason\n// more context\nunsafe { code }", 3, true },
    stops_at_code_line = { "// SAFETY: old\nfn other() {}\nunsafe { code }", 3, false },
    no_comment_returns_false = { "unsafe { code }", 1, false },
)]
fn has_justification_comment_cases(content: &str, line: u32, expected: bool) {
    assert_eq!(
        has_justification_comment(content, line, "// SAFETY:"),
        expected,
        "content {:?} at line {} should {} have justification",
        content,
        line,
        if expected { "" } else { "not" }
    );
}

#[parameterized(
    c_style_single = { "// comment", true },
    c_style_indented = { "  // indented", true },
    c_style_block = { "/* block */", true },
    block_continuation = { " * continuation", true },
    shell_style = { "# comment", true },
    shell_indented = { "  # indented", true },
    code_fn = { "fn main() {}", false },
    code_let = { "let x = 1;", false },
)]
fn is_comment_line_cases(input: &str, expected: bool) {
    assert_eq!(
        is_comment_line(input),
        expected,
        "input {:?} should {} be a comment line",
        input,
        if expected { "" } else { "not" }
    );
}

#[parameterized(
    // Match is before comment start - NOT in comment
    match_before_comment = { "eval cmd // explanation", 0, false },
    match_in_code = { "eval cmd", 0, false },
    // Match is after comment start - IN comment
    match_in_c_comment = { "code // don't use eval here", 20, true },
    match_in_shell_comment = { "code # used with eval", 10, true },
    // Match at start of line with // comment
    whole_line_comment = { "// don't use eval", 0, true },
    // Shell comment at line start
    shell_line_comment = { "# used with eval", 0, true },
    // Match exactly at comment boundary
    at_comment_start = { "x // eval", 5, true },
)]
fn is_match_in_comment_cases(line: &str, offset: usize, expected: bool) {
    assert_eq!(
        is_match_in_comment(line, offset),
        expected,
        "line {:?} with match at offset {} should {} be in comment",
        line,
        offset,
        if expected { "" } else { "not" }
    );
}

#[parameterized(
    ignores_embedded_patterns = {
        "code  // VIOLATION: missing // SAFETY: comment\nmore code",
        1,
        false
    },
    finds_standalone_pattern = {
        "// SAFETY: this is safe\nunsafe { *ptr }",
        2,
        true
    },
    finds_pattern_on_same_line = {
        "unsafe { *ptr }  // SAFETY: this is safe",
        1,
        true
    },
    extra_text_after_pattern = {
        "// SAFETY: reason here // more notes",
        1,
        true
    },
    embedded_at_end_does_not_match = {
        "code // error message about // SAFETY:",
        1,
        false
    },
)]
fn comment_boundary_cases(content: &str, line: u32, expected: bool) {
    assert_eq!(
        has_justification_comment(content, line, "// SAFETY:"),
        expected,
        "content {:?} at line {} should {} have justification",
        content,
        line,
        if expected { "" } else { "not" }
    );
}

#[test]
fn doc_comment_variants() {
    // Triple-slash doc comments should match
    let content = "/// SAFETY: reason\nunsafe { code }";
    assert!(has_justification_comment(content, 2, "// SAFETY:"));

    // Inner doc comments should match
    let content = "//! SAFETY: reason\nunsafe { code }";
    assert!(has_justification_comment(content, 2, "// SAFETY:"));
}

#[parameterized(
    single_line = { "// SAFETY:", "SAFETY:" },
    single_line_indented = { "  // SAFETY:", "SAFETY:" },
    doc_triple_slash = { "/// SAFETY:", "SAFETY:" },
    doc_inner = { "//! SAFETY:", "SAFETY:" },
    shell_comment = { "# SAFETY:", "SAFETY:" },
)]
fn strip_comment_markers_cases(input: &str, expected: &str) {
    assert_eq!(
        strip_comment_markers(input),
        expected,
        "input {:?} should strip to {:?}",
        input,
        expected
    );
}

// Performance micro-benchmarks
// Run with: cargo test --package quench -- bench_ --ignored --nocapture
mod benchmarks {
    use super::*;
    use crate::pattern::CompiledPattern;
    use std::time::Instant;

    /// Generate benchmark content with escape patterns.
    fn generate_content(lines: usize, pattern_frequency: usize) -> String {
        (0..lines)
            .map(|i| {
                if i % pattern_frequency == 0 {
                    format!("let x = foo.unwrap();  // line {}\n", i)
                } else {
                    format!("let x = normal_code();  // line {}\n", i)
                }
            })
            .collect()
    }

    #[test]
    #[ignore = "benchmark only"]
    fn bench_pattern_matching_performance() {
        // Generate content with ~100 lines, some with escape patterns
        let content = generate_content(100, 10); // 10% with patterns

        let pattern = CompiledPattern::compile(r"\.unwrap\(\)").unwrap();

        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = pattern.find_all_with_lines(&content);
        }
        let elapsed = start.elapsed();

        println!("=== Pattern Matching Performance ===");
        println!("Content: 100 lines, 10% with .unwrap() pattern");
        println!("{} iterations: {:?}", iterations, elapsed);
        println!("Per match call: {:?}", elapsed / iterations);
        println!("Target: < 1ms per 100-line file");
        println!();

        // Verify we found the expected matches
        let matches = pattern.find_all_with_lines(&content);
        println!("Found {} matches in 100 lines", matches.len());
    }

    #[test]
    #[ignore = "benchmark only"]
    fn bench_comment_search_performance() {
        // Generate content with justification comments
        let content: String = (0..100)
            .map(|i| {
                if i % 20 == 0 {
                    "// SAFETY: this is safe\n".to_string()
                } else if i % 10 == 0 {
                    "let x = foo.unwrap();\n".to_string()
                } else {
                    format!("let x = code();  // line {}\n", i)
                }
            })
            .collect();

        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            // Search from line 50 (middle of file)
            let _ = has_justification_comment(&content, 50, "// SAFETY:");
        }
        let elapsed = start.elapsed();

        println!("=== Comment Search Performance ===");
        println!("Content: 100 lines with SAFETY comments every 20 lines");
        println!("{} iterations: {:?}", iterations, elapsed);
        println!("Per search: {:?}", elapsed / iterations);
        println!("Target: < 0.1ms per search");
    }

    #[test]
    #[ignore = "benchmark only"]
    fn bench_large_file_pattern_matching() {
        // Simulate larger file (1000 lines)
        let content = generate_content(1000, 11); // ~9% with patterns

        let pattern = CompiledPattern::compile(r"\.unwrap\(\)").unwrap();

        let iterations = 1_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = pattern.find_all_with_lines(&content);
        }
        let elapsed = start.elapsed();

        let matches = pattern.find_all_with_lines(&content);
        println!("=== Large File Pattern Matching ===");
        println!("Content: 1000 lines, {} matches", matches.len());
        println!("{} iterations: {:?}", iterations, elapsed);
        println!("Per match call: {:?}", elapsed / iterations);
        println!("Target: < 10ms per 1000-line file");
    }

    #[test]
    #[ignore = "benchmark only"]
    fn bench_multi_pattern_todo_fixme() {
        // Test the TODO/FIXME pattern which is an alternation
        let content: String = (0..100)
            .map(|i| {
                if i % 15 == 0 {
                    format!("// TODO: fix this {}\n", i)
                } else if i % 20 == 0 {
                    format!("// FIXME: broken {}\n", i)
                } else {
                    format!("let x = code_{};  // line {}\n", i, i)
                }
            })
            .collect();

        let pattern = CompiledPattern::compile(r"\b(TODO|FIXME|XXX)\b").unwrap();

        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = pattern.find_all_with_lines(&content);
        }
        let elapsed = start.elapsed();

        let matches = pattern.find_all_with_lines(&content);
        println!("=== Multi-Pattern (TODO|FIXME|XXX) Performance ===");
        println!("Content: 100 lines, {} matches", matches.len());
        println!("{} iterations: {:?}", iterations, elapsed);
        println!("Per match call: {:?}", elapsed / iterations);
    }

    #[test]
    #[ignore = "benchmark only"]
    fn bench_line_deduplication() {
        use crate::pattern::LineMatch;
        use std::collections::HashSet;

        // Simulate matches with duplicate lines
        let matches: Vec<LineMatch> = (0..100)
            .map(|i| LineMatch {
                line: (i % 20) as u32, // Only 20 unique lines
                text: ".unwrap()".to_string(),
                offset: i * 50,
                line_content: format!("let x = foo.unwrap(); // line {}", i % 20),
            })
            .collect();

        let iterations = 100_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut seen_lines = HashSet::new();
            let _unique: Vec<_> = matches.iter().filter(|m| seen_lines.insert(m.line)).collect();
        }
        let elapsed = start.elapsed();

        println!("=== Line Deduplication Performance ===");
        println!("Input: 100 matches, 20 unique lines");
        println!("{} iterations: {:?}", iterations, elapsed);
        println!("Per dedup: {:?}", elapsed / iterations);
        println!("Expected: negligible (<1µs)");
    }

    #[test]
    #[ignore = "benchmark only"]
    fn bench_file_classification() {
        use crate::adapter::GenericAdapter;
        use std::path::PathBuf;

        let root = std::path::Path::new("/project");
        let test_patterns = default_test_patterns();
        let paths: Vec<PathBuf> =
            (0..1000).map(|i| PathBuf::from(format!("/project/src/module_{}.rs", i))).collect();

        // Measure adapter creation cost (done once)
        let start = Instant::now();
        let adapter = GenericAdapter::new(&[], &test_patterns);
        let adapter_creation = start.elapsed();
        println!("=== File Classification Performance ===");
        println!("Adapter creation: {:?}", adapter_creation);

        // Optimized: reuse adapter for all classifications
        let iterations = 100;
        let start = Instant::now();
        for _ in 0..iterations {
            for path in &paths {
                let _ = classify_file(&adapter, path, root);
            }
        }
        let elapsed = start.elapsed();
        let total_classifications = iterations * paths.len();
        println!(
            "{}K classifications with reused adapter: {:?}",
            total_classifications / 1000,
            elapsed
        );
        println!("Per classification: {:?}", elapsed / total_classifications as u32);
        println!("Target: < 1µs per classification");
    }
}
