// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use tempfile::TempDir;

/// Get the output directory from config or detect from bundler.
fn get_output_dir(
    root: &std::path::Path,
    config_output_dir: Option<&str>,
    bundler: Bundler,
) -> String {
    config_output_dir
        .map(|s| s.to_string())
        .or_else(|| find_output_dir(root, bundler).map(|p| p.to_string_lossy().into_owned()))
        .unwrap_or_else(|| bundler.default_output_dir().to_string())
}

// =============================================================================
// BUNDLE SIZE MEASUREMENT TESTS
// =============================================================================

#[test]
fn measure_bundle_size_calculates_raw_and_gzip() {
    let dir = TempDir::new().unwrap();
    let bundle_path = dir.path().join("bundle.js");

    // Create a file with some content
    let content = "function hello() { console.log('Hello, world!'); }".repeat(100);
    std::fs::write(&bundle_path, &content).unwrap();

    let size = measure_bundle_size(&bundle_path).unwrap();

    // Raw size should match content length
    assert_eq!(size.raw, content.len() as u64);

    // Gzipped should be smaller (this content is compressible)
    assert!(size.gzipped < size.raw);
    assert!(size.gzipped > 0);
}

#[test]
fn measure_bundle_size_missing_file() {
    let dir = TempDir::new().unwrap();
    let result = measure_bundle_size(&dir.path().join("nonexistent.js"));
    assert!(result.is_err());
}

// =============================================================================
// BUNDLE FILE DETECTION TESTS
// =============================================================================

#[test]
fn is_bundle_file_js() {
    assert!(is_bundle_file(Path::new("bundle.js")));
    assert!(is_bundle_file(Path::new("dist/index.js")));
}

#[test]
fn is_bundle_file_mjs() {
    assert!(is_bundle_file(Path::new("bundle.mjs")));
}

#[test]
fn is_bundle_file_cjs() {
    assert!(is_bundle_file(Path::new("bundle.cjs")));
}

#[test]
fn is_bundle_file_excludes_source_maps() {
    assert!(!is_bundle_file(Path::new("bundle.js.map")));
    assert!(!is_bundle_file(Path::new("dist/index.js.map")));
}

#[test]
fn is_bundle_file_excludes_non_js() {
    assert!(!is_bundle_file(Path::new("style.css")));
    assert!(!is_bundle_file(Path::new("index.html")));
    assert!(!is_bundle_file(Path::new("config.json")));
}

// =============================================================================
// BUILD SCRIPT DETECTION TESTS
// =============================================================================

#[test]
fn has_build_script_true() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"scripts": {"build": "vite build"}}"#)
        .unwrap();

    assert!(has_build_script(dir.path()));
}

#[test]
fn has_build_script_false_no_build() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"scripts": {"start": "vite"}}"#).unwrap();

    assert!(!has_build_script(dir.path()));
}

#[test]
fn has_build_script_false_no_scripts() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"name": "test"}"#).unwrap();

    assert!(!has_build_script(dir.path()));
}

#[test]
fn has_build_script_false_no_package_json() {
    let dir = TempDir::new().unwrap();
    assert!(!has_build_script(dir.path()));
}

// =============================================================================
// OUTPUT DIRECTORY DETECTION TESTS
// =============================================================================

#[test]
fn find_output_dir_bundler_specific() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("dist")).unwrap();

    let output = find_output_dir(dir.path(), Bundler::Vite);
    assert!(output.is_some());
    assert!(output.unwrap().ends_with("dist"));
}

#[test]
fn find_output_dir_nextjs() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".next/static")).unwrap();

    let output = find_output_dir(dir.path(), Bundler::NextJs);
    assert!(output.is_some());
    assert!(output.unwrap().to_string_lossy().contains(".next"));
}

#[test]
fn find_output_dir_fallback_build() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("build")).unwrap();

    let output = find_output_dir(dir.path(), Bundler::Unknown);
    assert!(output.is_some());
    assert!(output.unwrap().ends_with("build"));
}

#[test]
fn find_output_dir_fallback_out() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("out")).unwrap();

    let output = find_output_dir(dir.path(), Bundler::Unknown);
    assert!(output.is_some());
    assert!(output.unwrap().ends_with("out"));
}

#[test]
fn find_output_dir_none() {
    let dir = TempDir::new().unwrap();

    let output = find_output_dir(dir.path(), Bundler::Unknown);
    assert!(output.is_none());
}

// =============================================================================
// BUNDLE SCANNING TESTS
// =============================================================================

#[test]
fn scan_bundle_files_finds_js_files() {
    let dir = TempDir::new().unwrap();
    let dist = dir.path().join("dist");
    std::fs::create_dir(&dist).unwrap();

    std::fs::write(dist.join("index.js"), "").unwrap();
    std::fs::write(dist.join("vendor.js"), "").unwrap();
    std::fs::write(dist.join("style.css"), "").unwrap();

    let bundles = scan_bundle_files(&dist);
    assert_eq!(bundles.len(), 2);
}

#[test]
fn scan_bundle_files_excludes_source_maps() {
    let dir = TempDir::new().unwrap();
    let dist = dir.path().join("dist");
    std::fs::create_dir(&dist).unwrap();

    std::fs::write(dist.join("index.js"), "").unwrap();
    std::fs::write(dist.join("index.js.map"), "").unwrap();

    let bundles = scan_bundle_files(&dist);
    assert_eq!(bundles.len(), 1);
    assert!(bundles[0].to_string_lossy().ends_with("index.js"));
}

#[test]
fn scan_bundle_files_recurses() {
    let dir = TempDir::new().unwrap();
    let dist = dir.path().join("dist");
    let assets = dist.join("assets");
    std::fs::create_dir_all(&assets).unwrap();

    std::fs::write(dist.join("index.js"), "").unwrap();
    std::fs::write(assets.join("chunk.js"), "").unwrap();

    let bundles = scan_bundle_files(&dist);
    assert_eq!(bundles.len(), 2);
}

// =============================================================================
// TARGET RESOLUTION TESTS
// =============================================================================

#[test]
fn resolve_js_targets_explicit() {
    let dir = TempDir::new().unwrap();

    let targets = resolve_js_targets(
        dir.path(),
        &["dist/app.js".to_string(), "dist/vendor.js".to_string()],
        Bundler::Vite,
    );

    assert_eq!(targets, vec!["dist/app.js", "dist/vendor.js"]);
}

#[test]
fn resolve_js_targets_auto_detect() {
    let dir = TempDir::new().unwrap();
    let dist = dir.path().join("dist");
    std::fs::create_dir(&dist).unwrap();

    std::fs::write(dist.join("index.js"), "").unwrap();
    std::fs::write(dist.join("vendor.js"), "").unwrap();

    let targets = resolve_js_targets(dir.path(), &[], Bundler::Vite);

    assert_eq!(targets.len(), 2);
    assert!(targets.iter().any(|t| t.contains("index.js")));
    assert!(targets.iter().any(|t| t.contains("vendor.js")));
}

#[test]
fn resolve_js_targets_empty_when_no_output() {
    let dir = TempDir::new().unwrap();

    let targets = resolve_js_targets(dir.path(), &[], Bundler::Vite);

    assert!(targets.is_empty());
}

// =============================================================================
// OUTPUT DIRECTORY CONFIG TESTS
// =============================================================================

#[test]
fn get_output_dir_from_config() {
    let dir = TempDir::new().unwrap();

    let output = get_output_dir(dir.path(), Some("custom-dist"), Bundler::Vite);
    assert_eq!(output, "custom-dist");
}

#[test]
fn get_output_dir_detected() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("dist")).unwrap();

    let output = get_output_dir(dir.path(), None, Bundler::Vite);
    assert!(output.contains("dist"));
}

#[test]
fn get_output_dir_bundler_default() {
    let dir = TempDir::new().unwrap();

    let output = get_output_dir(dir.path(), None, Bundler::Vite);
    assert_eq!(output, "dist");
}
