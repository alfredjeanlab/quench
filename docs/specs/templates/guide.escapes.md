# Escape Hatches Configuration Guide

Configuration reference for the `escapes` check.

## Basic Pattern (Comment)

Require a justification comment when an escape hatch is used.

```toml
[check.escapes]
check = "error"

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"
comment = "// SAFETY:"
advice = "Add a // SAFETY: comment explaining the invariants."
```

## Basic Pattern (Forbid)

Never allow a pattern in source code. Escape hatches are always allowed in tests
unless `in_tests` is set.

```toml
[check.escapes]
check = "error"

[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"
advice = "Handle the error case or use .expect() with a message."
```

## Basic Pattern (Count)

Count occurrences without requiring comments. Fails if the count exceeds the
threshold (default: 0).

```toml
[check.escapes]
check = "error"

[[check.escapes.patterns]]
name = "todo"
pattern = "TODO|FIXME|XXX"
action = "count"
threshold = 10
advice = "Reduce TODO/FIXME comments before shipping."
```

## Override for Tests

By default, escape hatches are allowed in test code. Set `in_tests` to apply
the rule to tests as well.

```toml
[check.escapes]
check = "error"

[[check.escapes.patterns]]
name = "debugger"
pattern = "breakpoint\\(\\)"
action = "forbid"
in_tests = "forbid"
advice = "Remove debugger before committing."
```

## Rust Patterns

```toml
[check.escapes]
check = "error"

[[check.escapes.patterns]]
pattern = "unsafe {"
action = "comment"
comment = "// SAFETY:"

[[check.escapes.patterns]]
pattern = "mem::transmute"
action = "comment"
comment = "// SAFETY:"

[[check.escapes.patterns]]
pattern = "\\.unwrap\\(\\)"
action = "forbid"
```

## Shell Patterns

```toml
[check.escapes]
check = "error"

[[check.escapes.patterns]]
pattern = "set \\+e"
action = "comment"
comment = "# OK:"

[[check.escapes.patterns]]
pattern = "eval "
action = "comment"
comment = "# OK:"

[[check.escapes.patterns]]
pattern = "# shellcheck disable="
action = "forbid"
in_tests = "allow"
```

## Go Patterns

```toml
[check.escapes]
check = "error"

[[check.escapes.patterns]]
pattern = "unsafe\\.Pointer"
action = "comment"
comment = "// SAFETY:"

[[check.escapes.patterns]]
pattern = "//go:linkname"
action = "comment"
comment = "// LINKNAME:"

[[check.escapes.patterns]]
pattern = "//go:noescape"
action = "comment"
comment = "// NOESCAPE:"
```

## JavaScript/TypeScript Patterns

```toml
[check.escapes]
check = "error"

[[check.escapes.patterns]]
pattern = "as unknown"
action = "comment"
comment = "// CAST:"

[[check.escapes.patterns]]
pattern = "@ts-ignore"
action = "forbid"
advice = "Use @ts-expect-error instead."
```

## Python Patterns

```toml
[check.escapes]
check = "error"

[[check.escapes.patterns]]
pattern = "breakpoint\\(\\)"
action = "forbid"
in_tests = "forbid"

[[check.escapes.patterns]]
pattern = "eval\\("
action = "comment"
comment = "# EVAL:"

[[check.escapes.patterns]]
pattern = "exec\\("
action = "comment"
comment = "# EXEC:"
```

## Ruby Patterns

```toml
[check.escapes]
check = "error"

[[check.escapes.patterns]]
pattern = "binding\\.pry"
action = "forbid"
in_tests = "forbid"

[[check.escapes.patterns]]
pattern = "eval\\("
action = "comment"
comment = "# METAPROGRAMMING:"
```

## Per-Package Overrides

Stricter thresholds for specific packages.

```toml
[check.escapes]
check = "error"

[[check.escapes.patterns]]
name = "todo"
pattern = "TODO|FIXME"
action = "count"
threshold = 10

[check.escapes.package.cli]
[[check.escapes.package.cli.patterns]]
name = "todo"
threshold = 5
```
