# Phase 401: Shell Adapter - Specs

**Root Feature:** `quench-a0ea`

## Overview

Write behavioral specifications (tests) for the Shell language adapter. This phase defines expected behavior through ignored specs that will drive subsequent implementation phases. The Shell adapter provides:

- **Auto-detection** via `*.sh` files in root, `bin/`, or `scripts/`
- **Default patterns** for shell source and test files
- **Shell-specific escape patterns** (`set +e`, `eval`)
- **Shellcheck suppress** handling (`# shellcheck disable=`)

Reference docs:
- `docs/specs/langs/shell.md`
- `docs/specs/10-language-adapters.md`
- `docs/specs/checks/escape-hatches.md`

## Project Structure

```
quench/
├── tests/
│   ├── specs/
│   │   └── adapters/
│   │       ├── mod.rs          # UPDATE: Add shell module
│   │       └── shell.rs        # NEW: Shell adapter behavioral specs
│   └── fixtures/
│       └── shell/              # NEW: Shell-specific fixtures
│           ├── auto-detect/    # Has *.sh in scripts/, no quench.toml
│           ├── set-e-ok/       # set +e with # OK: comment
│           ├── set-e-fail/     # set +e without # OK: comment
│           ├── eval-ok/        # eval with # OK: comment
│           ├── eval-fail/      # eval without # OK: comment
│           ├── shellcheck-forbid/ # shellcheck disable= in source
│           └── shellcheck-test/   # shellcheck disable= in test (allowed)
└── plans/
    └── phase-401.md
```

## Dependencies

No new external dependencies. Uses existing:
- Test harness from `tests/specs/prelude.rs`
- Fixture infrastructure from `tests/fixtures/`
- Adapter trait from `crates/cli/src/adapter/mod.rs`
- Existing `tests/fixtures/shell-scripts/` as reference

## Implementation Phases

### Phase 1: Add Shell Module to Adapters

Update the adapters spec module to include shell tests.

**Update `tests/specs/adapters/mod.rs`:**

```rust
//! Behavioral specs for language adapters.
//!
//! Tests that quench correctly detects and applies language-specific behavior.
//!
//! Reference: docs/specs/10-language-adapters.md

pub mod rust;
pub mod shell;  // NEW
```

**Milestone:** Module compiles, no tests yet.

**Verification:**
```bash
cargo test --test specs -- adapters::shell
```

---

### Phase 2: Auto-Detection Specs

Write specs for Shell project detection.

**Create `tests/specs/adapters/shell.rs`:**

```rust
//! Behavioral specs for the Shell language adapter.
//!
//! Tests that quench correctly:
//! - Detects Shell projects via *.sh files in root, bin/, or scripts/
//! - Applies default source/test patterns
//! - Applies Shell-specific escape patterns
//!
//! Reference: docs/specs/langs/shell.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// AUTO-DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/10-language-adapters.md#adapter-selection
///
/// > shell | *.sh files in root, bin/, or scripts/ | **/*.sh, **/*.bash
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_auto_detected_when_sh_files_in_scripts() {
    // Project has .sh files in scripts/ but no quench.toml [shell] section
    // Should still apply Shell defaults
    let result = cli().on("shell/auto-detect").json().passes();
    let checks = result.checks();

    // escapes check should have shell-specific patterns active
    assert!(
        checks.iter().any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/10-language-adapters.md#adapter-selection
///
/// > shell | *.sh files in root, bin/, or scripts/
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_auto_detected_when_sh_files_in_bin() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("bin")).unwrap();
    std::fs::write(
        dir.path().join("bin/build"),
        "#!/bin/bash\necho 'building'\n",
    ).unwrap();

    let result = cli().pwd(dir.path()).json().passes();
    let checks = result.checks();

    assert!(
        checks.iter().any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/10-language-adapters.md#adapter-selection
///
/// > shell | *.sh files in root, bin/, or scripts/
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_auto_detected_when_sh_files_in_root() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("setup.sh"),
        "#!/bin/bash\necho 'setup'\n",
    ).unwrap();

    let result = cli().pwd(dir.path()).json().passes();
    let checks = result.checks();

    assert!(
        checks.iter().any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}
```

**Milestone:** Auto-detection specs compile and are ignored.

**Verification:**
```bash
cargo test --test specs shell_adapter_auto -- --ignored 2>&1 | grep "3 ignored"
```

---

### Phase 3: Default Pattern Specs

Write specs for default source and test patterns.

**Add to `tests/specs/adapters/shell.rs`:**

```rust
// =============================================================================
// DEFAULT PATTERN SPECS
// =============================================================================

/// Spec: docs/specs/langs/shell.md#default-patterns
///
/// > source = ["**/*.sh", "**/*.bash"]
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_default_source_pattern_matches_sh_files() {
    let cloc = check("cloc").on("shell/auto-detect").json().passes();
    let metrics = cloc.require("metrics");

    // Should count .sh files as source
    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(source_lines > 0, "should count .sh files as source");
}

/// Spec: docs/specs/langs/shell.md#default-patterns
///
/// > source = ["**/*.sh", "**/*.bash"]
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_default_source_pattern_matches_bash_files() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    std::fs::write(
        dir.path().join("scripts/deploy.bash"),
        "#!/bin/bash\necho 'deploying'\n",
    ).unwrap();

    let cloc = check("cloc").pwd(dir.path()).json().passes();
    let metrics = cloc.require("metrics");

    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(source_lines > 0, "should count .bash files as source");
}

/// Spec: docs/specs/langs/shell.md#default-patterns
///
/// > tests = ["tests/**/*.bats", "test/**/*.bats", "*_test.sh"]
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_default_test_pattern_matches_bats_files() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    std::fs::create_dir_all(dir.path().join("tests")).unwrap();
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\necho 'building'\n",
    ).unwrap();
    std::fs::write(
        dir.path().join("tests/build.bats"),
        "#!/usr/bin/env bats\n@test 'builds' { run ./scripts/build.sh; }\n",
    ).unwrap();

    let cloc = check("cloc").pwd(dir.path()).json().passes();
    let metrics = cloc.require("metrics");

    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(test_lines > 0, "should count .bats files as test");
}

/// Spec: docs/specs/langs/shell.md#test-code-detection
///
/// > *_test.sh files
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_default_test_pattern_matches_test_sh_files() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\necho 'building'\n",
    ).unwrap();
    std::fs::write(
        dir.path().join("scripts/build_test.sh"),
        "#!/bin/bash\n./scripts/build.sh && echo 'passed'\n",
    ).unwrap();

    let cloc = check("cloc").pwd(dir.path()).json().passes();
    let metrics = cloc.require("metrics");

    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(test_lines > 0, "should count *_test.sh files as test");
}
```

**Milestone:** Default pattern specs compile and are ignored.

**Verification:**
```bash
cargo test --test specs shell_adapter_default -- --ignored
```

---

### Phase 4: Escape Pattern Specs

Write specs for Shell-specific escape patterns.

**Add to `tests/specs/adapters/shell.rs`:**

```rust
// =============================================================================
// ESCAPE PATTERN SPECS
// =============================================================================

/// Spec: docs/specs/langs/shell.md#default-escape-patterns
///
/// > set +e | comment | # OK:
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_set_plus_e_without_ok_comment_fails() {
    check("escapes")
        .on("shell/set-e-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("# OK:");
}

/// Spec: docs/specs/langs/shell.md#default-escape-patterns
///
/// > set +e | comment | # OK:
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_set_plus_e_with_ok_comment_passes() {
    check("escapes").on("shell/set-e-ok").passes();
}

/// Spec: docs/specs/langs/shell.md#default-escape-patterns
///
/// > eval | comment | # OK:
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_eval_without_ok_comment_fails() {
    check("escapes")
        .on("shell/eval-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("# OK:");
}

/// Spec: docs/specs/langs/shell.md#default-escape-patterns
///
/// > eval | comment | # OK:
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_eval_with_ok_comment_passes() {
    check("escapes").on("shell/eval-ok").passes();
}

/// Spec: docs/specs/langs/shell.md#default-escape-patterns
///
/// > set +e and eval allowed in test code without comment
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_escape_patterns_allowed_in_tests() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    std::fs::create_dir_all(dir.path().join("tests")).unwrap();
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\necho 'building'\n",
    ).unwrap();
    // Test file with set +e and eval, no comments
    std::fs::write(
        dir.path().join("tests/integration.bats"),
        "#!/usr/bin/env bats\nset +e\neval \"echo test\"\n@test 'works' { true; }\n",
    ).unwrap();

    check("escapes").pwd(dir.path()).passes();
}
```

**Milestone:** Escape pattern specs compile and are ignored.

**Verification:**
```bash
cargo test --test specs shell_adapter -- set_plus_e --ignored
cargo test --test specs shell_adapter -- eval --ignored
```

---

### Phase 5: Shellcheck Suppress Specs

Write specs for `# shellcheck disable=` handling.

**Add to `tests/specs/adapters/shell.rs`:**

```rust
// =============================================================================
// SHELLCHECK SUPPRESS SPECS
// =============================================================================

/// Spec: docs/specs/langs/shell.md#suppress
///
/// > "forbid" - Never allowed (default)
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_shellcheck_disable_forbidden_by_default() {
    check("escapes")
        .on("shell/shellcheck-forbid")
        .fails()
        .stdout_has("# shellcheck disable=");
}

/// Spec: docs/specs/langs/shell.md#suppress
///
/// > [shell.suppress.test] check = "allow" - tests can suppress freely
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_shellcheck_disable_allowed_in_tests() {
    check("escapes").on("shell/shellcheck-test").passes();
}

/// Spec: docs/specs/langs/shell.md#suppress
///
/// > "comment" - Requires justification comment
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_shellcheck_disable_with_comment_when_configured() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[shell.suppress]
check = "comment"
"#,
    ).unwrap();
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    // Has justification comment before shellcheck disable
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\n# This variable is exported for subprocesses\n# shellcheck disable=SC2034\nUNUSED_VAR=1\n",
    ).unwrap();

    check("escapes").pwd(dir.path()).passes();
}

/// Spec: docs/specs/langs/shell.md#suppress
///
/// > [shell.suppress.source] allow = ["SC2034"]
#[test]
#[ignore = "TODO: Phase 402 - Shell Adapter Implementation"]
fn shell_adapter_shellcheck_allow_list_skips_check() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[shell.suppress]
check = "forbid"
[shell.suppress.source]
allow = ["SC2034"]
"#,
    ).unwrap();
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    // SC2034 is in allow list, no comment needed
    std::fs::write(
        dir.path().join("scripts/build.sh"),
        "#!/bin/bash\n# shellcheck disable=SC2034\nUNUSED_VAR=1\n",
    ).unwrap();

    check("escapes").pwd(dir.path()).passes();
}
```

**Milestone:** Suppress specs compile and are ignored.

**Verification:**
```bash
cargo test --test specs shell_adapter -- shellcheck --ignored
```

---

### Phase 6: Create Test Fixtures

Create fixtures for the specs.

**Create `tests/fixtures/shell/auto-detect/`:**
```
shell/auto-detect/
├── scripts/
│   ├── build.sh       # #!/bin/bash\necho 'building'\n
│   └── deploy.sh      # #!/bin/bash\necho 'deploying'\n
└── tests/
    └── scripts.bats   # Basic bats test
```

**Create `tests/fixtures/shell/set-e-ok/`:**
```
shell/set-e-ok/
└── scripts/
    └── build.sh       # # OK: Need to continue on error for cleanup
                       # set +e
```

**Create `tests/fixtures/shell/set-e-fail/`:**
```
shell/set-e-fail/
└── scripts/
    └── build.sh       # set +e (no comment)
```

**Create `tests/fixtures/shell/eval-ok/`:**
```
shell/eval-ok/
└── scripts/
    └── build.sh       # # OK: User-provided command is validated above
                       # eval "$user_cmd"
```

**Create `tests/fixtures/shell/eval-fail/`:**
```
shell/eval-fail/
└── scripts/
    └── build.sh       # eval "$user_cmd" (no comment)
```

**Create `tests/fixtures/shell/shellcheck-forbid/`:**
```
shell/shellcheck-forbid/
└── scripts/
    └── build.sh       # # shellcheck disable=SC2034
                       # UNUSED_VAR=1
```

**Create `tests/fixtures/shell/shellcheck-test/`:**
```
shell/shellcheck-test/
├── scripts/
│   └── build.sh       # Clean source, no shellcheck disable
└── tests/
    └── test.bats      # # shellcheck disable=SC2034 (allowed in test)
```

**Milestone:** All fixtures created and valid.

**Verification:**
```bash
ls tests/fixtures/shell/
# Should show: auto-detect set-e-ok set-e-fail eval-ok eval-fail shellcheck-forbid shellcheck-test
```

---

## Key Implementation Details

### Spec Naming Convention

All specs follow the pattern:
```
shell_adapter_{feature}_{condition}_{expected_result}
```

Examples:
- `shell_adapter_auto_detected_when_sh_files_in_scripts`
- `shell_adapter_set_plus_e_without_ok_comment_fails`
- `shell_adapter_shellcheck_disable_forbidden_by_default`

### Fixture Design

Fixtures are minimal but complete:
- Each fixture tests one specific behavior
- `*.sh` files in `scripts/` trigger shell detection
- `quench.toml` only included when testing config-specific behavior
- Shell scripts are small (< 5 lines) to keep fixtures readable

### Test Code Boundaries

Per the spec, test code includes:
1. `*.bats` files (BATS test framework)
2. `*_test.sh` files
3. Files in `tests/` or `test/` directories

No inline test code convention for shell (unlike Rust's `#[cfg(test)]`).

### Escape Pattern Defaults

When Shell adapter is active, these patterns are applied by default:

| Pattern | Action | Comment |
|---------|--------|---------|
| `set +e` | comment | `# OK:` |
| `eval ` | comment | `# OK:` |

### Suppress Defaults

The `# shellcheck disable=` directive is controlled by `[shell.suppress]`:

| Setting | Behavior |
|---------|----------|
| `"forbid"` | Never allowed (default) |
| `"comment"` | Requires justification comment |
| `"allow"` | Always allowed |

Test code uses `[shell.suppress.test]` which defaults to `"allow"`.

---

## Verification Plan

### After Each Phase

```bash
# Compile check
cargo build --tests

# Run relevant specs (should all be ignored)
cargo test --test specs shell_adapter -- --ignored

# Check for clippy warnings
cargo clippy --all-targets --all-features -- -D warnings
```

### End-to-End Verification

```bash
# All shell adapter specs (should be ~14 ignored)
cargo test --test specs shell_adapter 2>&1 | grep -E "^test.*ignored"

# Count total ignored
cargo test --test specs -- --ignored 2>&1 | tail -1

# Full quality gates
make check
```

### Test Matrix

| Spec Category | Count | Fixture Required |
|--------------|-------|------------------|
| Auto-detection | 3 | shell/auto-detect, temp_project() |
| Default patterns | 4 | shell/auto-detect, temp_project() |
| set +e pattern | 3 | shell/set-e-ok, shell/set-e-fail |
| eval pattern | 2 | shell/eval-ok, shell/eval-fail |
| Shellcheck suppress | 4 | shell/shellcheck-forbid, shell/shellcheck-test |

**Total: 16 specs**

---

## Summary

| Phase | Task | Key Files | Status |
|-------|------|-----------|--------|
| 1 | Add shell module to adapters | `tests/specs/adapters/mod.rs` | [ ] Pending |
| 2 | Auto-detection specs | `tests/specs/adapters/shell.rs` | [ ] Pending |
| 3 | Default pattern specs | `tests/specs/adapters/shell.rs` | [ ] Pending |
| 4 | Escape pattern specs | `tests/specs/adapters/shell.rs` | [ ] Pending |
| 5 | Shellcheck suppress specs | `tests/specs/adapters/shell.rs` | [ ] Pending |
| 6 | Create test fixtures | `tests/fixtures/shell/` | [ ] Pending |

## Future Phases

- **Phase 402**: Shell Adapter Implementation (remove `#[ignore]` attributes)
- **Phase 403**: Shell Policy Integration (lint_changes, .shellcheckrc)
- **Phase 404**: Shell Coverage Integration (kcov with bats)
