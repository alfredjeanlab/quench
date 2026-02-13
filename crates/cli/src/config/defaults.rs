// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Centralized default values for configuration.
//!
//! All default values are documented here for easy reference.
//! Individual config structs delegate to these constants via their `default_*` methods.

/// Default file size limits.
pub mod size {
    /// Default max lines for source files (800).
    pub const MAX_LINES: usize = 800;

    /// Default max lines for test files (1000).
    pub const MAX_LINES_TEST: usize = 1000;

    /// Default max tokens (~5k words, suitable for LLM context).
    pub const MAX_TOKENS: usize = 20000;

    /// Default max lines for spec files (1000).
    pub const MAX_LINES_SPEC: usize = 1000;
}

/// Default advice messages.
pub mod advice {
    /// Round `n` down to the nearest multiple of `step`.
    fn round_down(n: usize, step: usize) -> usize {
        (n / step) * step
    }

    /// Format a target range string like "150–250 lines".
    pub fn target_range(threshold: usize) -> String {
        let lo = threshold / 5;
        let hi = threshold / 3;
        // Only round for meaningful thresholds; small values lose
        // information when rounded to the nearest 10.
        let (lo, hi) =
            if threshold >= 100 { (round_down(lo, 10), round_down(hi, 10)) } else { (lo, hi) };
        format!("{lo}–{hi} lines")
    }

    /// Default advice for source file cloc violations.
    pub fn cloc_source(threshold: usize) -> String {
        let range = target_range(threshold);
        format!(
            "First, look for repetitive patterns that could be extracted into helper \
functions, or refactor to be more unit testable and concise.\n\
\n\
Then split the remainder into smaller files by semantic concern \
(target {range} each). Identify distinct responsibilities—types, \
matching, classification, orchestration—and give each its own module. \
Prefer a folder with focused submodules over a single large file.\n\
\n\
Avoid removing individual lines to satisfy the linter; \
prefer extracting testable code blocks."
        )
    }

    /// Default advice for test file cloc violations.
    pub fn cloc_test(threshold: usize) -> String {
        let range = target_range(threshold);
        format!(
            "First, look for tests that can be parameterized or share fixtures, \
and extract repetitive setup into helper functions.\n\
\n\
Then split by the semantic area they cover (target {range} each). \
Group tests by the concern they exercise and place each group in a \
subfolder or its own sibling test file."
        )
    }
}

/// Default glob patterns for test file detection.
pub mod test_patterns {
    /// Generic test patterns that work across languages.
    pub fn generic() -> Vec<String> {
        vec![
            "**/tests/**".to_string(),
            "**/test/**".to_string(),
            "**/*_test.*".to_string(),
            "**/*_tests.*".to_string(),
            "**/*.test.*".to_string(),
            "**/*.spec.*".to_string(),
            "**/test_*.*".to_string(),
        ]
    }
}
