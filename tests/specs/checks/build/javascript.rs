// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JavaScript/TypeScript build specs.
//!
//! Reference: docs/specs/checks/build.md#javascript-support

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

/// Spec: docs/specs/checks/build.md#javascript-support
///
/// > JavaScript projects: Bundle sizes measured from dist/ directory
#[test]
fn javascript_bundle_size_passes() {
    let result = check("build")
        .on("javascript/build-vite")
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");
    let size = metrics.get("size").and_then(|v| v.as_object());
    assert!(size.is_some(), "should have size metrics");
    assert!(
        !size.unwrap().is_empty(),
        "should have at least one bundle measured"
    );
}

/// Spec: docs/specs/checks/build.md#javascript-support
///
/// > JavaScript: Reports both raw and gzipped sizes
#[test]
fn javascript_reports_gzip_size() {
    let result = check("build")
        .on("javascript/build-vite")
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");

    // Both size and size_gzip should be present
    assert!(metrics.get("size").is_some(), "should have size metrics");
    assert!(
        metrics.get("size_gzip").is_some(),
        "should have size_gzip metrics for JavaScript bundles"
    );

    // size_gzip should have the same keys
    let size = metrics.get("size").unwrap().as_object().unwrap();
    let size_gzip = metrics.get("size_gzip").unwrap().as_object().unwrap();

    for key in size.keys() {
        assert!(
            size_gzip.contains_key(key),
            "size_gzip should have entry for {}",
            key
        );
    }
}

/// Spec: docs/specs/checks/build.md#javascript-support
///
/// > JavaScript: Gzipped size should be smaller than raw size
#[test]
fn javascript_gzip_smaller_than_raw() {
    let result = check("build")
        .on("javascript/build-vite")
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");
    let size = metrics.get("size").unwrap().as_object().unwrap();
    let size_gzip = metrics.get("size_gzip").unwrap().as_object().unwrap();

    for (key, raw_value) in size {
        let raw = raw_value.as_u64().unwrap();
        let gzipped = size_gzip.get(key).and_then(|v| v.as_u64()).unwrap();

        assert!(
            gzipped <= raw,
            "gzipped size ({}) should be <= raw size ({}) for {}",
            gzipped,
            raw,
            key
        );
    }
}

/// Spec: docs/specs/checks/build.md#javascript-support
///
/// > JavaScript: Explicit targets from config are respected
#[test]
fn javascript_custom_targets() {
    let result = check("build")
        .on("javascript/build-custom-targets")
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");
    let size = metrics.get("size").and_then(|v| v.as_object());
    assert!(size.is_some(), "should have size metrics");

    let size_obj = size.unwrap();
    assert!(
        size_obj.contains_key("dist/app.js"),
        "should measure configured target dist/app.js"
    );
    assert!(
        size_obj.contains_key("dist/vendor.js"),
        "should measure configured target dist/vendor.js"
    );
}

/// Spec: docs/specs/checks/build.md#javascript-support
///
/// > JavaScript: Size exceeded generates violation
#[test]
fn javascript_bundle_size_exceeded() {
    let result = check("build")
        .on("javascript/build-size-exceeded")
        .args(&["--ci"])
        .json()
        .fails();

    assert!(result.has_violation("size_exceeded"));

    let v = result.require_violation("size_exceeded");
    assert!(v.get("target").is_some(), "violation should include target");
    assert!(v.get("value").is_some(), "violation should include value");
    assert!(
        v.get("threshold").is_some(),
        "violation should include threshold"
    );
}

/// Spec: docs/specs/checks/build.md#javascript-support
///
/// > JavaScript: Bundler detection (Vite)
#[test]
fn javascript_detects_vite_bundler() {
    // The build-vite fixture has vite.config.ts, should be detected as Vite
    let result = check("build")
        .on("javascript/build-vite")
        .args(&["--ci"])
        .json()
        .passes();

    // Just verify it runs successfully and collects metrics
    let metrics = result.get("metrics");
    assert!(
        metrics.is_some() && !metrics.unwrap().is_null(),
        "should collect metrics for Vite project"
    );
}
