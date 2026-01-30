# Plan: Language-Aware Commit Exclude Defaults

## Overview

The `check.tests.commit.exclude` defaults currently hardcode Rust-specific patterns
(`**/mod.rs`, `**/lib.rs`, `**/main.rs`) regardless of which language is detected.
A Python project gets `**/lib.rs` in its exclude list, which is meaningless.

This plan makes the default exclude patterns language-dependent, so only relevant
entry-point/declaration patterns appear for each detected language.

## Project Structure

Key files to modify:

```
crates/cli/src/
├── adapter/
│   ├── patterns.rs          # correlation_exclude_defaults() — primary change
│   └── patterns_tests.rs    # Unit tests for the function
├── config/
│   └── tests_check.rs       # TestsCommitConfig::default_exclude() — make generic
tests/specs/
├── verbose.rs               # Verbose output tests that assert exact exclude lines
└── checks/tests/
    └── correlation.rs        # Exclusion behavior spec
```

## Dependencies

No new external dependencies.

## Implementation Phases

### Phase 1: Refactor `correlation_exclude_defaults()` to be language-aware

**File:** `crates/cli/src/adapter/patterns.rs` (lines 114–143)

Currently the function starts with a base set that includes Rust patterns for all
languages, then appends language-specific extras:

```rust
// BEFORE
pub fn correlation_exclude_defaults(lang: super::ProjectLanguage) -> Vec<String> {
    let mut patterns = vec![
        "**/generated/**".to_string(),
        "**/mod.rs".to_string(),      // Rust-only
        "**/lib.rs".to_string(),      // Rust-only
        "**/main.rs".to_string(),     // Rust-only
    ];
    match lang {
        Go => patterns.push("**/main.go".to_string()),
        Python => patterns.push("**/__init__.py".to_string()),
        JavaScript => patterns.extend(["**/index.js", "**/index.ts"].map(String::from)),
        Rust | Ruby | Shell | Generic => {}
    }
    patterns
}
```

Refactor so only truly universal patterns are in the base, and Rust patterns move
into the `Rust` arm:

```rust
// AFTER
pub fn correlation_exclude_defaults(lang: super::ProjectLanguage) -> Vec<String> {
    // Universal: generated code is never test-required
    let mut patterns = vec!["**/generated/**".to_string()];

    // Language-specific entry points and declarations
    match lang {
        super::ProjectLanguage::Rust => {
            patterns.extend(
                ["**/mod.rs", "**/lib.rs", "**/main.rs"].map(String::from),
            );
        }
        super::ProjectLanguage::Go => {
            patterns.push("**/main.go".to_string());
        }
        super::ProjectLanguage::Python => {
            patterns.push("**/__init__.py".to_string());
        }
        super::ProjectLanguage::JavaScript => {
            patterns.extend(
                ["**/index.js", "**/index.ts", "**/index.jsx", "**/index.tsx"]
                    .map(String::from),
            );
        }
        super::ProjectLanguage::Ruby => {}
        super::ProjectLanguage::Shell => {}
        super::ProjectLanguage::Generic => {}
    }
    patterns
}
```

### Phase 2: Update `TestsCommitConfig::default_exclude()` to be minimal

**File:** `crates/cli/src/config/tests_check.rs` (lines 176–183)

This serde default function provides the exclude list when the user omits it from
`quench.toml`. Since `correlation_exclude_defaults(lang)` is called at runtime when
the config exclude is empty (see `checks/testing/mod.rs:120–124`), the serde default
should be an empty vec — indicating "use language-aware defaults at runtime."

However, the current logic checks `config.exclude.is_empty()` to decide whether to
call `correlation_exclude_defaults()`. The serde default populates exclude with
Rust-specific patterns even when the user didn't set anything.

The fix: change the serde default to return an empty vec, so the existing
`is_empty()` check at runtime correctly triggers the language-aware path.

```rust
// BEFORE
fn default_exclude() -> Vec<String> {
    vec![
        "**/mod.rs".to_string(),
        "**/lib.rs".to_string(),
        "**/main.rs".to_string(),
        "**/generated/**".to_string(),
    ]
}

// AFTER
fn default_exclude() -> Vec<String> {
    vec![]
}
```

This is safe because the runtime check in `checks/testing/mod.rs:120-124` already
handles the empty case:

```rust
exclude_patterns: if config.exclude.is_empty() {
    correlation_exclude_defaults(lang)  // ← language-aware defaults
} else {
    config.exclude.clone()              // ← user-provided
},
```

And `cmd_check.rs:134-138` has the same fallback for verbose output.

### Phase 3: Update unit tests

**File:** `crates/cli/src/adapter/patterns_tests.rs`

Add tests to verify the new per-language behavior:

- `correlation_defaults_rust_includes_rs_patterns` — Rust includes `mod.rs`, `lib.rs`, `main.rs`
- `correlation_defaults_go_includes_main_go` — Go includes `main.go`, not `.rs` files
- `correlation_defaults_python_includes_init` — Python includes `__init__.py`, not `.rs` files
- `correlation_defaults_js_includes_index` — JS includes `index.js/ts`, not `.rs` files
- `correlation_defaults_generic_only_generated` — Generic only has `**/generated/**`
- `correlation_defaults_all_include_generated` — Every language includes `**/generated/**`

### Phase 4: Update behavioral specs

**File:** `tests/specs/verbose.rs` (lines 203, 257)

The verbose output tests hardcode the expected exclude line. Update them to expect
language-aware output. The test fixtures use `Project::empty()` (no `Cargo.toml`),
so the detected language is `Generic`, and the exclude should be just
`**/generated/**`.

```rust
// BEFORE (Generic project, no Cargo.toml)
check.tests.commit.exclude: **/generated/**, **/mod.rs, **/lib.rs, **/main.rs

// AFTER
check.tests.commit.exclude: **/generated/**
```

**File:** `tests/specs/checks/tests/correlation.rs` (lines 310–344)

The `excluded_files_dont_require_tests` spec tests that `mod.rs`, `lib.rs`, and
`main.rs` are excluded by default. This test uses `Project::empty()` (no
`Cargo.toml`), so with the fix these files would no longer be excluded for a Generic
project.

Fix: add a `Cargo.toml` marker file to the fixture so the project is detected as
Rust, which is the correct context for testing Rust-specific excludes.

Also add a new spec to verify that non-Rust projects don't exclude `.rs` entry points
(they'd be treated as normal source files in a non-Rust project).

### Phase 5: Update spec documentation

**File:** `docs/specs/checks/tests.md` (lines 301–307)

Update the documented default exclude list to note that patterns are language-dependent:

```toml
# Exclude patterns (never require tests)
# Defaults are language-dependent. For Rust:
exclude = [
  "**/mod.rs",           # Module declarations
  "**/lib.rs",           # Library roots
  "**/main.rs",          # Binary entry points
  "**/generated/**",     # Generated code (all languages)
]
```

## Key Implementation Details

1. **The `is_empty()` gate is already correct.** Both `checks/testing/mod.rs:120` and
   `cmd_check.rs:134` check if `config.exclude.is_empty()` and call
   `correlation_exclude_defaults(lang)` when true. By making the serde default return
   an empty vec, we let the runtime language detection work as intended.

2. **No breaking change for users who set `exclude` explicitly.** If a user has
   `exclude = ["**/lib.rs", "**/mod.rs"]` in their `quench.toml`, that value is used
   directly — no defaults apply.

3. **Existing Rust projects are unaffected.** Rust projects have `Cargo.toml`,
   so `detect_language()` returns `Rust`, and `correlation_exclude_defaults(Rust)`
   returns the same patterns as before.

4. **New language patterns can be added per-language.** The match arm structure makes
   it easy to add Ruby (`**/config.ru`?) or Shell patterns in the future.

## Verification Plan

1. **Unit tests** — `cargo test -p quench -- patterns_tests` verifies per-language defaults
2. **Behavioral specs** — `cargo test --test specs -- correlation` verifies exclude behavior
3. **Verbose specs** — `cargo test --test specs -- verbose` verifies output format
4. **Full check** — `make check` runs fmt, clippy, all tests, build, audit, deny
5. **Manual smoke test** — Run `quench check --verbose` in a non-Rust project and confirm
   `check.tests.commit.exclude` no longer shows `**/lib.rs`
