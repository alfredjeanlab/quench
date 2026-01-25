// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Git check benchmarks.
//!
//! Measures performance of:
//! - Conventional commit parsing
//! - Agent documentation detection
//! - Git subprocess calls (commit fetching)
//! - End-to-end git check on various sizes

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::path::{Path, PathBuf};
use std::process::Command;

use quench::checks::git::parse::{ParseResult, parse_conventional_commit};
use quench::git::{get_all_branch_commits, get_commits_since};

/// Path to benchmark fixtures (tests/fixtures/bench-git-*)
fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(name)
}

// =============================================================================
// Phase 3: Commit Parsing Benchmarks
// =============================================================================

/// Benchmark conventional commit parsing.
///
/// Tests parse_conventional_commit() with various input types.
fn bench_commit_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("git_parsing");

    // Valid conventional commits of varying complexity
    let messages = [
        ("simple", "feat: add feature"),
        ("with_scope", "feat(api): add endpoint"),
        (
            "long_desc",
            "fix(core): resolve the issue with the parser that was causing problems in edge cases",
        ),
        ("breaking", "feat!: breaking change"),
        ("breaking_scope", "feat(api)!: breaking API change"),
    ];

    for (name, msg) in messages {
        group.bench_with_input(BenchmarkId::new("valid", name), &msg, |b, msg| {
            b.iter(|| black_box(parse_conventional_commit(msg)))
        });
    }

    // Invalid formats (should return NonConventional quickly)
    let invalid = [
        ("no_colon", "update stuff"),
        ("no_type", ": description"),
        ("empty", ""),
        ("whitespace_only", "   "),
    ];

    for (name, msg) in invalid {
        group.bench_with_input(BenchmarkId::new("invalid", name), &msg, |b, msg| {
            b.iter(|| black_box(parse_conventional_commit(msg)))
        });
    }

    // Edge cases
    let edge_cases = [
        (
            "long_message",
            "feat(core): this is a very long commit message that tests the parser with extended descriptions containing many words",
        ),
        ("unicode_scope", "feat(módulo): feature with unicode scope"),
        (
            "nested_scope",
            "fix(deeply/nested/scope): fix nested scope issue",
        ),
        ("minimal_type", "f: minimal"),
    ];

    for (name, msg) in edge_cases {
        group.bench_with_input(BenchmarkId::new("edge", name), &msg, |b, msg| {
            b.iter(|| black_box(parse_conventional_commit(msg)))
        });
    }

    group.finish();
}

/// Benchmark type and scope validation on parsed commits.
fn bench_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("git_validation");

    // Pre-parse commits for validation benchmarks
    let parsed = match parse_conventional_commit("feat(api): add endpoint") {
        ParseResult::Conventional(p) => p,
        _ => panic!("Expected valid commit"),
    };

    // Type validation
    let default_types: Option<&[String]> = None;
    group.bench_function("type_default", |b| {
        b.iter(|| black_box(parsed.is_type_allowed(default_types)))
    });

    let custom_types = vec!["feat".to_string(), "fix".to_string(), "chore".to_string()];
    group.bench_function("type_custom", |b| {
        b.iter(|| black_box(parsed.is_type_allowed(Some(&custom_types))))
    });

    // Scope validation
    let no_scopes: Option<&[String]> = None;
    group.bench_function("scope_none", |b| {
        b.iter(|| black_box(parsed.is_scope_allowed(no_scopes)))
    });

    let allowed_scopes = vec!["api".to_string(), "cli".to_string(), "core".to_string()];
    group.bench_function("scope_allowed", |b| {
        b.iter(|| black_box(parsed.is_scope_allowed(Some(&allowed_scopes))))
    });

    group.finish();
}

// =============================================================================
// Phase 4: Agent Docs Detection Benchmarks
// =============================================================================

/// Benchmark agent documentation detection.
///
/// Tests the regex-based detection of commit format documentation.
fn bench_docs_detection(c: &mut Criterion) {
    use quench::checks::git::docs::has_commit_documentation;

    let mut group = c.benchmark_group("git_docs");

    // Various CLAUDE.md content patterns
    let scenarios = [
        (
            "minimal",
            "# Project\n\n## Commits\n\nfeat: format\nfix: bugs",
        ),
        (
            "verbose",
            r#"# Project Documentation

This is a longer document with more content.

## Commits

Use conventional commit format: `type(scope): description`

Types:
- feat: A new feature
- fix: A bug fix
- chore: Maintenance tasks
- docs: Documentation only
- test: Adding tests
- refactor: Code restructuring
- perf: Performance improvements

Example: `feat(api): add user authentication endpoint`
"#,
        ),
        (
            "no_docs",
            "# Project\n\nThis project has no commit format documentation.\n\n## Setup\n\nRun npm install",
        ),
        (
            "conventional_phrase",
            "# Project\n\nWe use conventional commits for all changes.",
        ),
    ];

    for (name, content) in scenarios {
        group.bench_with_input(BenchmarkId::new("detect", name), &content, |b, content| {
            b.iter(|| black_box(has_commit_documentation(content)))
        });
    }

    group.finish();
}

// =============================================================================
// Phase 5: Git Subprocess Benchmarks
// =============================================================================

/// Benchmark git subprocess calls for commit retrieval.
///
/// These are I/O-bound operations and expected to dominate E2E time.
fn bench_git_subprocess(c: &mut Criterion) {
    let mut group = c.benchmark_group("git_subprocess");
    group.sample_size(20); // Fewer samples for I/O-heavy operations

    let fixtures = [
        ("small_10", "bench-git-small"),
        ("medium_50", "bench-git-medium"),
        ("large_500", "bench-git-large"),
    ];

    for (name, fixture) in fixtures {
        let path = fixture_path(fixture);
        if !path.exists() {
            eprintln!(
                "Skipping {name}: run ./scripts/fixtures/generate-bench-git to create fixtures"
            );
            continue;
        }

        // Benchmark get_commits_since (partial history)
        group.bench_function(BenchmarkId::new("commits_since_head5", name), |b| {
            b.iter(|| black_box(get_commits_since(&path, "HEAD~5")))
        });

        // Benchmark get_all_branch_commits (full branch history)
        group.bench_function(BenchmarkId::new("all_commits", name), |b| {
            b.iter(|| black_box(get_all_branch_commits(&path)))
        });
    }

    group.finish();
}

// =============================================================================
// Phase 2 & 6: End-to-End Benchmarks
// =============================================================================

/// End-to-end benchmarks for git check.
///
/// Tests full CLI invocation on benchmark fixtures.
fn bench_git_e2e(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let mut group = c.benchmark_group("git_e2e");
    group.sample_size(20);

    let fixtures = [
        ("small", "bench-git-small"),
        ("medium", "bench-git-medium"),
        ("large", "bench-git-large"),
        ("worst_case", "bench-git-worst-case"),
    ];

    for (name, fixture) in fixtures {
        let path = fixture_path(fixture);
        if !path.exists() {
            eprintln!(
                "Skipping {name}: run ./scripts/fixtures/generate-bench-git to create fixtures"
            );
            continue;
        }

        group.bench_function(name, |b| {
            b.iter(|| {
                let output = Command::new(quench_bin)
                    .args(["check", "--git", "--ci"])
                    .current_dir(&path)
                    .output()
                    .expect("quench should run");
                black_box(output)
            })
        });
    }

    group.finish();
}

/// Benchmark git check in warm mode (no CI, smaller scope).
fn bench_git_warm(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let mut group = c.benchmark_group("git_warm");
    group.sample_size(30);

    // Warm mode with a base branch specified (simulates local development)
    let path = fixture_path("bench-git-small");
    if path.exists() {
        group.bench_function("small_base", |b| {
            b.iter(|| {
                let output = Command::new(quench_bin)
                    .args(["check", "--git", "--base", "main"])
                    .current_dir(&path)
                    .output()
                    .expect("quench should run");
                black_box(output)
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_commit_parsing,
    bench_validation,
    bench_docs_detection,
    bench_git_subprocess,
    bench_git_e2e,
    bench_git_warm,
);
criterion_main!(benches);
