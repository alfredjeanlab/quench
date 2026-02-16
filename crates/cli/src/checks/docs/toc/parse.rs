// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TOC parsing utilities.
//!
//! Extracts fenced code blocks and parses directory tree entries.

/// A fenced code block extracted from markdown.
#[derive(Debug)]
pub(crate) struct FencedBlock {
    /// Line number where the block starts (1-indexed, after the opening ```).
    pub(crate) start_line: u32,
    /// Content lines within the block.
    pub(crate) lines: Vec<String>,
    /// Language tag from the opening fence (e.g., "rust", "bash", "text").
    /// None if no tag was specified.
    pub(crate) language: Option<String>,
}

/// A parsed entry from a directory tree.
#[derive(Debug, PartialEq)]
pub(crate) struct TreeEntry {
    /// Relative line offset within the block (0-indexed).
    pub(crate) line_offset: u32,
    /// The extracted path (may be file or directory).
    pub(crate) path: String,
    /// True if this appears to be a directory (ends with /).
    pub(crate) is_dir: bool,
}

/// Extract all fenced code blocks from markdown content.
pub(crate) fn extract_fenced_blocks(content: &str) -> Vec<FencedBlock> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current_lines = Vec::new();
    let mut start_line = 0u32;
    let mut current_language: Option<String> = None;

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx as u32 + 1;
        let trimmed = line.trim();

        if !in_block && trimmed.starts_with("```") {
            // Start of fenced block
            in_block = true;
            start_line = line_num + 1; // Content starts on next line
            current_lines.clear();

            // Extract language tag after ```
            let after_fence = trimmed.strip_prefix("```").unwrap_or("").trim();
            current_language = if after_fence.is_empty() {
                None
            } else {
                // Take first word as language (handles ```rust,linenos)
                Some(
                    after_fence
                        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                        .next()
                        .unwrap_or("")
                        .to_lowercase(),
                )
                .filter(|s| !s.is_empty())
            };
        } else if in_block && trimmed == "```" {
            // End of fenced block
            in_block = false;
            blocks.push(FencedBlock {
                start_line,
                lines: std::mem::take(&mut current_lines),
                language: current_language.take(),
            });
        } else if in_block {
            current_lines.push(line.to_string());
        }
    }

    blocks
}

/// Parse a directory tree block into entries.
pub(crate) fn parse_tree_block(block: &FencedBlock) -> Vec<TreeEntry> {
    let mut entries = Vec::new();
    let mut current_path_stack: Vec<String> = Vec::new();

    for (offset, line) in block.lines.iter().enumerate() {
        if let Some(entry) = parse_tree_line(line, offset as u32, &mut current_path_stack) {
            entries.push(entry);
        }
    }

    entries
}

/// Parse a single line of a directory tree.
///
/// Returns Some(TreeEntry) if the line contains a path entry.
fn parse_tree_line(
    line: &str,
    line_offset: u32,
    path_stack: &mut Vec<String>,
) -> Option<TreeEntry> {
    // Strip box-drawing characters and measure indent
    let (indent_level, name) = extract_indent_and_name(line)?;

    // Strip trailing comment (after #)
    let name = strip_comment(&name);
    if name.is_empty() {
        return None;
    }

    // Ignore ellipsis, continuation markers, and directory reference entries
    // These are placeholders, not actual files to validate
    let name_without_slash = name.trim_end_matches('/');
    if matches!(name_without_slash, "." | ".." | "..." | "etc..." | "etc.") {
        return None;
    }

    let is_dir = name.ends_with('/');
    let name = name.trim_end_matches('/');

    // Adjust path stack to current indent level
    path_stack.truncate(indent_level);

    // Build full path
    let full_path = if path_stack.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", path_stack.join("/"), name)
    };

    // If directory, push to stack for children
    if is_dir {
        path_stack.push(name.to_string());
    }

    Some(TreeEntry { line_offset, path: full_path, is_dir })
}

/// Extract indent level and name from a tree line.
///
/// Handles both box-drawing (├── name) and indentation (  name) formats.
fn extract_indent_and_name(line: &str) -> Option<(usize, String)> {
    // Check if this is a box-drawing tree line
    let has_box_drawing = line.contains('├') || line.contains('└') || line.contains('│');

    if has_box_drawing {
        // Box-drawing format
        // The indent is determined by the column position of ├ or └
        // Each tree level typically takes 4 characters: "│   " or "├── "
        let mut column = 0usize;
        let mut chars = line.chars().peekable();

        loop {
            match chars.peek() {
                Some(&c @ ('│' | '├' | '└')) => {
                    // Found a box-drawing character
                    chars.next();
                    column += 1;

                    if c == '├' || c == '└' {
                        // Branch marker - skip "── " that follows and extract name
                        while let Some(&c) = chars.peek() {
                            if c == '─' || c == ' ' {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        break;
                    }
                    // For │, continue to count more prefix
                }
                Some(' ') => {
                    chars.next();
                    column += 1;
                }
                _ => break,
            }
        }

        // Calculate indent level: typically 4 columns per level
        // But the first level starts at column 0 with ├──
        // So column 0-3 = level 1, column 4-7 = level 2, etc.
        let indent = column.div_ceil(4);

        let remaining: String = chars.collect();
        let name = remaining.trim().to_string();

        if name.is_empty() { None } else { Some((indent, name)) }
    } else {
        // Indentation format (spaces/tabs)
        let mut indent = 0usize;
        let mut chars = line.chars().peekable();

        // Count leading whitespace
        let mut spaces: usize = 0;
        while let Some(&c) = chars.peek() {
            match c {
                ' ' => {
                    spaces += 1;
                    chars.next();
                }
                '\t' => {
                    indent += 1;
                    chars.next();
                }
                _ => break,
            }
        }

        // Convert spaces to indent level (2 spaces = 1 level)
        if spaces > 0 {
            indent += spaces.div_ceil(2);
        }

        let remaining: String = chars.collect();
        let name = remaining.trim().to_string();

        if name.is_empty() { None } else { Some((indent, name)) }
    }
}

/// Strip comment or trailing description from an entry name.
///
/// Strips both `# hash comments` and trailing descriptions after 3+ consecutive spaces.
/// For example: `runner.rs       command dispatch` → `runner.rs`
fn strip_comment(name: &str) -> &str {
    // Strip hash comments
    let name = if let Some(pos) = name.find('#') { name[..pos].trim() } else { name.trim() };
    // Strip trailing descriptions after 3+ consecutive spaces
    if let Some(pos) = name.find("   ") { name[..pos].trim_end() } else { name }
}

/// Normalize a path by stripping `.`/`./` prefix when it represents current directory.
/// Does NOT strip from hidden files/directories like `.tmpXXX/`.
pub(super) fn normalize_dot_prefix(path: &str) -> &str {
    // `./foo` → `foo`
    if let Some(rest) = path.strip_prefix("./") {
        return rest;
    }
    // `.` alone → empty (current directory)
    if path == "." {
        return "";
    }
    // `.foo` is a hidden file/directory, not current directory reference
    path
}
