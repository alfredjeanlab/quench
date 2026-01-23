# Phase 220: Escapes Check - Output

**Root Feature:** `quench-6c9b`

## Overview

Implement metrics and output generation for the `escapes` check. Phase 215 established action logic (count/comment/forbid); this phase adds:
- **Metrics tracking**: Source and test counts per pattern
- **Per-package breakdown**: Counts per workspace package
- **JSON output**: Full metrics structure with `metrics` and `by_package` fields
- **Text output enhancement**: Better threshold violation formatting

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── checks/
│   │   ├── escapes.rs       # Add metrics collection, by_package
│   │   └── escapes_tests.rs # Unit tests for metrics
│   └── output/
│       └── text.rs          # Threshold violation formatting
├── tests/
│   ├── specs/
│   │   └── checks/escapes.rs # Enable ignored specs
│   └── fixtures/
│       └── escapes/
│           ├── metrics/     # Existing fixture (enhance)
│           └── packages/    # New fixture for by_package
└── plans/
    └── phase-220.md
```

## Dependencies

No new dependencies. Uses existing:
- `serde_json` for metrics serialization (from check.rs)
- `std::collections::HashMap` for package tracking

## Implementation Phases

### Phase 1: Metrics Collection Structure

Add data structures to track counts per pattern, separated by source/test.

**Update `crates/cli/src/checks/escapes.rs`:**

```rust
use std::collections::HashMap;
use serde_json::{json, Value as JsonValue};

/// Metrics tracked during escapes check.
#[derive(Default)]
struct EscapesMetrics {
    /// Counts per pattern for source files.
    source: HashMap<String, usize>,
    /// Counts per pattern for test files.
    test: HashMap<String, usize>,
    /// Per-package breakdown (only if workspace configured).
    packages: HashMap<String, PackageMetrics>,
}

#[derive(Default)]
struct PackageMetrics {
    source: HashMap<String, usize>,
    test: HashMap<String, usize>,
}

impl EscapesMetrics {
    fn new() -> Self {
        Self::default()
    }

    fn increment(&mut self, pattern_name: &str, is_test: bool) {
        let map = if is_test { &mut self.test } else { &mut self.source };
        *map.entry(pattern_name.to_string()).or_insert(0) += 1;
    }

    fn increment_package(&mut self, package: &str, pattern_name: &str, is_test: bool) {
        let pkg = self.packages.entry(package.to_string()).or_default();
        let map = if is_test { &mut pkg.test } else { &mut pkg.source };
        *map.entry(pattern_name.to_string()).or_insert(0) += 1;
    }

    fn source_count(&self, pattern_name: &str) -> usize {
        self.source.get(pattern_name).copied().unwrap_or(0)
    }

    /// Convert to JSON metrics structure.
    fn to_json(&self, pattern_names: &[String]) -> JsonValue {
        // Include all configured patterns, even with 0 count
        let mut source_obj = serde_json::Map::new();
        let mut test_obj = serde_json::Map::new();

        for name in pattern_names {
            source_obj.insert(name.clone(), json!(self.source.get(name).copied().unwrap_or(0)));
            test_obj.insert(name.clone(), json!(self.test.get(name).copied().unwrap_or(0)));
        }

        json!({
            "source": source_obj,
            "test": test_obj
        })
    }

    /// Convert to by_package structure (only if packages exist).
    fn to_by_package(&self, pattern_names: &[String]) -> Option<HashMap<String, JsonValue>> {
        if self.packages.is_empty() {
            return None;
        }

        let mut result = HashMap::new();
        for (pkg_name, pkg_metrics) in &self.packages {
            let mut source_obj = serde_json::Map::new();
            let mut test_obj = serde_json::Map::new();

            for name in pattern_names {
                source_obj.insert(name.clone(), json!(pkg_metrics.source.get(name).copied().unwrap_or(0)));
                test_obj.insert(name.clone(), json!(pkg_metrics.test.get(name).copied().unwrap_or(0)));
            }

            result.insert(pkg_name.clone(), json!({
                "source": source_obj,
                "test": test_obj
            }));
        }

        Some(result)
    }
}
```

**Milestone:** Metrics structures compile and pass unit tests.

**Verification:**
```bash
cargo build
cargo test checks::escapes -- metrics
```

---

### Phase 2: Package Detection

Add logic to determine which package a file belongs to.

**Update `crates/cli/src/checks/escapes.rs`:**

```rust
/// Find which package a file belongs to, if any.
fn find_package(path: &Path, root: &Path, packages: &[String]) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let relative_str = relative.to_string_lossy();

    // Check if file is under any package directory
    for pkg in packages {
        // Package glob patterns (e.g., "crates/*", "packages/core")
        if relative_str.starts_with(pkg.trim_end_matches("/*")) {
            // Extract package name from path
            let parts: Vec<&str> = relative_str.split('/').collect();
            if let Some(name) = parts.get(1) {
                return Some((*name).to_string());
            } else if !pkg.ends_with("/*") {
                // Direct package reference like "packages/core"
                return Some(pkg.split('/').last().unwrap_or(pkg).to_string());
            }
        }
    }

    None
}
```

**Milestone:** Package detection correctly maps files to packages.

**Verification:**
```bash
cargo test checks::escapes -- find_package
```

---

### Phase 3: Integrate Metrics into Run Loop

Update the main `run()` function to collect metrics for all matches.

**Update `crates/cli/src/checks/escapes.rs` run method:**

```rust
fn run(&self, ctx: &CheckContext) -> CheckResult {
    let config = &ctx.config.check.escapes;
    // ... existing setup ...

    // Collect pattern names for metrics output
    let pattern_names: Vec<String> = patterns.iter().map(|p| p.name.clone()).collect();

    // Get workspace packages for by_package tracking
    let packages = &ctx.config.workspace.packages;

    let mut metrics = EscapesMetrics::new();
    let mut violations = Vec::new();
    let mut limit_reached = false;

    for file in ctx.files {
        if limit_reached {
            break;
        }

        if !is_source_file(&file.path) {
            continue;
        }

        let content = match std::fs::read_to_string(&file.path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let relative = file.path.strip_prefix(ctx.root).unwrap_or(&file.path);
        let is_test = classify_file(&file.path, ctx.root, &test_patterns) == FileKind::Test;
        let package = find_package(&file.path, ctx.root, packages);

        for pattern in &patterns {
            let matches = pattern.matcher.find_all_with_lines(&content);

            for m in matches {
                // Always track metrics (both source and test)
                metrics.increment(&pattern.name, is_test);
                if let Some(ref pkg) = package {
                    metrics.increment_package(pkg, &pattern.name, is_test);
                }

                // Test code: tracked in metrics but no violations
                if is_test {
                    continue;
                }

                // Source code: apply action logic (existing code)
                match pattern.action {
                    EscapeAction::Count => {
                        // Threshold check happens after all files
                    }
                    EscapeAction::Comment => {
                        // ... existing comment logic ...
                    }
                    EscapeAction::Forbid => {
                        // ... existing forbid logic ...
                    }
                }
            }
            // ... limit checking ...
        }
    }

    // Check count thresholds using metrics (replaces source_counts)
    for pattern in &patterns {
        if pattern.action == EscapeAction::Count {
            let count = metrics.source_count(&pattern.name);
            if count > pattern.threshold {
                // ... create threshold violation ...
            }
        }
    }

    // Build result with metrics
    let result = if violations.is_empty() {
        CheckResult::passed(self.name())
    } else {
        CheckResult::failed(self.name(), violations)
    };

    // Add metrics to result
    let result = result.with_metrics(metrics.to_json(&pattern_names));

    // Add by_package if workspace configured
    if let Some(by_package) = metrics.to_by_package(&pattern_names) {
        result.with_by_package(by_package)
    } else {
        result
    }
}
```

**Milestone:** Metrics are collected and included in `CheckResult`.

**Verification:**
```bash
cargo test --test specs escapes_count_action_counts_occurrences
cargo test --test specs escapes_test_code_counted_separately_in_metrics
```

---

### Phase 4: Test Fixtures for Metrics

Create fixtures to verify metrics output.

**Update `tests/fixtures/escapes/metrics/quench.toml`:**

```toml
version = 1

[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "count"
threshold = 100  # High threshold so check passes

[[check.escapes.patterns]]
name = "todo"
pattern = "TODO"
action = "count"
threshold = 100
```

**Ensure `tests/fixtures/escapes/metrics/src/lib.rs` has source escapes:**

```rust
pub fn source_func() {
    // TODO: refactor this
    let x = Some(1).unwrap();
}
```

**Ensure `tests/fixtures/escapes/metrics/tests/lib_test.rs` has test escapes:**

```rust
#[test]
fn test_func() {
    // TODO: improve test
    let x = Some(1).unwrap();
}
```

**Create `tests/fixtures/escapes/packages/` for by_package testing:**

```
tests/fixtures/escapes/packages/
├── quench.toml
├── crates/
│   ├── core/
│   │   └── src/lib.rs
│   └── cli/
│       └── src/lib.rs
```

**Milestone:** Fixtures exercise metrics and by_package output.

**Verification:**
```bash
cargo test --test specs escapes
```

---

### Phase 5: Enhanced Threshold Violation Text Output

Improve text output for threshold violations to show file breakdown.

**Update `crates/cli/src/output/text.rs` to format threshold violations:**

The current implementation shows:
```
escapes: FAIL
  threshold_exceeded (23 vs 10)
    Reduce TODO/FIXME comments or increase threshold.
```

Update to match spec format:
```
escapes: FAIL
  "todo": 23 occurrences (max: 10)
    src/parser.rs: 8 occurrences
    src/lexer.rs: 7 occurrences
    src/compiler.rs: 5 occurrences
    (3 more files...)
    Reduce TODO/FIXME comments or increase threshold.
```

This requires tracking per-file counts in the violation. Update `Violation` to support a `file_breakdown` field:

```rust
// In check.rs - Violation struct
#[serde(skip_serializing_if = "Option::is_none")]
pub file_breakdown: Option<Vec<FileCount>>,

#[derive(Debug, Clone, Serialize)]
pub struct FileCount {
    pub file: PathBuf,
    pub count: usize,
}
```

**Note:** This is optional enhancement. The core functionality works without it.

**Milestone:** Threshold violations show readable breakdown in text mode.

**Verification:**
```bash
cargo test --test specs escapes_count_action_fails_when_threshold_exceeded
```

---

### Phase 6: Enable Spec Tests and Final Verification

Remove `#[ignore]` from Phase 220 spec tests and verify all pass.

**Update `tests/specs/checks/escapes.rs`:**

Remove `#[ignore = "TODO: Phase 220 - Escapes Metrics"]` from:
- `escapes_count_action_counts_occurrences`
- `escapes_test_code_counted_separately_in_metrics`
- `escapes_json_includes_source_test_breakdown_per_pattern`

**Milestone:** All escapes specs pass.

**Verification:**
```bash
cargo test --test specs escapes
make check
```

---

## Key Implementation Details

### Metrics JSON Structure

Per docs/specs/checks/escape-hatches.md#json-output:

```json
{
  "metrics": {
    "source": { "unsafe": 3, "unwrap": 0, "expect": 0 },
    "test": { "unsafe": 0, "unwrap": 47, "expect": 5 }
  },
  "by_package": {
    "cli": {
      "source": { "unsafe": 1, "unwrap": 0 },
      "test": { "unsafe": 0, "unwrap": 23 }
    }
  }
}
```

- All configured patterns appear in metrics, even with count 0
- `by_package` is omitted if no workspace packages configured
- Pattern names are the `name` field from config, not the regex

### Test Code Identification

Test files are identified using patterns from `project.tests` or defaults:
- `**/tests/**`, `**/test/**`
- `**/*_test.*`, `**/*_tests.*`
- `**/*.test.*`, `**/*.spec.*`

Test code is:
- Always counted in `metrics.test`
- Never generates violations
- Counted in `by_package.*.test` if packages configured

### Package Mapping

Files map to packages based on workspace.packages patterns:
- `crates/*` - wildcard matches first directory component
- `packages/core` - exact path match

Example:
```toml
[workspace]
packages = ["crates/*"]
```

File `crates/cli/src/main.rs` → package `cli`

### Early Termination

Existing limit checking continues to work:
- `ctx.limit` controls max violations shown
- Metrics are always collected (no early termination for metrics)
- In non-CI mode, stop collecting after limit+buffer violations

---

## Verification Plan

### After Each Phase

```bash
# Compile check
cargo build

# Run relevant unit tests
cargo test checks::escapes

# Check lints
cargo clippy --all-targets --all-features -- -D warnings
```

### End-to-End Verification

```bash
# Run Phase 220 specs (remove #[ignore])
cargo test --test specs escapes_count_action_counts_occurrences
cargo test --test specs escapes_test_code_counted_separately_in_metrics
cargo test --test specs escapes_json_includes_source_test_breakdown_per_pattern

# All escapes specs
cargo test --test specs escapes

# Full quality gates
make check
```

### Test Matrix

| Test Case | Expected JSON Field |
|-----------|---------------------|
| Source matches only | `metrics.source` has counts, `metrics.test` all 0 |
| Test matches only | `metrics.test` has counts, `metrics.source` all 0 |
| Both source and test | Both have counts |
| No workspace | `by_package` omitted |
| With workspace | `by_package` has per-package breakdown |
| Pattern with 0 matches | Still appears in metrics with count 0 |

---

## Summary

| Phase | Task | Key Files | Status |
|-------|------|-----------|--------|
| 1 | Metrics collection structure | `checks/escapes.rs` | [ ] Pending |
| 2 | Package detection | `checks/escapes.rs` | [ ] Pending |
| 3 | Integrate metrics into run loop | `checks/escapes.rs` | [ ] Pending |
| 4 | Test fixtures for metrics | `tests/fixtures/escapes/` | [ ] Pending |
| 5 | Enhanced threshold violation output | `output/text.rs` | [ ] Pending |
| 6 | Enable spec tests | `tests/specs/checks/escapes.rs` | [ ] Pending |

## Future Phases

- **Phase 225**: Per-package thresholds and overrides
- **Phase 230**: HTML report output for escapes metrics
