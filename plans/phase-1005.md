# Phase 1005: Build Check - Targets

## Overview

Wire up build target configuration for the `build` check. This phase connects the existing target detection logic with explicit configuration overrides and per-target threshold enforcement. The infrastructure exists (Cargo.toml parsing, `BuildConfig.targets`, `BuildConfig.target` HashMap) but isn't connected to violation generation.

**Goal:** Enable users to:
1. Override auto-detected targets via `targets = ["myapp", "myserver"]`
2. Set per-target size thresholds via `[check.build.target.myapp]`
3. Generate violations when targets exceed thresholds or are missing

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── checks/build/
│   │   └── mod.rs                 # Modify: target resolution, threshold checking
│   └── config/
│       └── mod.rs                 # Reference: BuildConfig, BuildTargetConfig (exists)
├── tests/
│   └── specs/checks/
│       └── build.rs               # Modify: remove #[ignore] from target specs
└── docs/specs/checks/
    └── build.md                   # Reference: configuration spec (exists)
```

## Dependencies

**Existing (no new dependencies):**
- `toml` - Cargo.toml parsing
- `serde_json` - JSON metrics output

**Config structs already defined in `crates/cli/src/config/mod.rs`:**
```rust
pub struct BuildConfig {
    pub targets: Vec<String>,                           // Explicit targets
    pub size_max: Option<String>,                       // Global threshold
    pub target: HashMap<String, BuildTargetConfig>,     // Per-target config
    // ...
}

pub struct BuildTargetConfig {
    pub size_max: Option<String>,                       // Per-target override
}
```

## Implementation Phases

### Phase 1: Target Resolution Logic

**Goal:** Use explicit `targets` config when set, otherwise auto-detect.

**File:** `crates/cli/src/checks/build/mod.rs`

**Changes:**
```rust
/// Resolve build targets: explicit config > auto-detection
fn resolve_targets(ctx: &CheckContext, language: ProjectLanguage) -> Vec<String> {
    // Use explicit config if provided
    if !ctx.config.check.build.targets.is_empty() {
        return ctx.config.check.build.targets.clone();
    }

    // Fall back to auto-detection
    get_build_targets(ctx.root, language)
}
```

**Update `run()` to use `resolve_targets()`:**
```rust
let targets = resolve_targets(ctx, language);
```

**Verification:**
```bash
cargo test --test specs build_detects  # Existing detection tests still pass
```

### Phase 2: Per-Target Threshold Lookup

**Goal:** Look up size threshold for each target, with per-target overrides.

**File:** `crates/cli/src/checks/build/mod.rs`

**Add helper function:**
```rust
use crate::config::duration::parse_size;

/// Get size threshold for a target: per-target > global > None
fn get_size_threshold(ctx: &CheckContext, target: &str) -> Option<u64> {
    // Check per-target config first
    if let Some(target_config) = ctx.config.check.build.target.get(target) {
        if let Some(ref size_str) = target_config.size_max {
            return parse_size(size_str).ok();
        }
    }

    // Fall back to global threshold
    ctx.config.check.build.size_max.as_ref()
        .and_then(|s| parse_size(s).ok())
}
```

**Note:** `parse_size` is implemented in `crates/cli/src/config/duration.rs` and handles formats like `"10 MB"`, `"500 KB"`, `"1 GB"`.

**Verification:**
```bash
cargo test --test specs build_per_target  # Per-target threshold test
```

### Phase 3: Size Threshold Violations

**Goal:** Generate `size_exceeded` violations when binary exceeds threshold.

**File:** `crates/cli/src/checks/build/mod.rs`

**Add violation generation in `run()`:**
```rust
use crate::check::Violation;

fn run(&self, ctx: &CheckContext) -> CheckResult {
    // ... existing setup ...

    let mut violations = Vec::new();

    // Measure binary sizes and check thresholds
    for target in targets {
        if let Some(size) = measure_binary_size(ctx.root, &target, language) {
            metrics.sizes.insert(target.clone(), size);

            // Check threshold
            if let Some(threshold) = get_size_threshold(ctx, &target) {
                if size > threshold {
                    violations.push(Violation {
                        violation_type: "size_exceeded".to_string(),
                        target: Some(target.clone()),
                        value: Some(size as i64),
                        threshold: Some(threshold as i64),
                        advice: "Reduce binary size. Check for unnecessary dependencies.".to_string(),
                        ..Default::default()
                    });
                }
            }
        }
    }

    // ... build time logic (unchanged) ...

    let passed = violations.is_empty();
    CheckResult {
        name: self.name().to_string(),
        passed,
        violations,
        metrics: Some(metrics.to_json()),
    }
}
```

**Verification:**
```bash
cargo test --test specs build_size_exceeded  # Size violation tests
```

### Phase 4: Missing Target Violations

**Goal:** Generate `missing_target` violations for explicitly configured targets that don't exist.

**File:** `crates/cli/src/checks/build/mod.rs`

**Add check for missing targets:**
```rust
// In run(), after measuring all targets
for target in &targets {
    if !metrics.sizes.contains_key(target) {
        // Target was configured but not found/measured
        violations.push(Violation {
            violation_type: "missing_target".to_string(),
            target: Some(target.clone()),
            advice: "Configured build target not found. Verify target exists and builds successfully.".to_string(),
            ..Default::default()
        });
    }
}
```

**Important:** Only generate `missing_target` when targets are explicitly configured, not when auto-detected (auto-detection already filters to existing targets).

**Verification:**
```bash
cargo test --test specs build_violation_type_is_missing_target
```

### Phase 5: Update Specs

**Goal:** Remove `#[ignore]` from now-passing specs.

**File:** `tests/specs/checks/build.rs`

**Specs to enable:**
1. `build_size_exceeded_generates_violation` - Remove ignore
2. `build_size_under_threshold_passes` - Remove ignore
3. `build_per_target_size_max` - Remove ignore
4. `build_violation_type_is_size_exceeded` - Remove ignore
5. `build_violation_type_is_missing_target` - Remove ignore
6. `build_size_exceeded_has_correct_advice` - Remove ignore

**Verification:**
```bash
cargo test --test specs -- checks_build
make check
```

## Key Implementation Details

### Threshold Resolution Order

Per spec (`docs/specs/checks/build.md#configuration`):
1. `[check.build.target.<name>].size_max` - Per-target override
2. `[check.build].size_max` - Global default
3. None - No threshold (always pass)

### Violation Types from Spec

| Type | When Generated | Fields |
|------|----------------|--------|
| `size_exceeded` | `size > threshold` | target, value, threshold, advice |
| `missing_target` | Explicit target not found | target, advice |

### Advice Messages

From `docs/specs/checks/build.md#fail-threshold-exceeded`:

```rust
const ADVICE_SIZE_EXCEEDED: &str =
    "Reduce binary size. Check for unnecessary dependencies.";
const ADVICE_MISSING_TARGET: &str =
    "Configured build target not found. Verify target exists and builds successfully.";
```

### Size Parsing

The existing `parse_size()` in `crates/cli/src/config/duration.rs` handles:
- `"10 MB"` → 10,485,760 bytes (1024-based)
- `"500 KB"` → 512,000 bytes
- `"1 GB"` → 1,073,741,824 bytes

## Verification Plan

### Unit Tests

```bash
# Run all build specs
cargo test --test specs -- checks_build

# Run specific threshold tests
cargo test --test specs build_size_exceeded
cargo test --test specs build_per_target
cargo test --test specs build_violation_type
```

### Integration Verification

```bash
# Test on quench itself with tiny threshold (should fail)
quench check --build --ci -o json 2>&1 | jq '.checks[] | select(.name == "build")'

# Verify violation structure
cat > /tmp/test-quench.toml << 'EOF'
version = 1
[check.build]
check = "error"
size_max = "100 bytes"
EOF

cd /tmp/test-project && quench check --build --ci -c /tmp/test-quench.toml -o json
```

### Checklist

- [ ] `resolve_targets()` uses explicit config when set
- [ ] `get_size_threshold()` checks per-target before global
- [ ] `size_exceeded` violation generated with target, value, threshold fields
- [ ] `missing_target` violation generated for configured-but-missing targets
- [ ] All relevant `#[ignore]` attributes removed from specs
- [ ] `make check` passes

### Exit Criteria

All the following specs pass:

1. `build_detects_bin_from_cargo_toml` - Target auto-detection (existing)
2. `build_detects_default_binary_from_main_rs` - Default binary (existing)
3. `build_detects_multiple_bins` - Multiple targets (existing)
4. `build_size_exceeded_generates_violation` - Size threshold violation
5. `build_size_under_threshold_passes` - Size within threshold
6. `build_per_target_size_max` - Per-target override
7. `build_violation_type_is_size_exceeded` - Violation type correctness
8. `build_violation_type_is_missing_target` - Missing target handling
9. `build_size_exceeded_has_correct_advice` - Advice message
