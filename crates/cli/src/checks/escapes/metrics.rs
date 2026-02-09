// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Metrics tracking for escape hatch detection.
//!
//! Tracks pattern match counts for source and test files,
//! with optional per-package breakdown for workspaces.

use std::collections::HashMap;

use serde_json::{Value as JsonValue, json};

/// Metrics tracked during escapes check.
#[derive(Default)]
pub(super) struct EscapesMetrics {
    /// Counts per pattern for source files.
    source: HashMap<String, usize>,
    /// Counts per pattern for test files.
    test: HashMap<String, usize>,
    /// Per-package breakdown (only if workspace configured).
    packages: HashMap<String, PackageMetrics>,
}

#[derive(Default)]
struct PackageMetrics {
    source: HashMap<String, usize>,
    test: HashMap<String, usize>,
}

impl EscapesMetrics {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn increment(&mut self, pattern_name: &str, is_test: bool) {
        let map = if is_test { &mut self.test } else { &mut self.source };
        *map.entry(pattern_name.to_string()).or_insert(0) += 1;
    }

    pub(super) fn increment_package(&mut self, package: &str, pattern_name: &str, is_test: bool) {
        let pkg = self.packages.entry(package.to_string()).or_default();
        let map = if is_test { &mut pkg.test } else { &mut pkg.source };
        *map.entry(pattern_name.to_string()).or_insert(0) += 1;
    }

    pub(super) fn source_count(&self, pattern_name: &str) -> usize {
        self.source.get(pattern_name).copied().unwrap_or(0)
    }

    /// Convert to JSON metrics structure.
    pub(super) fn to_json(&self, pattern_names: &[String]) -> JsonValue {
        // Include all configured patterns, even with 0 count
        let mut source_obj = serde_json::Map::new();
        let mut test_obj = serde_json::Map::new();

        for name in pattern_names {
            source_obj.insert(name.clone(), json!(self.source.get(name).copied().unwrap_or(0)));
            test_obj.insert(name.clone(), json!(self.test.get(name).copied().unwrap_or(0)));
        }

        json!({
            "source": source_obj,
            "test": test_obj
        })
    }

    /// Convert to by_package structure (only if packages exist).
    pub(super) fn to_by_package(
        &self,
        pattern_names: &[String],
    ) -> Option<HashMap<String, JsonValue>> {
        if self.packages.is_empty() {
            return None;
        }

        let mut result = HashMap::new();
        for (pkg_name, pkg_metrics) in &self.packages {
            let mut source_obj = serde_json::Map::new();
            let mut test_obj = serde_json::Map::new();

            for name in pattern_names {
                source_obj.insert(
                    name.clone(),
                    json!(pkg_metrics.source.get(name).copied().unwrap_or(0)),
                );
                test_obj
                    .insert(name.clone(), json!(pkg_metrics.test.get(name).copied().unwrap_or(0)));
            }

            result.insert(
                pkg_name.clone(),
                json!({
                    "source": source_obj,
                    "test": test_obj
                }),
            );
        }

        Some(result)
    }
}
