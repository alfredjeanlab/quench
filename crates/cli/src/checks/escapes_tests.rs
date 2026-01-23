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
