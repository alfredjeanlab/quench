// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Content validation for spec files.
//!
//! Validates sections, content rules, and size limits.

use std::path::Path;

use crate::check::Violation;
use crate::checks::agents::content::{
    check_line_count, check_token_count, detect_box_diagrams, detect_mermaid_blocks, detect_tables,
};
use crate::checks::agents::sections::validate_sections;
use crate::config::{ContentRule, SectionsConfig, SpecsConfig, SpecsSectionsConfig};

/// Validate content of a single spec file.
pub fn validate_spec_content(path: &Path, content: &str, config: &SpecsConfig) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Section validation
    validate_spec_sections(path, content, &config.sections, &mut violations);

    // Content rules
    validate_content_rules(path, content, config, &mut violations);

    // Size limits
    validate_size_limits(path, content, config, &mut violations);

    violations
}

fn validate_spec_sections(
    path: &Path,
    content: &str,
    config: &SpecsSectionsConfig,
    violations: &mut Vec<Violation>,
) {
    // Convert SpecsSectionsConfig to agents SectionsConfig for reuse
    let agent_sections_config =
        SectionsConfig { required: config.required.clone(), forbid: config.forbid.clone() };

    let result = validate_sections(content, &agent_sections_config);

    for missing in result.missing {
        let advice = match &missing.advice {
            Some(a) => format!("Add a \"## {}\" section: {}", missing.name, a),
            None => format!("Add a \"## {}\" section.", missing.name),
        };
        violations.push(
            Violation::file_only(path, "missing_section", advice).with_section(&missing.name),
        );
    }

    for forbidden in result.forbidden {
        violations.push(
            Violation::file(
                path,
                forbidden.line,
                "forbidden_section",
                format!(
                    "Section \"{}\" is forbidden (matched pattern: {}).",
                    forbidden.heading, forbidden.matched_pattern
                ),
            )
            .with_section(&forbidden.heading),
        );
    }
}

fn validate_content_rules(
    path: &Path,
    content: &str,
    config: &SpecsConfig,
    violations: &mut Vec<Violation>,
) {
    // Tables
    if config.tables == ContentRule::Forbid {
        for issue in detect_tables(content) {
            violations.push(Violation::file(
                path,
                issue.line,
                issue.content_type.violation_type(),
                issue.content_type.advice(),
            ));
        }
    }

    // Box diagrams
    if config.box_diagrams == ContentRule::Forbid {
        for issue in detect_box_diagrams(content) {
            violations.push(Violation::file(
                path,
                issue.line,
                issue.content_type.violation_type(),
                issue.content_type.advice(),
            ));
        }
    }

    // Mermaid
    if config.mermaid == ContentRule::Forbid {
        for issue in detect_mermaid_blocks(content) {
            violations.push(Violation::file(
                path,
                issue.line,
                issue.content_type.violation_type(),
                issue.content_type.advice(),
            ));
        }
    }
}

fn validate_size_limits(
    path: &Path,
    content: &str,
    config: &SpecsConfig,
    violations: &mut Vec<Violation>,
) {
    // Line limit
    if let Some(max_lines) = config.max_lines
        && let Some(violation) = check_line_count(content, max_lines)
    {
        violations.push(
            Violation::file_only(
                path,
                "spec_too_large",
                violation.limit_type.advice(violation.value, violation.threshold),
            )
            .with_threshold(violation.value as i64, violation.threshold as i64),
        );
    }

    // Token limit
    if let Some(max_tokens) = config.max_tokens
        && let Some(violation) = check_token_count(content, max_tokens)
    {
        violations.push(
            Violation::file_only(
                path,
                "spec_too_large",
                violation.limit_type.advice(violation.value, violation.threshold),
            )
            .with_threshold(violation.value as i64, violation.threshold as i64),
        );
    }
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;
