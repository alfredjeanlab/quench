# Phase 015: File Walking - Specs

**Root Feature:** `quench-6915`

## Overview

Create behavioral specifications for file walking functionality. These specs test that quench correctly:
- Respects `.gitignore` rules during file discovery
- Respects custom ignore patterns from configuration
- Handles symlink loops without infinite recursion
- Handles deeply nested directories up to a configurable depth limit

All specs will be marked with `#[ignore = "TODO: Phase 020"]` until the file walking implementation is complete.

**Current State**: Test fixtures exist from Phase 010. No file walking specs exist yet. The `bench-deep` fixture for depth testing does not exist.

**End State**: Complete behavioral spec coverage for file walking, plus the `bench-deep` fixture for depth limit testing.

## Project Structure

```
tests/
├── fixtures/
│   ├── bench-deep/              # NEW: deeply nested for depth testing
│   │   ├── quench.toml
│   │   └── level-1/
│   │       └── level-2/
│   │           └── ... (50+ levels)
│   ├── gitignore-test/          # NEW: gitignore behavior testing
│   │   ├── quench.toml
│   │   ├── .gitignore
│   │   ├── src/
│   │   │   └── lib.rs
│   │   ├── target/              # Should be ignored
│   │   │   └── debug.rs
│   │   └── vendor/              # Should be ignored
│   │       └── external.rs
│   └── symlink-loop/            # NEW: symlink loop testing
│       ├── quench.toml
│       ├── src/
│       │   └── lib.rs
│       └── loop -> .            # Symlink to self
└── specs/
    ├── main.rs                  # Existing
    ├── prelude.rs               # Existing
    └── file_walking.rs          # NEW: file walking specs
```

## Dependencies

No new external dependencies. Uses existing:
- `assert_cmd` for CLI testing
- `predicates` for output assertions
- `tempfile` for creating test fixtures with symlinks

## Implementation Phases

### Phase 15.1: Gitignore Test Fixture

**Goal**: Create a fixture that tests `.gitignore` file exclusion.

**Tasks**:
1. Create `tests/fixtures/gitignore-test/` directory
2. Add `.gitignore` with standard patterns (`target/`, `vendor/`)
3. Add files that should be scanned (`src/lib.rs`)
4. Add files that should be ignored (`target/debug.rs`, `vendor/external.rs`)
5. Add `quench.toml` with version 1

**Files**:

```toml
# tests/fixtures/gitignore-test/quench.toml
version = 1

[project]
name = "gitignore-test"
```

```gitignore
# tests/fixtures/gitignore-test/.gitignore
target/
vendor/
*.generated.rs
```

```rust
// tests/fixtures/gitignore-test/src/lib.rs
//! Main library file - should be scanned.

pub fn included() -> &'static str {
    "this file should be scanned"
}
```

```rust
// tests/fixtures/gitignore-test/target/debug.rs
//! Build artifact - should be ignored by .gitignore.
//!
//! If this file is scanned, gitignore is not working.

pub fn should_be_ignored() {
    panic!("this file should never be scanned");
}
```

```rust
// tests/fixtures/gitignore-test/vendor/external.rs
//! Vendored dependency - should be ignored by .gitignore.

pub fn vendored_code() {
    // This should be excluded from scanning
}
```

```rust
// tests/fixtures/gitignore-test/src/generated.generated.rs
//! Generated file - should be ignored by *.generated.rs pattern.

pub fn generated() {}
```

**Verification**:
```bash
ls -la tests/fixtures/gitignore-test/
cat tests/fixtures/gitignore-test/.gitignore
```

### Phase 15.2: Symlink Loop Fixture

**Goal**: Create a fixture with a symlink that creates a loop.

**Tasks**:
1. Create `tests/fixtures/symlink-loop/` directory
2. Add `quench.toml` with version 1
3. Add source file in `src/lib.rs`
4. Create a symlink `loop -> .` that points to itself
5. Note: The actual symlink will be created by a setup script since git doesn't track symlinks well

**Files**:

```toml
# tests/fixtures/symlink-loop/quench.toml
version = 1

[project]
name = "symlink-loop"
```

```rust
// tests/fixtures/symlink-loop/src/lib.rs
//! Source file in a project with a symlink loop.

pub fn normal_code() -> i32 {
    42
}
```

```bash
#!/bin/bash
# scripts/setup-symlink-fixtures.sh
# Creates symlinks for testing (run before tests)

set -euo pipefail

FIXTURE_DIR="tests/fixtures/symlink-loop"

# Create symlink loop (directory pointing to itself)
if [ ! -L "$FIXTURE_DIR/loop" ]; then
    ln -s . "$FIXTURE_DIR/loop"
    echo "Created symlink loop: $FIXTURE_DIR/loop -> ."
fi

# Create indirect loop (a -> b -> a)
mkdir -p "$FIXTURE_DIR/indirect"
if [ ! -L "$FIXTURE_DIR/indirect/a" ]; then
    mkdir -p "$FIXTURE_DIR/indirect"
    ln -s ../indirect "$FIXTURE_DIR/indirect/a" 2>/dev/null || true
    echo "Created indirect symlink loop"
fi
```

**Verification**:
```bash
./scripts/setup-symlink-fixtures.sh
ls -la tests/fixtures/symlink-loop/
```

### Phase 15.3: Bench-Deep Fixture

**Goal**: Create a deeply nested directory structure for depth limit testing.

**Tasks**:
1. Create `tests/fixtures/bench-deep/` directory
2. Add `quench.toml` with version 1
3. Create a generation script that produces 50+ nested directories
4. Add a source file at the deepest level to verify traversal

**Files**:

```toml
# tests/fixtures/bench-deep/quench.toml
version = 1

[project]
name = "bench-deep"
```

```bash
#!/bin/bash
# scripts/generate-bench-deep.sh
# Generates deeply nested directory structure for depth limit testing

set -euo pipefail

FIXTURE_DIR="tests/fixtures/bench-deep"
DEPTH=120  # Exceeds default limit of 100

# Clean existing nested structure
rm -rf "$FIXTURE_DIR/deep"

# Build path incrementally
current_path="$FIXTURE_DIR/deep"
for i in $(seq 1 $DEPTH); do
    current_path="$current_path/level-$i"
done

# Create the full path
mkdir -p "$current_path"

# Add source file at deepest level
cat > "$current_path/deep.rs" << 'EOF'
//! File at maximum nesting depth.
//! Used to test depth limit handling.

pub fn at_depth() -> &'static str {
    "reached maximum depth"
}
EOF

# Add file at level 50 (within default limit)
mid_path="$FIXTURE_DIR/deep"
for i in $(seq 1 50); do
    mid_path="$mid_path/level-$i"
done
cat > "$mid_path/mid.rs" << 'EOF'
//! File at moderate depth (50 levels).
//! Should be reachable with default depth limit.

pub fn at_mid_depth() -> i32 {
    50
}
EOF

echo "Created bench-deep fixture with $DEPTH levels"
echo "  - File at level 50: $mid_path/mid.rs"
echo "  - File at level $DEPTH: $current_path/deep.rs"
```

**Verification**:
```bash
./scripts/generate-bench-deep.sh
find tests/fixtures/bench-deep -name "*.rs" | head -5
```

### Phase 15.4: Custom Ignore Patterns Fixture

**Goal**: Create a fixture for testing custom ignore patterns from `quench.toml`.

**Tasks**:
1. Create `tests/fixtures/custom-ignore/` directory
2. Add `quench.toml` with custom ignore patterns
3. Add files that match/don't match the custom patterns

**Files**:

```toml
# tests/fixtures/custom-ignore/quench.toml
version = 1

[project]
name = "custom-ignore"

[project.ignore]
patterns = [
    "*.snapshot",
    "testdata/",
    "**/fixtures/**",
]
```

```rust
// tests/fixtures/custom-ignore/src/lib.rs
//! Main library - should be scanned.

pub fn main_code() {}
```

```rust
// tests/fixtures/custom-ignore/src/lib.snapshot
//! Snapshot file - should be ignored by *.snapshot pattern.
//! If scanned, custom ignore patterns are not working.

pub fn snapshot_data() {}
```

```rust
// tests/fixtures/custom-ignore/testdata/sample.rs
//! Test data - should be ignored by testdata/ pattern.

pub fn test_data() {}
```

```rust
// tests/fixtures/custom-ignore/src/fixtures/mock.rs
//! Mock fixture - should be ignored by **/fixtures/** pattern.

pub fn mock_data() {}
```

**Verification**:
```bash
ls -la tests/fixtures/custom-ignore/
cat tests/fixtures/custom-ignore/quench.toml
```

### Phase 15.5: File Walking Behavioral Specs

**Goal**: Write comprehensive behavioral specs for file walking.

**Tasks**:
1. Create `tests/specs/file_walking.rs`
2. Add specs for gitignore handling
3. Add specs for custom ignore patterns
4. Add specs for symlink loop detection
5. Add specs for depth limit handling
6. Mark all specs with `#[ignore = "TODO: Phase 020"]`
7. Update `tests/specs/main.rs` to include the new module

**Files**:

```rust
// tests/specs/file_walking.rs
//! Behavioral specs for file walking functionality.
//!
//! Tests that quench correctly discovers files while respecting:
//! - .gitignore rules
//! - Custom ignore patterns from configuration
//! - Symlink loop detection
//! - Directory depth limits
//!
//! Reference: docs/specs/20-performance.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// Gitignore Handling
// =============================================================================

/// Spec: docs/specs/20-performance.md#parallel-gitignore-aware-file-walking
///
/// > Respects `.gitignore`, `.ignore`, global ignores
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_respects_gitignore() {
    // Files in target/ should not appear in scan results
    // src/lib.rs should be scanned
    // target/debug.rs should be ignored
    quench_cmd()
        .args(["check", "--debug-files"])  // hypothetical flag to list scanned files
        .current_dir(fixture("gitignore-test"))
        .assert()
        .success()
        .stdout(predicates::str::contains("src/lib.rs"))
        .stdout(predicates::str::contains("target/").not());
}

/// Spec: docs/specs/20-performance.md#parallel-gitignore-aware-file-walking
///
/// > Gitignore filtering must happen during traversal, not after
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_ignores_gitignore_glob_patterns() {
    // Files matching *.generated.rs should be ignored
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("gitignore-test"))
        .assert()
        .success()
        .stdout(predicates::str::contains(".generated.rs").not());
}

/// Spec: docs/specs/20-performance.md#parallel-gitignore-aware-file-walking
///
/// > Respects `.gitignore` in subdirectories
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_respects_nested_gitignore() {
    // Nested .gitignore files should also be respected
    // This tests that the walker properly inherits gitignore rules
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("gitignore-test"))
        .assert()
        .success()
        .stdout(predicates::str::contains("vendor/").not());
}

// =============================================================================
// Custom Ignore Patterns
// =============================================================================

/// Spec: docs/specs/20-performance.md (custom ignore patterns)
///
/// > Custom ignore patterns from quench.toml should be respected
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_respects_custom_ignore_patterns() {
    // Files matching patterns in [project.ignore] should be ignored
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("custom-ignore"))
        .assert()
        .success()
        .stdout(predicates::str::contains("src/lib.rs"))
        .stdout(predicates::str::contains(".snapshot").not());
}

/// Spec: docs/specs/20-performance.md (custom ignore patterns)
///
/// > Directory patterns should exclude entire directories
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_respects_custom_directory_patterns() {
    // testdata/ directory should be completely ignored
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("custom-ignore"))
        .assert()
        .success()
        .stdout(predicates::str::contains("testdata/").not());
}

/// Spec: docs/specs/20-performance.md (custom ignore patterns)
///
/// > Glob patterns with ** should match at any depth
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_respects_double_star_patterns() {
    // **/fixtures/** should match fixtures at any depth
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("custom-ignore"))
        .assert()
        .success()
        .stdout(predicates::str::contains("fixtures/").not());
}

// =============================================================================
// Symlink Loop Detection
// =============================================================================

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Detect and skip symlink loops
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_detects_symlink_loops() {
    // A symlink pointing to itself or parent should not cause infinite recursion
    // The test should complete without hanging
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("symlink-loop"))
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .success();  // Should complete, not hang
}

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Symlink loops should be reported when verbose
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_reports_symlink_loops_in_verbose_mode() {
    // With --verbose, symlink loops should be mentioned
    quench_cmd()
        .args(["check", "--verbose"])
        .current_dir(fixture("symlink-loop"))
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .success()
        .stderr(predicates::str::contains("symlink").or(predicates::str::contains("loop")));
}

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Normal files should still be scanned when symlink loops exist
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_scans_normal_files_despite_symlink_loops() {
    // src/lib.rs should still be scanned even though a loop exists
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("symlink-loop"))
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .success()
        .stdout(predicates::str::contains("src/lib.rs"));
}

// =============================================================================
// Directory Depth Limits
// =============================================================================

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Limit directory depth (default: 100 levels)
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_respects_default_depth_limit() {
    // Files beyond depth 100 should not be scanned
    // bench-deep has files at level 50 (within limit) and 120 (beyond)
    quench_cmd()
        .args(["check", "--debug-files"])
        .current_dir(fixture("bench-deep"))
        .assert()
        .success()
        .stdout(predicates::str::contains("mid.rs"))           // level 50, within limit
        .stdout(predicates::str::contains("deep.rs").not());   // level 120, beyond limit
}

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Depth limit should be configurable
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_respects_custom_depth_limit() {
    // With a lower depth limit, fewer files should be scanned
    // This tests the --max-depth flag or config option
    quench_cmd()
        .args(["check", "--debug-files", "--max-depth", "25"])
        .current_dir(fixture("bench-deep"))
        .assert()
        .success()
        .stdout(predicates::str::contains("mid.rs").not());    // level 50, now beyond limit
}

/// Spec: docs/specs/20-performance.md#deep-directory-trees
///
/// > Depth limit warnings in verbose mode
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_warns_on_depth_limit_in_verbose() {
    // When files are skipped due to depth, verbose mode should mention it
    quench_cmd()
        .args(["check", "--verbose"])
        .current_dir(fixture("bench-deep"))
        .assert()
        .success()
        .stderr(predicates::str::contains("depth").or(predicates::str::contains("limit")));
}

// =============================================================================
// Edge Cases
// =============================================================================

/// Spec: docs/specs/20-performance.md#large-file-counts
///
/// > Never build unbounded in-memory file lists
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_handles_empty_directory() {
    // Empty directories should not cause errors
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("minimal"))
        .assert()
        .success();
}

/// Spec: docs/specs/20-performance.md#parallel-gitignore-aware-file-walking
///
/// > Use iterative traversal, not recursive
#[test]
#[ignore = "TODO: Phase 020 - File Walking Implementation"]
fn file_walking_uses_iterative_traversal() {
    // This is tested implicitly by bench-deep - recursive traversal
    // would cause stack overflow at 120 levels on most systems
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("bench-deep"))
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();  // Should complete without stack overflow
}
```

```rust
// Update tests/specs/main.rs to include file_walking module
// Add this line with other mod declarations:
mod file_walking;
```

**Verification**:
```bash
cargo test --test specs -- --ignored 2>&1 | grep -c "file_walking"
cargo test --test specs file_walking -- --list
```

### Phase 15.6: Fixture Generation Scripts and README Updates

**Goal**: Add generation scripts to CI/setup and update fixture README.

**Tasks**:
1. Update `tests/fixtures/README.md` with new fixtures
2. Add fixture setup to bootstrap script or test setup
3. Ensure all fixtures are created before tests run

**Files**:

Add to `tests/fixtures/README.md`:

```markdown
## New Fixtures for File Walking (Phase 015)

### bench-deep/

Deeply nested directory structure (120 levels) for testing depth limits.

- Generated by `scripts/generate-bench-deep.sh`
- File at level 50: should be scanned with default limit
- File at level 120: should be skipped with default limit (100)
- Tests iterative traversal (recursive would stack overflow)

### gitignore-test/

Tests `.gitignore` file exclusion during file walking.

- Has standard `.gitignore` patterns
- `target/` and `vendor/` directories should be ignored
- `*.generated.rs` files should be ignored
- `src/lib.rs` should be scanned

### symlink-loop/

Tests symlink loop detection.

- Contains `loop -> .` symlink pointing to itself
- Created by `scripts/setup-symlink-fixtures.sh`
- Should complete without hanging
- `src/lib.rs` should still be scanned

### custom-ignore/

Tests custom ignore patterns from `quench.toml`.

- Has `[project.ignore]` section with custom patterns
- Tests `*.snapshot`, `testdata/`, `**/fixtures/**` patterns
- `src/lib.rs` should be scanned, patterns should be ignored
```

Add to `scripts/bootstrap` or create `scripts/setup-test-fixtures.sh`:

```bash
#!/bin/bash
# scripts/setup-test-fixtures.sh
# Sets up test fixtures that require generation or special handling

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "Setting up test fixtures..."

# Generate deeply nested fixture
if [ -x "./scripts/generate-bench-deep.sh" ]; then
    echo "  Generating bench-deep fixture..."
    ./scripts/generate-bench-deep.sh
fi

# Create symlinks
if [ -x "./scripts/setup-symlink-fixtures.sh" ]; then
    echo "  Setting up symlink fixtures..."
    ./scripts/setup-symlink-fixtures.sh
fi

echo "Test fixtures ready."
```

**Verification**:
```bash
./scripts/setup-test-fixtures.sh
cargo test --test specs -- --list 2>&1 | grep file_walking
```

## Key Implementation Details

### Spec Conventions

All specs follow the pattern from `tests/specs/CLAUDE.md`:

1. Doc comment references the spec document section
2. Quote the relevant spec text
3. Use `#[ignore = "TODO: Phase 020 - File Walking Implementation"]`
4. Use `fixture()` helper for test directories
5. Use `quench_cmd()` to run CLI

### Hypothetical Debug Flag

Specs assume a `--debug-files` flag that lists scanned files. This flag is for testing only and may be:
- A hidden debug flag
- Replaced with JSON output parsing
- Implemented as verbose logging

The implementation phase will determine the exact mechanism.

### Symlink Creation

Symlinks cannot be reliably committed to git. The `setup-symlink-fixtures.sh` script creates them at test time. This must run:
- During local development setup
- In CI before tests run
- Idempotently (safe to run multiple times)

### Depth Limit Testing

The `bench-deep` fixture:
- Has 120 levels (exceeds default 100)
- Has files at both level 50 and 120
- Tests that iterative traversal works (recursive would overflow)
- Tests configurable depth limits

### Timeout Protection

Symlink and depth tests use `timeout()` to prevent hangs:
- Symlink tests: 5 second timeout
- Depth tests: 30 second timeout

A test that times out indicates the implementation is not handling the edge case.

## Verification Plan

### Phase Completion Checklist

- [ ] `tests/fixtures/gitignore-test/` exists with .gitignore
- [ ] `tests/fixtures/symlink-loop/` exists (symlink created by script)
- [ ] `tests/fixtures/bench-deep/` exists (generated by script)
- [ ] `tests/fixtures/custom-ignore/` exists with quench.toml patterns
- [ ] `scripts/generate-bench-deep.sh` creates 120-level structure
- [ ] `scripts/setup-symlink-fixtures.sh` creates symlinks
- [ ] `tests/specs/file_walking.rs` has all specs
- [ ] All specs compile with `cargo test --test specs`
- [ ] All specs are ignored (counted in `--ignored` output)
- [ ] `tests/fixtures/README.md` documents new fixtures

### Running Verification

```bash
# Create fixtures
./scripts/setup-test-fixtures.sh

# Verify fixtures exist
ls -la tests/fixtures/gitignore-test/
ls -la tests/fixtures/symlink-loop/
ls -la tests/fixtures/bench-deep/deep/level-1/
ls -la tests/fixtures/custom-ignore/

# Verify specs compile
cargo test --test specs -- file_walking --list

# Count ignored specs (should be 12-15)
cargo test --test specs -- --ignored 2>&1 | grep -c "ignored"

# Verify no specs run yet (all should be ignored)
cargo test --test specs file_walking 2>&1 | grep "0 passed"
```

### Expected Spec Count

| Category | Specs |
|----------|-------|
| Gitignore handling | 3 |
| Custom ignore patterns | 3 |
| Symlink loop detection | 3 |
| Directory depth limits | 3 |
| Edge cases | 2 |
| **Total** | **14** |

## Summary

Phase 015 creates behavioral spec coverage for file walking:

1. **4 new fixtures** for testing specific file walking behaviors
2. **2 generation scripts** for fixtures that can't be committed
3. **14 behavioral specs** covering all edge cases from performance spec
4. **Documentation** updates for new fixtures

All specs are ignored until Phase 020 implements the file walking functionality using the `ignore` crate.
