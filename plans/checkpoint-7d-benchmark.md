# Checkpoint 7D: Benchmark - Docs Check

**Root Feature:** `quench-0862`

## Overview

Add comprehensive benchmarks for the docs check to measure performance of TOC validation, link validation, specs validation, and commit checking. The benchmarks follow the existing Criterion-based infrastructure and test both unit-level parsing functions and end-to-end CLI invocations.

**Performance Targets** (from `docs/specs/20-performance.md`):
| Mode | Target | Acceptable | Unacceptable |
|------|--------|------------|--------------|
| Fast checks (cold) | < 500ms | < 1s | > 2s |
| Fast checks (warm) | < 100ms | < 200ms | > 500ms |
| CI checks | < 5s | < 15s | > 30s |

## Project Structure

```
crates/cli/
├── benches/
│   └── docs.rs               # NEW: Docs check benchmarks
└── Cargo.toml                # Add bench entry

tests/fixtures/
├── docs/                     # Existing (small fixtures)
└── stress-docs/              # NEW: Large fixtures for benchmarks
    ├── many-files/           # 500 markdown files
    ├── large-toc/            # Complex nested TOC structures
    ├── deep-links/           # Deep link chains for BFS traversal
    └── many-links/           # Single file with 500+ links
```

## Dependencies

No new dependencies required. Uses existing:
- `criterion = "0.5"` (already in dev-dependencies)
- `quench` crate internals for unit benchmarks

## Implementation Phases

### Phase 1: Benchmark Infrastructure Setup

Create the benchmark file with basic structure and helper functions.

**File: `crates/cli/benches/docs.rs`**

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Docs check benchmarks.
//!
//! Measures performance of:
//! - TOC parsing and validation
//! - Link extraction and resolution
//! - Specs validation (TOC and Linked modes)
//! - End-to-end docs check on various sizes

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
        .join("tests/fixtures/docs")
        .join(name)
}

fn stress_fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/stress-docs")
        .join(name)
}
```

**Update: `crates/cli/Cargo.toml`**

Add bench entry:
```toml
[[bench]]
name = "docs"
harness = false
```

**Verification**: `cargo bench --bench docs -- --list` shows benchmark functions.

---

### Phase 2: TOC Parsing Benchmarks

Benchmark TOC block detection, parsing, and path resolution.

**Benchmarks:**
1. `toc_detect` - Detecting directory trees (box-drawing vs indentation)
2. `toc_parse` - Parsing tree structure into paths
3. `toc_resolve` - Path resolution strategies (relative, root, strip-parent)

```rust
use quench::checks::docs::toc::{detect_tree_format, parse_tree_block, resolve_paths};

fn bench_toc_detect(c: &mut Criterion) {
    let mut group = c.benchmark_group("docs_toc_detect");

    let box_tree = r#"```
src/
├── main.rs
├── lib.rs
└── utils/
    ├── helpers.rs
    └── mod.rs
```"#;

    let indent_tree = r#"```
src/
  main.rs
  lib.rs
  utils/
    helpers.rs
    mod.rs
```"#;

    group.bench_function("box_drawing", |b| {
        b.iter(|| black_box(detect_tree_format(box_tree)))
    });

    group.bench_function("indentation", |b| {
        b.iter(|| black_box(detect_tree_format(indent_tree)))
    });

    group.finish();
}

fn bench_toc_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("docs_toc_parse");

    // Generate trees of varying depth
    for depth in [5, 10, 20] {
        let tree = generate_nested_tree(depth);
        group.bench_with_input(
            BenchmarkId::new("depth", depth),
            &tree,
            |b, tree| b.iter(|| black_box(parse_tree_block(tree))),
        );
    }

    // Generate trees of varying width
    for width in [10, 50, 100] {
        let tree = generate_wide_tree(width);
        group.bench_with_input(
            BenchmarkId::new("width", width),
            &tree,
            |b, tree| b.iter(|| black_box(parse_tree_block(tree))),
        );
    }

    group.finish();
}
```

**Verification**: Run `cargo bench --bench docs toc` and confirm sub-millisecond parsing.

---

### Phase 3: Link Validation Benchmarks

Benchmark markdown link extraction and path resolution.

**Benchmarks:**
1. `links_extract` - Regex-based link extraction
2. `links_resolve` - Path resolution for links
3. `links_many` - File with many links (stress test)

```rust
use quench::checks::docs::links::{extract_links, resolve_link};

fn bench_links_extract(c: &mut Criterion) {
    let mut group = c.benchmark_group("docs_links_extract");

    // Content with varying link densities
    for links in [10, 50, 200] {
        let content = generate_content_with_links(links);
        group.bench_with_input(
            BenchmarkId::new("count", links),
            &content,
            |b, content| b.iter(|| black_box(extract_links(content))),
        );
    }

    group.finish();
}

fn bench_links_resolve(c: &mut Criterion) {
    let mut group = c.benchmark_group("docs_links_resolve");
    let fixture = fixture_path("link-ok");

    // Relative links
    group.bench_function("relative", |b| {
        b.iter(|| black_box(resolve_link(&fixture, "../other.md")))
    });

    // Absolute-style links
    group.bench_function("from_root", |b| {
        b.iter(|| black_box(resolve_link(&fixture, "docs/specs/overview.md")))
    });

    group.finish();
}
```

**Verification**: Run `cargo bench --bench docs links` and confirm link extraction scales linearly.

---

### Phase 4: Specs Validation Benchmarks

Benchmark specs index detection and reachability checking.

**Benchmarks:**
1. `specs_index_detect` - Finding index file (CLAUDE.md, overview.md, etc.)
2. `specs_toc_mode` - TOC-based reachability
3. `specs_linked_mode` - BFS link traversal
4. `specs_sections` - Section presence validation

```rust
use quench::checks::docs::specs::{detect_index, validate_toc_mode, validate_linked_mode};

fn bench_specs_index_detect(c: &mut Criterion) {
    let mut group = c.benchmark_group("docs_specs_index");

    let fixtures = ["index-toc", "index-linked", "index-auto"];
    for fixture in fixtures {
        let path = fixture_path(fixture);
        if path.exists() {
            group.bench_function(fixture, |b| {
                b.iter(|| black_box(detect_index(&path)))
            });
        }
    }

    group.finish();
}

fn bench_specs_validation_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("docs_specs_mode");
    group.sample_size(50);

    // TOC mode on varying spec counts
    let toc_fixture = stress_fixture_path("many-files");
    if toc_fixture.exists() {
        group.bench_function("toc_500_files", |b| {
            b.iter(|| black_box(validate_toc_mode(&toc_fixture)))
        });
    }

    // Linked mode on varying link depths
    let linked_fixture = stress_fixture_path("deep-links");
    if linked_fixture.exists() {
        group.bench_function("linked_deep", |b| {
            b.iter(|| black_box(validate_linked_mode(&linked_fixture)))
        });
    }

    group.finish();
}
```

**Verification**: Run `cargo bench --bench docs specs` and verify BFS traversal performance.

---

### Phase 5: Stress Test Fixtures

Create large fixtures for stress benchmarks.

**Fixture: `tests/fixtures/stress-docs/many-files/`**

500 markdown files with interconnected links:
```
stress-docs/many-files/
├── quench.toml
├── docs/
│   └── specs/
│       ├── CLAUDE.md           # Index with TOC listing all 500 files
│       ├── 001-overview.md
│       ├── 002-cli.md
│       ...
│       └── 500-appendix.md
```

**Fixture: `tests/fixtures/stress-docs/deep-links/`**

Deep link chain (50 levels) for BFS stress:
```
stress-docs/deep-links/
├── quench.toml
├── docs/
│   └── specs/
│       ├── CLAUDE.md → 01-level.md
│       ├── 01-level.md → 02-level.md
│       ...
│       └── 50-level.md
```

**Fixture: `tests/fixtures/stress-docs/large-toc/`**

Single file with 100+ entry nested TOC:
```
stress-docs/large-toc/
├── quench.toml
├── CLAUDE.md                   # Contains 100-entry nested tree
└── src/
    └── [100 files matching TOC]
```

**Generation script: `scripts/fixtures/generate-docs-stress`**

```bash
#!/usr/bin/env bash
# Generate stress fixtures for docs benchmarks
set -euo pipefail

FIXTURES_DIR="tests/fixtures/stress-docs"
mkdir -p "$FIXTURES_DIR"

# many-files: 500 markdown specs
mkdir -p "$FIXTURES_DIR/many-files/docs/specs"
echo '[check.docs]' > "$FIXTURES_DIR/many-files/quench.toml"
# ... generate 500 files and index

# deep-links: 50-level link chain
mkdir -p "$FIXTURES_DIR/deep-links/docs/specs"
# ... generate chain

# large-toc: 100-entry TOC
mkdir -p "$FIXTURES_DIR/large-toc/src"
# ... generate tree
```

**Verification**: Fixtures exist and benchmarks use them when available.

---

### Phase 6: End-to-End Benchmarks

Benchmark full CLI invocation on docs-heavy projects.

```rust
fn bench_docs_e2e(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let mut group = c.benchmark_group("docs_e2e");
    group.sample_size(20);

    // Small fixture (existing)
    let small = fixture_path("toc-ok");
    if small.exists() {
        group.bench_function("small_toc", |b| {
            b.iter(|| {
                Command::new(quench_bin)
                    .args(["check", "--docs"])
                    .current_dir(&small)
                    .output()
                    .expect("quench should run")
            })
        });
    }

    // Stress fixtures
    let fixtures = [
        ("many-files", "500 spec files"),
        ("deep-links", "50-level link chain"),
        ("large-toc", "100-entry TOC"),
    ];

    for (name, description) in fixtures {
        let path = stress_fixture_path(name);
        if !path.exists() {
            eprintln!("Skipping {name} ({description}): run scripts/fixtures/generate-docs-stress");
            continue;
        }

        group.bench_function(name, |b| {
            b.iter(|| {
                Command::new(quench_bin)
                    .args(["check", "--docs"])
                    .current_dir(&path)
                    .output()
                    .expect("quench should run")
            })
        });
    }

    group.finish();
}

fn bench_docs_ci_mode(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let mut group = c.benchmark_group("docs_ci");
    group.sample_size(10);

    // CI mode includes commit checking (git subprocess)
    let project = fixture_path("docs-project");
    if project.exists() {
        group.bench_function("with_commit_check", |b| {
            b.iter(|| {
                Command::new(quench_bin)
                    .args(["check", "--ci", "--docs"])
                    .current_dir(&project)
                    .output()
                    .expect("quench should run")
            })
        });
    }

    group.finish();
}
```

**Verification**: `cargo bench --bench docs e2e` completes within acceptable thresholds.

## Key Implementation Details

### Benchmark Organization

Follow the existing pattern from `benches/stress.rs`:
- Use `criterion_group!` to organize related benchmarks
- Use `BenchmarkId::new(group, variant)` for parameterized benchmarks
- Set `sample_size` appropriately (lower for slow benchmarks)
- Skip gracefully when fixtures don't exist

### Exposing Internal Functions

The docs check internal functions (`toc::parse`, `links::extract`, etc.) are currently private. Two approaches:

**Option A (Preferred)**: Use `#[doc(hidden)]` + `pub` for benchmark access
```rust
// In crates/cli/src/checks/docs/toc/mod.rs
#[doc(hidden)]
pub use parse::parse_tree_block;
```

**Option B**: Test via public API only (end-to-end benchmarks)

### Generated Content Helpers

```rust
/// Generate a nested directory tree of specified depth.
fn generate_nested_tree(depth: usize) -> String {
    let mut lines = vec!["```".to_string(), "src/".to_string()];
    let mut indent = "├── ";
    for level in 1..=depth {
        lines.push(format!("{}level_{}/", "│   ".repeat(level - 1) + indent, level));
        if level == depth {
            lines.push(format!("{}mod.rs", "│   ".repeat(level)));
        }
    }
    lines.push("```".to_string());
    lines.join("\n")
}

/// Generate markdown content with N links.
fn generate_content_with_links(count: usize) -> String {
    (0..count)
        .map(|i| format!("See [file {}](path/to/file_{}.md) for details.\n", i, i))
        .collect()
}
```

### Performance Assertions (Optional)

For CI regression detection:
```rust
// In tests/specs/docs_perf.rs
#[test]
#[ignore = "benchmark, not unit test"]
fn docs_check_under_500ms() {
    let start = std::time::Instant::now();
    // Run docs check on medium fixture
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "Docs check took {}ms", elapsed.as_millis());
}
```

## Verification Plan

### Phase 1 Verification
```bash
# Verify benchmark file compiles
cargo build --bench docs

# Verify benchmark is registered
cargo bench --bench docs -- --list
```

### Phase 2-4 Verification
```bash
# Run specific benchmark groups
cargo bench --bench docs toc
cargo bench --bench docs links
cargo bench --bench docs specs

# Check for sub-millisecond unit parsing
# Expect: toc_parse/depth_20 < 1ms
```

### Phase 5 Verification
```bash
# Generate stress fixtures
./scripts/fixtures/generate-docs-stress

# Verify fixtures exist
ls tests/fixtures/stress-docs/
```

### Phase 6 Verification
```bash
# Run end-to-end benchmarks
cargo bench --bench docs e2e

# Verify against performance targets
# many-files should complete in < 500ms (fast mode)
```

### Full Verification
```bash
# Run all docs benchmarks
cargo bench --bench docs

# Compare against baseline (if exists)
cargo bench --bench docs -- --baseline main

# Generate HTML report
cargo bench --bench docs -- --save-baseline feature/docs-bench
```

## Success Criteria

1. All benchmark groups execute without errors
2. TOC parsing benchmarks show linear scaling with tree size
3. Link extraction benchmarks show linear scaling with link count
4. Specs validation (linked mode) handles 50-level depth without degradation
5. End-to-end benchmarks complete within performance targets:
   - Small fixtures: < 100ms
   - 500-file fixture: < 500ms (fast mode target)
   - CI mode with commit checking: < 5s
6. Criterion HTML reports generated in `target/criterion/`
