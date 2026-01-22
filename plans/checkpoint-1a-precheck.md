# Checkpoint 1A: Pre-Checkpoint Fix - CLI Runs

**Root Feature:** `quench-d48b`

## Overview

Verify and fix any formatting, lint failures, and failing specs/tests before checkpoint validation. This ensures all Phase 001-040 work is complete and the codebase is in a clean, verifiable state.

**Current State**: Phase 040 implementation complete. All check framework specs implemented and passing.

**End State**: All `make check` commands pass, no `#[ignore]` tags on implemented specs, clean build.

## Project Structure

No structural changes. This checkpoint validates existing code:

```
crates/cli/src/
├── check.rs            # Check trait and types (Phase 040)
├── checks/             # Check implementations (Phase 040)
│   ├── mod.rs          # Registry (8 checks)
│   ├── cloc.rs         # Lines of code check
│   ├── git.rs          # Git check
│   └── stub.rs         # Stub for unimplemented checks
├── runner.rs           # Parallel check runner (Phase 040)
├── cli.rs              # Check toggle flags (Phase 040)
└── main.rs             # Check runner integration (Phase 040)

tests/specs/
├── checks.rs           # Phase 035 specs (now passing)
├── file_walking.rs     # File walking specs
└── output.rs           # Output format specs
```

## Dependencies

No new dependencies. Validation only.

## Implementation Phases

### Phase 1A.1: Format Check

**Goal**: Ensure all code is properly formatted.

**Commands**:
```bash
cargo fmt --all -- --check
```

**If failing**: Run `cargo fmt --all` to auto-fix formatting issues.

**Expected**: Clean output (no formatting differences).

---

### Phase 1A.2: Lint Check

**Goal**: Ensure no clippy warnings.

**Commands**:
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

**If failing**: Fix each warning. Common fixes:
- Unused imports: Remove them
- Dead code: Remove or add `#[allow(dead_code)]` with justification
- Redundant patterns: Simplify match arms
- Missing docs: Add documentation or `#[allow(missing_docs)]`

**Expected**: `Finished` with no warnings.

---

### Phase 1A.3: Test Suite

**Goal**: All specs from phases 003, 015, 025, 035 pass.

**Commands**:
```bash
cargo test --all
```

**Expected results**:
- Unit tests: All pass
- Spec tests: 77+ pass, 0 fail
- Allowed ignored: Only `bench-deep` fixture tests (4 tests)

**If failing**: Debug specific tests:
```bash
# Run single test with output
cargo test test_name -- --nocapture

# Run specific test file
cargo test --test specs checks::
```

---

### Phase 1A.4: Ignore Tag Audit

**Goal**: Verify all implemented specs have `#[ignore]` removed.

**Commands**:
```bash
grep -r '#\[ignore' tests/specs/*.rs
```

**Expected**: Only see:
- `tests/specs/CLAUDE.md` - Documentation references
- `tests/specs/file_walking.rs` - `bench-deep` fixture tests (future work)

**Prohibited**: Any `#[ignore = "TODO: Phase 035"]` or similar for implemented features.

---

### Phase 1A.5: Build Verification

**Goal**: Clean release build.

**Commands**:
```bash
cargo build --all
```

**Expected**: `Finished` with no errors or warnings.

---

### Phase 1A.6: Full CI Check

**Goal**: Complete CI validation.

**Commands**:
```bash
make check
```

This runs:
1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all`
4. `cargo build --all`
5. `./scripts/bootstrap` (file sizes, test conventions)
6. `cargo audit` (security vulnerabilities)
7. `cargo deny check licenses bans sources` (license compliance)

**Expected**: All checks pass.

## Key Implementation Details

### Test Conventions

Unit tests use sibling `_tests.rs` files:
```rust
// src/parser.rs
#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
```

Test files begin with:
```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
```

### Current Ignored Tests

Only 4 tests are allowed to be ignored (all for future `bench-deep` fixture):
- `file_walking_respects_custom_depth_limit`
- `file_walking_respects_default_depth_limit`
- `file_walking_uses_iterative_traversal`
- `file_walking_warns_on_depth_limit_in_verbose`

### Known Check Status

All 8 checks registered:
- `cloc` - Implemented (counts lines of code)
- `git` - Implemented (checks git state)
- `escapes`, `agents`, `docs`, `tests`, `build`, `license` - Stubs (return empty results)

## Verification Plan

### Automated Verification

```bash
# Full CI check
make check

# Expected output: All checks pass
```

### Manual Verification

1. **Help shows all check flags**:
   ```bash
   cargo run -- check --help | grep -E '\-\-(no\-)?cloc'
   ```

2. **Check toggles work**:
   ```bash
   cargo run -- check --cloc    # Only cloc
   cargo run -- check --no-cloc # All except cloc
   ```

3. **JSON output valid**:
   ```bash
   cargo run -- check --format json | jq .
   ```

### Success Criteria

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --all` shows 77+ passed, 0 failed
- [ ] Only 4 `bench-deep` tests ignored
- [ ] `cargo build --all` succeeds
- [ ] `./scripts/bootstrap` passes
- [ ] `cargo audit` shows no vulnerabilities
- [ ] `cargo deny check` shows bans ok, licenses ok, sources ok
