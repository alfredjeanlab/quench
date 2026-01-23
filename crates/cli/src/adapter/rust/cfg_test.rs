//! #[cfg(test)] block detection.
//!
//! Parses Rust source files to identify line ranges inside #[cfg(test)] blocks.

use std::ops::Range;

/// Result of parsing a Rust file for #[cfg(test)] blocks.
#[derive(Debug, Default)]
pub struct CfgTestInfo {
    /// Line ranges (0-indexed) that are inside #[cfg(test)] blocks.
    pub test_ranges: Vec<Range<usize>>,
}

impl CfgTestInfo {
    /// Parse a Rust source file to find #[cfg(test)] block ranges.
    ///
    /// Uses a simplified brace-counting approach:
    /// 1. Scan for #[cfg(test)] attribute
    /// 2. Count { and } to track block depth (skipping string literals)
    /// 3. Block ends when brace depth returns to 0
    ///
    /// Limitations (acceptable for v1):
    /// - Raw strings (`r#"..."#`) are not fully handled
    /// - Multi-line attributes not fully supported
    /// - `mod tests;` (external module) declarations need file-level classification
    pub fn parse(content: &str) -> Self {
        let mut info = Self::default();
        let mut in_cfg_test = false;
        let mut brace_depth: i32 = 0;
        let mut block_start = 0;

        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Check for #[cfg(test)] attribute
            if !in_cfg_test && is_cfg_test_attr(trimmed) {
                in_cfg_test = true;
                block_start = line_idx;
                brace_depth = 0;
                continue;
            }

            if in_cfg_test {
                // Count braces outside of string literals
                let mut in_string = false;
                let mut prev_char = '\0';

                for ch in trimmed.chars() {
                    if ch == '"' && prev_char != '\\' {
                        in_string = !in_string;
                    } else if !in_string {
                        match ch {
                            '{' => brace_depth += 1,
                            '}' => {
                                brace_depth -= 1;
                                if brace_depth == 0 {
                                    // End of #[cfg(test)] block
                                    info.test_ranges.push(block_start..line_idx + 1);
                                    in_cfg_test = false;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    prev_char = ch;
                }
            }
        }

        info
    }

    /// Check if a line (0-indexed) is inside a #[cfg(test)] block.
    pub fn is_test_line(&self, line_idx: usize) -> bool {
        self.test_ranges.iter().any(|r| r.contains(&line_idx))
    }
}

/// Check if a line is a #[cfg(test)] attribute.
pub(crate) fn is_cfg_test_attr(line: &str) -> bool {
    // Match #[cfg(test)] with optional whitespace
    line.starts_with("#[cfg(test)]")
        || line.starts_with("#[cfg( test )]")
        || line.contains("#[cfg(test)]")
}

#[cfg(test)]
#[path = "cfg_test_tests.rs"]
mod tests;
