#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

// Unit tests for escapes check internals
// Behavioral tests are in tests/specs/checks/escapes.rs

use super::*;

mod comment_detection {
    use super::*;

    #[test]
    fn finds_comment_on_same_line() {
        let content = "unsafe { code } // SAFETY: reason";
        assert!(has_justification_comment(content, 1, "// SAFETY:"));
    }

    #[test]
    fn finds_comment_on_preceding_line() {
        let content = "// SAFETY: reason\nunsafe { code }";
        assert!(has_justification_comment(content, 2, "// SAFETY:"));
    }

    #[test]
    fn finds_comment_through_blank_lines() {
        let content = "// SAFETY: reason\n\nunsafe { code }";
        assert!(has_justification_comment(content, 3, "// SAFETY:"));
    }

    #[test]
    fn finds_comment_through_other_comments() {
        let content = "// SAFETY: reason\n// more context\nunsafe { code }";
        assert!(has_justification_comment(content, 3, "// SAFETY:"));
    }

    #[test]
    fn stops_at_code_line() {
        let content = "// SAFETY: old\nfn other() {}\nunsafe { code }";
        assert!(!has_justification_comment(content, 3, "// SAFETY:"));
    }

    #[test]
    fn no_comment_returns_false() {
        let content = "unsafe { code }";
        assert!(!has_justification_comment(content, 1, "// SAFETY:"));
    }
}

mod is_comment_line_tests {
    use super::*;

    #[test]
    fn c_style_single() {
        assert!(is_comment_line("// comment"));
        assert!(is_comment_line("  // indented"));
    }

    #[test]
    fn c_style_block() {
        assert!(is_comment_line("/* block */"));
        assert!(is_comment_line(" * continuation"));
    }

    #[test]
    fn shell_style() {
        assert!(is_comment_line("# comment"));
        assert!(is_comment_line("  # indented"));
    }

    #[test]
    fn code_is_not_comment() {
        assert!(!is_comment_line("fn main() {}"));
        assert!(!is_comment_line("let x = 1;"));
    }
}

mod comment_boundary_tests {
    use super::*;

    #[test]
    fn comment_search_ignores_embedded_patterns() {
        // Pattern appears embedded in another comment - should NOT match
        let content = "code  // VIOLATION: missing // SAFETY: comment\nmore code";
        assert!(!has_justification_comment(content, 1, "// SAFETY:"));
    }

    #[test]
    fn comment_search_finds_standalone_pattern() {
        // Pattern is the actual comment start - should match
        let content = "// SAFETY: this is safe\nunsafe { *ptr }";
        assert!(has_justification_comment(content, 2, "// SAFETY:"));
    }

    #[test]
    fn comment_search_finds_pattern_on_same_line() {
        // Pattern at start of inline comment - should match
        let content = "unsafe { *ptr }  // SAFETY: this is safe";
        assert!(has_justification_comment(content, 1, "// SAFETY:"));
    }

    #[test]
    fn comment_search_matches_doc_comment_variants() {
        // Triple-slash doc comments should match
        let content = "/// SAFETY: reason\nunsafe { code }";
        assert!(has_justification_comment(content, 2, "// SAFETY:"));

        // Inner doc comments should match
        let content = "//! SAFETY: reason\nunsafe { code }";
        assert!(has_justification_comment(content, 2, "// SAFETY:"));
    }

    #[test]
    fn comment_search_with_extra_text_after_pattern() {
        // Pattern with additional text should match
        let content = "// SAFETY: reason here // more notes";
        assert!(has_justification_comment(content, 1, "// SAFETY:"));
    }

    #[test]
    fn embedded_pattern_at_end_of_line_does_not_match() {
        // Pattern embedded at end should NOT match
        let content = "code // error message about // SAFETY:";
        assert!(!has_justification_comment(content, 1, "// SAFETY:"));
    }
}

mod strip_comment_markers_tests {
    use super::*;

    #[test]
    fn strips_single_line_comment() {
        assert_eq!(strip_comment_markers("// SAFETY:"), "SAFETY:");
        assert_eq!(strip_comment_markers("  // SAFETY:"), "SAFETY:");
    }

    #[test]
    fn strips_doc_comment() {
        assert_eq!(strip_comment_markers("/// SAFETY:"), "SAFETY:");
        assert_eq!(strip_comment_markers("//! SAFETY:"), "SAFETY:");
    }

    #[test]
    fn strips_shell_comment() {
        assert_eq!(strip_comment_markers("# SAFETY:"), "SAFETY:");
    }

    #[test]
    fn handles_pattern_with_marker() {
        // Pattern like "// SAFETY:" should extract "SAFETY:"
        assert_eq!(strip_comment_markers("// SAFETY:"), "SAFETY:");
    }
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
            let _unique: Vec<_> = matches
                .iter()
                .filter(|m| seen_lines.insert(m.line))
                .collect();
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
        use crate::adapter::{Adapter, GenericAdapter};
        use std::path::PathBuf;

        let root = std::path::Path::new("/project");
        let test_patterns = default_test_patterns();
        let paths: Vec<PathBuf> = (0..1000)
            .map(|i| PathBuf::from(format!("/project/src/module_{}.rs", i)))
            .collect();

        // Current approach: new adapter per file
        let iterations = 1;
        let start = Instant::now();
        for _ in 0..iterations {
            for path in &paths {
                let _ = classify_file(path, root, &test_patterns);
            }
        }
        let elapsed_new = start.elapsed();
        println!("=== File Classification Performance ===");
        println!("1K classifications (new adapter each): {:?}", elapsed_new);
        println!("Per classification: {:?}", elapsed_new / 1000);

        // Optimized: reuse adapter
        let adapter = GenericAdapter::new(&[], &test_patterns);
        let start = Instant::now();
        for _ in 0..iterations {
            for path in &paths {
                let relative = path.strip_prefix(root).unwrap_or(path);
                let _ = adapter.classify(relative);
            }
        }
        let elapsed_reuse = start.elapsed();
        println!("1K classifications (reused adapter): {:?}", elapsed_reuse);
        println!("Per classification: {:?}", elapsed_reuse / 1000);

        let speedup = elapsed_new.as_nanos() as f64 / elapsed_reuse.as_nanos() as f64;
        println!("Speedup from reuse: {:.1}x", speedup);
    }
}
