// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Git diff analysis for inline test detection.

use std::path::Path;

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;

/// Specifies the git diff range for inline test detection.
#[derive(Debug, Clone, Copy)]
pub enum DiffRange<'a> {
    /// Staged changes (--cached)
    Staged,
    /// Branch changes (base..HEAD)
    Branch(&'a str),
    /// Single commit (hash^..hash)
    Commit(&'a str),
}

/// Check if a Rust source file has inline test changes (#[cfg(test)] blocks).
///
/// Returns true if the file's diff contains changes within a #[cfg(test)] module.
pub fn has_inline_test_changes(file_path: &Path, root: &Path, range: DiffRange<'_>) -> bool {
    let diff_content = match get_file_diff(file_path, root, range) {
        Ok(content) => content,
        Err(_) => return false,
    };

    changes_in_cfg_test(&diff_content)
}

/// Get the diff for a specific file.
fn get_file_diff(file_path: &Path, root: &Path, range: DiffRange<'_>) -> Result<String, String> {
    use std::process::Command;

    let rel_path = file_path.strip_prefix(root).unwrap_or(file_path);
    let rel_path_str = rel_path.to_str().ok_or_else(|| "invalid path".to_string())?;

    let range_str = match range {
        DiffRange::Staged => String::new(),
        DiffRange::Branch(base) => format!("{}..HEAD", base),
        DiffRange::Commit(hash) => format!("{}^..{}", hash, hash),
    };

    let args: Vec<&str> = if range_str.is_empty() {
        vec!["diff", "--cached", "--", rel_path_str]
    } else {
        vec!["diff", &range_str, "--", rel_path_str]
    };

    let output = Command::new("git")
        .args(&args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("failed to run git: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git diff failed: {}", stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Parse diff content to detect if changes are within #[cfg(test)] blocks.
///
/// Tracks state machine:
/// - Looking for `#[cfg(test)]` marker
/// - Once found, track brace depth to identify block extent
/// - Check if any `+` lines are within the block
pub fn changes_in_cfg_test(diff_content: &str) -> bool {
    let mut in_cfg_test = false;
    let mut brace_depth = 0;
    let mut found_changes_in_test = false;

    for line in diff_content.lines() {
        // Skip diff metadata lines
        if line.starts_with("diff ")
            || line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("@@ ")
        {
            continue;
        }

        // Get the actual content (strip +/- prefix for analysis)
        let content = line
            .strip_prefix('+')
            .or_else(|| line.strip_prefix('-'))
            .or_else(|| line.strip_prefix(' '))
            .unwrap_or(line);

        let trimmed = content.trim();

        // Detect #[cfg(test)] marker
        if trimmed.contains("#[cfg(test)]") {
            in_cfg_test = true;
            brace_depth = 0;
            continue;
        }

        // Track brace depth when inside cfg(test)
        if in_cfg_test {
            // Count braces in content
            for ch in content.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => {
                        brace_depth -= 1;
                        if brace_depth <= 0 {
                            in_cfg_test = false;
                        }
                    }
                    _ => {}
                }
            }

            // Check if this is an added line within the test block
            if line.starts_with('+') && brace_depth > 0 {
                found_changes_in_test = true;
            }
        }
    }

    found_changes_in_test
}
