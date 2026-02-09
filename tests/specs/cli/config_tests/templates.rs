// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Template validation specs.

use std::path::PathBuf;

// =============================================================================
// Guide Template Validation
// =============================================================================

/// Spec: All TOML code blocks in guide.*.md files must be valid parseable config
///
/// > Configuration guide files contain TOML examples that users will copy.
/// > Every code block must be valid TOML that quench can parse.
#[test]
fn guide_templates_contain_valid_toml() {
    let templates_dir =
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/specs/templates"));

    // Find all guide.*.md files
    let guide_files: Vec<_> = std::fs::read_dir(&templates_dir)
        .expect("templates directory should exist")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let filename = path.file_name()?.to_str()?;

            if filename.starts_with("guide.") && filename.ends_with(".md") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    assert!(!guide_files.is_empty(), "No guide.*.md files found in docs/specs/templates/");

    let mut failures = Vec::new();

    for guide_path in guide_files {
        let filename = guide_path.file_name().unwrap().to_str().unwrap();
        let content = std::fs::read_to_string(&guide_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", filename, e));

        // Extract all ```toml code blocks
        let code_blocks = extract_toml_blocks(&content);

        if code_blocks.is_empty() {
            failures
                .push(format!("{}: No TOML code blocks found (expected at least one)", filename));
            continue;
        }

        // Validate each code block
        for (block_num, block) in code_blocks.iter().enumerate() {
            match toml::from_str::<toml::Value>(block) {
                Ok(_) => {
                    // Valid TOML - great!
                }
                Err(e) => {
                    failures.push(format!(
                        "{} (block {}): Invalid TOML\n  Error: {}\n  Block:\n{}",
                        filename,
                        block_num + 1,
                        e,
                        indent_lines(block, 4)
                    ));
                }
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "\n\nGuide template validation failures:\n\n{}\n\n\
             All TOML code blocks in guide.*.md files must be valid parseable TOML.\n\
             Fix the syntax errors in the guide files.\n",
            failures.join("\n\n")
        );
    }
}

/// Spec: Guide templates should not contain version = 1
///
/// > Guide files are configuration fragments, not complete files.
/// > They should not include `version = 1` as that's boilerplate.
#[test]
fn guide_templates_exclude_version_tag() {
    let templates_dir =
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/specs/templates"));

    let guide_files: Vec<_> = std::fs::read_dir(&templates_dir)
        .expect("templates directory should exist")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let filename = path.file_name()?.to_str()?;

            if filename.starts_with("guide.") && filename.ends_with(".md") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    let mut violations = Vec::new();

    for guide_path in guide_files {
        let filename = guide_path.file_name().unwrap().to_str().unwrap();
        let content = std::fs::read_to_string(&guide_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", filename, e));

        let code_blocks = extract_toml_blocks(&content);

        for (block_num, block) in code_blocks.iter().enumerate() {
            if block.contains("version = 1") || block.contains("version=1") {
                violations.push(format!(
                    "{} (block {}): Contains 'version = 1'",
                    filename,
                    block_num + 1
                ));
            }
        }
    }

    if !violations.is_empty() {
        panic!(
            "\n\nGuide templates should not include 'version = 1':\n\n{}\n\n\
             Guide files are configuration fragments, not complete files.\n\
             Remove 'version = 1' from these code blocks.\n",
            violations.join("\n")
        );
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Extract all ```toml code blocks from markdown content
fn extract_toml_blocks(markdown: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut in_toml_block = false;
    let mut current_block = String::new();

    for line in markdown.lines() {
        if line.trim() == "```toml" {
            in_toml_block = true;
            current_block.clear();
        } else if in_toml_block && line.trim() == "```" {
            in_toml_block = false;
            if !current_block.trim().is_empty() {
                blocks.push(current_block.clone());
            }
        } else if in_toml_block {
            current_block.push_str(line);
            current_block.push('\n');
        }
    }

    blocks
}

/// Indent each line by n spaces
fn indent_lines(text: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    text.lines().map(|line| format!("{}{}", indent, line)).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn extract_toml_blocks_finds_simple_block() {
        let markdown = r#"
# Config

```toml
[check.cloc]
max_lines = 100
```

More text.
"#;
        let blocks = extract_toml_blocks(markdown);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].contains("max_lines = 100"));
    }

    #[test]
    fn extract_toml_blocks_finds_multiple_blocks() {
        let markdown = r#"
```toml
[check.cloc]
max_lines = 100
```

Some text.

```toml
[check.tests]
check = "error"
```
"#;
        let blocks = extract_toml_blocks(markdown);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].contains("max_lines"));
        assert!(blocks[1].contains("check.tests"));
    }

    #[test]
    fn extract_toml_blocks_ignores_other_code_blocks() {
        let markdown = r#"
```bash
cargo test
```

```toml
[check.cloc]
max_lines = 100
```

```rust
fn main() {}
```
"#;
        let blocks = extract_toml_blocks(markdown);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].contains("max_lines"));
    }

    #[test]
    fn extract_toml_blocks_handles_empty_blocks() {
        let markdown = r#"
```toml
```

```toml
[check.cloc]
max_lines = 100
```
"#;
        let blocks = extract_toml_blocks(markdown);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].contains("max_lines"));
    }
}
