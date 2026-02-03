// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for the escapes (escape hatches) check.
//!
//! Tests that quench correctly:
//! - Detects pattern matches in source files
//! - Applies actions (count, comment, forbid)
//! - Separates source and test code
//! - Generates correct violation types
//! - Outputs metrics in JSON format
//!
//! Reference: docs/specs/checks/escape-hatches.md

mod actions;
mod edge_cases;
mod output;
mod suppress_other;
mod suppress_rust;

/// Helper: project with exclude pattern for generated files.
fn exclude_project() -> crate::prelude::Project {
    use crate::prelude::*;

    let temp = Project::empty();
    temp.config(
        r#"[check.escapes]
exclude = ["**/generated/**"]

[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"
"#,
    );
    temp
}
