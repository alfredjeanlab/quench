// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Content validation for agent files.
//!
//! Detects tables, box diagrams, and mermaid blocks in markdown content.

/// A detected content issue.
#[derive(Debug)]
pub struct ContentIssue {
    /// Line number where the issue starts (1-indexed).
    pub line: u32,
    /// Type of content detected.
    pub content_type: ContentType,
}

/// Type of content detected.
#[derive(Debug, Clone, Copy)]
pub enum ContentType {
    /// Markdown table (pipe-delimited).
    MarkdownTable,
    /// Box diagram (Unicode box-drawing characters).
    BoxDiagram,
    /// Mermaid code block.
    MermaidBlock,
}

impl ContentType {
    /// Get the violation type string for this content type.
    pub fn violation_type(&self) -> &'static str {
        match self {
            ContentType::MarkdownTable => "forbidden_table",
            ContentType::BoxDiagram => "forbidden_diagram",
            ContentType::MermaidBlock => "forbidden_mermaid",
        }
    }

    /// Get advice for fixing this content type violation.
    pub fn advice(&self) -> &'static str {
        match self {
            ContentType::MarkdownTable => {
                "Tables are not token-efficient. Convert to a list or prose."
            }
            ContentType::BoxDiagram => {
                "Box diagrams are not token-efficient. Use a simple list or description."
            }
            ContentType::MermaidBlock => {
                "Mermaid diagrams are not token-efficient. Use a simple list or description."
            }
        }
    }

    /// Get lowercase advice for use with a location prefix.
    ///
    /// Suggests allowed alternatives based on what content types are permitted.
    pub fn advice_with_alternatives(&self, mermaid_allowed: bool, box_allowed: bool) -> String {
        let alternatives = match self {
            ContentType::MarkdownTable => {
                // Tables can suggest either mermaid or box diagrams if allowed
                if mermaid_allowed {
                    "a list or mermaid diagram"
                } else if box_allowed {
                    "a list or ASCII diagram"
                } else {
                    "a list"
                }
            }
            ContentType::BoxDiagram => {
                if mermaid_allowed {
                    "a list or mermaid diagram"
                } else {
                    "a list"
                }
            }
            ContentType::MermaidBlock => {
                if box_allowed {
                    "a list or ASCII diagram"
                } else {
                    "a list"
                }
            }
        };

        let kind = match self {
            ContentType::MarkdownTable => "tables are",
            ContentType::BoxDiagram => "box diagrams are",
            ContentType::MermaidBlock => "mermaid diagrams are",
        };

        format!("{} forbidden. Use {} instead.", kind, alternatives)
    }
}

/// Scan content for markdown tables.
///
/// A markdown table is detected when:
/// - A line starts with `|` and ends with `|`
/// - Followed by a separator line with `|` and dashes
pub fn detect_tables(content: &str) -> Vec<ContentIssue> {
    let mut issues = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Check for table header row: | col | col |
        if trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.contains(" | ") {
            // Check if next line is a separator: |---|---|
            if let Some(next_line) = lines.get(i + 1) {
                let next_trimmed = next_line.trim();
                if next_trimmed.starts_with('|')
                    && next_trimmed.contains('-')
                    && next_trimmed.ends_with('|')
                {
                    issues.push(ContentIssue {
                        line: (i + 1) as u32, // 1-indexed
                        content_type: ContentType::MarkdownTable,
                    });
                }
            }
        }
    }

    issues
}

/// Scan content for box diagrams (ASCII art with box-drawing characters).
///
/// Detects lines containing Unicode box-drawing characters:
/// `\u{250C}` `\u{2510}` `\u{2514}` `\u{2518}` `\u{2502}` `\u{2500}` `\u{251C}` `\u{2524}` `\u{252C}` `\u{2534}` `\u{253C}`
pub fn detect_box_diagrams(content: &str) -> Vec<ContentIssue> {
    let mut issues = Vec::new();
    let box_chars = [
        '\u{250C}', // ┌
        '\u{2510}', // ┐
        '\u{2514}', // └
        '\u{2518}', // ┘
        '\u{2502}', // │
        '\u{2500}', // ─
        '\u{251C}', // ├
        '\u{2524}', // ┤
        '\u{252C}', // ┬
        '\u{2534}', // ┴
        '\u{253C}', // ┼
    ];

    for (i, line) in content.lines().enumerate() {
        // Check for box-drawing characters (need at least 2 to be a diagram)
        let box_count = line.chars().filter(|c| box_chars.contains(c)).count();
        if box_count >= 2 {
            issues.push(ContentIssue {
                line: (i + 1) as u32,
                content_type: ContentType::BoxDiagram,
            });
            // Only report first occurrence per file to avoid noise
            break;
        }
    }

    issues
}

/// Scan content for mermaid code blocks.
///
/// Detects ```mermaid and ~~~mermaid fenced code blocks.
pub fn detect_mermaid_blocks(content: &str) -> Vec<ContentIssue> {
    let mut issues = Vec::new();

    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```mermaid") || trimmed.starts_with("~~~mermaid") {
            issues.push(ContentIssue {
                line: (i + 1) as u32,
                content_type: ContentType::MermaidBlock,
            });
        }
    }

    issues
}

/// Size limit violation details.
#[derive(Debug)]
pub struct SizeViolation {
    /// Whether this is a line count or token count violation.
    pub limit_type: SizeLimitType,
    /// Actual value found.
    pub value: usize,
    /// Configured threshold.
    pub threshold: usize,
}

/// Type of size limit.
#[derive(Debug, Clone, Copy)]
pub enum SizeLimitType {
    /// Line count limit.
    Lines,
    /// Token count limit.
    Tokens,
}

impl SizeLimitType {
    /// Generate advice message for this limit type.
    pub fn advice(&self, value: usize, threshold: usize) -> String {
        match self {
            SizeLimitType::Lines => format!(
                "File has {} lines (max: {}). Split into smaller files or reduce content.",
                value, threshold
            ),
            SizeLimitType::Tokens => format!(
                "File has ~{} tokens (max: {}). Reduce content for token efficiency.",
                value, threshold
            ),
        }
    }

    /// Generate lowercase advice for use with a location prefix.
    pub fn advice_lowercase(&self, value: usize, threshold: usize) -> String {
        match self {
            SizeLimitType::Lines => format!(
                "file has {} lines (max: {}). Split into smaller files or reduce content.",
                value, threshold
            ),
            SizeLimitType::Tokens => format!(
                "file has ~{} tokens (max: {}). Reduce content for token efficiency.",
                value, threshold
            ),
        }
    }
}

/// Check if content exceeds the line limit.
pub fn check_line_count(content: &str, max_lines: usize) -> Option<SizeViolation> {
    let line_count = content.lines().count();
    if line_count > max_lines {
        Some(SizeViolation {
            limit_type: SizeLimitType::Lines,
            value: line_count,
            threshold: max_lines,
        })
    } else {
        None
    }
}

/// Check if content exceeds the token limit.
///
/// Uses `chars / 4` as a fast approximation.
pub fn check_token_count(content: &str, max_tokens: usize) -> Option<SizeViolation> {
    let token_estimate = content.chars().count() / 4;
    if token_estimate > max_tokens {
        Some(SizeViolation {
            limit_type: SizeLimitType::Tokens,
            value: token_estimate,
            threshold: max_tokens,
        })
    } else {
        None
    }
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;
