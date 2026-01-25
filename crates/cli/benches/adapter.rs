// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Adapter-specific benchmarks.
//!
//! Measures overhead of language adapter operations:
//! - Adapter creation (GlobSet compilation)
//! - File classification
//! - Line classification with #[cfg(test)] parsing
//! - Workspace detection
//! - Suppress attribute parsing

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use globset::{Glob, GlobSetBuilder};
use std::path::{Path, PathBuf};

use quench::adapter::go::{GoAdapter, parse_go_mod, parse_nolint_directives};
use quench::adapter::rust::{CargoWorkspace, CfgTestInfo, RustAdapter, parse_suppress_attrs};
use quench::adapter::shell::{ShellAdapter, parse_shellcheck_suppresses};
use quench::adapter::{Adapter, GenericAdapter, enumerate_packages};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(name)
}

/// Benchmark adapter creation (GlobSet compilation).
fn bench_adapter_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("adapter_creation");

    group.bench_function("RustAdapter::new", |b| {
        b.iter(|| black_box(RustAdapter::new()))
    });

    group.bench_function("GenericAdapter::with_defaults", |b| {
        b.iter(|| black_box(GenericAdapter::with_defaults()))
    });

    group.bench_function("ShellAdapter::new", |b| {
        b.iter(|| black_box(ShellAdapter::new()))
    });

    // GenericAdapter with custom patterns (like a configured project)
    let source_patterns: Vec<String> = vec!["**/*.rs".to_string(), "**/*.py".to_string()];
    let test_patterns: Vec<String> = vec![
        "tests/**".to_string(),
        "*_test.rs".to_string(),
        "test_*.py".to_string(),
    ];

    group.bench_function("GenericAdapter::new_with_patterns", |b| {
        b.iter(|| black_box(GenericAdapter::new(&source_patterns, &test_patterns)))
    });

    group.finish();
}

/// Benchmark individual GlobSet pattern compilation.
///
/// Investigates why Shell adapter creation (6 patterns, 2 builds) is slower
/// than Rust adapter creation (6 patterns, 3 builds).
fn bench_globset_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("globset_patterns");

    // Shell source patterns (2 patterns)
    group.bench_function("shell_source_patterns", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("**/*.sh").unwrap());
            builder.add(Glob::new("**/*.bash").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    // Shell test patterns (3 patterns - *_test.sh removed as redundant with **/*_test.sh)
    group.bench_function("shell_test_patterns", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("tests/**/*.bats").unwrap());
            builder.add(Glob::new("test/**/*.bats").unwrap());
            builder.add(Glob::new("**/*_test.sh").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    // Rust source pattern (1 pattern)
    group.bench_function("rust_source_pattern", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("**/*.rs").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    // Rust test patterns (4 patterns)
    group.bench_function("rust_test_patterns", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("tests/**").unwrap());
            builder.add(Glob::new("test/**/*.rs").unwrap());
            builder.add(Glob::new("*_test.rs").unwrap());
            builder.add(Glob::new("*_tests.rs").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    // Rust ignore pattern (1 pattern)
    group.bench_function("rust_ignore_pattern", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("target/**").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    // Individual pattern compilation time
    group.bench_function("single_pattern_star_sh", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("**/*.sh").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    group.bench_function("single_pattern_star_rs", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("**/*.rs").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    group.bench_function("single_pattern_star_bash", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("**/*.bash").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    group.bench_function("single_pattern_star_bats", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("**/*.bats").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    // Test combined single build (optimization candidate)
    group.bench_function("shell_combined_single_build", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            // All shell patterns in single build (optimized - 5 patterns)
            builder.add(Glob::new("**/*.sh").unwrap());
            builder.add(Glob::new("**/*.bash").unwrap());
            builder.add(Glob::new("tests/**/*.bats").unwrap());
            builder.add(Glob::new("test/**/*.bats").unwrap());
            builder.add(Glob::new("**/*_test.sh").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    group.bench_function("rust_combined_single_build", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            // All rust patterns in single build
            builder.add(Glob::new("**/*.rs").unwrap());
            builder.add(Glob::new("tests/**").unwrap());
            builder.add(Glob::new("test/**/*.rs").unwrap());
            builder.add(Glob::new("*_test.rs").unwrap());
            builder.add(Glob::new("*_tests.rs").unwrap());
            builder.add(Glob::new("target/**").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    group.finish();
}

/// Benchmark file classification.
fn bench_classify(c: &mut Criterion) {
    let rust_adapter = RustAdapter::new();
    let generic_adapter = GenericAdapter::with_defaults();
    let shell_adapter = ShellAdapter::new();

    // Generate test paths
    let source_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("src/module_{}.rs", i)))
        .collect();
    let test_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("tests/test_{}.rs", i)))
        .collect();
    let nested_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("crates/pkg_{}/src/lib.rs", i)))
        .collect();

    // Shell paths
    let shell_source_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("scripts/script_{}.sh", i)))
        .collect();
    let shell_bash_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("lib/util_{}.bash", i)))
        .collect();
    let shell_test_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("tests/test_{}.bats", i)))
        .collect();
    let shell_bin_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("bin/cmd_{}.sh", i)))
        .collect();

    let mut group = c.benchmark_group("classify");

    group.bench_function("rust_1k_source", |b| {
        b.iter(|| {
            for path in &source_paths {
                black_box(rust_adapter.classify(path));
            }
        })
    });

    group.bench_function("rust_1k_test", |b| {
        b.iter(|| {
            for path in &test_paths {
                black_box(rust_adapter.classify(path));
            }
        })
    });

    group.bench_function("rust_1k_nested", |b| {
        b.iter(|| {
            for path in &nested_paths {
                black_box(rust_adapter.classify(path));
            }
        })
    });

    group.bench_function("generic_1k_source", |b| {
        b.iter(|| {
            for path in &source_paths {
                black_box(generic_adapter.classify(path));
            }
        })
    });

    group.bench_function("generic_1k_test", |b| {
        b.iter(|| {
            for path in &test_paths {
                black_box(generic_adapter.classify(path));
            }
        })
    });

    group.bench_function("shell_1k_source_scripts", |b| {
        b.iter(|| {
            for path in &shell_source_paths {
                black_box(shell_adapter.classify(path));
            }
        })
    });

    group.bench_function("shell_1k_bash_libs", |b| {
        b.iter(|| {
            for path in &shell_bash_paths {
                black_box(shell_adapter.classify(path));
            }
        })
    });

    group.bench_function("shell_1k_bats_tests", |b| {
        b.iter(|| {
            for path in &shell_test_paths {
                black_box(shell_adapter.classify(path));
            }
        })
    });

    group.bench_function("shell_1k_bin_scripts", |b| {
        b.iter(|| {
            for path in &shell_bin_paths {
                black_box(shell_adapter.classify(path));
            }
        })
    });

    group.finish();
}

/// Benchmark cfg test parsing.
fn bench_cfg_test_parse(c: &mut Criterion) {
    // Content with cfg test block at line 50
    let cfg_test_attr = "#[cfg(test)]";
    let content_with_cfg: String = (0..100)
        .map(|i| {
            if i == 50 {
                format!("{}\nmod tests {{\n    use super::*;\n    #[test]\n    fn test_it() {{\n        assert!(true);\n    }}\n}}\n", cfg_test_attr)
            } else {
                format!("pub fn func_{}() {{ /* impl */ }}\n", i)
            }
        })
        .collect();

    // Content without #[cfg(test)]
    let content_without_cfg: String = (0..100)
        .map(|i| format!("pub fn func_{}() {{ /* impl */ }}\n", i))
        .collect();

    let mut group = c.benchmark_group("cfg_test_parse");

    group.bench_function("with_cfg_test_100_lines", |b| {
        b.iter(|| black_box(CfgTestInfo::parse(&content_with_cfg)))
    });

    group.bench_function("without_cfg_test_100_lines", |b| {
        b.iter(|| black_box(CfgTestInfo::parse(&content_without_cfg)))
    });

    // Larger file (~1000 lines)
    let large_content: String = content_with_cfg.repeat(10);
    group.bench_function("with_cfg_test_1000_lines", |b| {
        b.iter(|| black_box(CfgTestInfo::parse(&large_content)))
    });

    // Very large file (~5000 lines)
    let very_large_content: String = content_with_cfg.repeat(50);
    group.bench_function("with_cfg_test_5000_lines", |b| {
        b.iter(|| black_box(CfgTestInfo::parse(&very_large_content)))
    });

    group.finish();
}

/// Benchmark classify_lines (full line classification with cfg_test).
fn bench_classify_lines(c: &mut Criterion) {
    let adapter = RustAdapter::new();
    let source_path = Path::new("src/lib.rs");
    let test_path = Path::new("tests/integration.rs");

    // Mixed source/test content (simulates file with inline tests)
    let cfg_test_attr = "#[cfg(test)]";
    let mixed_content: String = (0..100)
        .map(|i| {
            if (60..80).contains(&i) {
                if i == 60 {
                    format!("{}\nmod tests {{\n    use super::*;\n", cfg_test_attr)
                } else if i == 79 {
                    "}\n".to_string()
                } else {
                    "    #[test]\n    fn test() { assert!(true); }\n".to_string()
                }
            } else {
                format!("pub fn func_{}() {{ /* implementation */ }}\n", i)
            }
        })
        .collect();

    // Pure source content (no inline tests)
    let source_content: String = (0..100)
        .map(|i| format!("pub fn func_{}() {{ /* implementation */ }}\n", i))
        .collect();

    let mut group = c.benchmark_group("classify_lines");

    group.bench_function("source_file_100_lines_mixed", |b| {
        b.iter(|| black_box(adapter.classify_lines(source_path, &mixed_content)))
    });

    group.bench_function("source_file_100_lines_pure", |b| {
        b.iter(|| black_box(adapter.classify_lines(source_path, &source_content)))
    });

    group.bench_function("test_file_100_lines", |b| {
        b.iter(|| black_box(adapter.classify_lines(test_path, &mixed_content)))
    });

    // Larger content
    let large_mixed: String = mixed_content.repeat(10);
    group.bench_function("source_file_1000_lines_mixed", |b| {
        b.iter(|| black_box(adapter.classify_lines(source_path, &large_mixed)))
    });

    group.finish();
}

/// Benchmark workspace detection.
fn bench_workspace_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("workspace_detection");

    let rust_simple = fixture_path("rust-simple");
    let rust_workspace = fixture_path("rust-workspace");
    let bench_rust = fixture_path("bench-rust");

    if rust_simple.exists() {
        group.bench_with_input(
            BenchmarkId::new("from_root", "rust-simple"),
            &rust_simple,
            |b, path| b.iter(|| black_box(CargoWorkspace::from_root(path))),
        );
    }

    if rust_workspace.exists() {
        group.bench_with_input(
            BenchmarkId::new("from_root", "rust-workspace"),
            &rust_workspace,
            |b, path| b.iter(|| black_box(CargoWorkspace::from_root(path))),
        );
    }

    if bench_rust.exists() {
        group.bench_with_input(
            BenchmarkId::new("from_root", "bench-rust"),
            &bench_rust,
            |b, path| b.iter(|| black_box(CargoWorkspace::from_root(path))),
        );
    }

    group.finish();
}

/// Benchmark suppress attribute parsing.
fn bench_suppress_parse(c: &mut Criterion) {
    // Content with various suppress attributes (~10% of lines)
    let allow_dead = "#[allow(dead_code)]";
    let content_with_suppresses: String = (0..100)
        .map(|i| {
            if i % 10 == 0 {
                format!("{}\npub fn func() {{}}\n", allow_dead)
            } else if i % 15 == 0 {
                "// REASON: test fixture\n#[expect(unused_variables)]\npub fn func() {}\n"
                    .to_string()
            } else if i % 20 == 0 {
                "#[allow(clippy::unwrap_used)]\npub fn func() {}\n".to_string()
            } else {
                format!("pub fn func_{}() {{}}\n", i)
            }
        })
        .collect();

    // Content without suppress attributes
    let content_without: String = (0..100)
        .map(|i| format!("pub fn func_{}() {{}}\n", i))
        .collect();

    let mut group = c.benchmark_group("suppress_parse");

    group.bench_function("with_attrs_100_lines", |b| {
        b.iter(|| black_box(parse_suppress_attrs(&content_with_suppresses, None)))
    });

    group.bench_function("without_attrs_100_lines", |b| {
        b.iter(|| black_box(parse_suppress_attrs(&content_without, None)))
    });

    // With required comment pattern
    group.bench_function("with_attrs_100_lines_pattern", |b| {
        b.iter(|| {
            black_box(parse_suppress_attrs(
                &content_with_suppresses,
                Some("// REASON:"),
            ))
        })
    });

    // Larger file
    let large_with_suppresses: String = content_with_suppresses.repeat(10);
    group.bench_function("with_attrs_1000_lines", |b| {
        b.iter(|| black_box(parse_suppress_attrs(&large_with_suppresses, None)))
    });

    group.finish();
}

/// Benchmark shellcheck suppress parsing.
fn bench_shellcheck_suppress_parse(c: &mut Criterion) {
    // Content with shellcheck suppresses (~10% of lines)
    let content_with_suppresses: String = (0..100)
        .map(|i| {
            if i % 10 == 0 {
                "# shellcheck disable=SC2034  # OK: intentional\nUNUSED_VAR=1\n".to_string()
            } else if i % 15 == 0 {
                "# shellcheck disable=SC2086,SC2046\n# OK: word splitting needed\necho $var\n"
                    .to_string()
            } else {
                format!("echo \"line {}\"\n", i)
            }
        })
        .collect();

    // Content without suppresses
    let content_without: String = (0..100).map(|i| format!("echo \"line {}\"\n", i)).collect();

    let mut group = c.benchmark_group("shellcheck_suppress_parse");

    group.bench_function("with_suppresses_100_lines", |b| {
        b.iter(|| black_box(parse_shellcheck_suppresses(&content_with_suppresses, None)))
    });

    group.bench_function("without_suppresses_100_lines", |b| {
        b.iter(|| black_box(parse_shellcheck_suppresses(&content_without, None)))
    });

    // With comment pattern requirement
    group.bench_function("with_suppresses_100_lines_pattern", |b| {
        b.iter(|| {
            black_box(parse_shellcheck_suppresses(
                &content_with_suppresses,
                Some("# OK:"),
            ))
        })
    });

    // Larger file (~1000 lines)
    let large_with_suppresses: String = content_with_suppresses.repeat(10);
    group.bench_function("with_suppresses_1000_lines", |b| {
        b.iter(|| black_box(parse_shellcheck_suppresses(&large_with_suppresses, None)))
    });

    group.finish();
}

// =============================================================================
// GO ADAPTER BENCHMARKS
// =============================================================================

/// Benchmark GoAdapter creation (GlobSet compilation).
fn bench_go_adapter_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("go_adapter_creation");

    group.bench_function("GoAdapter::new", |b| b.iter(|| black_box(GoAdapter::new())));

    group.finish();
}

/// Benchmark Go file classification.
fn bench_go_classify(c: &mut Criterion) {
    let go_adapter = GoAdapter::new();

    // Generate test paths
    let source_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("pkg/module_{}/handler.go", i)))
        .collect();
    let test_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("pkg/module_{}/handler_test.go", i)))
        .collect();
    let vendor_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("vendor/github.com/pkg/lib_{}.go", i)))
        .collect();

    let mut group = c.benchmark_group("go_classify");

    group.bench_function("go_1k_source", |b| {
        b.iter(|| {
            for path in &source_paths {
                black_box(go_adapter.classify(path));
            }
        })
    });

    group.bench_function("go_1k_test", |b| {
        b.iter(|| {
            for path in &test_paths {
                black_box(go_adapter.classify(path));
            }
        })
    });

    group.bench_function("go_1k_vendor_ignored", |b| {
        b.iter(|| {
            for path in &vendor_paths {
                black_box(go_adapter.classify(path));
            }
        })
    });

    group.finish();
}

/// Benchmark go.mod parsing.
fn bench_go_mod_parse(c: &mut Criterion) {
    let simple_go_mod = "module github.com/example/project\n\ngo 1.21\n";
    let complex_go_mod = r#"
module github.com/example/complex-project

go 1.22

require (
    github.com/pkg/errors v0.9.1
    github.com/stretchr/testify v1.8.4
    golang.org/x/sync v0.3.0
)

require (
    github.com/davecgh/go-spew v1.1.1 // indirect
    github.com/pmezard/go-difflib v1.0.0 // indirect
    gopkg.in/yaml.v3 v3.0.1 // indirect
)
"#;

    let mut group = c.benchmark_group("go_mod_parse");

    group.bench_function("simple_go_mod", |b| {
        b.iter(|| black_box(parse_go_mod(simple_go_mod)))
    });

    group.bench_function("complex_go_mod", |b| {
        b.iter(|| black_box(parse_go_mod(complex_go_mod)))
    });

    group.finish();
}

/// Benchmark package enumeration on fixtures.
fn bench_package_enumeration(c: &mut Criterion) {
    let go_simple = fixture_path("go-simple");
    let go_multi = fixture_path("go-multi");
    let golang = fixture_path("golang");

    let mut group = c.benchmark_group("go_package_enumeration");

    if go_simple.exists() {
        group.bench_with_input(
            BenchmarkId::new("enumerate_packages", "go-simple"),
            &go_simple,
            |b, path| b.iter(|| black_box(enumerate_packages(path))),
        );
    }

    if go_multi.exists() {
        group.bench_with_input(
            BenchmarkId::new("enumerate_packages", "go-multi"),
            &go_multi,
            |b, path| b.iter(|| black_box(enumerate_packages(path))),
        );
    }

    if golang.exists() {
        group.bench_with_input(
            BenchmarkId::new("enumerate_packages", "golang"),
            &golang,
            |b, path| b.iter(|| black_box(enumerate_packages(path))),
        );
    }

    group.finish();
}

/// Benchmark //nolint directive parsing.
fn bench_nolint_parse(c: &mut Criterion) {
    // Content with various nolint directives
    let content_with_nolint: String = (0..100)
        .map(|i| {
            if i % 10 == 0 {
                "//nolint:errcheck // OK: error is logged\nfunc process() error {\n".to_string()
            } else if i % 15 == 0 {
                "//nolint:gosec,govet\n// REASON: legacy code\nfunc legacy() {}\n".to_string()
            } else if i % 20 == 0 {
                "//nolint\nfunc skip() {}\n".to_string()
            } else {
                format!("func handler_{}() {{}}\n", i)
            }
        })
        .collect();

    let content_without: String = (0..100)
        .map(|i| format!("func handler_{}() {{}}\n", i))
        .collect();

    let mut group = c.benchmark_group("nolint_parse");

    group.bench_function("with_nolint_100_lines", |b| {
        b.iter(|| black_box(parse_nolint_directives(&content_with_nolint, None)))
    });

    group.bench_function("without_nolint_100_lines", |b| {
        b.iter(|| black_box(parse_nolint_directives(&content_without, None)))
    });

    group.bench_function("with_nolint_100_lines_pattern", |b| {
        b.iter(|| {
            black_box(parse_nolint_directives(
                &content_with_nolint,
                Some("// REASON:"),
            ))
        })
    });

    // Larger file
    let large_content: String = content_with_nolint.repeat(10);
    group.bench_function("with_nolint_1000_lines", |b| {
        b.iter(|| black_box(parse_nolint_directives(&large_content, None)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_adapter_creation,
    bench_globset_patterns,
    bench_classify,
    bench_cfg_test_parse,
    bench_classify_lines,
    bench_workspace_detection,
    bench_suppress_parse,
    bench_shellcheck_suppress_parse,
    // Go adapter benchmarks
    bench_go_adapter_creation,
    bench_go_classify,
    bench_go_mod_parse,
    bench_package_enumeration,
    bench_nolint_parse,
);
criterion_main!(benches);
