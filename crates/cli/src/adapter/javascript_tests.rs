#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use super::*;
use crate::adapter::{Adapter, EscapeAction, FileKind};
use yare::parameterized;

// =============================================================================
// FILE CLASSIFICATION TESTS
// =============================================================================

#[parameterized(
    src_js = { "src/index.js", FileKind::Source },
    src_jsx = { "src/App.jsx", FileKind::Source },
    src_ts = { "src/index.ts", FileKind::Source },
    src_tsx = { "src/App.tsx", FileKind::Source },
    src_mjs = { "src/utils.mjs", FileKind::Source },
    src_mts = { "src/utils.mts", FileKind::Source },
    nested = { "packages/core/src/lib.ts", FileKind::Source },
    test_dot_test_js = { "src/index.test.js", FileKind::Test },
    test_dot_test_ts = { "src/index.test.ts", FileKind::Test },
    test_dot_test_jsx = { "src/App.test.jsx", FileKind::Test },
    test_dot_test_tsx = { "src/App.test.tsx", FileKind::Test },
    test_dot_spec_js = { "src/index.spec.js", FileKind::Test },
    test_dot_spec_ts = { "src/index.spec.ts", FileKind::Test },
    test_dunder_tests = { "src/__tests__/helper.ts", FileKind::Test },
    test_dir = { "test/integration.ts", FileKind::Test },
    tests_dir = { "tests/e2e.ts", FileKind::Test },
    node_modules = { "node_modules/lodash/index.js", FileKind::Other },
    dist = { "dist/bundle.js", FileKind::Other },
    build = { "build/output.js", FileKind::Other },
    next = { ".next/cache/data.js", FileKind::Other },
    coverage = { "coverage/lcov/lib.js", FileKind::Other },
    readme = { "README.md", FileKind::Other },
    json = { "package.json", FileKind::Other },
)]
fn classify_path(path: &str, expected: FileKind) {
    let adapter = JavaScriptAdapter::new();
    assert_eq!(
        adapter.classify(Path::new(path)),
        expected,
        "path {:?} should be {:?}",
        path,
        expected
    );
}

// =============================================================================
// IGNORE PATTERN TESTS
// =============================================================================

#[parameterized(
    node_modules = { "node_modules/lodash/index.js", true },
    dist = { "dist/bundle.js", true },
    build = { "build/output.js", true },
    next = { ".next/cache.js", true },
    coverage = { "coverage/lcov.js", true },
    src = { "src/index.js", false },
    packages = { "packages/core/lib.ts", false },
)]
fn should_ignore_path(path: &str, expected: bool) {
    let adapter = JavaScriptAdapter::new();
    assert_eq!(
        adapter.should_ignore(Path::new(path)),
        expected,
        "path {:?} should_ignore = {}",
        path,
        expected
    );
}

// =============================================================================
// ADAPTER METADATA TESTS
// =============================================================================

#[test]
fn has_correct_name_and_extensions() {
    let adapter = JavaScriptAdapter::new();
    assert_eq!(adapter.name(), "javascript");
    assert_eq!(
        adapter.extensions(),
        &["js", "jsx", "ts", "tsx", "mjs", "mts"]
    );
}

#[test]
fn default_escapes_has_js_patterns() {
    let adapter = JavaScriptAdapter::new();
    let escapes = adapter.default_escapes();

    assert_eq!(escapes.len(), 2);

    // Verify as_unknown pattern
    let as_unknown = escapes.iter().find(|p| p.name == "as_unknown").unwrap();
    assert_eq!(as_unknown.action, EscapeAction::Comment);
    assert_eq!(as_unknown.comment, Some("// CAST:"));

    // Verify ts_ignore pattern
    let ts_ignore = escapes.iter().find(|p| p.name == "ts_ignore").unwrap();
    assert_eq!(ts_ignore.action, EscapeAction::Forbid);
    assert!(ts_ignore.comment.is_none());
}

// =============================================================================
// ESCAPE PATTERN MATCHING TESTS
// =============================================================================

#[test]
fn as_unknown_pattern_matches() {
    use crate::pattern::CompiledPattern;

    let adapter = JavaScriptAdapter::new();
    let pattern = adapter
        .default_escapes()
        .iter()
        .find(|p| p.name == "as_unknown")
        .unwrap();

    let compiled = CompiledPattern::compile(pattern.pattern).unwrap();

    // Should match
    assert!(!compiled.find_all("data as unknown as UserData").is_empty());
    assert!(!compiled.find_all("value as  unknown").is_empty()); // extra space

    // Should not match
    assert!(compiled.find_all("as UnknownType").is_empty()); // not the keyword
    // Note: "// as unknown" still matches the pattern - comment detection is handled separately
}

#[test]
fn ts_ignore_pattern_matches() {
    use crate::pattern::CompiledPattern;

    let adapter = JavaScriptAdapter::new();
    let pattern = adapter
        .default_escapes()
        .iter()
        .find(|p| p.name == "ts_ignore")
        .unwrap();

    let compiled = CompiledPattern::compile(pattern.pattern).unwrap();

    // Should match
    assert!(!compiled.find_all("// @ts-ignore").is_empty());
    assert!(
        !compiled
            .find_all("// @ts-ignore next line is wrong")
            .is_empty()
    );

    // Should not match
    assert!(compiled.find_all("// @ts-expect-error").is_empty()); // allowed alternative
    assert!(compiled.find_all("// ts-ignore").is_empty()); // missing @
}

// =============================================================================
// PACKAGE NAME EXTRACTION TESTS
// =============================================================================

#[test]
fn package_name_returns_none_for_missing_file() {
    let name = JavaScriptAdapter::package_name(Path::new("/nonexistent"));
    assert!(name.is_none());
}
