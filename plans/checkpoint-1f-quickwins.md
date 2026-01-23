# Checkpoint 1F: Quick Wins Cleanup

**Root Feature:** `quench-409c`

## Overview

Quick cleanup pass to consolidate duplicate code, assess placeholder implementations, and ensure the codebase is free of backwards-compat shims and dead code. After analysis, the codebase is already quite clean - clippy passes without warnings, no dead code detected, and no backwards-compatibility shims present.

**Key finding:** The main cleanup opportunity is consolidating the repetitive check enable/disable flag logic in `cli.rs`.

## Project Structure

```
quench/
├── crates/cli/
│   └── src/
│       ├── cli.rs              # Check flag logic (consolidate)
│       ├── checks/
│       │   ├── mod.rs          # Check registry (document stubs)
│       │   ├── stub.rs         # Stub check impl (keep - intentional)
│       │   └── git.rs          # Git check (stub comment is accurate)
│       ├── runner_tests.rs     # MockBehavior::Skip (keep - KEEP UNTIL annotation)
│       └── main.rs             # Report command (assess)
└── plans/
    └── checkpoint-1f-quickwins.md
```

## Dependencies

No new dependencies required. This is a cleanup-only checkpoint.

## Implementation Phases

### Phase 1: Audit Current State

**Goal:** Confirm the cleanup opportunities and verify no additional issues exist.

**Tasks:**
1. Run `cargo clippy --all-targets --all-features -- -D warnings` to verify no warnings
2. Run `cargo build --all` to verify no unused code warnings
3. Document findings for each cleanup category:

| Category | Finding |
|----------|---------|
| Backwards-compat code | None found |
| Dead code | None (clippy clean) |
| Unused imports | None (clippy clean) |
| Partial refactors | None - stubs are intentional placeholders |
| TODO for completed work | None found |
| Duplicate code | `cli.rs` lines 144-201 |
| Scaffolding/placeholder | Stubs are intentional, not scaffolding |

**Verification:** `make check` passes, confirming clean baseline.

---

### Phase 2: Consolidate Check Flag Logic

**Goal:** Reduce duplicate code in CLI argument processing.

**Location:** `crates/cli/src/cli.rs` lines 144-201

**Current state:** Two methods with 8 near-identical if-statements each:
```rust
pub fn enabled_checks(&self) -> Vec<String> {
    let mut enabled = Vec::new();
    if self.cloc { enabled.push("cloc".to_string()); }
    if self.escapes { enabled.push("escapes".to_string()); }
    // ... 6 more identical patterns
    enabled
}

pub fn disabled_checks(&self) -> Vec<String> {
    let mut disabled = Vec::new();
    if self.no_cloc { disabled.push("cloc".to_string()); }
    if self.no_escapes { disabled.push("escapes".to_string()); }
    // ... 6 more identical patterns
    disabled
}
```

**Approach:** Use a macro to generate the repetitive logic:

```rust
/// Collect check names from boolean flags.
macro_rules! collect_checks {
    ($self:expr, $($flag:ident => $name:expr),+ $(,)?) => {{
        let mut checks = Vec::new();
        $(
            if $self.$flag {
                checks.push($name.to_string());
            }
        )+
        checks
    }};
}

impl CheckArgs {
    pub fn enabled_checks(&self) -> Vec<String> {
        collect_checks!(self,
            cloc => "cloc",
            escapes => "escapes",
            agents => "agents",
            docs => "docs",
            tests_check => "tests",
            git => "git",
            build => "build",
            license => "license",
        )
    }

    pub fn disabled_checks(&self) -> Vec<String> {
        collect_checks!(self,
            no_cloc => "cloc",
            no_escapes => "escapes",
            no_agents => "agents",
            no_docs => "docs",
            no_tests => "tests",
            no_git => "git",
            no_build => "build",
            no_license => "license",
        )
    }
}
```

**Alternative:** If macros are undesirable, use a const array approach:
```rust
const CHECK_FLAGS: &[(&str, fn(&CheckArgs) -> bool, fn(&CheckArgs) -> bool)] = &[
    ("cloc", |a| a.cloc, |a| a.no_cloc),
    ("escapes", |a| a.escapes, |a| a.no_escapes),
    // ...
];
```

**Verification:**
- `cargo test -p quench cli` passes
- Existing CLI tests still pass

---

### Phase 3: Assess Placeholder Implementations

**Goal:** Document intentional placeholders vs cleanup candidates.

**Tasks:**

1. **Stub checks** (`checks/stub.rs`, `checks/mod.rs`):
   - Status: **Keep** - Intentional placeholders for unimplemented checks
   - These allow CLI flags (`--escapes`, `--agents`, etc.) to work
   - Future phases will implement: escapes, agents, docs, tests, build, license
   - No action needed

2. **GitCheck stub** (`checks/git.rs:32`):
   - Status: **Keep** - Comment accurately describes current state
   - The comment "pass for now (stub)" is documentation, not dead code
   - Full implementation is a future feature
   - No action needed

3. **MockBehavior::Skip** (`runner_tests.rs:22-23`):
   - Status: **Keep** - Has explicit `KEEP UNTIL: Phase 050` annotation
   - Will be used when skip behavior testing is implemented
   - No action needed

4. **Report command** (`main.rs:214-220`):
   - Status: **Assess** - Currently prints placeholder messages
   - Options:
     a) Keep as-is (entry point for future metrics feature)
     b) Hide from CLI with `#[arg(hide = true)]` until implemented
     c) Remove entirely and re-add later
   - Recommendation: Keep as-is - it's a registered command, not scaffolding

**Verification:** Document decisions in commit message.

---

### Phase 4: Final Verification

**Goal:** Ensure all quality gates pass after changes.

**Tasks:**
1. Run full check suite:
   ```bash
   make check
   ```

2. Verify spec tests pass:
   ```bash
   cargo test -p quench --test specs
   ```

3. Verify CLI behavior unchanged:
   ```bash
   cargo run -- check --help
   cargo run -- check --cloc
   cargo run -- check --no-cloc
   ```

4. Commit changes with findings summary.

**Verification:** `make check` passes, CLI behavior unchanged.

## Key Implementation Details

### Why Keep Stub Checks?

The stub checks serve multiple purposes:
1. Enable CLI flags (`--escapes`, `--agents`, etc.) to work now
2. Provide correct "enabled by default" semantics
3. Allow check filtering logic to work correctly
4. Document intended check list for future implementation

Removing them would break the CLI and require re-adding later.

### Why Use a Macro for Check Flags?

The macro approach:
- Reduces 32 lines to ~20 lines
- Eliminates copy-paste errors when adding new checks
- Keeps the flag-to-name mapping explicit and maintainable
- Compiles to identical code (zero runtime overhead)

### Items NOT Cleaned Up (Intentionally Kept)

| Item | Location | Reason |
|------|----------|--------|
| StubCheck | `checks/stub.rs` | Intentional placeholder for unimplemented checks |
| 5 stub check instances | `checks/mod.rs` | Enable CLI flags, document future checks |
| "pass for now (stub)" | `git.rs:32` | Accurate documentation of current state |
| MockBehavior::Skip | `runner_tests.rs:22` | Explicit "KEEP UNTIL: Phase 050" annotation |
| run_report placeholder | `main.rs:214` | Entry point for future metrics feature |

## Verification Plan

1. **Clippy clean:** `cargo clippy --all-targets --all-features -- -D warnings` passes
2. **Tests pass:** `cargo test --all` passes
3. **CLI unchanged:** Check enable/disable flags work identically
4. **Spec tests pass:** `cargo test -p quench --test specs` passes
5. **Full check:** `make check` passes

### Success Criteria

- [ ] Duplicate check flag logic consolidated
- [ ] Stub check decisions documented
- [ ] No behavior changes (tests pass)
- [ ] `make check` passes
- [ ] Commit includes cleanup audit findings
