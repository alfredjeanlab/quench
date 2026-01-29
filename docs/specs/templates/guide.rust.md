# Rust Configuration Guide

Configuration reference for Rust language support.

## File Patterns

```toml
[rust]
source = ["**/*.rs"]
tests = ["**/tests/**", "**/*_test.rs", "**/*_tests.rs"]
ignore = ["target/", "examples/"]
```

## CFG Test Split

Controls how `#[cfg(test)]` blocks are counted for LOC:

- `"count"` — split into test LOC (default)
- `"require"` — fail if inline tests found (enforce sibling `_tests.rs` files)
- `"off"` — count all as source LOC

```toml
[rust]
cfg_test_split = "count"
```

## Build Metrics

Track release binary sizes and build times. Override target auto-detection
from `Cargo.toml` with an explicit list.

```toml
[rust]
binary_size = true
build_time = true
targets = ["myapp", "myserver"]
```

## CLOC Advice

```toml
[rust.cloc]
check = "error"
advice = "Custom advice for oversized Rust files."
```

## Suppress Directives

Controls how `#[allow(...)]` and `#[expect(...)]` attributes are handled:

- `"forbid"` — never allowed
- `"comment"` — requires justification comment (default for source)
- `"allow"` — always allowed (default for tests)

```toml
[rust.suppress]
check = "comment"

[rust.suppress.test]
check = "allow"
```

## Suppress with Allowlist/Denylist

Exempt specific lints from the comment requirement, or forbid suppressing them entirely.

```toml
[rust.suppress]
check = "comment"

[rust.suppress.source]
allow = ["dead_code"]     # No comment needed for these lints
forbid = ["unsafe_code"]  # Never allowed to suppress

[rust.suppress.test]
check = "allow"
```

## Suppress with Per-Lint Comment Patterns

Require a specific comment pattern for individual lint suppressions.
Can also use inline array syntax: `dead_code = ["// KEEP UNTIL:", "// NOTE(compat):"]`

```toml
[rust.suppress]
check = "comment"

[rust.suppress.source.dead_code]
comment = "// LEGACY:"

[rust.suppress.test]
check = "allow"
```

## Lint Config Policy

Require lint config changes (`rustfmt.toml`, `clippy.toml`) in standalone PRs.

```toml
[rust.policy]
check = "error"
lint_changes = "standalone"
lint_config = ["rustfmt.toml", "clippy.toml"]
```

## Escape Patterns

Rust-specific escape hatches:

```toml
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

[[check.escapes.patterns]]
pattern = "\\.unwrap\\(\\)"
action = "forbid"
advice = "Use .context() from anyhow or handle the error explicitly."
```

## Complete Example

```toml
[rust]
source = ["**/*.rs"]
tests = ["**/tests/**", "**/*_test.rs", "**/*_tests.rs"]
ignore = ["target/"]
cfg_test_split = "count"
targets = ["myapp", "myserver"]
binary_size = true
build_time = true

[rust.cloc]
check = "error"
advice = "Custom advice for Rust source files."

[rust.suppress]
check = "comment"

[rust.suppress.source]
allow = ["dead_code"]
forbid = ["unsafe_code"]

[rust.suppress.test]
check = "allow"

[rust.policy]
check = "error"
lint_changes = "standalone"
lint_config = ["rustfmt.toml", "clippy.toml"]

[[check.escapes.patterns]]
pattern = "unsafe {"
action = "comment"
comment = "// SAFETY:"

[[check.escapes.patterns]]
pattern = "mem::transmute"
action = "comment"
comment = "// SAFETY:"
```
