// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TOC (directory tree) validation.

use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::check::{CheckContext, Violation};

/// A fenced code block extracted from markdown.
#[derive(Debug)]
pub(super) struct FencedBlock {
    /// Line number where the block starts (1-indexed, after the opening ```).
    pub(super) start_line: u32,
    /// Content lines within the block.
    pub(super) lines: Vec<String>,
    /// Language tag from the opening fence (e.g., "rust", "bash", "text").
    /// None if no tag was specified.
    pub(super) language: Option<String>,
}

/// A parsed entry from a directory tree.
#[derive(Debug, PartialEq)]
pub(super) struct TreeEntry {
    /// Relative line offset within the block (0-indexed).
    pub(super) line_offset: u32,
    /// The extracted path (may be file or directory).
    pub(super) path: String,
    /// True if this appears to be a directory (ends with /).
    pub(super) is_dir: bool,
}

/// Extract all fenced code blocks from markdown content.
pub(super) fn extract_fenced_blocks(content: &str) -> Vec<FencedBlock> {
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
pub(super) fn parse_tree_block(block: &FencedBlock) -> Vec<TreeEntry> {
    let mut entries = Vec::new();
    let mut current_path_stack: Vec<String> = Vec::new();

    for (offset, line) in block.lines.iter().enumerate() {
        if let Some(entry) = parse_tree_line(line, offset as u32, &mut current_path_stack) {
            entries.push(entry);
        }
    }

    entries
}

/// Check if a block matches a valid tree format (box-drawing or indentation).
pub(super) fn is_valid_tree_format(block: &FencedBlock) -> bool {
    if block.lines.is_empty() {
        return false;
    }
    let has_format = block.lines.iter().any(|line| {
        let t = line.trim();
        line.contains('├')
            || line.contains('└')
            || line.contains('│')
            || line.starts_with(' ')
            || line.starts_with('\t')
            || (t.ends_with('/') && !t.contains(' '))
    });
    has_format && !parse_tree_block(block).is_empty()
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

    Some(TreeEntry {
        line_offset,
        path: full_path,
        is_dir,
    })
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

        if name.is_empty() {
            None
        } else {
            Some((indent, name))
        }
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

        if name.is_empty() {
            None
        } else {
            Some((indent, name))
        }
    }
}

/// Strip comment suffix (everything after #).
fn strip_comment(name: &str) -> &str {
    if let Some(pos) = name.find('#') {
        name[..pos].trim()
    } else {
        name.trim()
    }
}

/// Resolution strategy for TOC entries.
#[derive(Debug, Clone, Copy)]
enum ResolutionStrategy {
    /// Relative to the markdown file's directory (`.`/`./` treated as current directory)
    RelativeToFile,
    /// Relative to project root
    RelativeToRoot,
    /// Strip markdown file's parent directory name prefix
    StripParentDirName,
}

impl ResolutionStrategy {
    fn description(&self) -> &'static str {
        match self {
            Self::RelativeToFile => "relative to markdown file",
            Self::RelativeToRoot => "relative to project root",
            Self::StripParentDirName => "stripping parent directory prefix",
        }
    }
}

/// Normalize a path by stripping `.`/`./` prefix when it represents current directory.
/// Does NOT strip from hidden files/directories like `.tmpXXX/`.
fn normalize_dot_prefix(path: &str) -> &str {
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

/// Check if a path contains glob wildcards.
fn is_glob_pattern(path: &str) -> bool {
    path.contains('*')
}

/// Try to resolve a glob pattern by finding any matching file.
/// Uses the `ignore` crate for fast parallel directory walking.
fn try_resolve_glob(base: &Path, pattern: &str) -> bool {
    let Ok(glob) = Glob::new(pattern) else {
        return false;
    };
    let matcher = glob.compile_matcher();

    // Use ignore crate's WalkBuilder for fast traversal
    let walker = ignore::WalkBuilder::new(base).max_depth(Some(10)).build();

    for entry in walker.flatten() {
        let path = entry.path();
        // Get path relative to base for matching
        if let Ok(relative) = path.strip_prefix(base)
            && matcher.is_match(relative)
        {
            return true;
        }
    }
    false
}

/// Try to resolve a path using a specific strategy.
fn try_resolve(
    root: &Path,
    md_file: &Path,
    entry_path: &str,
    strategy: ResolutionStrategy,
) -> bool {
    // Normalize `.`/`./` prefix for all strategies
    let normalized = normalize_dot_prefix(entry_path);

    // Handle glob patterns
    if is_glob_pattern(normalized) {
        return match strategy {
            ResolutionStrategy::RelativeToFile => {
                if let Some(parent) = md_file.parent() {
                    try_resolve_glob(parent, normalized)
                } else {
                    false
                }
            }
            ResolutionStrategy::RelativeToRoot => try_resolve_glob(root, normalized),
            ResolutionStrategy::StripParentDirName => {
                if let Some(parent) = md_file.parent()
                    && let Some(parent_name) = parent.file_name().and_then(|n| n.to_str())
                {
                    let prefix = format!("{}/", parent_name);
                    if let Some(stripped) = normalized.strip_prefix(&prefix) {
                        return try_resolve_glob(parent, stripped);
                    }
                }
                false
            }
        };
    }

    match strategy {
        ResolutionStrategy::RelativeToFile => {
            if let Some(parent) = md_file.parent() {
                parent.join(normalized).exists()
            } else {
                false
            }
        }
        ResolutionStrategy::RelativeToRoot => root.join(normalized).exists(),
        ResolutionStrategy::StripParentDirName => {
            // Get the parent directory name of the markdown file
            if let Some(parent) = md_file.parent()
                && let Some(parent_name) = parent.file_name().and_then(|n| n.to_str())
            {
                // Try stripping the parent dir name prefix
                // e.g., "quality/foo.sh" in checks/quality/README.md → checks/quality/foo.sh
                let prefix = format!("{}/", parent_name);
                if let Some(stripped) = normalized.strip_prefix(&prefix) {
                    return parent.join(stripped).exists();
                }
            }
            false
        }
    }
}

/// Try all resolution strategies for a block of entries.
/// Returns None if a strategy resolves all entries, or Some with unresolved entries.
fn try_resolve_block<'a>(
    root: &Path,
    md_file: &Path,
    entries: &'a [TreeEntry],
    strategy: ResolutionStrategy,
) -> Option<Vec<&'a TreeEntry>> {
    let unresolved: Vec<_> = entries
        .iter()
        .filter(|e| !e.is_dir && !try_resolve(root, md_file, &e.path, strategy))
        .collect();

    if unresolved.is_empty() {
        None
    } else {
        Some(unresolved)
    }
}

/// Build a GlobSet from patterns.
fn build_glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

/// Language tag that forces TOC validation.
const TOC_LANGUAGE: &str = "toc";

/// Language tags that indicate the block is NOT a directory tree.
#[rustfmt::skip]
const NON_TREE_LANGUAGES: &[&str] = &[
    // Explicit skip annotations
    "no-toc", "ignore",
    // Code languages
    "rust", "rs", "go", "python", "py", "javascript", "js", "typescript", "ts",
    "java", "c", "cpp", "csharp", "cs", "ruby", "rb", "php", "swift", "kotlin",
    "scala", "perl", "lua", "r", "julia", "haskell", "hs", "ocaml", "ml",
    "elixir", "ex", "erlang", "clojure", "clj", "lisp", "scheme", "racket",
    "zig", "nim", "d", "v", "odin", "jai", "carbon",
    // Shell and scripting
    "bash", "sh", "zsh", "fish", "powershell", "pwsh", "bat", "cmd",
    // Config and data
    "toml", "yaml", "yml", "json", "xml", "ini", "cfg",
    // Output and plain text
    "text", "txt", "output", "console", "terminal", "log",
    // Markup and other
    "html", "css", "scss", "sass", "less", "sql", "graphql", "gql",
    "dockerfile", "makefile", "cmake",
];

/// Check if a fenced block looks like a directory tree.
pub(super) fn looks_like_tree(block: &FencedBlock) -> bool {
    // Explicit toc tag forces validation
    if block.language.as_deref() == Some(TOC_LANGUAGE) {
        return true;
    }
    // Skip known non-tree language tags
    if let Some(ref lang) = block.language
        && NON_TREE_LANGUAGES.contains(&lang.as_str())
    {
        return false;
    }
    if block.lines.is_empty() {
        return false;
    }
    // Box diagrams (with top corners) are not trees
    if block
        .lines
        .iter()
        .any(|l| l.contains('┌') || l.contains('╔') || l.contains('╭'))
    {
        return false;
    }
    // Count tree signals
    let has_box = |l: &str| l.contains('├') || l.contains('└') || l.contains('│');
    let is_dir = |l: &str| l.trim().ends_with('/') && !l.contains(' ') && !l.contains('=');
    let (box_ct, dir_ct, file_ct) = block.lines.iter().fold((0, 0, 0), |(b, d, f), line| {
        (
            b + has_box(line) as usize,
            d + is_dir(line) as usize,
            f + is_tree_line(line) as usize,
        )
    });
    // Strong: box-drawing chars, or directory lines with file-like content
    if box_ct >= 1 || (dir_ct >= 1 && file_ct >= 2) {
        return true;
    }
    // Weak: 3+ file-like lines without error output patterns
    file_ct >= 3
        && !block
            .lines
            .iter()
            .any(|l| looks_like_error_output(l.trim()))
}

/// Check if a line looks like compiler/linter error output.
///
/// Matches patterns like:
/// - `file.ext:123:` (file:line:)
/// - `file.ext:123:45:` (file:line:col:)
/// - `file.ext:123: message` (file:line: message)
fn looks_like_error_output(line: &str) -> bool {
    // Look for pattern: something.ext:digits:
    // Must have: extension with dot, colon, digits, colon
    let Some(colon_pos) = line.find(':') else {
        return false;
    };

    let before_colon = &line[..colon_pos];

    // Must look like a file path (contains dot for extension)
    if !before_colon.contains('.') {
        return false;
    }

    // Check if what follows the colon starts with digits
    let after_colon = &line[colon_pos + 1..];
    let first_after = after_colon.chars().next();

    match first_after {
        Some(c) if c.is_ascii_digit() => {
            // Looks like file.ext:123...
            // Check if followed by another colon (file:line: or file:line:col:)
            if let Some(next_colon) = after_colon.find(':') {
                let between = &after_colon[..next_colon];
                // All digits between first and second colon
                if between.chars().all(|c| c.is_ascii_digit()) {
                    return true;
                }
            }
        }
        _ => {}
    }

    false
}

/// Check if a line looks like a directory tree entry.
fn is_tree_line(line: &str) -> bool {
    let trimmed = line.trim();

    // Empty lines don't count
    if trimmed.is_empty() {
        return false;
    }

    // Box-drawing characters are strong tree indicators
    if trimmed.contains('├') || trimmed.contains('└') || trimmed.contains('│') {
        return true;
    }

    // Reject TOML/config patterns
    // [section] headers
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return false;
    }
    // key = value assignments
    if trimmed.contains(" = ") {
        return false;
    }
    // TOML table arrays [[...]]
    if trimmed.starts_with("[[") {
        return false;
    }

    // Reject error output patterns
    // Pattern: file.ext:line: or file.ext:line:col:
    // Examples: "foo.rs:23:", "src/main.go:45:12:", "script.sh:10: error"
    if looks_like_error_output(trimmed) {
        return false;
    }

    // Directory paths ending with /
    if trimmed.ends_with('/') && !trimmed.contains(' ') && !trimmed.contains('=') {
        return true;
    }

    // Check for file-like patterns that aren't code or config
    // A file path typically looks like: foo/bar.rs, lib.rs, etc.
    if trimmed.contains('.') && !trimmed.starts_with('.') {
        // Reject if it looks like code or config
        let code_patterns = ['(', ')', '=', ';', '{', '}', '"', '\'', '[', ']'];
        let code_keywords = [
            "let ", "fn ", "use ", "pub ", "mod ", "const ", "static ", "name ", "path ",
        ];

        let has_code_pattern = code_patterns.iter().any(|&c| trimmed.contains(c));
        let has_code_keyword = code_keywords.iter().any(|kw| trimmed.contains(kw));

        if !has_code_pattern && !has_code_keyword && !trimmed.contains("//") {
            // Looks like a file path: no spaces except possibly in comments
            let before_comment = trimmed.split('#').next().unwrap_or(trimmed).trim();
            if !before_comment.contains(' ')
                || before_comment.starts_with("├")
                || before_comment.starts_with("└")
            {
                return true;
            }
        }
    }

    false
}

/// Validate TOC entries in all markdown files.
pub fn validate_toc(ctx: &CheckContext, violations: &mut Vec<Violation>) {
    let config = &ctx.config.check.docs.toc;

    // Check if TOC validation is disabled
    let check_level = config
        .check
        .as_deref()
        .or(ctx.config.check.docs.check.as_deref())
        .unwrap_or("error");
    if check_level == "off" {
        return;
    }

    // Build include/exclude matchers
    let include_set = build_glob_set(&config.include);
    let exclude_set = build_glob_set(&config.exclude);

    // Process each markdown file
    for walked in ctx.files {
        // Get relative path for glob matching
        let relative_path = walked.path.strip_prefix(ctx.root).unwrap_or(&walked.path);
        let path_str = relative_path.to_string_lossy();

        // Check include patterns
        if !include_set.is_match(&*path_str) {
            continue;
        }

        // Check exclude patterns
        if exclude_set.is_match(&*path_str) {
            continue;
        }

        // Read file content (walked.path is already absolute)
        let content = match std::fs::read_to_string(&walked.path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Extract and validate TOC blocks
        validate_file_toc(ctx, relative_path, &content, violations);
    }
}

/// Validate TOC entries in a single file.
fn validate_file_toc(
    ctx: &CheckContext,
    relative_path: &Path,
    content: &str,
    violations: &mut Vec<Violation>,
) {
    let blocks = extract_fenced_blocks(content);
    let strategies = [
        ResolutionStrategy::RelativeToFile,
        ResolutionStrategy::RelativeToRoot,
        ResolutionStrategy::StripParentDirName,
    ];

    for block in blocks {
        // For explicit toc tag, validate format
        if block.language.as_deref() == Some(TOC_LANGUAGE) && !is_valid_tree_format(&block) {
            violations.push(Violation::file(
                relative_path,
                block.start_line,
                "invalid_toc_format",
                "Code block marked as `toc` doesn't match box-drawing or indentation format.\n\
                 Use box-drawing (├──, └──, │) or consistent indentation.",
            ));
            continue;
        }

        // Skip blocks that don't look like directory trees
        if !looks_like_tree(&block) {
            continue;
        }

        let entries = parse_tree_block(&block);
        let abs_file = ctx.root.join(relative_path);
        let file_entries: Vec<_> = entries.iter().filter(|e| !e.is_dir).collect();

        // Find entries that fail ALL strategies (truly broken)
        let resolves_any = |e: &&TreeEntry| {
            strategies
                .iter()
                .any(|&s| try_resolve(ctx.root, &abs_file, &e.path, s))
        };
        let failed_all: Vec<_> = file_entries.iter().filter(|e| !resolves_any(e)).collect();

        let total = file_entries.len();
        let (to_report, advice) = if !failed_all.is_empty() {
            let tried: Vec<_> = strategies.iter().map(|s| s.description()).collect();
            let (valid, failed) = (total - failed_all.len(), failed_all.len());
            (
                failed_all.into_iter().copied().collect(),
                format!(
                    "File does not exist ({valid} of {total} paths valid, {failed} failed).\n\
                     This check ensures directory trees in documentation stay up-to-date.\n\
                     Update the table of contents or directory tree to match actual files.\n\
                     If this is illustrative, add a ```no-toc language tag.\n\nTried: {}",
                    tried.join(", ")
                ),
            )
        } else {
            // Check if any single strategy resolves all entries
            let mut best: Option<(ResolutionStrategy, Vec<&TreeEntry>)> = None;
            for strategy in strategies {
                match try_resolve_block(ctx.root, &abs_file, &entries, strategy) {
                    None => {
                        best = None;
                        break;
                    }
                    Some(f) if best.as_ref().is_none_or(|(_, b)| f.len() < b.len()) => {
                        best = Some((strategy, f));
                    }
                    _ => {}
                }
            }
            let Some((strategy, failed)) = best else {
                continue;
            };
            let (valid, failed_count) = (total - failed.len(), failed.len());
            (
                failed,
                format!(
                    "File does not exist ({valid} of {total} paths valid, {failed_count} failed).\n\
                     This check ensures directory trees in documentation stay up-to-date.\n\
                     TOC entries should use a consistent path style (resolving {}).\n\
                     If this is illustrative, add a ```no-toc language tag.",
                    strategy.description()
                ),
            )
        };

        for entry in to_report {
            violations.push(
                Violation::file(
                    relative_path,
                    block.start_line + entry.line_offset,
                    "broken_toc",
                    &advice,
                )
                .with_path(entry.path.clone()),
            );
        }
    }
}

#[cfg(test)]
#[path = "toc_tests.rs"]
mod tests;
