//! Behavioral specs for the build check.
//!
//! Tests that quench correctly:
//! - Detects binary targets from Cargo.toml/package.json
//! - Measures binary/bundle sizes
//! - Generates violations for size/time thresholds
//!
//! Reference: docs/specs/checks/build.md

mod javascript;
mod rust;
