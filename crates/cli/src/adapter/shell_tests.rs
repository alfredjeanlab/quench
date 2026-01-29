// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for the Shell adapter.

use std::path::Path;

use yare::parameterized;

use super::ShellAdapter;
use crate::adapter::{Adapter, FileKind};

// =============================================================================
// FILE CLASSIFICATION
// =============================================================================

#[parameterized(
    // Source files
    src_root_sh = { "build.sh", FileKind::Source },
    src_scripts_sh = { "scripts/deploy.sh", FileKind::Source },
    src_nested_sh = { "scripts/ci/build.sh", FileKind::Source },
    src_bash = { "scripts/setup.bash", FileKind::Source },
    src_bin_sh = { "bin/run.sh", FileKind::Source },
    // Test files
    test_bats = { "tests/integration.bats", FileKind::Test },
    test_dir_bats = { "test/unit.bats", FileKind::Test },
    test_nested_bats = { "tests/e2e/smoke.bats", FileKind::Test },
    test_suffix = { "build_test.sh", FileKind::Test },
    test_suffix_nested = { "scripts/build_test.sh", FileKind::Test },
    // Other files
    other_toml = { "quench.toml", FileKind::Other },
    other_md = { "README.md", FileKind::Other },
    other_rs = { "src/lib.rs", FileKind::Other },
)]
fn classify_path(path: &str, expected: FileKind) {
    let adapter = ShellAdapter::new();
    assert_eq!(
        adapter.classify(Path::new(path)),
        expected,
        "path {:?} should be {:?}",
        path,
        expected
    );
}

#[test]
fn name_returns_shell() {
    let adapter = ShellAdapter::new();
    assert_eq!(adapter.name(), "shell");
}

#[test]
fn extensions_include_sh_bash_bats() {
    let adapter = ShellAdapter::new();
    let exts = adapter.extensions();
    assert!(exts.contains(&"sh"), "should include sh");
    assert!(exts.contains(&"bash"), "should include bash");
    assert!(exts.contains(&"bats"), "should include bats");
}

// =============================================================================
// EXCLUDE PATTERN TESTS
// =============================================================================

#[test]
fn should_exclude_returns_false_by_default() {
    let adapter = ShellAdapter::new();
    assert!(
        !adapter.should_exclude(Path::new("scripts/build.sh")),
        "default adapter should not exclude anything"
    );
}

#[test]
fn should_exclude_with_custom_patterns() {
    use crate::adapter::patterns::ResolvedPatterns;

    let patterns = ResolvedPatterns {
        source: vec!["**/*.sh".to_string()],
        test: vec!["**/*_test.sh".to_string()],
        exclude: vec!["tmp/**".to_string()],
    };
    let adapter = ShellAdapter::with_patterns(patterns);
    assert!(
        adapter.should_exclude(Path::new("tmp/test.sh")),
        "tmp/ should be excluded"
    );
    assert!(
        !adapter.should_exclude(Path::new("scripts/build.sh")),
        "scripts/ should not be excluded"
    );
}

#[test]
fn classify_excludes_paths_matching_exclude_patterns() {
    use crate::adapter::patterns::ResolvedPatterns;

    let patterns = ResolvedPatterns {
        source: vec!["**/*.sh".to_string()],
        test: vec!["**/*_test.sh".to_string()],
        exclude: vec!["tmp/**".to_string()],
    };
    let adapter = ShellAdapter::with_patterns(patterns);
    assert_eq!(
        adapter.classify(Path::new("tmp/script.sh")),
        FileKind::Other,
        "excluded paths should classify as Other"
    );
}

// =============================================================================
// DEFAULT ESCAPE PATTERNS
// =============================================================================

#[test]
fn default_escapes_returns_patterns() {
    let adapter = ShellAdapter::new();
    let escapes = adapter.default_escapes();
    assert!(!escapes.is_empty(), "should return escape patterns");
}

#[test]
fn default_escapes_includes_set_plus_e() {
    let adapter = ShellAdapter::new();
    let escapes = adapter.default_escapes();
    let found = escapes.iter().any(|p| p.name == "set_plus_e");
    assert!(found, "should include set_plus_e pattern");
}

#[test]
fn default_escapes_includes_eval() {
    let adapter = ShellAdapter::new();
    let escapes = adapter.default_escapes();
    let found = escapes.iter().any(|p| p.name == "eval");
    assert!(found, "should include eval pattern");
}

#[test]
fn escape_patterns_use_ok_comment() {
    let adapter = ShellAdapter::new();
    for pattern in adapter.default_escapes() {
        assert_eq!(
            pattern.comment,
            Some("# OK:"),
            "pattern {} should require # OK: comment",
            pattern.name
        );
    }
}
