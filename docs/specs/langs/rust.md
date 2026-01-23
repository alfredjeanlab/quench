# Rust Language Support

Rust-specific behavior for quench checks.

## Detection

Detected when `Cargo.toml` exists in project root.

## Profile Defaults

When using [`quench init --profile rust`](../01-cli.md#profile-selection-recommended), the following opinionated defaults are configured:

```toml
[rust]
split_cfg_test = true
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
pattern = ".unwrap()"
action = "forbid"
in_tests = "allow"
advice = "Use ? operator or handle the error explicitly."

[[check.escapes.patterns]]
pattern = ".expect("
action = "forbid"
in_tests = "allow"
advice = "Use ? operator or handle the error explicitly."

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
tests = ["tests/**", "test/**/*.rs", "*_test.rs", "*_tests.rs"]
ignore = ["target/"]
```

## Test Code Detection

**Test files** (entire file is test code):
- `*_test.rs`, `*_tests.rs`
- Files in `tests/` directory

**Inline test code** (within source files):
- Lines inside `#[cfg(test)]` blocks are counted as test LOC
- Configurable: `split_cfg_test = true` (default)

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
tests = ["tests/**", "test/**/*.rs", "*_test.rs", "*_tests.rs"]
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

## Default Escape Patterns

| Pattern | Action | Comment Required |
|---------|--------|------------------|
| `unsafe { }` | comment | `// SAFETY:` |
| `.unwrap()` | forbid | - |
| `.expect(` | forbid | - |
| `mem::transmute` | comment | `// SAFETY:` |

## Suppress

Controls `#[allow(...)]` and `#[expect(...)]` attributes.

| Setting | Behavior |
|---------|----------|
| `"forbid"` | Never allowed |
| `"comment"` | Requires justification comment |
| `"allow"` | Always allowed |

Default: `"comment"` for source, `"allow"` for test code.

```toml
[rust.suppress]
check = "comment"              # forbid | comment | allow
# comment = "// JUSTIFIED:"    # optional: require specific pattern (default: any)

[rust.suppress.source]
allow = ["dead_code"]          # no comment needed
forbid = ["unsafe_code"]       # never allowed

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
# tests = ["tests/**", "test/**/*.rs", "*_test.rs", "*_tests.rs"]
# ignore = ["target/"]

split_cfg_test = true            # Count #[cfg(test)] as test LOC

# Build targets (default: all [[bin]] entries)
# targets = ["myapp", "myserver"]

# Build metrics (CI mode) - see [check.build] for thresholds
binary_size = true
build_time = true

[rust.suppress]
check = "comment"

[rust.suppress.test]
check = "allow"

[rust.policy]
lint_changes = "standalone"
lint_config = ["rustfmt.toml", "clippy.toml"]
```

Test suites and coverage thresholds are configured in `[check.tests]`.
