// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Documentation validation check.
//!
//! Validates:
//! - TOC entries reference existing files
//! - Markdown links point to existing files
//! - (Future) Specs have required sections

mod links;
mod toc;

use crate::check::{Check, CheckContext, CheckResult};

pub struct DocsCheck;

impl Check for DocsCheck {
    fn name(&self) -> &'static str {
        "docs"
    }

    fn description(&self) -> &'static str {
        "Documentation validation"
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        let mut violations = Vec::new();

        // Check if docs check is disabled
        let check_level = ctx.config.check.docs.check.as_deref().unwrap_or("error");
        if check_level == "off" {
            return CheckResult::passed("docs");
        }

        // Run TOC validation
        toc::validate_toc(ctx, &mut violations);

        // Run link validation
        links::validate_links(ctx, &mut violations);

        // Respect violation limit
        if let Some(limit) = ctx.limit {
            violations.truncate(limit);
        }

        if violations.is_empty() {
            CheckResult::passed("docs")
        } else {
            CheckResult::failed("docs", violations)
        }
    }

    fn default_enabled(&self) -> bool {
        true
    }
}
