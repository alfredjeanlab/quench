// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for the agents check.
//!
//! Tests that quench correctly:
//! - Detects agent context files (CLAUDE.md, .cursorrules)
//! - Validates file synchronization
//! - Checks required/forbidden sections
//! - Enforces content rules (tables, max_lines, max_tokens)
//! - Generates correct violation types
//! - Outputs metrics in JSON format
//!
//! Reference: docs/specs/checks/agents.md

mod content;
mod cursor;
mod defaults;
mod detection;
mod edge_cases;
mod output;
