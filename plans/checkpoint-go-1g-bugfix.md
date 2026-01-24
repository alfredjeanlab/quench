# Checkpoint Go-1G: Bug Fix Stabilization

**Plan:** `checkpoint-go-1g-bugfix`
**Root Feature:** `quench-0d07`

## Overview

Post-checkpoint stabilization phase to fix bugs introduced during prior refactors, performance optimizations, and quick wins cleanup. This checkpoint is fixes-only - no new features.

## Current State Assessment

### Test Suite Status

Full test suite executed via `make check`:

| Test Category | Count | Status |
|--------------|-------|--------|
| Unit tests | 505 | PASS |
| Spec tests | 269 | PASS |
| Ignored (expected) | 13 | OK |

**Ignored tests** (documented deferrals):
- 7 unit benchmarks (run separately)
- 4 `bench-deep` fixture tests (fixture not created yet)
- 2 multi-line attribute parsing tests (deferred to future work)

### Quality Gates Status

| Gate | Status |
|------|--------|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --all` | PASS |
| `cargo build --all` | PASS |
| `./scripts/bootstrap` | PASS |
| `cargo audit` | PASS (1 allowed warning: bincode unmaintained) |
| `cargo deny check` | PASS |

### Bugs Found

| Bug | Source | Severity | Fix |
|-----|--------|----------|-----|
| `go-multi` fixture uses deprecated `[golang]` key | checkpoint-go-1f quick wins | Low | Update to `[go]` |

The quick wins cleanup (checkpoint-go-1f) removed the `golang` config alias, but the `go-multi` test fixture still uses it. This causes a warning during tests:

```
quench: warning: tests/fixtures/go-multi/quench.toml: unrecognized field `golang` (ignored)
```

## Project Structure

Files to fix:

```
tests/fixtures/go-multi/
└── quench.toml    # Line 6: [golang] → [go]
```

## Dependencies

None - internal fix only.

## Implementation Phases

### Phase 1: Fix `golang` Config Key Regression

**Goal:** Update go-multi fixture to use `[go]` instead of deprecated `[golang]`.

**File:** `tests/fixtures/go-multi/quench.toml`

**Before:**
```toml
version = 1

[project]
name = "go-multi"

[golang]
targets = ["cmd/server", "cmd/cli"]
```

**After:**
```toml
version = 1

[project]
name = "go-multi"

[go]
targets = ["cmd/server", "cmd/cli"]
```

**Verification:**
```bash
cargo run --quiet -p quench -- check tests/fixtures/go-multi 2>&1 | grep -i warning
# Should produce no output (no warnings)
```

**Milestone:** go-multi fixture runs without deprecation warning.

---

### Phase 2: Re-run Go-specific Tests

**Goal:** Verify Go adapter tests still pass after the fixture fix.

```bash
cargo test golang
cargo test go
```

**Expected:**
- 24 behavioral specs in `tests/specs/adapters/golang.rs`
- 40+ unit tests in `crates/cli/src/adapter/go/*_tests.rs`
- All tests pass

**Milestone:** All Go-related tests pass.

---

### Phase 3: Full Test Suite Verification

**Goal:** Ensure no regressions from the fix.

```bash
make check
```

This runs:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `./scripts/bootstrap`
- `cargo audit`
- `cargo deny check`

**Milestone:** All quality gates pass.

---

### Phase 4: Verify Fixture Output

**Goal:** Confirm quench produces correct output on key fixtures.

```bash
# Go fixtures
cargo run --quiet -p quench -- check tests/fixtures/go-simple 2>&1
cargo run --quiet -p quench -- check tests/fixtures/go-multi 2>&1

# Main project
cargo run --quiet -p quench -- check 2>&1
```

**Expected:**
- No unexpected warnings
- Output shows expected check results
- go-multi specifically should have no `unrecognized field` warning

**Milestone:** All fixture outputs correct.

## Key Implementation Details

### Why This Bug Occurred

The checkpoint-go-1f quick wins plan removed the `golang` backwards compatibility shim from the config parser:

1. Removed `golang` field from `FlexibleConfig` struct
2. Removed `"golang"` from `KNOWN_KEYS` array
3. Removed `.or(flexible.golang.as_ref())` fallback

This was correct for production code, but the test fixture wasn't updated to match.

### Scope of Fix

- **1 file** affected: `tests/fixtures/go-multi/quench.toml`
- **1 line** to change: `[golang]` → `[go]`
- **No code changes** - only test fixture update

### What We're NOT Fixing

Items explicitly deferred to future work:

1. **Multi-line attribute parsing** - 2 ignored specs in `tests/specs/adapters/rust.rs`
2. **`bench-deep` fixture** - 4 ignored specs in `tests/specs/modes/file_walking.rs`
3. **bincode unmaintained warning** - Allowed in `cargo audit` (not a security issue)

## Verification Plan

| Phase | Command | Expected Result |
|-------|---------|-----------------|
| 1 | `cargo run -p quench -- check tests/fixtures/go-multi 2>&1` | No `unrecognized field` warning |
| 2 | `cargo test golang && cargo test go` | All tests pass |
| 3 | `make check` | All quality gates pass |
| 4 | `cargo run -p quench -- check` | PASS: cloc, escapes, agents |

## Summary

| Phase | Task | Effort | Status |
|-------|------|--------|--------|
| 1 | Fix `golang` → `go` in go-multi fixture | ~2 min | [ ] Pending |
| 2 | Re-run Go tests | ~1 min | [ ] Pending |
| 3 | Full test suite | ~2 min | [ ] Pending |
| 4 | Verify fixture output | ~1 min | [ ] Pending |

**Total estimated effort:** ~6 minutes

**Checkpoint Criteria:**
- [x] Run full test suite, fix any regressions
- [ ] Re-validate checkpoint criteria still pass
- [ ] Fix any issues introduced by quick wins cleanup
- [ ] Fix any issues introduced by perf optimizations (none found)
- [ ] Ensure quench check still produces correct output on fixtures
