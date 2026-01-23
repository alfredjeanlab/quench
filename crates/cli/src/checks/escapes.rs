//! Escapes (escape hatches) check.
//!
//! Detects patterns that bypass type safety or error handling.
//! See docs/specs/checks/escape-hatches.md.

use std::sync::atomic::Ordering;

use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::{CheckLevel, EscapeAction, EscapesConfig};
use crate::pattern::{CompiledPattern, PatternError};

/// Compiled escape pattern ready for matching.
struct CompiledEscapePattern {
    name: String,
    matcher: CompiledPattern,
    #[allow(dead_code)] // KEEP UNTIL: Phase 215 implements actions
    action: EscapeAction,
    advice: String,
}

/// The escapes check detects escape hatch patterns.
pub struct EscapesCheck;

impl Check for EscapesCheck {
    fn name(&self) -> &'static str {
        "escapes"
    }

    fn description(&self) -> &'static str {
        "Escape hatch detection"
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        let config = &ctx.config.check.escapes;

        if config.check == CheckLevel::Off {
            return CheckResult::passed(self.name());
        }

        // No patterns configured = nothing to check
        if config.patterns.is_empty() {
            return CheckResult::passed(self.name());
        }

        // Compile patterns once
        let patterns = match compile_patterns(config) {
            Ok(p) => p,
            Err(e) => return CheckResult::skipped(self.name(), e.to_string()),
        };

        let mut violations = Vec::new();

        for file in ctx.files {
            // Read file content
            let content = match std::fs::read_to_string(&file.path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let relative = file.path.strip_prefix(ctx.root).unwrap_or(&file.path);

            // Find matches for each pattern
            for pattern in &patterns {
                let matches = pattern.matcher.find_all_with_lines(&content);

                for m in matches {
                    // For Phase 210, all matches become violations (action logic in Phase 215)
                    if let Some(v) = try_create_violation(ctx, relative, m.line, pattern) {
                        violations.push(v);
                    } else {
                        // Limit reached
                        return CheckResult::failed(self.name(), violations);
                    }
                }
            }
        }

        if violations.is_empty() {
            CheckResult::passed(self.name())
        } else {
            CheckResult::failed(self.name(), violations)
        }
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

fn compile_patterns(config: &EscapesConfig) -> Result<Vec<CompiledEscapePattern>, PatternError> {
    config
        .patterns
        .iter()
        .map(|p| {
            let matcher = CompiledPattern::compile(&p.pattern)?;
            let advice = p
                .advice
                .clone()
                .unwrap_or_else(|| default_advice(&p.action));
            Ok(CompiledEscapePattern {
                name: p.name.clone(),
                matcher,
                action: p.action,
                advice,
            })
        })
        .collect()
}

fn default_advice(action: &EscapeAction) -> String {
    match action {
        EscapeAction::Forbid => "Remove this escape hatch from production code.".to_string(),
        EscapeAction::Comment => "Add a justification comment.".to_string(),
        EscapeAction::Count => "Reduce escape hatch usage.".to_string(),
    }
}

fn try_create_violation(
    ctx: &CheckContext,
    path: &std::path::Path,
    line: u32,
    pattern: &CompiledEscapePattern,
) -> Option<Violation> {
    let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
    if let Some(limit) = ctx.limit
        && current >= limit
    {
        return None;
    }

    Some(Violation::file(path, line, "forbidden", &pattern.advice).with_pattern(&pattern.name))
}

#[cfg(test)]
#[path = "escapes_tests.rs"]
mod tests;
