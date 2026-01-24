# Rust Language Support

Rust-specific behavior for quench checks.

## Detection

Detected when `Cargo.toml` exists in project root.

## Profile Defaults

When using [`quench init --with rust`](../01-cli.md#explicit-profiles), the following opinionated defaults are configured:

```toml
[rust]
cfg_test_split = "count"         # count | require | off (default: "count")
binary_size = true
build_time = true

[rust.suppress]
check = "comment"

[rust.suppress.test]
check = "allow"

[rust.policy]
lint_changes = "standalone"
lint_config = ["rustfmt.toml", ".rustfmt.toml", "clippy.toml", ".clippy.toml"]

[[check.escapes.patterns]]
pattern = "unsafe {"
action = "comment"
comment = "// SAFETY:"
advice = "Add a // SAFETY: comment explaining the invariants."

[[check.escapes.patterns]]
pattern = "mem::transmute"
action = "comment"
comment = "// SAFETY:"
advice = "Add a // SAFETY: comment explaining type compatibility."
```

**Landing the Plane items** (added to agent files when combined with `claude` or `cursor` profile):
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- `cargo build`

## Default Patterns

```toml
[rust]
source = ["**/*.rs"]
tests = ["tests/**", "test/**/*.rs", "benches/**", "*_test.rs", "*_tests.rs"]
ignore = ["target/"]
```

## Test Code Detection

**Test files** (entire file is test code):
- `*_test.rs`, `*_tests.rs`
- Files in `tests/` directory

**Inline test code** (within source files):
- Lines inside `#[cfg(test)]` blocks handling is configurable
- See [CFG Test Split Modes](#cfg-test-split-modes) below

```rust
pub fn add(a: i32, b: i32) -> i32 {  // ← source LOC
    a + b
}

#[cfg(test)]                          // ← test LOC starts
mod tests {
    #[test]
    fn test_add() {
        assert_eq!(add(1, 2), 3);
    }
}                                     // ← test LOC ends
```

**External test modules** (`mod tests;` declarations):

External test modules are detected via file patterns, not by parsing the `mod` declaration. The default test patterns are:

```toml
tests = ["tests/**", "test/**/*.rs", "benches/**", "*_test.rs", "*_tests.rs"]
```

Example using the sibling `_tests.rs` convention:

```rust
// src/lib.rs
#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;  // Points to src/lib_tests.rs → matched by *_tests.rs
```

Similarly:
- `tests/*.rs` → matched by `tests/**` pattern
- `src/foo_tests.rs` → matched by `*_tests.rs` pattern

For test correlation, changes to `#[cfg(test)]` blocks satisfy the test requirement.

### Supported Patterns

```rust
// Single-line attribute
#[cfg(test)]
mod tests { }

// Multi-line attribute
#[cfg(
    test
)]
mod tests { }
```

### Escapes in Test Code

Escape patterns (`unwrap`, `expect`, `panic`, etc.) are allowed in test code:
- **Test files**: Matched by test file patterns (`*_tests.rs`, `tests/**`)
- **Inline test blocks**: Lines inside `#[cfg(test)]` blocks in source files

This matches Clippy's behavior where `#[cfg(test)]` code is exempt from `unwrap_used`, `expect_used`, and `panic` lints.

### Limitations

**Macro-generated test blocks**: `#[cfg(test)]` attributes inside macro definitions are not detected for test LOC counting:

```rust
macro_rules! make_tests {
    () => {
        #[cfg(test)]
        mod tests {
            #[test]
            fn it_works() {}
        }
    };
}
make_tests!();  // #[cfg(test)] inside macro not detected
```

## CFG Test Split Modes

The `cfg_test_split` option controls how `#[cfg(test)]` blocks are handled for LOC counting:

```toml
[rust]
cfg_test_split = "count"  # count | require | off (default: "count")
```

| Mode | Behavior |
|------|----------|
| `"count"` | Split `#[cfg(test)]` blocks into test LOC (default) |
| `"require"` | Fail if source files contain inline `#[cfg(test)]` blocks; require separate `_tests.rs` files |
| `"off"` | Count all lines as source LOC, don't parse for `#[cfg(test)]` |

### Mode: "count" (Default)

Lines inside `#[cfg(test)]` blocks are counted as test LOC:

```rust
pub fn add(a: i32, b: i32) -> i32 {  // ← source LOC
    a + b
}

#[cfg(test)]                          // ← test LOC starts
mod tests {
    #[test]
    fn test_add() {
        assert_eq!(add(1, 2), 3);
    }
}                                     // ← test LOC ends
```

Result: separate source and test line counts.

### Mode: "require"

Projects using `"require"` mode enforce the sibling test file convention:

```
src/parser.rs       # source only, no #[cfg(test)]
src/parser_tests.rs # all tests here
```

External module declarations (`#[cfg(test)] mod tests;`) are **not** flagged as violations. Only inline test blocks fail:

```rust
// ✓ ALLOWED (external module)
#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;

// ✗ VIOLATION (inline test block)
#[cfg(test)]
mod tests {
    #[test]
    fn test_it() { }
}
```

Violations report the location of the inline test:

```
src/parser.rs:150: inline_cfg_test
  Move tests to a sibling _tests.rs file.
```

This pairs with the sibling `_tests.rs` convention documented in the project's CLAUDE.md.

### Mode: "off"

All lines are counted as source LOC. The parser doesn't look for `#[cfg(test)]` blocks at all. This is faster but less accurate for projects with inline tests.

## Default Escape Patterns

| Pattern | Action | Comment Required |
|---------|--------|------------------|
| `unsafe { }` | comment | `// SAFETY:` |
| `mem::transmute` | comment | `// SAFETY:` |

Quench does not forbid usage directly, and assumes you are already running Clippy. Instead it ensures escapes and suppressions are commented.

## Suppress

Controls `#[allow(...)]` and `#[expect(...)]` attributes.

| Setting | Behavior |
|---------|----------|
| `"forbid"` | Never allowed |
| `"comment"` | Requires justification comment |
| `"allow"` | Always allowed |

Default: `"comment"` for source, `"allow"` for test code.

### Default Per-Lint Patterns

By default, common lint suppressions require specific comment patterns:

| Lint Code | Required Comment (any of) |
|-----------|---------------------------|
| `dead_code` | `// KEEP UNTIL:`, `// NOTE(compat):`, `// NOTE(compatibility):`, `// NOTE(lifetime):` |
| `clippy::too_many_arguments` | `// TODO(refactor):` |
| `clippy::cast_possible_truncation` | `// CORRECTNESS:`, `// SAFETY:` |
| `deprecated` | `// TODO(refactor):`, `// NOTE(compat):`, `// NOTE(compatibility):` |

These defaults ensure suppressions have meaningful justifications. Override per-project if needed.

### Default Per-Lint Guidance

Each built-in lint code has thoughtful guidance to help developers evaluate the suppression:

| Lint Code | Guidance Question |
|-----------|-------------------|
| `dead_code` | Is this code still needed? It is usually best to remove dead code. |
| `clippy::too_many_arguments` | Can this function be refactored? |
| `clippy::cast_possible_truncation` | Is this cast safe? |
| `deprecated` | Can this deprecated API be replaced? |

For custom lint codes without built-in guidance, a generic message is used: "Is this suppression necessary?"

### Violation Messages

When a suppression is missing a required comment, the error message provides:
1. A general statement that justification is required
2. A lint-specific question or guidance
3. The list of acceptable comment patterns (when multiple options exist)

**Example outputs:**

```
crates/cli/src/git_hooks.rs:109: suppress_missing_comment: #[allow(dead_code)]
  Lint suppression requires justification.

  Is this code still needed?
  It is usually best to remove dead code.

  If it should be kept, add one of:
    // KEEP UNTIL: ...
    // NOTE(compat): ...
    // NOTE(compatibility): ...

crates/cli/src/display.rs:106: suppress_missing_comment: #[allow(clippy::too_many_arguments)]
  Lint suppression requires justification.
  Can this function be refactored?
  If not, add:
    // TODO(refactor): ...

crates/cli/src/daemon/runner.rs:598: suppress_missing_comment: #[allow(clippy::cast_possible_truncation)]
  Lint suppression requires justification.
  Is this cast safe?
  If so, add one of:
    // CORRECTNESS: ...
    // SAFETY: ...
```

The guidance matches the [default per-lint guidance](#default-per-lint-guidance) table above.

### Supported Patterns

```rust
// Outer attribute (item-level)
#[allow(dead_code)]
fn unused() {}

// Inner attribute (module-level)
#![allow(clippy::unwrap_used)]

// Multi-line attribute
#[allow(
    dead_code,
    unused_variables
)]
fn unused() {}

// Attribute inside macro_rules (the definition requires justification, not call sites)
macro_rules! allow_unused {
    ($item:item) => {
        #[allow(dead_code)]  // ← this usage requires a comment
        $item
    };
}
```

**Note:** Inner attributes (`#![...]`) apply to the module or crate they appear in. They follow the same comment requirement rules as outer attributes.

```toml
[rust.suppress]
check = "comment"              # forbid | comment | allow
# comment = "// JUSTIFIED:"    # optional: global pattern (default: any)

[rust.suppress.source]
allow = ["dead_code"]          # no comment needed for this lint
forbid = ["unsafe_code"]       # never allowed

# Per-lint patterns (override defaults)
# Table syntax:
[rust.suppress.source.dead_code]
comment = "// LEGACY:"         # single pattern

# Or inline array syntax:
# dead_code = ["// KEEP UNTIL:", "// NOTE(compat):"]  # multiple patterns
# deprecated = "// TODO(refactor):"                    # single pattern

[rust.suppress.test]
check = "allow"                # tests can suppress freely
```

## Policy

Enforce lint configuration hygiene.

```toml
[rust.policy]
lint_changes = "standalone"    # lint config changes must be standalone PRs
lint_config = [                # files that trigger standalone requirement
  "rustfmt.toml",
  ".rustfmt.toml",
  "clippy.toml",
  ".clippy.toml",
]
```

When `lint_changes = "standalone"`, changing any `lint_config` files alongside source/test changes fails:

```
rust: FAIL
  lint config changes must be standalone
    Changed: rustfmt.toml
    Also changed: src/parser.rs, src/lexer.rs
  Submit lint config changes in a separate PR.
```

## Build Metrics

Rust build metrics are part of the `build` check. See [checks/build.md](../checks/build.md) for full details.

### Targets

Build targets for coverage and binary size tracking. Auto-detected from `[[bin]]` entries in Cargo.toml.

```toml
[rust]
targets = ["myapp", "myserver"]    # Override auto-detection
```

### Binary Size

Track release binary sizes (CI mode).

```
build: size
  quench: 4.2 MB
  server: 12.1 MB
```

With threshold:
```
build: FAIL
  quench: 5.1 MB (max: 5 MB)
```

### Build Time

Track build times (CI mode):

- **Cold**: `cargo clean && cargo build --release`
- **Hot**: Incremental debug rebuild

```
build: time
  cold (release): 45.2s
  hot (debug): 1.8s
```

## Coverage

The `cargo` runner provides implicit Rust coverage via `cargo llvm-cov`. Other runners (bats, pytest) can also contribute Rust coverage by specifying build targets:

```toml
[[check.tests.suite]]
runner = "cargo"
# Implicit: covers Rust code via llvm-cov

[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
setup = "cargo build"
targets = ["myapp"]             # Instrument Rust binary
```

Multiple test suites contribute to coverage via LLVM profile merging.

## Configuration

```toml
[rust]
# Source/test patterns (defaults shown)
# source = ["**/*.rs"]
# tests = ["tests/**", "test/**/*.rs", "benches/**", "*_test.rs", "*_tests.rs"]
# ignore = ["target/"]

cfg_test_split = "count"         # count | require | off (default: "count")
                                 # Boolean still works: true="count", false="off"

# Build targets (default: all [[bin]] entries)
# targets = ["myapp", "myserver"]

# Build metrics (CI mode) - see [check.build] for thresholds
binary_size = true
build_time = true

[rust.cloc]
check = "error"                  # error | warn | off
# advice = "..."                 # Custom advice for oversized Rust files

[rust.suppress]
check = "comment"

[rust.suppress.test]
check = "allow"

[rust.policy]
lint_changes = "standalone"
lint_config = ["rustfmt.toml", "clippy.toml"]
```

Test suites and coverage thresholds are configured in `[check.tests]`.
