//! Behavioral specs for the docs check.
//!
//! Tests that quench correctly:
//! - Validates TOC directory trees in markdown files
//! - Validates markdown links to local files
//! - Detects and validates specs index files
//! - Checks required/forbidden sections in spec files
//! - Checks feature commits have doc updates (CI mode)
//! - Generates correct violation types
//!
//! Reference: docs/specs/checks/docs.md

mod commit;
mod index;
mod links;
mod output;
mod sections;
mod toc;
