# Checkpoint 10A: Pre-Checkpoint Fix - Dogfooding Milestone 2

**Root Feature:** `quench-10a`
**Follows:** checkpoint-9h-techdebt (merge commit handling)

## Overview

Prepare for Dogfooding Milestone 2 by verifying all fast checks pass on the quench codebase and setting up `quench check --staged` to run on every commit. This checkpoint ensures the tool is ready to validate its own commits.

**Dogfooding Milestone 2 Requirements:**
- `quench check --staged` runs on every commit
- All fast checks pass on quench codebase

**Current State:**
- `make check` passes (all 515 tests pass)
- `quench check` passes on itself (cloc, escapes, agents, docs, tests, git, placeholders)
- `quench check --staged` works but no pre-commit hook installed
- `quench.toml` exists with basic configuration

## Project Structure

```
quench/
├── .git/hooks/
│   └── pre-commit              # CREATE: pre-commit hook
├── quench.toml                 # REVIEW: ensure complete config
├── reports/
│   └── dogfood-m2.md           # CREATE: milestone verification report
└── plans/
    └── checkpoint-10a-precheck.md  # THIS FILE
```

## Dependencies

No new dependencies required. Uses existing:
- `quench` CLI (built from this repo)

## Implementation Phases

### Phase 1: Verify Current State

Confirm all fast checks pass on the quench codebase.

**Commands:**
```bash
# Build release binary
cargo build --release

# Run all fast checks
./target/release/quench check

# Run staged mode (should pass with clean working tree)
./target/release/quench check --staged

# Run with --base to check all commits since main
./target/release/quench check --base main

# Run with timing to establish baseline
./target/release/quench check --timing
```

**Expected Results:**
- All checks pass: cloc, escapes, agents, docs, tests, git, placeholders
- No violations reported
- Timing shows reasonable performance (<200ms cold, <50ms warm)

**Verification:**
- `./target/release/quench check` exits 0
- Output shows `PASS: cloc, escapes, agents, docs, tests, git, placeholders`

---

### Phase 2: Review Configuration

Review `quench.toml` to ensure complete configuration for dogfooding.

**File:** `quench.toml`

**Current:**
```toml
version = 1

[rust]
cfg_test_split = "require"

[check.cloc]
advice_test = """
Can tests be parameterized using `yare` to be more concise?
If not, split large test files into a {module}_tests/ folder.
"""
exclude = [
    "tests/fixtures/cloc/**",
    "tests/fixtures/bench-*/**",
]

[check.agents]
required = ["CLAUDE.md"]
```

**Review checklist:**
- [ ] Git check configuration (types, scopes, skip_merge)
- [ ] Escapes patterns for Rust-specific checks
- [ ] Test correlation exclusions (if needed)

**Potential additions:**
```toml
# Already has sensible defaults, but consider:
[check.git.commit]
check = "warn"          # Enable git check in warn mode
agents = true           # Require commit format in CLAUDE.md
skip_merge = true       # Skip merge commits (default)
```

**Note:** Git check is disabled by default. Decide whether to enable at "warn" or "error" level.

**Verification:**
- `./target/release/quench check --config-only` exits 0

---

### Phase 3: Install Pre-Commit Hook

Create and install a pre-commit hook that runs `quench check --staged`.

**File:** `.git/hooks/pre-commit`

```bash
#!/bin/sh
# Pre-commit hook for quench quality checks
# Installed by: checkpoint-10a-precheck

set -e

# Use local build if available, otherwise fallback to installed quench
if [ -x "./target/release/quench" ]; then
    QUENCH="./target/release/quench"
elif [ -x "./target/debug/quench" ]; then
    QUENCH="./target/debug/quench"
elif command -v quench >/dev/null 2>&1; then
    QUENCH="quench"
else
    echo "quench: not found (run 'cargo build --release')" >&2
    exit 1
fi

exec $QUENCH check --staged
```

**Installation:**
```bash
# Create hook
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/sh
# Pre-commit hook for quench quality checks

set -e

if [ -x "./target/release/quench" ]; then
    QUENCH="./target/release/quench"
elif [ -x "./target/debug/quench" ]; then
    QUENCH="./target/debug/quench"
elif command -v quench >/dev/null 2>&1; then
    QUENCH="quench"
else
    echo "quench: not found (run 'cargo build --release')" >&2
    exit 1
fi

exec $QUENCH check --staged
EOF

# Make executable
chmod +x .git/hooks/pre-commit
```

**Verification:**
- Hook exists and is executable
- Test with a trivial staged change

---

### Phase 4: Test Pre-Commit Hook

Verify the pre-commit hook works correctly.

**Test valid commit:**
```bash
# Stage a valid change
echo "# Comment" >> quench.toml
git add quench.toml

# Commit should succeed
git commit -m "test: verify pre-commit hook"

# Reset
git reset --soft HEAD~1
git checkout -- quench.toml
```

**Test invalid commit (optional):**
```bash
# Create a file that violates checks
echo "fn main() { unsafe { } }" > /tmp/bad.rs

# Stage the bad file
cp /tmp/bad.rs src/bad.rs
git add src/bad.rs

# Commit should fail
git commit -m "test: should fail"
# Expected: Non-zero exit from pre-commit hook

# Clean up
git reset HEAD src/bad.rs
rm src/bad.rs
```

**Verification:**
- Valid commits succeed with hook output
- Invalid commits are rejected (optional - depends on escapes config)

---

### Phase 5: Document Verification

Create a milestone verification report documenting the results.

**File:** `reports/dogfood-m2.md`

```markdown
# Dogfooding Milestone 2 Report

**Date:** [YYYY-MM-DD]
**Branch:** feature/checkpoint-10a-precheck

## Milestone Criteria

| Requirement | Status |
|-------------|--------|
| `quench check --staged` runs on every commit | PASS |
| All fast checks pass on quench codebase | PASS |

## Check Results

```
$ quench check
PASS: cloc, escapes, agents, docs, tests, git, placeholders
```

## Timing Baseline

```
$ quench check --timing
[timing output here]
```

## Pre-Commit Hook

- Installed: `.git/hooks/pre-commit`
- Tested: Yes
- Approach: Uses local build if available, fallback to installed quench

## Known Gaps

[None / List any issues found]

## Next Steps

- Dogfooding Milestone 2 complete
- Continue to Phase 901: CI Mode
```

**Verification:**
- Report file exists with complete information

---

### Phase 6: Final Cleanup

Commit all changes and verify the commit passes the pre-commit hook.

**Checklist:**
- [ ] Pre-commit hook installed
- [ ] quench.toml reviewed
- [ ] Verification report created
- [ ] `make check` passes
- [ ] Pre-commit hook runs on this commit

**Commit:**
```bash
git add .git/hooks/pre-commit reports/dogfood-m2.md
git commit -m "chore(dogfood): complete Dogfooding Milestone 2

- Install pre-commit hook for quench check --staged
- All fast checks pass on quench codebase
- Document verification in reports/dogfood-m2.md"
```

---

## Key Implementation Details

### Pre-Commit Hook Strategy

The hook prioritizes local builds:
1. `./target/release/quench` - Release build (fastest)
2. `./target/debug/quench` - Debug build (available during development)
3. `quench` in PATH - Installed version (fallback)

This ensures developers use the latest local version during development.

### Fast Checks Definition

"Fast checks" are those that don't require building or running tests:
- cloc: Line counting
- escapes: Pattern matching for escape hatches
- agents: Agent file validation
- docs: Documentation structure
- tests: Test correlation (commit-scoped, not execution)
- git: Commit message format
- placeholders: TODO/ignore detection

Build and license checks are "slow" and only run with `--ci`.

### Git Check Default

Git check is disabled by default (`default_enabled() -> false`). This is intentional to avoid breaking existing workflows. For dogfooding, consider enabling at "warn" level:

```toml
[check.git.commit]
check = "warn"
```

This validates commit format but doesn't block commits.

## Verification Plan

1. **Phase 1:** `./target/release/quench check` exits 0
2. **Phase 2:** `./target/release/quench check --config-only` exits 0
3. **Phase 3:** `.git/hooks/pre-commit` exists and is executable
4. **Phase 4:** Test commit succeeds with hook output
5. **Phase 5:** `reports/dogfood-m2.md` exists
6. **Phase 6:** `make check` passes, commit succeeds

**Final verification:**
```bash
make check                              # All tests pass
./target/release/quench check           # Dogfood passes
git status                              # Clean working tree
```

## Checklist

- [ ] Verify `quench check` passes on quench codebase
- [ ] Review quench.toml configuration
- [ ] Install pre-commit hook
- [ ] Test pre-commit hook with valid commit
- [ ] Create verification report in reports/dogfood-m2.md
- [ ] Run `make check`
- [ ] Commit changes (hook runs automatically)
