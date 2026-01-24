# Checkpoint 15G: Bug Fixes - Ratcheting

**Root Feature:** `quench-0765`

## Overview

This checkpoint addresses bugs and code quality issues discovered in the ratcheting system following the 15F quick wins release. The fixes focus on correct output messaging, proper numeric formatting, and removing dead code.

**Bugs identified:**
1. **Redundant message logic** - Both branches in `--fix` message output print the same text
2. **Truncated decimal precision** - Float metrics (timing) lose precision when cast to `i64`
3. **Unused parameter** - `_improvements` parameter in `update_baseline` is never used

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── main.rs                    # FIX: Correct --fix message branching
│   ├── output/
│   │   └── text.rs                # FIX: Preserve decimal precision for timing
│   └── ratchet.rs                 # FIX: Remove unused _improvements parameter
├── tests/
│   └── specs/modes/
│       └── ratchet.rs             # ADD: Tests for fix message variants
└── plans/
    └── checkpoint-15g-bugfix.md   # This plan
```

## Dependencies

No new dependencies. Uses existing infrastructure.

## Implementation Phases

### Phase 1: Fix Redundant Message Logic

**Goal:** Differentiate messaging for `--fix` with improvements vs. without.

**Problem:** `crates/cli/src/main.rs:458-470` has an if/else where both branches print the same message:

```rust
// BUG: Both branches say "updated baseline"
if baseline_path.exists() && result.improvements.is_empty() {
    eprintln!("ratchet: updated baseline");  // ← Same message
} else {
    eprintln!("ratchet: updated baseline");  // ← Same message
    for improvement in &result.improvements { ... }
}
```

**Fix:** The first branch (no improvements) should indicate the baseline was synced without improvements. The condition also appears inverted - when improvements exist, we should show them.

**File:** `crates/cli/src/main.rs`

```rust
if result.improvements.is_empty() {
    // Baseline exists and metrics match - sync without announcement
    // (or just confirm the sync happened)
    eprintln!("ratchet: baseline synced");
} else {
    // Improvements detected - announce what changed
    eprintln!("ratchet: updated baseline");
    for improvement in &result.improvements {
        eprintln!(
            "  {}: {} -> {} (new ceiling)",
            improvement.name,
            improvement.old_value as i64,
            improvement.new_value as i64
        );
    }
}
```

**Verification:**
```bash
# Test shows different messages
cargo test --test specs fix_baseline_message
```

---

### Phase 2: Fix Decimal Precision in Output

**Goal:** Display timing metrics with appropriate decimal precision instead of truncating to integers.

**Problem:** `crates/cli/src/output/text.rs:364,380` casts float metrics to `i64`:

```rust
// BUG: Loses precision for timing metrics (2.5s displays as "2")
comp.name, comp.current as i64, comp.baseline as i64
```

Build time and test time metrics are measured in seconds with decimal precision. Casting to `i64` hides meaningful differences (e.g., 10.2s vs 12.8s appears as "10" vs "12").

**Fix:** Format based on metric type - use integers for counts (escapes, binary size) and decimals for timing.

**File:** `crates/cli/src/output/text.rs`

Add a helper to format values appropriately:

```rust
impl MetricComparison {
    /// Format the value based on metric type.
    fn format_value(&self, value: f64) -> String {
        if self.name.starts_with("build_time.") || self.name.starts_with("test_time.") {
            format!("{:.1}s", value)  // 1 decimal place for timing
        } else if self.name.starts_with("coverage.") {
            format!("{:.1}%", value * 100.0)  // Percentage for coverage
        } else {
            format!("{}", value as i64)  // Integer for counts
        }
    }
}
```

Update the output lines:

```rust
// For failures
writeln!(
    self.stdout,
    "  {}: {} (max: {} from baseline)",
    comp.name,
    comp.format_value(comp.current),
    comp.format_value(comp.baseline)
)?;

// For improvements
writeln!(
    self.stdout,
    "  {}: {} (baseline: {}) improved",
    comp.name,
    comp.format_value(comp.current),
    comp.format_value(comp.baseline)
)?;
```

Also fix the `--fix` improvement output in `main.rs`:

```rust
eprintln!(
    "  {}: {} -> {} (new ceiling)",
    improvement.name,
    format_improvement_value(&improvement.name, improvement.old_value),
    format_improvement_value(&improvement.name, improvement.new_value),
);
```

**Verification:**
```bash
# Timing metrics show decimals
cargo test ratchet_timing_format
```

---

### Phase 3: Remove Unused Parameter

**Goal:** Clean up dead code by removing the unused `_improvements` parameter.

**Problem:** `crates/cli/src/ratchet.rs:391` has an unused parameter:

```rust
pub fn update_baseline(
    baseline: &mut Baseline,
    current: &CurrentMetrics,
    _improvements: &[MetricImprovement],  // Never used
) {
```

The function updates the baseline with ALL current metrics regardless of what improved. The improvements list is informational only.

**Fix:** Remove the unused parameter and update all call sites.

**File:** `crates/cli/src/ratchet.rs`

```rust
/// Update baseline with current metrics.
pub fn update_baseline(baseline: &mut Baseline, current: &CurrentMetrics) {
    // ... existing implementation unchanged
}
```

**File:** `crates/cli/src/main.rs` (two call sites)

```rust
// Line ~452: After detecting improvements
ratchet::update_baseline(&mut baseline, &current);

// Line ~477: Creating initial baseline
ratchet::update_baseline(&mut baseline, &current);
```

**Verification:**
```bash
# No clippy warnings about unused parameters
cargo clippy -- -W clippy::unused-self
```

---

### Phase 4: Add Missing Test Coverage

**Goal:** Add behavioral tests for the fixed bugs.

**File:** `tests/specs/modes/ratchet.rs`

```rust
/// Spec: docs/specs/04-ratcheting.md#fix-message-variants
///
/// > --fix reports "baseline synced" when no improvements detected.
#[test]
fn fix_baseline_synced_message() {
    let temp = Project::empty();
    temp.config(RATCHET_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);

    // Create baseline with 2 unsafe
    fs::create_dir_all(temp.path().join(".quench")).unwrap();
    fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{"version": 1, "updated": "2026-01-20T00:00:00Z", "metrics": {"escapes": {"source": {"unsafe": 2}}}}"#,
    ).unwrap();

    // Source has 2 unsafe -> no improvement, just sync
    temp.file("src/lib.rs", "fn f() {\n    unsafe {}\n    unsafe {}\n}");

    quench_cmd()
        .args(["check", "--fix"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("baseline synced"));
}

/// Spec: docs/specs/04-ratcheting.md#fix-message-variants
///
/// > --fix reports "updated baseline" with improvement details when metrics improve.
#[test]
fn fix_baseline_updated_with_improvements() {
    let temp = Project::empty();
    temp.config(RATCHET_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);

    // Create baseline with 5 unsafe
    fs::create_dir_all(temp.path().join(".quench")).unwrap();
    fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{"version": 1, "updated": "2026-01-20T00:00:00Z", "metrics": {"escapes": {"source": {"unsafe": 5}}}}"#,
    ).unwrap();

    // Source has 2 unsafe -> improvement
    temp.file("src/lib.rs", "fn f() {\n    unsafe {}\n    unsafe {}\n}");

    quench_cmd()
        .args(["check", "--fix"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("updated baseline"))
        .stderr(predicates::str::contains("5 -> 2"));
}
```

**Verification:**
```bash
cargo test --test specs fix_baseline
```

---

### Phase 5: Final Verification

**Goal:** Ensure all fixes work together and CI passes.

**Verification checklist:**

```bash
# All unit tests pass
cargo test --all

# All spec tests pass
cargo test --test specs

# Clippy has no warnings
cargo clippy --all-targets --all-features -- -D warnings

# Full CI check
make check

# Dogfooding
cargo run -- check
```

## Key Implementation Details

### Message Distinction Logic

The `--fix` output should clearly communicate what happened:

| Scenario | Message |
|----------|---------|
| Initial baseline created | `ratchet: created initial baseline at ...` |
| Baseline updated with improvements | `ratchet: updated baseline` + improvement list |
| Baseline synced (no improvements) | `ratchet: baseline synced` |

### Value Formatting Strategy

Format values based on the metric type prefix:

| Metric prefix | Format | Example |
|---------------|--------|---------|
| `escapes.*` | Integer | `5` |
| `binary_size.*` | Integer with units | `1024` (bytes) |
| `build_time.*` | 1 decimal + "s" | `12.5s` |
| `test_time.*` | 1 decimal + "s" | `3.2s` |
| `coverage.*` | 1 decimal + "%" | `85.3%` |

### Removing the Unused Parameter

The `update_baseline` function is intentionally simple - it snapshots current metrics to the baseline. The improvements list is computed separately for display purposes only. Removing the parameter:

1. Clarifies the function's actual behavior
2. Removes dead code paths
3. Simplifies call sites

## Verification Plan

### Phase 1: Message Logic
```bash
# Create baseline, run --fix with no changes, verify "synced" message
# Create baseline, run --fix with improvements, verify "updated" message with details
cargo test --test specs fix_baseline
```

### Phase 2: Decimal Precision
```bash
# Unit test for format_value helper
cargo test ratchet::format_value

# Visual check: timing metrics show decimals
cargo run -- check --verbose  # (with timing baseline)
```

### Phase 3: Unused Parameter
```bash
# Compiles without warnings
cargo build --all-targets 2>&1 | grep -c "unused"
# Should be 0

# Clippy clean
cargo clippy --all-targets
```

### Phase 4: Test Coverage
```bash
# New tests exist and pass
cargo test --test specs fix_baseline_synced
cargo test --test specs fix_baseline_updated
```

### Phase 5: Full CI
```bash
make check
cargo run -- check
```

## Exit Criteria

- [ ] `--fix` outputs "baseline synced" when no improvements detected
- [ ] `--fix` outputs "updated baseline" with improvement list when improvements exist
- [ ] Timing metrics display with 1 decimal place (e.g., `12.5s`)
- [ ] `update_baseline` function no longer has unused `_improvements` parameter
- [ ] Behavioral tests cover message variants
- [ ] All tests pass: `make check`
- [ ] Dogfooding passes: `quench check` on quench
