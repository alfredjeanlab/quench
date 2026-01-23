#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::path::Path;
use yare::parameterized;

#[parameterized(
    // Test directory patterns
    tests_dir = { "tests/foo.rs", FileKind::Test },
    test_dir = { "test/bar.py", FileKind::Test },
    nested_tests = { "crate/tests/unit.rs", FileKind::Test },

    // Test file patterns
    suffix_test = { "src/foo_test.rs", FileKind::Test },
    suffix_tests = { "src/bar_tests.rs", FileKind::Test },
    dot_test = { "src/baz.test.js", FileKind::Test },
    dot_spec = { "src/qux.spec.ts", FileKind::Test },

    // Source files
    src_file = { "src/lib.rs", FileKind::Source },
    root_file = { "main.py", FileKind::Source },
    nested_src = { "pkg/internal/util.go", FileKind::Source },
)]
fn classify_with_defaults(path: &str, expected: FileKind) {
    let adapter = GenericAdapter::with_defaults();
    assert_eq!(adapter.classify(Path::new(path)), expected);
}

#[test]
fn classify_with_source_patterns() {
    let adapter = GenericAdapter::new(
        &["src/**/*".to_string(), "lib/**/*".to_string()],
        &["**/tests/**".to_string()],
    );

    assert_eq!(adapter.classify(Path::new("src/main.rs")), FileKind::Source);
    assert_eq!(adapter.classify(Path::new("lib/util.rs")), FileKind::Source);
    assert_eq!(adapter.classify(Path::new("bin/cli.rs")), FileKind::Other);
    assert_eq!(
        adapter.classify(Path::new("src/tests/unit.rs")),
        FileKind::Test
    );
}

#[test]
fn no_default_escapes() {
    let adapter = GenericAdapter::with_defaults();
    assert!(adapter.default_escapes().is_empty());
}
