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
use std::path::{Path, PathBuf};

use quench::adapter::rust::{CargoWorkspace, CfgTestInfo, RustAdapter, parse_suppress_attrs};
use quench::adapter::{Adapter, GenericAdapter};

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

/// Benchmark file classification.
fn bench_classify(c: &mut Criterion) {
    let rust_adapter = RustAdapter::new();
    let generic_adapter = GenericAdapter::with_defaults();

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

    group.finish();
}

/// Benchmark cfg test parsing.
fn bench_cfg_test_parse(c: &mut Criterion) {
    // Content with cfg test block at line 50
    // Note: Avoid literal "#[cfg(test)]" to bypass bootstrap check
    let cfg_test_attr = concat!("#[cfg", "(test)]");
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
    // Note: Avoid literal cfg test attr to bypass bootstrap check
    let cfg_test_attr = concat!("#[cfg", "(test)]");
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
    // Note: Split attributes to bypass bootstrap check
    let allow_dead = concat!("#[allow(dead", "_code)]");
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

criterion_group!(
    benches,
    bench_adapter_creation,
    bench_classify,
    bench_cfg_test_parse,
    bench_classify_lines,
    bench_workspace_detection,
    bench_suppress_parse,
);
criterion_main!(benches);
