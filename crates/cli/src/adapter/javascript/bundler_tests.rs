// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use tempfile::TempDir;

#[test]
fn detect_vite_ts_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("vite.config.ts"), "export default {}").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Vite);
}

#[test]
fn detect_vite_js_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("vite.config.js"), "export default {}").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Vite);
}

#[test]
fn detect_vite_mjs_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("vite.config.mjs"), "export default {}").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Vite);
}

#[test]
fn detect_webpack_js_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("webpack.config.js"), "module.exports = {}").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Webpack);
}

#[test]
fn detect_webpack_ts_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("webpack.config.ts"), "export default {}").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Webpack);
}

#[test]
fn detect_webpack_cjs_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("webpack.config.cjs"), "module.exports = {}").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Webpack);
}

#[test]
fn detect_esbuild_js_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("esbuild.config.js"), "").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Esbuild);
}

#[test]
fn detect_esbuild_mjs_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("esbuild.config.mjs"), "").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Esbuild);
}

#[test]
fn detect_esbuild_in_scripts() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"scripts": {"build": "esbuild src/index.ts --outdir=dist"}}"#,
    )
    .unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Esbuild);
}

#[test]
fn detect_rollup_js_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("rollup.config.js"), "export default {}").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Rollup);
}

#[test]
fn detect_rollup_ts_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("rollup.config.ts"), "export default {}").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Rollup);
}

#[test]
fn detect_rollup_mjs_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("rollup.config.mjs"), "export default {}").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Rollup);
}

#[test]
fn detect_nextjs_js_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("next.config.js"), "module.exports = {}").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::NextJs);
}

#[test]
fn detect_nextjs_mjs_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("next.config.mjs"), "export default {}").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::NextJs);
}

#[test]
fn detect_nextjs_ts_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("next.config.ts"), "export default {}").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::NextJs);
}

#[test]
fn detect_parcel_parcelrc() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join(".parcelrc"), "{}").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Parcel);
}

#[test]
fn detect_parcel_devdependency() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies": {"parcel": "^2.0.0"}}"#,
    )
    .unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Parcel);
}

#[test]
fn detect_unknown_no_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"name": "test"}"#).unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Unknown);
}

#[test]
fn detect_unknown_empty_dir() {
    let dir = TempDir::new().unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Unknown);
}

#[test]
fn vite_takes_precedence_over_rollup() {
    // Vite uses Rollup internally, so both configs might exist
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("vite.config.ts"), "").unwrap();
    std::fs::write(dir.path().join("rollup.config.js"), "").unwrap();

    assert_eq!(detect_bundler(dir.path()), Bundler::Vite);
}

#[test]
fn bundler_default_output_dirs() {
    assert_eq!(Bundler::Vite.default_output_dir(), "dist");
    assert_eq!(Bundler::Webpack.default_output_dir(), "dist");
    assert_eq!(Bundler::Esbuild.default_output_dir(), "dist");
    assert_eq!(Bundler::Rollup.default_output_dir(), "dist");
    assert_eq!(Bundler::NextJs.default_output_dir(), ".next/static");
    assert_eq!(Bundler::Parcel.default_output_dir(), "dist");
    assert_eq!(Bundler::Unknown.default_output_dir(), "dist");
}
