# Checkpoint 9D: Benchmark - Git Check

**Root Feature:** `quench-2744`

## Overview

Create performance benchmarks for the git check to establish baselines, validate against performance targets, and enable regression tracking. This follows the established benchmark checkpoint pattern (7D docs, 8D tests) with Criterion-based benchmarks and stress test fixtures.

The git check validates commit message format (conventional commits), checks for documentation in agent files, and creates `.gitmessage` templates. Key operations to benchmark include commit parsing, agent docs detection, and git subprocess calls.

## Project Structure

```
quench/
├── crates/cli/
│   ├── benches/
│   │   └── git.rs                 # NEW: Criterion benchmarks
│   └── src/checks/git/
│       ├── mod.rs                 # Existing: GitCheck implementation
│       └── parse.rs               # Existing: Conventional commit parsing
├── tests/fixtures/
│   ├── bench-git-small/           # NEW: 10 commits, basic validation
│   ├── bench-git-medium/          # NEW: 50 commits, mixed scenarios
│   ├── bench-git-large/           # NEW: 500 commits, stress test
│   └── bench-git-worst-case/      # NEW: Pathological patterns
├── scripts/fixtures/
│   └── generate-bench-git         # NEW: Fixture generation script
└── reports/
    └── checkpoint-9d-benchmarks.md  # NEW: Results analysis
```

## Dependencies

Already available in the project:
- `criterion = "0.5"` - Benchmarking framework (dev-dependency)
- Git CLI - Commit operations and subprocess calls

No new dependencies required.

## Implementation Phases

### Phase 1: Create Benchmark Fixtures

Create git repositories with varying commit histories for reproducible benchmarks.

**Files:**
- `tests/fixtures/bench-git-small/` - 10 commits with valid format
- `tests/fixtures/bench-git-medium/` - 50 commits with mixed validity
- `tests/fixtures/bench-git-large/` - 500 commits for stress testing
- `tests/fixtures/bench-git-worst-case/` - Edge cases (long messages, unicode scopes)

**Fixture generation script:** `scripts/fixtures/generate-bench-git`

```bash
#!/usr/bin/env bash
set -euo pipefail

# Generate bench-git-small
create_small_fixture() {
    local dir="tests/fixtures/bench-git-small"
    rm -rf "$dir"
    mkdir -p "$dir"
    cd "$dir"

    git init
    echo 'version = 1' > quench.toml
    echo -e '[git.commit]\ncheck = "error"\nagents = false' >> quench.toml
    echo '# Bench Git Small' > CLAUDE.md

    git add -A
    git commit -m "chore: initial commit"

    for i in {1..10}; do
        echo "// file $i" >> src/lib.rs
        git add -A
        git commit -m "feat(core): add feature $i"
    done
}

# Similar for medium (50 commits) and large (500 commits)
```

**Verification:** Fixtures exist and `quench check --git --ci` runs without errors on each.

### Phase 2: Add Criterion Benchmark Harness

Create the benchmark file with basic setup and registration.

**File:** `crates/cli/benches/git.rs`

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Git check benchmarks.
//!
//! Measures performance of:
//! - Conventional commit parsing
//! - Agent documentation detection
//! - Template generation
//! - Git subprocess calls (commit fetching)
//! - End-to-end git check on various sizes

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(name)
}

fn bench_git_e2e(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let mut group = c.benchmark_group("git_e2e");
    group.sample_size(20);

    for (name, fixture) in [
        ("small", "bench-git-small"),
        ("medium", "bench-git-medium"),
        ("large", "bench-git-large"),
        ("worst-case", "bench-git-worst-case"),
    ] {
        let path = fixture_path(fixture);
        if !path.exists() {
            eprintln!("Skipping {name}: run ./scripts/fixtures/generate-bench-git");
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

criterion_group!(benches, bench_git_e2e);
criterion_main!(benches);
```

**Update:** Add to `crates/cli/Cargo.toml`:
```toml
[[bench]]
name = "git"
harness = false
```

**Verification:** `cargo bench --bench git -- --list` shows benchmarks.

### Phase 3: Benchmark Core Parsing Operations

Add benchmarks for individual parsing operations to identify bottlenecks.

**Operations to benchmark:**

1. **Conventional commit parsing** - `parse_conventional_commit()` for various inputs
2. **Type validation** - Checking against allowed types list
3. **Scope validation** - Checking against allowed scopes list

```rust
use quench::checks::git::parse::{parse_conventional_commit, ParseResult};

fn bench_commit_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("git_parsing");

    // Valid conventional commits of varying complexity
    let messages = [
        ("simple", "feat: add feature"),
        ("with_scope", "feat(api): add endpoint"),
        ("long_desc", "fix(core): resolve the issue with the parser that was causing problems in edge cases"),
        ("breaking", "feat!: breaking change"),
        ("breaking_scope", "feat(api)!: breaking API change"),
    ];

    for (name, msg) in messages {
        group.bench_with_input(
            BenchmarkId::new("parse", name),
            &msg,
            |b, msg| b.iter(|| black_box(parse_conventional_commit(msg))),
        );
    }

    // Invalid formats (should return NonConventional quickly)
    let invalid = [
        ("no_colon", "update stuff"),
        ("no_type", ": description"),
        ("empty", ""),
    ];

    for (name, msg) in invalid {
        group.bench_with_input(
            BenchmarkId::new("parse_invalid", name),
            &msg,
            |b, msg| b.iter(|| black_box(parse_conventional_commit(msg))),
        );
    }

    group.finish();
}
```

**Verification:** Parsing benchmarks run and show sub-microsecond times.

### Phase 4: Benchmark Agent Docs Detection

Add benchmarks for agent file scanning and commit format detection.

```rust
use quench::checks::git::docs::{check_commit_docs, DocsResult};

fn bench_docs_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("git_docs");

    // Create temp directories with various CLAUDE.md content sizes
    let scenarios = [
        ("minimal", "# Project\n\n## Commits\n\nfeat: format"),
        ("verbose", include_str!("../../tests/fixtures/claude-md-large.md")),
        ("no_docs", "# Project\n\nNo commit format here."),
    ];

    for (name, content) in scenarios {
        // Setup temp dir with content
        group.bench_function(name, |b| {
            let temp = tempfile::tempdir().unwrap();
            std::fs::write(temp.path().join("CLAUDE.md"), content).unwrap();

            b.iter(|| black_box(check_commit_docs(temp.path())))
        });
    }

    group.finish();
}
```

**Verification:** Docs detection benchmarks complete without errors.

### Phase 5: Benchmark Git Subprocess Calls

Add benchmarks for git operations (the likely bottleneck).

```rust
use quench::git::{get_commits_since, get_all_branch_commits};

fn bench_git_subprocess(c: &mut Criterion) {
    let mut group = c.benchmark_group("git_subprocess");
    group.sample_size(20); // Fewer samples for I/O-heavy operations

    for (name, fixture) in [
        ("small_10", "bench-git-small"),
        ("medium_50", "bench-git-medium"),
        ("large_500", "bench-git-large"),
    ] {
        let path = fixture_path(fixture);
        if !path.exists() {
            continue;
        }

        // Benchmark get_commits_since
        group.bench_function(BenchmarkId::new("commits_since", name), |b| {
            b.iter(|| black_box(get_commits_since(&path, "HEAD~5").unwrap()))
        });

        // Benchmark get_all_branch_commits
        group.bench_function(BenchmarkId::new("all_commits", name), |b| {
            b.iter(|| black_box(get_all_branch_commits(&path).unwrap()))
        });
    }

    group.finish();
}
```

**Verification:** Git subprocess benchmarks show timing breakdown.

### Phase 6: Validate Performance and Document Results

Run benchmarks and compare against targets from `docs/specs/20-performance.md`:
- Fast check (warm): < 100ms
- CI check: < 5s

**File:** `reports/checkpoint-9d-benchmarks.md`

```markdown
# Benchmark Results: Git Check

## Summary

| Fixture | Commits | E2E (ms) | Target | Status |
|---------|---------|----------|--------|--------|
| small   | 10      | XX       | <100ms | PASS   |
| medium  | 50      | XX       | <100ms | PASS   |
| large   | 500     | XX       | <500ms | PASS   |

## Component Breakdown

### Commit Parsing
| Input Type | Time (ns) | Notes |
|------------|-----------|-------|
| simple     | XX        | Baseline |
| with_scope | XX        | +scope parsing |
| long_desc  | XX        | Long description |

### Git Subprocess Calls
| Fixture | get_commits_since (ms) | get_all_branch_commits (ms) |
|---------|------------------------|----------------------------|
| small   | XX                     | XX                         |
| medium  | XX                     | XX                         |
| large   | XX                     | XX                         |

## Bottleneck Analysis

1. **Git subprocess calls** dominate (~90%+ of E2E time)
2. **Commit parsing** is negligible (<1μs per message)
3. **Docs detection** is fast (<1ms for typical files)

## Recommendations

- Caching parsed commits would help repeated runs
- Consider git2 library for direct access (avoid subprocess overhead)
```

**Verification:** Report complete with all metrics captured.

## Key Implementation Details

### Fixture Generation Strategy

Fixtures simulate realistic git histories:
- **Small:** Single branch, 10 well-formed commits
- **Medium:** 50 commits with mix of types (feat, fix, chore)
- **Large:** 500 commits simulating a mature project
- **Worst-case:** Long messages, unicode in scopes, edge cases

### Benchmark Isolation

Each benchmark run should:
1. Use separate git repositories per fixture
2. Avoid modifying the repository state during benchmarks
3. Use `sample_size` appropriate for I/O-bound operations
4. Run git operations against committed state (not working tree)

### Expected Bottleneck

Based on the implementation, git subprocess calls are expected to dominate:
- `git log` spawns a subprocess per call
- Parsing output is fast (regex matching)
- Agent docs scanning is file I/O only

This benchmark suite will validate this assumption and provide data for optimization decisions (e.g., git2 library vs subprocess).

## Verification Plan

1. **Phase 1:** Run `ls tests/fixtures/bench-git-*` to confirm fixture creation
2. **Phase 2:** Run `cargo bench --bench git -- --list` to verify benchmark registration
3. **Phase 3:** Run `cargo bench --bench git -- bench_commit_parsing` to test parsing ops
4. **Phase 4:** Run `cargo bench --bench git -- bench_docs_detection` to test docs scanning
5. **Phase 5:** Run `cargo bench --bench git -- bench_git_subprocess` to test git calls
6. **Phase 6:** Review `reports/checkpoint-9d-benchmarks.md` for completeness

**Final verification:**
```bash
cargo bench --bench git
# Review output, all benchmarks complete without error
# Check reports/checkpoint-9d-benchmarks.md has baseline metrics
make check  # Ensure CI compatibility
```
