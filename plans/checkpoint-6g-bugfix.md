# Checkpoint 6G: Bug Fixes - Dogfooding Milestone 1

**Root Feature:** `quench-2bcc`

## Overview

This checkpoint addresses known bugs and incomplete features discovered during dogfooding milestone 1. The primary issues are:

1. **Multi-line `#[cfg(test)]` not parsed** - Rust adapter fails to detect test code in multi-line cfg attributes
2. **Multi-line `#[allow(...)]` not parsed** - Rust adapter fails to detect suppressions spanning multiple lines
3. **Missing `bench-deep` fixture** - 4 tests ignored due to missing deep directory fixture
4. **Unmaintained dependency warning** - bincode crate advisory (allowed but should be addressed)

All core functionality is working (`make check` passes), but these edge cases reduce quality signal accuracy.

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── adapter/rust/
│   │   ├── cfg_test.rs         # UPDATE: Multi-line #[cfg(test)] parsing
│   │   ├── cfg_test_tests.rs   # UPDATE: Multi-line tests
│   │   ├── suppress.rs         # UPDATE: Multi-line #[allow(...)] parsing
│   │   └── suppress_tests.rs   # UPDATE: Multi-line tests
│   └── walker.rs               # VERIFY: Depth limit behavior
├── tests/
│   ├── fixtures/
│   │   ├── rust/
│   │   │   ├── multiline-cfg-test/   # NEW: Multi-line cfg fixture
│   │   │   └── multiline-allow/      # NEW: Multi-line allow fixture
│   │   └── bench-deep/               # NEW: Deep directory fixture
│   └── specs/
│       ├── adapters/rust.rs    # UPDATE: Enable ignored tests
│       └── modes/file_walking.rs # UPDATE: Enable ignored tests
└── Cargo.toml                  # EVALUATE: bincode replacement
```

## Dependencies

No new external dependencies. This checkpoint uses existing infrastructure:

- `regex` - Pattern matching (exists, will use multiline mode)
- `insta` - Snapshot testing (exists)

**Dependency evaluation:**
- `bincode 1.3.3` - Marked unmaintained (RUSTSEC-2025-0141). Currently used for cache serialization. Evaluate alternatives: `bincode2`, `postcard`, `rmp-serde`.

## Implementation Phases

### Phase 1: Multi-line `#[cfg(test)]` Support

**Goal:** Parse `#[cfg(test)]` attributes that span multiple lines.

**Current behavior:**
```rust
// This is detected:
#[cfg(test)]
mod tests { ... }

// This is NOT detected:
#[cfg(
    test
)]
mod tests { ... }
```

**File:** `crates/cli/src/adapter/rust/cfg_test.rs`

The current parser uses line-by-line matching. Update to:
1. Collect lines when `#[cfg(` is seen without closing `)]`
2. Continue accumulating until `)]` is found
3. Check if accumulated content contains `test`

**Implementation approach:**
```rust
/// Parse cfg_test attributes, handling multi-line cases.
fn find_cfg_test_spans(content: &str) -> Vec<CfgTestSpan> {
    let mut spans = Vec::new();
    let mut in_cfg = false;
    let mut cfg_start = 0;
    let mut cfg_content = String::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if !in_cfg {
            if trimmed.starts_with("#[cfg(") {
                if trimmed.contains(")]") {
                    // Single-line case
                    if is_cfg_test(trimmed) {
                        spans.push(CfgTestSpan { start: line_num, ... });
                    }
                } else {
                    // Multi-line starts here
                    in_cfg = true;
                    cfg_start = line_num;
                    cfg_content = trimmed.to_string();
                }
            }
        } else {
            cfg_content.push_str(trimmed);
            if trimmed.contains(")]") {
                // Multi-line ends here
                in_cfg = false;
                if is_cfg_test(&cfg_content) {
                    spans.push(CfgTestSpan { start: cfg_start, ... });
                }
                cfg_content.clear();
            }
        }
    }

    spans
}
```

**Fixture:** `tests/fixtures/rust/multiline-cfg-test/`
```rust
// src/lib.rs
pub fn production_code() {}

#[cfg(
    test
)]
mod tests {
    #[test]
    fn multi_line_cfg_works() {}
}
```

**Verification:**
```bash
cargo test --test specs rust_adapter_multiline_cfg_test_detected
```

### Phase 2: Multi-line `#[allow(...)]` Support

**Goal:** Parse `#[allow(...)]` and `#[expect(...)]` attributes spanning multiple lines.

**Current behavior:**
```rust
// This is detected:
#[allow(dead_code)]
fn unused() {}

// This is NOT detected:
#[allow(
    dead_code,
    unused_variables
)]
fn unused() {}
```

**File:** `crates/cli/src/adapter/rust/suppress.rs`

Update `parse_suppress_line()` to handle multi-line:
1. When `#[allow(` or `#[expect(` is seen without closing `)]`, enter multi-line mode
2. Accumulate content until `)]` is found
3. Parse the complete attribute

**Implementation approach:**
```rust
/// Parse suppress attributes from content.
/// Handles both single-line and multi-line attributes.
pub fn parse_suppress_attrs(content: &str, filter: Option<&str>) -> Vec<ParsedAttr> {
    let mut attrs = Vec::new();
    let mut pending: Option<PendingAttr> = None;

    for (line_num, line) in content.lines().enumerate() {
        if let Some(ref mut p) = pending {
            p.content.push_str(" ");
            p.content.push_str(line.trim());

            if line.contains(")]") || line.contains(")") {
                // Complete the multi-line attribute
                if let Some(attr) = parse_complete_attr(&p.content, p.start_line) {
                    attrs.push(attr);
                }
                pending = None;
            }
        } else if let Some((kind, rest)) = detect_attr_start(line) {
            if rest.contains(")") || rest.contains(")]") {
                // Single-line attribute
                if let Some(attr) = parse_suppress_line(line) {
                    attrs.push(attr.with_line(line_num));
                }
            } else {
                // Multi-line attribute starts
                pending = Some(PendingAttr {
                    content: line.trim().to_string(),
                    start_line: line_num,
                });
            }
        }
    }

    attrs
}
```

**Fixture:** `tests/fixtures/rust/multiline-allow/`
```rust
// src/lib.rs - uses check = "forbid" to trigger failure
#[allow(
    dead_code,
    unused_variables
)]
fn unused_function() {
    let _x = 1;
}
```

```toml
# quench.toml
version = 1
[rust.suppress]
check = "forbid"
```

**Verification:**
```bash
cargo test --test specs rust_adapter_multiline_allow_detected
```

### Phase 3: Create `bench-deep` Fixture

**Goal:** Enable depth limit tests by creating a fixture with deeply nested directories.

**Current state:** 4 tests ignored:
- `file_walking_respects_default_depth_limit`
- `file_walking_respects_custom_depth_limit`
- `file_walking_warns_on_depth_limit_in_verbose`
- `file_walking_uses_iterative_traversal`

**Fixture:** `tests/fixtures/bench-deep/`

Create a directory structure 150+ levels deep with small Rust files:
```
bench-deep/
├── quench.toml
├── src/
│   └── lib.rs
└── deep/
    └── level001/
        └── level002/
            └── ... (150 levels)
                └── bottom.rs
```

**Generation script:**
```bash
#!/bin/bash
# scripts/create-bench-deep.sh
FIXTURE_DIR="tests/fixtures/bench-deep"
mkdir -p "$FIXTURE_DIR/src"

# Create minimal quench.toml
cat > "$FIXTURE_DIR/quench.toml" << 'EOF'
version = 1
[project]
name = "bench-deep"
EOF

# Create lib.rs
echo 'pub fn top() {}' > "$FIXTURE_DIR/src/lib.rs"

# Create deep structure (150 levels)
CURRENT="$FIXTURE_DIR/deep"
for i in $(seq -w 1 150); do
    CURRENT="$CURRENT/level$i"
    mkdir -p "$CURRENT"
done

# Add file at bottom
echo 'fn bottom() {}' > "$CURRENT/bottom.rs"
```

**Verification:**
```bash
cargo test --test specs file_walking_respects_default_depth_limit
cargo test --test specs file_walking_respects_custom_depth_limit
```

### Phase 4: Evaluate bincode Replacement

**Goal:** Address unmaintained dependency advisory.

**Current usage:** `bincode 1.3.3` is used for cache serialization in `crates/cli/src/cache.rs`.

**Options:**
1. **postcard** - Modern, well-maintained, smaller output
2. **bincode2** - Fork of bincode with active maintenance
3. **rmp-serde** - MessagePack format, widely used
4. **Keep bincode** - Already allowed in advisory, low risk

**Evaluation criteria:**
- Serialization speed (cache is hot path)
- Output size (cache file size)
- API compatibility (minimize changes)
- Maintenance status

**Decision:** Document decision in plan, implement if low-risk. If bincode2 or postcard is drop-in compatible, switch. Otherwise, defer to future checkpoint.

**Verification:**
```bash
cargo audit  # Should show 0 warnings after switch
```

### Phase 5: Enable Ignored Tests

**Goal:** Remove `#[ignore]` from tests now that features are implemented.

**Files to update:**

1. `tests/specs/adapters/rust.rs`:
   - Remove `#[ignore]` from `rust_adapter_multiline_cfg_test_detected`
   - Remove `#[ignore]` from `rust_adapter_multiline_allow_detected`

2. `tests/specs/modes/file_walking.rs`:
   - Remove `#[ignore]` from `file_walking_respects_default_depth_limit`
   - Remove `#[ignore]` from `file_walking_respects_custom_depth_limit`
   - Remove `#[ignore]` from `file_walking_warns_on_depth_limit_in_verbose`
   - Remove `#[ignore]` from `file_walking_uses_iterative_traversal`

**Verification:**
```bash
cargo test --test specs -- --include-ignored  # All should pass
```

### Phase 6: Final Verification

**Goal:** Ensure all changes work together and pass CI.

**Steps:**
1. Run full test suite (including previously ignored tests)
2. Run `make check` for full CI validation
3. Dogfood: run quench on quench
4. Verify no regressions in snapshot tests

**Verification:**
```bash
# Full test suite
cargo test --all

# CI checks
make check

# Dogfooding
cargo run -- check
cargo run -- check -o json | jq '.passed'  # Should be true

# Previously ignored tests now passing
cargo test --test specs multiline
cargo test --test specs file_walking_respects
```

## Key Implementation Details

### Multi-line Attribute Parsing Strategy

Both `#[cfg(test)]` and `#[allow(...)]` use the same strategy:

1. **Detection:** Check if line starts with `#[attr(` or `#![attr(`
2. **Completion check:** If line contains `)` or `)]`, it's single-line
3. **Accumulation:** Otherwise, accumulate lines until `)` or `)]`
4. **Parsing:** Parse the complete accumulated content

Key considerations:
- Nested parentheses: `#[cfg(all(test, feature = "foo"))]`
- Multiple attributes on one line: Rare, handle single-line first
- Comment preservation: Keep track of preceding comments for suppression checking

### Depth Limit Test Strategy

The `bench-deep` fixture tests:
1. Default depth limit (100) skips files beyond that depth
2. Custom depth limit respects configuration
3. Verbose mode logs skipped files
4. Iterative traversal doesn't stack overflow

The fixture is intentionally deeper (150 levels) than the default limit (100) to ensure:
- Files at level 50 are scanned
- Files at level 110 are skipped with default limit
- Files at level 150 are skipped regardless

### Cache Serialization Migration

If switching from bincode:

```rust
// Old (bincode)
let data = bincode::serialize(&cache)?;
let cache: Cache = bincode::deserialize(&data)?;

// New (postcard)
let data = postcard::to_allocvec(&cache)?;
let cache: Cache = postcard::from_bytes(&data)?;

// Or (rmp-serde)
let data = rmp_serde::to_vec(&cache)?;
let cache: Cache = rmp_serde::from_slice(&data)?;
```

Cache files are stored in `.quench/cache/`. Version the format to handle migration:
```rust
const CACHE_VERSION: u8 = 2;  // Bump when format changes
```

## Verification Plan

### Phase 1 Verification
```bash
# Multi-line cfg test detection
cargo test cfg_test -- multiline
cargo test --test specs rust_adapter_multiline_cfg_test_detected
```

### Phase 2 Verification
```bash
# Multi-line allow detection
cargo test suppress -- multiline
cargo test --test specs rust_adapter_multiline_allow_detected
```

### Phase 3 Verification
```bash
# Verify fixture exists and has expected depth
find tests/fixtures/bench-deep -type d | wc -l  # Should be 150+

# Enable and run depth tests
cargo test --test specs file_walking_respects
```

### Phase 4 Verification
```bash
# Audit shows no warnings
cargo audit

# Cache still works
cargo run -- check  # Uses cache
rm -rf .quench/cache && cargo run -- check  # Rebuilds cache
```

### Phase 5 Verification
```bash
# All previously ignored tests now pass
cargo test --test specs -- multiline
cargo test --test specs -- file_walking
```

### Phase 6 (Final) Verification
```bash
# Full CI
make check

# Dogfooding
cargo run -- check
cargo run -- check --ci -o json

# Test count unchanged except ignored->passing
cargo test --test specs 2>&1 | grep -E "(passed|ignored)"
```

## Exit Criteria

- [ ] Multi-line `#[cfg(test)]` attributes detected and counted as test LOC
- [ ] Multi-line `#[allow(...)]` attributes detected for suppression checking
- [ ] `bench-deep` fixture created with 150+ nested directories
- [ ] Depth limit tests enabled and passing
- [ ] bincode dependency evaluated (switched or documented as acceptable)
- [ ] All 6 previously ignored tests now passing
- [ ] `make check` passes
- [ ] Dogfooding passes: `quench check` on quench
- [ ] No snapshot regressions
