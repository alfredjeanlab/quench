//! Pattern matching for escape hatch detection.
//!
//! Implements the pattern matching hierarchy from docs/specs/20-performance.md:
//! - Single literal: memchr::memmem
//! - Multiple literals: aho-corasick
//! - Complex regex: regex crate

pub mod matcher;

pub use matcher::{CompiledPattern, LineMatch, PatternError, PatternMatch, byte_offset_to_line};

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
