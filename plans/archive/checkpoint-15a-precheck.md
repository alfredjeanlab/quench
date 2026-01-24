# Checkpoint 15A: Pre-Checkpoint Validation - Init Command Complete

**Plan:** `checkpoint-15a-precheck`
**Root Feature:** `quench-init`
**Depends On:** Phase 1547 (Config Schema - [lang.policy].check - Implementation)

## Overview

Validate that all pre-checkpoint requirements pass before marking the Init Command checkpoint as complete. This pre-check ensures no formatting issues, lint warnings, or failing tests remain after Phases 1505-1547.

**Current State:**
- `cargo fmt --all -- --check`: ✅ Passing
- `cargo clippy --all-targets --all-features -- -D warnings`: ✅ Passing
- `cargo test --all`: ✅ 411 tests passing, 0 ignored
- `cargo build --all`: ✅ Passing
- `cargo audit`: ✅ No vulnerabilities
- `cargo deny check`: ✅ All checks pass
- `make check`: ✅ Passing

**Goal:** Confirm all checks pass and checkpoint is ready.

## Project Structure

```
crates/cli/src/
├── commands/init.rs    # --with flag, auto-detection
├── config.rs           # [lang.cloc], [lang.policy] config parsing
├── checks/
│   ├── cloc.rs         # Per-language check level support
│   └── policy.rs       # Per-language check level support

tests/specs/
├── cli_init.rs         # Init command behavioral specs
├── checks_cloc_lang.rs # Per-language cloc specs
└── checks_policy_lang.rs # Per-language policy specs
```

## Dependencies

No new dependencies required. Validation uses existing:
- `cargo fmt`, `cargo clippy`, `cargo test`
- `cargo audit`, `cargo deny`

## Implementation Phases

### Phase 1: Format Validation

**Goal:** Verify no formatting issues exist.

**Command:**
```bash
cargo fmt --all -- --check
```

**Expected:** No output (indicates clean formatting)

**If failures exist:**
```bash
cargo fmt --all
```

**Verification:**
```bash
cargo fmt --all -- --check && echo "PASS"
```

---

### Phase 2: Lint Validation

**Goal:** Verify no clippy warnings.

**Command:**
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

**Expected:** Completes without errors

**If failures exist:**
- Fix each warning according to clippy suggestions
- Common fixes: unused imports, dead code, missing docs

**Verification:**
```bash
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -5
# Should show "Finished" without warnings
```

---

### Phase 3: Test Validation

**Goal:** Verify all tests pass and no specs are ignored.

**Command:**
```bash
cargo test --all
```

**Expected:** All 411 tests passing

**Check for ignored tests:**
```bash
grep -rn '#\[ignore' tests/specs/*.rs
# Should return empty
```

**If failures exist:**
- Analyze test output
- Fix implementation or update test expectations
- Ensure Phase 1505-1547 specs all pass

**Verification:**
```bash
cargo test --all 2>&1 | grep "test result"
# Should show: ok. 411 passed; 0 failed; 0 ignored
```

---

### Phase 4: Build and Security

**Goal:** Verify clean build and no security issues.

**Commands:**
```bash
cargo build --all
cargo audit
cargo deny check
```

**Expected:** All pass without errors

**If failures exist:**
- `cargo build`: Fix compilation errors
- `cargo audit`: Update vulnerable dependencies
- `cargo deny`: Fix license/ban violations

**Verification:**
```bash
cargo build --all && cargo audit && cargo deny check
```

---

### Phase 5: Full Suite Validation

**Goal:** Run complete `make check` to confirm everything.

**Command:**
```bash
make check
```

**Expected:** All checks pass in sequence

**Verification checklist:**
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes
- [ ] `cargo test` passes (411 tests)
- [ ] `cargo build` passes
- [ ] `quench check` passes on self
- [ ] `cargo audit` passes
- [ ] `cargo deny check` passes

---

### Phase 6: Checkpoint Confirmation

**Goal:** Document checkpoint completion.

**Actions:**
1. Verify Phase 1547 deliverables:
   - `[lang.policy].check` implemented for all languages
   - Per-language policy check level works
   - Unit tests for per-language policy config

2. Confirm roadmap checkpoint items (from `.3-roadmap-init.md`):
   - [x] `quench init` on empty dir creates full template
   - [x] `quench init` on Rust project detects and adds `[rust]` section
   - [x] `quench init` on project with CLAUDE.md updates `[check.agents]`
   - [x] `quench init --with shell` creates shell-only config
   - [x] `quench init --with rust,claude` creates combined config
   - [x] All existing init tests updated and passing

**Commit message template:**
```
checkpoint(init): complete init command feature set

All Phase 1505-1547 specs pass:
- Init --with flag and auto-detection
- Language/agent auto-detection
- Per-language [cloc].check and [policy].check config
- Template generation with dotted keys

make check passes:
- 411 tests passing
- No clippy warnings
- No formatting issues
- No security vulnerabilities
```

## Key Implementation Details

### Phase 1547 Expected Implementation

Per-language policy check level:
```toml
# Each language can have independent policy check level
[rust.policy]
check = "error"    # Default for rust

[golang.policy]
check = "warn"     # Different level for go

[shell.policy]
check = "off"      # Disabled for shell
```

### Test Coverage

Behavioral specs that must pass:
- `cli_init::init_with_skips_auto_detection`
- `cli_init::init_without_with_triggers_auto_detection`
- `checks_policy_lang::rust_policy_check_off_disables_policy`
- `checks_policy_lang::golang_policy_check_off_disables_policy`
- `checks_policy_lang::javascript_policy_check_off_disables_policy`
- `checks_policy_lang::shell_policy_check_off_disables_policy`
- `checks_policy_lang::rust_policy_check_warn_reports_without_failing`
- `checks_policy_lang::golang_policy_check_warn_reports_without_failing`
- `checks_policy_lang::each_language_can_have_independent_policy_check_level`
- `checks_policy_lang::mixed_levels_go_warn_rust_error`

## Verification Plan

### Quick Validation
```bash
make check
```

### Detailed Validation
```bash
# Format
cargo fmt --all -- --check

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Tests
cargo test --all

# Build
cargo build --all

# Security
cargo audit
cargo deny check
```

### Phase 1505-1547 Specific Tests
```bash
# Init command specs
cargo test --test specs cli_init

# Per-language policy specs
cargo test --test specs checks_policy_lang
```

### Final Check
```bash
# Confirm no ignored tests remain
grep -r '#\[ignore' tests/specs/*.rs | wc -l
# Should be 0
```
