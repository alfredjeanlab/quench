# Phase 1010: Build Check - Size

## Overview

Implement size threshold enforcement for the build check. The build check already measures binary sizes but lacks threshold checking and violation generation. This phase adds:

- Human-readable size parsing (e.g., "10 MB" → bytes)
- Global `size_max` threshold enforcement
- Per-target `size_max` overrides
- `size_exceeded` violation generation with appropriate advice

The existing implementation in `crates/cli/src/checks/build/mod.rs` collects metrics; this phase adds the enforcement layer.

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── file_size.rs              # Extend: add parse_size()
│   ├── file_size_tests.rs        # Extend: add parsing tests
│   └── checks/build/
│       ├── mod.rs                # Extend: threshold checking
│       └── mod_tests.rs          # Extend: unit tests for thresholds
└── tests/specs/checks/
    └── build.rs                  # Remove #[ignore] from size specs
```

## Dependencies

**Existing:**
- `toml` - Already used for Cargo.toml parsing
- `serde_json` - Already used for metrics output

**No new external dependencies required.**

## Implementation Phases

### Phase 1: Size String Parsing

**Goal:** Add a `parse_size()` function to convert human-readable sizes to bytes.

**Files:**
- `crates/cli/src/file_size.rs` - Add parsing function
- `crates/cli/src/file_size_tests.rs` - Add parsing tests

**Pattern:** Follow `config/duration.rs` for parsing and serde deserializer patterns.

**Implementation:**

```rust
// crates/cli/src/file_size.rs

/// Parse a human-readable size string into bytes.
///
/// Supported formats:
/// - `"10 MB"` or `"10MB"` → 10 * 1024 * 1024
/// - `"500 KB"` or `"500KB"` → 500 * 1024
/// - `"1024"` or `"1024 bytes"` → 1024
/// - `"1.5 MB"` → 1.5 * 1024 * 1024 (truncated to u64)
pub fn parse_size(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty size string".to_string());
    }

    // Try MB first (case-insensitive)
    let upper = s.to_uppercase();
    if let Some(num) = upper.strip_suffix("MB") {
        let n: f64 = num.trim().parse()
            .map_err(|_| format!("invalid size: {s}"))?;
        return Ok((n * 1024.0 * 1024.0) as u64);
    }

    // Try KB
    if let Some(num) = upper.strip_suffix("KB") {
        let n: f64 = num.trim().parse()
            .map_err(|_| format!("invalid size: {s}"))?;
        return Ok((n * 1024.0) as u64);
    }

    // Try bytes suffix
    if let Some(num) = upper.strip_suffix("BYTES") {
        let n: u64 = num.trim().parse()
            .map_err(|_| format!("invalid size: {s}"))?;
        return Ok(n);
    }
    if let Some(num) = upper.strip_suffix('B') {
        let n: u64 = num.trim().parse()
            .map_err(|_| format!("invalid size: {s}"))?;
        return Ok(n);
    }

    // Try plain number (bytes)
    s.parse::<u64>()
        .map_err(|_| format!("invalid size format: {s} (use 10 MB, 500 KB, or bytes)"))
}
```

**Verification:**
```bash
cargo test --lib file_size
```

### Phase 2: Config Size Deserialization

**Goal:** Add serde deserializer for `size_max` config field.

**Files:**
- `crates/cli/src/file_size.rs` - Add deserializer
- `crates/cli/src/config/mod.rs` - Update BuildConfig to use deserializer

**Implementation:**

```rust
// crates/cli/src/file_size.rs

/// Deserialize an optional size string.
pub fn deserialize_option<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(s) => parse_size(&s)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}
```

```rust
// crates/cli/src/config/mod.rs - Update BuildConfig

/// Build check configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BuildConfig {
    pub check: Option<String>,
    #[serde(default)]
    pub targets: Vec<String>,

    /// Global maximum binary size (parsed from "10 MB" to bytes).
    #[serde(default, deserialize_with = "crate::file_size::deserialize_option")]
    pub size_max: Option<u64>,

    #[serde(default, deserialize_with = "crate::config::duration::deserialize_option")]
    pub time_cold_max: Option<Duration>,

    #[serde(default, deserialize_with = "crate::config::duration::deserialize_option")]
    pub time_hot_max: Option<Duration>,

    #[serde(default)]
    pub target: HashMap<String, BuildTargetConfig>,
}

/// Per-target build configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BuildTargetConfig {
    #[serde(default, deserialize_with = "crate::file_size::deserialize_option")]
    pub size_max: Option<u64>,
}
```

**Verification:**
```bash
cargo test --lib config
```

### Phase 3: Size Threshold Checking

**Goal:** Add threshold checking logic to the build check `run()` method.

**Files:**
- `crates/cli/src/checks/build/mod.rs` - Add threshold checking

**Implementation:**

```rust
// crates/cli/src/checks/build/mod.rs

fn run(&self, ctx: &CheckContext) -> CheckResult {
    if !ctx.ci_mode {
        return CheckResult::stub(self.name());
    }

    let mut metrics = BuildMetrics::default();
    let mut violations = Vec::new();
    let language = detect_language(ctx.root);

    // Measure binary sizes
    let targets = get_build_targets(ctx.root, language);
    for target in targets {
        if let Some(size) = measure_binary_size(ctx.root, &target, language) {
            metrics.sizes.insert(target.clone(), size);

            // Check size threshold
            if let Some(threshold) = get_size_threshold(&target, &ctx.config.check.build) {
                if size > threshold {
                    violations.push(create_size_violation(&target, size, threshold));
                }
            }
        }
    }

    // ... existing time measurement code ...

    // Return result with metrics and violations
    if metrics.sizes.is_empty() && metrics.time_cold.is_none() && metrics.time_hot.is_none() {
        CheckResult::stub(self.name())
    } else if violations.is_empty() {
        CheckResult::passed(self.name()).with_metrics(metrics.to_json())
    } else {
        CheckResult::failed(self.name(), violations).with_metrics(metrics.to_json())
    }
}

/// Get the size threshold for a target (per-target override or global).
fn get_size_threshold(target: &str, config: &BuildConfig) -> Option<u64> {
    // Check per-target override first
    if let Some(target_config) = config.target.get(target) {
        if let Some(max) = target_config.size_max {
            return Some(max);
        }
    }
    // Fall back to global
    config.size_max
}

/// Create a size_exceeded violation.
fn create_size_violation(target: &str, size: u64, threshold: u64) -> Violation {
    Violation {
        file: None,
        line: None,
        violation_type: "size_exceeded".to_string(),
        advice: "Reduce binary size. Check for unnecessary dependencies.".to_string(),
        value: Some(size as i64),
        threshold: Some(threshold as i64),
        target: Some(target.to_string()),
        ..Default::default()
    }
}
```

**Verification:**
```bash
cargo test --lib checks::build
```

### Phase 4: Violation Target Field

**Goal:** Add `target` field to Violation struct for build violations.

**Files:**
- `crates/cli/src/check.rs` - Add target field (if not exists, check first)

**Check:** The `target` field may already exist in `Violation` for broken_link violations. If so, reuse it. If not, add it.

Looking at the current `Violation` struct, `target` already exists:
```rust
/// Link target for broken_link violations.
#[serde(skip_serializing_if = "Option::is_none")]
pub target: Option<String>,
```

This can be reused for build violations - update the doc comment to reflect dual purpose.

**Implementation:**
```rust
// crates/cli/src/check.rs - Update doc comment

/// Target name for build violations or link target for broken_link violations.
#[serde(skip_serializing_if = "Option::is_none")]
pub target: Option<String>,
```

**Verification:**
```bash
cargo test --lib check
```

### Phase 5: Enable Behavioral Specs

**Goal:** Remove `#[ignore]` from size-related specs and verify they pass.

**Files:**
- `tests/specs/checks/build.rs` - Remove ignore from size specs

**Specs to enable:**
1. `build_size_exceeded_generates_violation`
2. `build_size_under_threshold_passes`
3. `build_per_target_size_max`
4. `build_violation_type_is_size_exceeded`
5. `build_size_exceeded_has_correct_advice`

**Verification:**
```bash
cargo test --test specs -- build_size
```

### Phase 6: Text Output Format

**Goal:** Ensure text output matches spec format for size violations.

**Expected output from spec:**
```
build: FAIL
  myapp: 5.1 MB (max: 5 MB)
    Reduce binary size. Check for unnecessary dependencies.
```

**Files:**
- `crates/cli/src/output/text.rs` - Verify/update build output formatting

**Check:** The text formatter likely handles violations generically. May need build-specific formatting for the size comparison display.

**Verification:**
```bash
cargo test --test specs build_size_exceeded
```

## Key Implementation Details

### Size Parsing Behavior

| Input | Result |
|-------|--------|
| `"10 MB"` | 10,485,760 bytes |
| `"10MB"` | 10,485,760 bytes |
| `"500 KB"` | 512,000 bytes |
| `"1024"` | 1,024 bytes |
| `"100 bytes"` | 100 bytes |
| `""` | Error |
| `"invalid"` | Error |

Size parsing is case-insensitive. Spaces are optional between number and unit.

### Threshold Resolution Order

1. `[check.build.target.<name>].size_max` - Per-target override
2. `[check.build].size_max` - Global default
3. No threshold (None) - Skip size checking for target

### Violation Structure

```json
{
  "file": null,
  "line": null,
  "type": "size_exceeded",
  "target": "myapp",
  "value": 5347737,
  "threshold": 5242880,
  "advice": "Reduce binary size. Check for unnecessary dependencies."
}
```

### Strip Handling

The build check measures the actual binary size after cargo builds it. If `strip = true` or `strip = "symbols"` is set in `[profile.release]`, cargo produces a smaller binary. No special handling is needed - we measure what cargo produces.

Example Cargo.toml:
```toml
[profile.release]
strip = true  # Binary will be smaller; we measure the stripped size
```

### CI-Only Enforcement

The build check only runs in `--ci` mode:
```rust
if !ctx.ci_mode {
    return CheckResult::stub(self.name());
}
```

This prevents slow build operations during fast local checks.

## Verification Plan

### Unit Tests

```bash
# Size parsing
cargo test --lib file_size

# Config parsing
cargo test --lib config

# Build check logic
cargo test --lib checks::build
```

### Behavioral Specs

```bash
# All size-related specs
cargo test --test specs -- build_size

# Full build check specs (excluding slow build time tests)
cargo test --test specs -- checks_build

# Include slow tests (CI only)
cargo test --test specs -- checks_build --include-ignored
```

### Integration Test

```bash
# Test on quench itself (should pass - no size_max configured)
quench check --build --ci

# Verify JSON output structure
quench check --build --ci -o json | jq '.checks[] | select(.name == "build")'
```

### Checklist

- [ ] `parse_size()` function added to `file_size.rs`
- [ ] Size parsing unit tests pass
- [ ] `BuildConfig.size_max` uses deserializer
- [ ] `BuildTargetConfig.size_max` uses deserializer
- [ ] `get_size_threshold()` implements resolution order
- [ ] `create_size_violation()` creates correct violation
- [ ] Build check returns failed result with violations
- [ ] Text output matches spec format
- [ ] All size specs pass (remove `#[ignore]`)
- [ ] `make check` passes

### Exit Criteria

All the following specs pass:
1. `build_size_exceeded_generates_violation`
2. `build_size_under_threshold_passes`
3. `build_per_target_size_max`
4. `build_violation_type_is_size_exceeded`
5. `build_size_exceeded_has_correct_advice`
