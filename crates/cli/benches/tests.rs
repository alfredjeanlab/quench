// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Benchmarks for tests correlation check.
//!
//! Tests performance of:
//! - Candidate test path generation
//! - Glob pattern matching for test files
//! - Inline test detection (#[cfg(test)] blocks)
//! - Git diff parsing
//! - End-to-end correlation detection

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::path::{Path, PathBuf};
use std::process::Command;

use globset::{Glob, GlobSet, GlobSetBuilder};

use quench::checks::testing::correlation::{
    CorrelationConfig, TestIndex, analyze_correlation, candidate_test_paths, changes_in_cfg_test,
    find_test_locations, has_correlated_test,
};
use quench::checks::testing::diff::{ChangeType, FileChange};

/// Create a Rust-like CorrelationConfig for benchmarking.
fn rust_correlation_config() -> CorrelationConfig {
    CorrelationConfig {
        test_patterns: vec![
            "tests/**/*".to_string(),
            "test/**/*".to_string(),
            "spec/**/*".to_string(),
            "**/__tests__/**".to_string(),
            "**/*_test.*".to_string(),
            "**/*_tests.*".to_string(),
            "**/*.test.*".to_string(),
            "**/*.spec.*".to_string(),
            "**/test_*.*".to_string(),
        ],
        source_patterns: vec!["src/**/*".to_string()],
        exclude_patterns: vec![
            "**/generated/**".to_string(),
            "**/mod.rs".to_string(),
            "**/lib.rs".to_string(),
            "**/main.rs".to_string(),
        ],
    }
}

/// Build a GlobSet from pattern strings for benchmarking.
fn build_glob_set(patterns: &[String]) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|e| e.to_string())?;
        builder.add(glob);
    }
    builder.build().map_err(|e| e.to_string())
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(name)
}

/// Count files in a directory recursively (for throughput metrics).
fn count_files(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }

    ignore::WalkBuilder::new(path)
        .hidden(true)
        .git_ignore(true)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .count() as u64
}

//=============================================================================
// Phase 2: Basic Correlation Detection Benchmarks
//=============================================================================

/// Benchmark end-to-end correlation detection on various fixture sizes.
fn bench_correlation_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("tests-correlation");

    let fixtures = [
        ("small", "bench-tests-small"),
        ("medium", "bench-tests-medium"),
        ("large", "bench-tests-large"),
        ("worst-case", "bench-tests-worst-case"),
    ];

    for (name, fixture) in fixtures {
        let path = fixture_path(fixture);
        if !path.exists() {
            eprintln!("Skipping {name}: fixture not found at {path:?}");
            continue;
        }

        let file_count = count_files(&path);
        group.throughput(Throughput::Elements(file_count));

        // Create simulated file changes for the entire fixture
        let changes = generate_changes_for_fixture(&path);
        let config = rust_correlation_config();

        group.bench_with_input(BenchmarkId::new("detect", name), &changes, |b, changes| {
            b.iter(|| {
                black_box(analyze_correlation(changes, &config, &path));
            });
        });
    }

    group.finish();
}

/// Generate synthetic file changes for a fixture.
fn generate_changes_for_fixture(root: &Path) -> Vec<FileChange> {
    let mut changes = Vec::new();

    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker.filter_map(|e| e.ok()) {
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            let path = entry.path().to_path_buf();
            if path.extension().map(|e| e == "rs").unwrap_or(false) {
                changes.push(FileChange {
                    path,
                    change_type: ChangeType::Modified,
                    lines_added: 10,
                    lines_deleted: 5,
                });
            }
        }
    }

    changes
}

//=============================================================================
// Phase 3: Core Operations Benchmarks
//=============================================================================

/// Benchmark candidate_test_paths() for various source files.
fn bench_candidate_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("tests-candidate-paths");

    let base_names = [
        "lib",
        "parser",
        "lexer",
        "codegen",
        "deeply_nested_module_name",
        "very_long_module_name_that_might_stress_string_allocations",
    ];

    for base_name in base_names {
        group.bench_function(base_name, |b| {
            b.iter(|| black_box(candidate_test_paths(base_name)));
        });
    }

    group.finish();
}

/// Benchmark find_test_locations() for various source paths.
fn bench_find_test_locations(c: &mut Criterion) {
    let mut group = c.benchmark_group("tests-find-locations");

    let paths = [
        Path::new("src/lib.rs"),
        Path::new("src/parser.rs"),
        Path::new("src/checks/tests/correlation.rs"),
        Path::new("crates/cli/src/main.rs"),
        Path::new("deeply/nested/path/to/some/module.rs"),
    ];

    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap();
        group.bench_function(name, |b| {
            b.iter(|| black_box(find_test_locations(path)));
        });
    }

    group.finish();
}

/// Benchmark glob pattern matching for test file identification.
fn bench_glob_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("tests-glob-matching");

    // Test patterns from rust_correlation_config()
    let test_patterns = vec![
        "tests/**/*".to_string(),
        "test/**/*".to_string(),
        "**/*_test.*".to_string(),
        "**/*_tests.*".to_string(),
        "**/*.spec.*".to_string(),
    ];

    // Build glob set once
    let glob_set = build_glob_set(&test_patterns).expect("valid patterns");

    // Paths to test against
    let test_paths = [
        "tests/parser_tests.rs",
        "test/unit/lexer_test.rs",
        "src/parser_tests.rs",
        "src/deeply/nested/module_tests.rs",
        "src/lib.rs", // Should not match
        "crates/cli/tests/integration.rs",
    ];

    for path in test_paths {
        let path_obj = Path::new(path);
        group.bench_function(path, |b| {
            b.iter(|| black_box(glob_set.is_match(path_obj)));
        });
    }

    // Bulk matching benchmark
    let many_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("src/module{}_tests.rs", i)))
        .collect();

    group.bench_function("1000_paths", |b| {
        b.iter(|| {
            for path in &many_paths {
                black_box(glob_set.is_match(path));
            }
        });
    });

    group.finish();
}

/// Benchmark has_correlated_test() matching.
fn bench_has_correlated_test(c: &mut Criterion) {
    let mut group = c.benchmark_group("tests-has-correlated");

    // Simulate changed test files
    let test_changes: Vec<PathBuf> = (0..100)
        .map(|i| PathBuf::from(format!("tests/module{}_tests.rs", i)))
        .collect();

    // Pre-extract base names
    let test_base_names: Vec<String> = test_changes
        .iter()
        .filter_map(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.strip_suffix("_tests").unwrap_or(s).to_string())
        })
        .collect();

    // Source paths to check
    let source_paths = [
        Path::new("src/module50.rs"),  // Has matching test
        Path::new("src/module999.rs"), // No matching test
        Path::new("src/parser.rs"),    // No matching test
        Path::new("src/module0.rs"),   // Has matching test (first)
        Path::new("src/module99.rs"),  // Has matching test (last)
    ];

    for source in source_paths {
        let name = source.file_stem().unwrap().to_str().unwrap();
        group.bench_function(name, |b| {
            b.iter(|| {
                black_box(has_correlated_test(source, &test_changes, &test_base_names));
            });
        });
    }

    group.finish();
}

/// Benchmark inline test detection (parsing #[cfg(test)] blocks in diffs).
fn bench_inline_test_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("tests-inline-detection");

    let cfg_test_attr = "#[cfg(test)]";

    // Small diff with no test changes
    let small_no_tests = r#"
diff --git a/src/lib.rs b/src/lib.rs
index abc..def 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,5 +1,6 @@
 pub fn foo() {
+    println!("hello");
 }
"#;

    // Small diff with inline test changes
    let small_with_tests = format!(
        r#"
diff --git a/src/lib.rs b/src/lib.rs
index abc..def 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -10,5 +10,10 @@
 }}

 {}
 mod tests {{
+    #[test]
+    fn new_test() {{
+        assert!(true);
+    }}
 }}
"#,
        cfg_test_attr
    );

    // Large diff simulating extensive changes
    let large_diff: String = (0..500)
        .map(|i| {
            format!(
                "@@ -{},{} +{},{} @@\n pub fn func_{}() {{}}\n+    // Added line {}\n",
                i * 10,
                5,
                i * 10,
                6,
                i,
                i
            )
        })
        .collect();

    // Large diff with test block
    let large_with_tests = format!(
        "{}\n{}\nmod tests {{\n+    #[test]\n+    fn test() {{}}\n}}",
        large_diff, cfg_test_attr
    );

    group.bench_function("small_no_tests", |b| {
        b.iter(|| black_box(changes_in_cfg_test(small_no_tests)));
    });

    group.bench_function("small_with_tests", |b| {
        b.iter(|| black_box(changes_in_cfg_test(&small_with_tests)));
    });

    group.bench_function("large_no_tests", |b| {
        b.iter(|| black_box(changes_in_cfg_test(&large_diff)));
    });

    group.bench_function("large_with_tests", |b| {
        b.iter(|| black_box(changes_in_cfg_test(&large_with_tests)));
    });

    group.finish();
}

//=============================================================================
// Phase 4: End-to-End CLI Benchmarks
//=============================================================================

/// Benchmark full quench check flow with tests check.
fn bench_cli_check(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");

    let mut group = c.benchmark_group("tests-cli");
    group.sample_size(20); // Fewer samples for slower benchmarks

    let fixtures = [
        ("small", "bench-tests-small"),
        ("medium", "bench-tests-medium"),
        ("large", "bench-tests-large"),
        ("worst-case", "bench-tests-worst-case"),
    ];

    for (name, fixture) in fixtures {
        let path = fixture_path(fixture);
        if !path.exists() {
            eprintln!("Skipping CLI benchmark {name}: fixture not found");
            continue;
        }

        // Cold run (no cache, full check)
        group.bench_function(BenchmarkId::new("cold", name), |b| {
            b.iter(|| {
                let output = Command::new(quench_bin)
                    .arg("check")
                    .current_dir(&path)
                    .output()
                    .expect("quench should run");
                black_box(output)
            });
        });

        // Warm run (cache populated from previous runs)
        // Note: In practice cache behavior depends on file mtime
        group.bench_function(BenchmarkId::new("warm", name), |b| {
            // Pre-warm the cache
            let _ = Command::new(quench_bin)
                .arg("check")
                .current_dir(&path)
                .output();

            b.iter(|| {
                let output = Command::new(quench_bin)
                    .arg("check")
                    .current_dir(&path)
                    .output()
                    .expect("quench should run");
                black_box(output)
            });
        });
    }

    group.finish();
}

//=============================================================================
// Phase 5: Optimization Comparison Benchmarks
//=============================================================================

/// Benchmark TestIndex creation and lookup vs linear search.
fn bench_optimization_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("tests-optimization");

    // Generate test files for index
    let test_files: Vec<PathBuf> = (0..100)
        .map(|i| PathBuf::from(format!("tests/module{}_tests.rs", i)))
        .collect();

    // Pre-extract base names for the old linear approach
    let test_base_names: Vec<String> = test_files
        .iter()
        .filter_map(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.strip_suffix("_tests").unwrap_or(s).to_string())
        })
        .collect();

    // Benchmark: Index creation
    group.bench_function("index_creation", |b| {
        b.iter(|| black_box(TestIndex::new(&test_files)))
    });

    // Create index once for lookup benchmarks
    let index = TestIndex::new(&test_files);

    // Benchmark: Index lookup hit (middle of range)
    group.bench_function("index_lookup_hit", |b| {
        let source = Path::new("src/module50.rs");
        b.iter(|| black_box(index.has_test_for(source)))
    });

    // Benchmark: Index lookup miss
    group.bench_function("index_lookup_miss", |b| {
        let source = Path::new("src/nonexistent.rs");
        b.iter(|| black_box(index.has_test_for(source)))
    });

    // Benchmark: Old linear has_correlated_test (for comparison)
    group.bench_function("linear_lookup_hit", |b| {
        let source = Path::new("src/module50.rs");
        b.iter(|| black_box(has_correlated_test(source, &test_files, &test_base_names)))
    });

    group.bench_function("linear_lookup_miss", |b| {
        let source = Path::new("src/nonexistent.rs");
        b.iter(|| black_box(has_correlated_test(source, &test_files, &test_base_names)))
    });

    // Benchmark: Multiple lookups (realistic scenario)
    let source_files: Vec<PathBuf> = (0..50)
        .map(|i| PathBuf::from(format!("src/module{}.rs", i)))
        .collect();

    group.bench_function("index_50_lookups", |b| {
        b.iter(|| {
            for source in &source_files {
                black_box(index.has_test_for(source));
            }
        })
    });

    group.bench_function("linear_50_lookups", |b| {
        b.iter(|| {
            for source in &source_files {
                black_box(has_correlated_test(source, &test_files, &test_base_names));
            }
        })
    });

    group.finish();
}

/// Benchmark early termination paths.
fn bench_early_termination(c: &mut Criterion) {
    let mut group = c.benchmark_group("tests-early-termination");
    let config = rust_correlation_config();
    let root = Path::new("/project");

    // Empty changes (should be very fast)
    group.bench_function("empty_changes", |b| {
        let changes: Vec<FileChange> = vec![];
        b.iter(|| black_box(analyze_correlation(&changes, &config, root)))
    });

    // Single source file (uses fast path)
    group.bench_function("single_source", |b| {
        let changes = vec![FileChange {
            path: PathBuf::from("/project/src/parser.rs"),
            change_type: ChangeType::Modified,
            lines_added: 10,
            lines_deleted: 5,
        }];
        b.iter(|| black_box(analyze_correlation(&changes, &config, root)))
    });

    // Test-only changes (no source files to correlate)
    group.bench_function("test_only", |b| {
        let changes: Vec<FileChange> = (0..10)
            .map(|i| FileChange {
                path: PathBuf::from(format!("/project/tests/module{}_tests.rs", i)),
                change_type: ChangeType::Modified,
                lines_added: 10,
                lines_deleted: 5,
            })
            .collect();
        b.iter(|| black_box(analyze_correlation(&changes, &config, root)))
    });

    group.finish();
}

//=============================================================================
// Criterion Configuration
//=============================================================================

criterion_group!(
    benches,
    // Phase 2: Basic correlation
    bench_correlation_detection,
    // Phase 3: Core operations
    bench_candidate_paths,
    bench_find_test_locations,
    bench_glob_matching,
    bench_has_correlated_test,
    bench_inline_test_detection,
    // Phase 4: CLI
    bench_cli_check,
    // Phase 5: Optimization comparison
    bench_optimization_comparison,
    bench_early_termination,
);
criterion_main!(benches);
