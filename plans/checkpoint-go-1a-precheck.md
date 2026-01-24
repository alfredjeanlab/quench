# Pre-checkpoint: Fix Formatting, Lint, and Failing Tests

**Root Feature:** `quench-32e5`

## Overview

Verification checkpoint to ensure the codebase is clean before proceeding. This plan validates that all formatting, linting, and tests pass, fixing any issues found.

## Current Status

All checks pass as of checkpoint analysis:

| Check | Status |
|-------|--------|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --all` | PASS (255 passed, 0 failed, 6 ignored) |
| `./scripts/bootstrap` | PASS |
| `cargo audit` | PASS (1 allowed warning: bincode unmaintained) |
| `cargo deny check licenses bans sources` | PASS |

### Ignored Tests (Not Phase 451-470)

The 6 ignored tests are deferred work items, not Phase 451-470 specs:

1. **Multi-line attribute support** (FIXME - deferred feature):
   - `tests/specs/adapters/rust.rs:199` - multi-line `#[cfg(test)]`
   - `tests/specs/adapters/rust.rs:467` - multi-line `#[allow(...)]`

2. **Bench-deep fixture** (TODO - infrastructure):
   - `tests/specs/modes/file_walking.rs:152`
   - `tests/specs/modes/file_walking.rs:168`
   - `tests/specs/modes/file_walking.rs:182`
   - `tests/specs/modes/file_walking.rs:209`

No Phase 451-470 specs are ignored.

## Dependencies

None required. This is a verification-only checkpoint.

## Implementation Phases

### Phase 1: Verify All Checks Pass

Run the complete check suite to confirm clean state:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
./scripts/bootstrap
cargo audit
cargo deny check licenses bans sources
```

All checks should pass. If any fail, fix the issues before proceeding.

### Phase 2: Run `make check`

Verify the consolidated check target passes:

```bash
make check
```

This runs all checks in sequence and must complete successfully.

## Key Implementation Details

### No Fixes Required

Current analysis shows all checks pass. This checkpoint is verification-only.

### Ignored Tests Are Acceptable

The 6 ignored tests fall into two categories:

1. **FIXME tests**: Multi-line attribute parsing is a known limitation tracked for future work
2. **TODO fixture tests**: Require `bench-deep` fixture creation, which is infrastructure work

Neither category blocks Phase 451-470 completion.

### Bincode Advisory

The `cargo audit` warning about bincode (RUSTSEC-2025-0141) is an allowed warning. The Makefile's `cargo deny check` skips advisories, running only `licenses bans sources`.

## Verification Plan

1. Run `cargo fmt --all -- --check` - expect no output (clean)
2. Run `cargo clippy --all-targets --all-features -- -D warnings` - expect no warnings
3. Run `cargo test --all` - expect 255 passed, 0 failed, 6 ignored
4. Run `./scripts/bootstrap` - expect "All bootstrap checks passed"
5. Run `make check` - expect success

If all pass, the pre-checkpoint is complete.
