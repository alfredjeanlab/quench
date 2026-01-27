// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::adapter::ProjectLanguage;
use tempfile::TempDir;

#[test]
fn get_rust_targets_from_cargo_toml() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(
        root.join("Cargo.toml"),
        r#"
[package]
name = "myapp"
version = "0.1.0"

[[bin]]
name = "myapp"
path = "src/main.rs"
"#,
    )
    .unwrap();

    let targets = get_rust_targets(root);
    assert_eq!(targets, vec!["myapp"]);
}

#[test]
fn get_rust_targets_default_binary() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        r#"
[package]
name = "myapp"
version = "0.1.0"
"#,
    )
    .unwrap();

    let targets = get_rust_targets(root);
    assert_eq!(targets, vec!["myapp"]);
}

#[test]
fn get_go_targets_from_go_mod() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(root.join("go.mod"), "module github.com/example/myapp\n").unwrap();

    let targets = get_go_targets(root);
    assert_eq!(targets, vec!["myapp"]);
}

#[test]
fn measure_binary_size_rust() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Create fake binary
    let release_dir = root.join("target/release");
    std::fs::create_dir_all(&release_dir).unwrap();
    std::fs::write(release_dir.join("myapp"), vec![0u8; 1024]).unwrap();

    let size = measure_binary_size(root, "myapp", ProjectLanguage::Rust);
    assert_eq!(size, Some(1024));
}

#[test]
fn measure_binary_size_missing() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    let size = measure_binary_size(root, "nonexistent", ProjectLanguage::Rust);
    assert_eq!(size, None);
}

#[test]
fn build_metrics_to_json() {
    let mut metrics = BuildMetrics::default();
    metrics.sizes.insert("myapp".to_string(), 1_000_000);
    metrics.time_cold = Some(Duration::from_secs(10));
    metrics.time_hot = Some(Duration::from_secs(2));

    let json = metrics.to_json();

    assert_eq!(json["size"]["myapp"], 1_000_000);
    assert_eq!(json["time"]["cold"], 10.0);
    assert_eq!(json["time"]["hot"], 2.0);
}

// =============================================================================
// JSON STRUCTURE VERIFICATION TESTS
// =============================================================================

#[test]
fn build_metrics_json_structure() {
    let mut metrics = BuildMetrics::default();
    metrics.sizes.insert("myapp".to_string(), 5_242_880);
    metrics.sizes.insert("myserver".to_string(), 2_097_152);
    metrics.time_cold = Some(Duration::from_secs_f64(45.234));
    metrics.time_hot = Some(Duration::from_secs_f64(2.456));

    let json = metrics.to_json();

    // Verify structure
    assert!(json.get("size").is_some(), "should have size object");
    assert!(json.get("time").is_some(), "should have time object");

    // Verify size values
    let size = json.get("size").unwrap();
    assert_eq!(
        size.get("myapp").and_then(|v| v.as_u64()),
        Some(5_242_880),
        "myapp size should be 5242880"
    );
    assert_eq!(
        size.get("myserver").and_then(|v| v.as_u64()),
        Some(2_097_152),
        "myserver size should be 2097152"
    );

    // Verify time values (as floats)
    let time = json.get("time").unwrap();
    let cold = time.get("cold").and_then(|v| v.as_f64()).unwrap();
    assert!(
        (cold - 45.234).abs() < 0.001,
        "cold time should be ~45.234, got {}",
        cold
    );
    let hot = time.get("hot").and_then(|v| v.as_f64()).unwrap();
    assert!(
        (hot - 2.456).abs() < 0.001,
        "hot time should be ~2.456, got {}",
        hot
    );
}

#[test]
fn build_metrics_json_empty_time() {
    let mut metrics = BuildMetrics::default();
    metrics.sizes.insert("myapp".to_string(), 1024);

    let json = metrics.to_json();

    // Verify time object exists with null cold and hot
    let time = json.get("time").unwrap();
    assert!(
        time.get("cold").unwrap().is_null(),
        "cold should be null when not measured"
    );
    assert!(
        time.get("hot").unwrap().is_null(),
        "hot should be null when not measured"
    );
}

#[test]
fn build_metrics_json_empty_sizes() {
    let metrics = BuildMetrics::default();

    let json = metrics.to_json();

    // Size should be an empty object
    let size = json.get("size").unwrap();
    assert!(size.is_object(), "size should be an object");
    assert!(
        size.as_object().unwrap().is_empty(),
        "size should be empty when no targets"
    );
}

#[test]
fn build_metrics_json_size_is_integer() {
    let mut metrics = BuildMetrics::default();
    metrics.sizes.insert("myapp".to_string(), 1024);

    let json = metrics.to_json();

    // Size values should be integers, not floats
    let size_value = json.get("size").unwrap().get("myapp").unwrap();
    assert!(
        size_value.is_u64(),
        "size should be an integer, not a float"
    );
}

#[test]
fn build_metrics_json_time_is_float() {
    let metrics = BuildMetrics {
        time_cold: Some(Duration::from_millis(1500)),
        ..Default::default()
    };

    let json = metrics.to_json();

    // Time values should be floats (seconds)
    let cold_value = json.get("time").unwrap().get("cold").unwrap();
    assert!(
        cold_value.is_f64(),
        "time should be a float representing seconds"
    );
    let cold_secs = cold_value.as_f64().unwrap();
    assert!(
        (cold_secs - 1.5).abs() < 0.001,
        "1500ms should be 1.5 seconds"
    );
}

// =============================================================================
// GZIP SIZE TESTS (JavaScript bundles)
// =============================================================================

#[test]
fn build_metrics_json_with_gzip_sizes() {
    let mut metrics = BuildMetrics::default();
    metrics.sizes.insert("dist/index.js".to_string(), 100_000);
    metrics
        .sizes_gzip
        .insert("dist/index.js".to_string(), 25_000);

    let json = metrics.to_json();

    // Verify size_gzip is present
    assert!(
        json.get("size_gzip").is_some(),
        "should have size_gzip when gzip sizes exist"
    );

    let size_gzip = json.get("size_gzip").unwrap();
    assert_eq!(
        size_gzip.get("dist/index.js").and_then(|v| v.as_u64()),
        Some(25_000),
        "gzip size should match"
    );
}

#[test]
fn build_metrics_json_no_gzip_without_js() {
    let mut metrics = BuildMetrics::default();
    metrics.sizes.insert("myapp".to_string(), 1024);
    // No gzip sizes added

    let json = metrics.to_json();

    // size_gzip should not be present
    assert!(
        json.get("size_gzip").is_none(),
        "should not have size_gzip when no gzip sizes"
    );
}

#[test]
fn build_metrics_has_metrics_with_sizes() {
    let mut metrics = BuildMetrics::default();
    metrics.sizes.insert("myapp".to_string(), 1024);

    assert!(metrics.has_metrics());
}

#[test]
fn build_metrics_has_metrics_with_time() {
    let metrics = BuildMetrics {
        time_cold: Some(Duration::from_secs(1)),
        ..Default::default()
    };

    assert!(metrics.has_metrics());
}

#[test]
fn build_metrics_has_metrics_empty() {
    let metrics = BuildMetrics::default();
    assert!(!metrics.has_metrics());
}

// =============================================================================
// JAVASCRIPT TARGET DETECTION TESTS
// =============================================================================

#[test]
fn get_build_targets_javascript() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Create package.json to mark as JS project
    std::fs::write(root.join("package.json"), r#"{"name": "test"}"#).unwrap();

    // Create dist directory with bundles
    let dist = root.join("dist");
    std::fs::create_dir(&dist).unwrap();
    std::fs::write(dist.join("index.js"), "console.log('hello')").unwrap();
    std::fs::write(dist.join("vendor.js"), "// vendor").unwrap();
    std::fs::write(dist.join("index.js.map"), "{}").unwrap(); // Should be excluded

    let targets = get_build_targets(root, ProjectLanguage::JavaScript);

    assert_eq!(targets.len(), 2);
    assert!(targets.iter().any(|t| t.contains("index.js")));
    assert!(targets.iter().any(|t| t.contains("vendor.js")));
    // Source maps should not be included
    assert!(!targets.iter().any(|t| t.contains(".map")));
}
