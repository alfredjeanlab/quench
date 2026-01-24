// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Sync checking logic for agent files.
//!
//! Provides section-level markdown parsing and comparison
//! to detect when agent files are out of sync.

use std::collections::{HashMap, HashSet};

/// A parsed markdown section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// Section name (normalized: lowercase, trimmed).
    pub name: String,
    /// Original heading text (for display).
    pub heading: String,
    /// Content below the heading (until next section or EOF).
    pub content: String,
    /// Line number where section starts (1-indexed).
    pub line: u32,
}

/// Parse markdown content into sections.
///
/// Sections are delimited by `## ` headings.
/// Content before the first `## ` is captured as a preamble section with an empty name.
pub fn parse_sections(content: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current_name = String::new();
    let mut current_heading = String::new();
    let mut current_content = String::new();
    let mut current_line: u32 = 1;
    let mut in_preamble = true;

    for (line_num, line) in content.lines().enumerate() {
        let line_number = (line_num + 1) as u32;

        if let Some(heading) = line.strip_prefix("## ") {
            // Save previous section
            if !in_preamble || !current_content.trim().is_empty() {
                sections.push(Section {
                    name: normalize_name(&current_name),
                    heading: current_heading.clone(),
                    content: current_content.trim_end().to_string(),
                    line: current_line,
                });
            }

            // Start new section
            current_name = heading.trim().to_string();
            current_heading = heading.trim().to_string();
            current_content = String::new();
            current_line = line_number;
            in_preamble = false;
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Save final section
    if !in_preamble || !current_content.trim().is_empty() {
        sections.push(Section {
            name: normalize_name(&current_name),
            heading: current_heading,
            content: current_content.trim_end().to_string(),
            line: current_line,
        });
    }

    sections
}

/// Normalize section name for comparison (lowercase, trimmed).
fn normalize_name(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Result of comparing two files.
#[derive(Debug)]
pub struct SyncComparison {
    /// True if files are in sync.
    pub in_sync: bool,
    /// Sections that differ between files.
    pub differences: Vec<SectionDiff>,
}

/// A difference between sections in two files.
#[derive(Debug)]
pub struct SectionDiff {
    /// Section name (normalized).
    pub section: String,
    /// Original heading in source file.
    pub source_heading: Option<String>,
    /// Original heading in target file.
    pub target_heading: Option<String>,
    /// Type of difference.
    pub diff_type: DiffType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffType {
    /// Section exists in source but not in target.
    MissingInTarget,
    /// Section exists in target but not in source.
    ExtraInTarget,
    /// Section exists in both but content differs.
    ContentDiffers,
}

/// Compare two files for sync.
pub fn compare_files(source_content: &str, target_content: &str) -> SyncComparison {
    let source_sections = parse_sections(source_content);
    let target_sections = parse_sections(target_content);

    let mut differences = Vec::new();

    // Build lookup map for target sections
    let target_map: HashMap<String, &Section> = target_sections
        .iter()
        .map(|s| (s.name.clone(), s))
        .collect();

    // Check each source section
    for source in &source_sections {
        match target_map.get(&source.name) {
            None => {
                // Missing in target
                differences.push(SectionDiff {
                    section: source.name.clone(),
                    source_heading: Some(source.heading.clone()),
                    target_heading: None,
                    diff_type: DiffType::MissingInTarget,
                });
            }
            Some(target) => {
                // Compare content (normalize whitespace)
                if normalize_content(&source.content) != normalize_content(&target.content) {
                    differences.push(SectionDiff {
                        section: source.name.clone(),
                        source_heading: Some(source.heading.clone()),
                        target_heading: Some(target.heading.clone()),
                        diff_type: DiffType::ContentDiffers,
                    });
                }
            }
        }
    }

    // Check for sections only in target
    let source_names: HashSet<_> = source_sections.iter().map(|s| &s.name).collect();
    for target in &target_sections {
        if !source_names.contains(&target.name) {
            differences.push(SectionDiff {
                section: target.name.clone(),
                source_heading: None,
                target_heading: Some(target.heading.clone()),
                diff_type: DiffType::ExtraInTarget,
            });
        }
    }

    SyncComparison {
        in_sync: differences.is_empty(),
        differences,
    }
}

/// Normalize content for comparison (collapse whitespace).
fn normalize_content(content: &str) -> String {
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
