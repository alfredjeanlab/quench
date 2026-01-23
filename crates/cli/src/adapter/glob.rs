//! Glob pattern utilities.

use globset::{Glob, GlobSet, GlobSetBuilder};

/// Build a GlobSet from pattern strings.
///
/// Invalid patterns are logged and skipped.
pub fn build_glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        match Glob::new(pattern) {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(e) => {
                tracing::warn!("invalid glob pattern '{}': {}", pattern, e);
            }
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

#[cfg(test)]
#[path = "glob_tests.rs"]
mod tests;
