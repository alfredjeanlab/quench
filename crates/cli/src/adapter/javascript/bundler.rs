// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JavaScript bundler detection.
//!
//! Detects the bundler used by a JavaScript/TypeScript project by checking
//! for configuration files in the project root.

use std::path::Path;

/// JavaScript bundler types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bundler {
    /// Vite bundler (vite.config.*)
    Vite,
    /// Webpack bundler (webpack.config.*)
    Webpack,
    /// esbuild bundler (esbuild.config.* or scripts)
    Esbuild,
    /// Rollup bundler (rollup.config.*)
    Rollup,
    /// Next.js framework (next.config.*)
    NextJs,
    /// Parcel bundler (.parcelrc or devDependency)
    Parcel,
    /// Unknown or no bundler detected
    Unknown,
}

impl Bundler {
    /// Get the default output directory for this bundler.
    pub fn default_output_dir(&self) -> &'static str {
        match self {
            Bundler::Vite => "dist",
            Bundler::Webpack => "dist",
            Bundler::Esbuild => "dist",
            Bundler::Rollup => "dist",
            Bundler::NextJs => ".next/static",
            Bundler::Parcel => "dist",
            Bundler::Unknown => "dist",
        }
    }
}

/// Detect the bundler used by a JavaScript project.
///
/// Detection order (first match wins):
/// 1. Vite: `vite.config.ts`, `vite.config.js`, `vite.config.mjs`
/// 2. Webpack: `webpack.config.js`, `webpack.config.ts`, `webpack.config.cjs`
/// 3. esbuild: `esbuild.config.js`, `esbuild.config.mjs`, or `esbuild` in scripts
/// 4. Rollup: `rollup.config.js`, `rollup.config.ts`, `rollup.config.mjs`
/// 5. Next.js: `next.config.js`, `next.config.mjs`, `next.config.ts`
/// 6. Parcel: `.parcelrc` or `parcel` in devDependencies
pub fn detect_bundler(root: &Path) -> Bundler {
    // Check Vite
    if root.join("vite.config.ts").exists()
        || root.join("vite.config.js").exists()
        || root.join("vite.config.mjs").exists()
    {
        return Bundler::Vite;
    }

    // Check Webpack
    if root.join("webpack.config.js").exists()
        || root.join("webpack.config.ts").exists()
        || root.join("webpack.config.cjs").exists()
    {
        return Bundler::Webpack;
    }

    // Check esbuild
    if root.join("esbuild.config.js").exists()
        || root.join("esbuild.config.mjs").exists()
        || has_esbuild_in_scripts(root)
    {
        return Bundler::Esbuild;
    }

    // Check Rollup
    if root.join("rollup.config.js").exists()
        || root.join("rollup.config.ts").exists()
        || root.join("rollup.config.mjs").exists()
    {
        return Bundler::Rollup;
    }

    // Check Next.js
    if root.join("next.config.js").exists()
        || root.join("next.config.mjs").exists()
        || root.join("next.config.ts").exists()
    {
        return Bundler::NextJs;
    }

    // Check Parcel
    if root.join(".parcelrc").exists() || has_parcel_dependency(root) {
        return Bundler::Parcel;
    }

    Bundler::Unknown
}

/// Check if esbuild is used in package.json scripts.
fn has_esbuild_in_scripts(root: &Path) -> bool {
    let pkg_path = root.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg_path)
        && let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(scripts) = pkg.get("scripts").and_then(|s| s.as_object())
    {
        return scripts
            .values()
            .filter_map(|v| v.as_str())
            .any(|script| script.contains("esbuild"));
    }
    false
}

/// Check if parcel is in devDependencies.
fn has_parcel_dependency(root: &Path) -> bool {
    let pkg_path = root.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg_path)
        && let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(deps) = pkg.get("devDependencies").and_then(|d| d.as_object())
    {
        return deps.contains_key("parcel");
    }
    false
}

#[cfg(test)]
#[path = "bundler_tests.rs"]
mod tests;
