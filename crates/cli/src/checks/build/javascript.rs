// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JavaScript/TypeScript build support for the build check.
//!
//! Provides:
//! - Bundle size measurement (raw and gzipped)
//! - JavaScript target resolution
//! - Build script detection

use std::io::Write;
use std::path::Path;

use crate::adapter::Bundler;

/// Output directories to scan for JavaScript bundles.
const JS_OUTPUT_DIRS: &[&str] = &["dist", "build", "out"];

/// Bundle size metrics.
#[derive(Debug, Clone, Copy)]
pub struct BundleSize {
    /// Raw file size in bytes.
    pub raw: u64,
    /// Gzipped file size in bytes.
    pub gzipped: u64,
}

/// Measure the size of a bundle file (raw and gzipped).
pub fn measure_bundle_size(path: &Path) -> std::io::Result<BundleSize> {
    let content = std::fs::read(path)?;
    let raw = content.len() as u64;

    // Gzip with default compression (level 6)
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&content)?;
    let gzipped = encoder.finish()?.len() as u64;

    Ok(BundleSize { raw, gzipped })
}

/// Check if a path is a JavaScript bundle file (not a source map).
pub fn is_bundle_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str());
    let is_js = matches!(ext, Some("js" | "mjs" | "cjs"));

    // Exclude source maps (*.js.map becomes *.map after extension check,
    // but we also check the file stem doesn't end with .js.map pattern)
    let path_str = path.to_string_lossy();
    is_js && !path_str.ends_with(".map") && !path_str.ends_with(".js.map")
}

/// Check if package.json has a "build" script.
pub fn has_build_script(root: &Path) -> bool {
    let pkg_path = root.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg_path)
        && let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content)
    {
        return pkg.get("scripts").and_then(|s| s.get("build")).is_some();
    }
    false
}

/// Find the output directory for a JavaScript project.
pub fn find_output_dir(root: &Path, bundler: Bundler) -> Option<std::path::PathBuf> {
    // Try bundler-specific default first
    let bundler_dir = root.join(bundler.default_output_dir());
    if bundler_dir.is_dir() {
        return Some(bundler_dir);
    }

    // Fall back to common output directories
    for dir in JS_OUTPUT_DIRS {
        let path = root.join(dir);
        if path.is_dir() {
            return Some(path);
        }
    }

    None
}

/// Scan a directory for JavaScript bundle files.
pub fn scan_bundle_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut bundles = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && is_bundle_file(&path) {
                bundles.push(path);
            } else if path.is_dir() {
                // Recurse into subdirectories (but not too deep)
                bundles.extend(scan_bundle_files(&path));
            }
        }
    }

    bundles
}

/// Resolve JavaScript build targets.
///
/// Priority:
/// 1. Explicit config targets (returned as-is)
/// 2. Auto-detected from output directory
pub fn resolve_js_targets(
    root: &Path,
    explicit_targets: &[String],
    bundler: Bundler,
) -> Vec<String> {
    // Use explicit targets if provided
    if !explicit_targets.is_empty() {
        return explicit_targets.to_vec();
    }

    // Auto-detect from output directory
    if let Some(output_dir) = find_output_dir(root, bundler) {
        let bundles = scan_bundle_files(&output_dir);
        return bundles
            .into_iter()
            .filter_map(|p| p.strip_prefix(root).ok().map(|rel| rel.to_string_lossy().into_owned()))
            .collect();
    }

    Vec::new()
}

#[cfg(test)]
#[path = "javascript_tests.rs"]
mod tests;
