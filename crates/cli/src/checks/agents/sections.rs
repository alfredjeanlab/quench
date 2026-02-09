// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Section validation logic for agent files.
//!
//! Validates required and forbidden sections in markdown files.

use crate::checks::agents::config::{RequiredSection, SectionsConfig};
use crate::checks::agents::sync::{Section, parse_sections};

/// Result of section validation.
#[derive(Debug)]
pub struct SectionValidation {
    /// Missing required sections.
    pub missing: Vec<MissingSection>,
    /// Present forbidden sections.
    pub forbidden: Vec<ForbiddenSection>,
}

/// A missing required section.
#[derive(Debug)]
pub struct MissingSection {
    /// Required section name.
    pub name: String,
    /// Advice for adding the section.
    pub advice: Option<String>,
}

/// A forbidden section that was found.
#[derive(Debug)]
pub struct ForbiddenSection {
    /// Matched section heading (original case).
    pub heading: String,
    /// Line number where section starts.
    pub line: u32,
    /// Pattern that matched (for advice).
    pub matched_pattern: String,
}

/// Validate sections in content against configuration.
pub fn validate_sections(content: &str, config: &SectionsConfig) -> SectionValidation {
    let sections = parse_sections(content);

    let missing = check_required(&sections, &config.required);
    let forbidden = check_forbidden(&sections, &config.forbid);

    SectionValidation { missing, forbidden }
}

/// Check for missing required sections.
fn check_required(sections: &[Section], required: &[RequiredSection]) -> Vec<MissingSection> {
    let section_names: Vec<String> = sections.iter().map(|s| s.name.clone()).collect();

    required
        .iter()
        .filter(|req| {
            let normalized = req.name.trim().to_lowercase();
            !section_names.contains(&normalized)
        })
        .map(|req| MissingSection { name: req.name.clone(), advice: req.advice.clone() })
        .collect()
}

/// Check for forbidden sections (supports glob patterns).
fn check_forbidden(sections: &[Section], forbid: &[String]) -> Vec<ForbiddenSection> {
    let mut forbidden = Vec::new();

    for section in sections {
        for pattern in forbid {
            if matches_section_pattern(&section.name, pattern) {
                forbidden.push(ForbiddenSection {
                    heading: section.heading.clone(),
                    line: section.line,
                    matched_pattern: pattern.clone(),
                });
                break; // One match per section is enough
            }
        }
    }

    forbidden
}

/// Check if a section name matches a pattern (case-insensitive, glob support).
fn matches_section_pattern(section_name: &str, pattern: &str) -> bool {
    let normalized_pattern = pattern.trim().to_lowercase();

    // Check for glob characters
    if normalized_pattern.contains('*') || normalized_pattern.contains('?') {
        // Use glob matching
        glob_match(&normalized_pattern, section_name)
    } else {
        // Exact match (case-insensitive, already normalized)
        section_name == normalized_pattern
    }
}

/// Simple glob matching for section names.
/// Supports * (any chars) and ? (single char).
fn glob_match(pattern: &str, text: &str) -> bool {
    let mut pattern_chars = pattern.chars().peekable();
    let mut text_chars = text.chars().peekable();

    while let Some(p) = pattern_chars.next() {
        match p {
            '*' => {
                // Match zero or more characters
                if pattern_chars.peek().is_none() {
                    return true; // Trailing * matches everything
                }
                // Try matching rest of pattern at each position
                let remaining_pattern: String = pattern_chars.collect();
                loop {
                    let remaining_text: String = text_chars.clone().collect();
                    if glob_match(&remaining_pattern, &remaining_text) {
                        return true;
                    }
                    if text_chars.next().is_none() {
                        return false;
                    }
                }
            }
            '?' => {
                // Match exactly one character
                if text_chars.next().is_none() {
                    return false;
                }
            }
            c => {
                // Literal match
                if text_chars.next() != Some(c) {
                    return false;
                }
            }
        }
    }

    text_chars.peek().is_none()
}

#[cfg(test)]
#[path = "sections_tests.rs"]
mod tests;
