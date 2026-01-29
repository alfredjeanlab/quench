# Go Configuration Guide

Configuration reference for Go language support.

## File Patterns

```toml
[golang]
source = ["**/*.go"]
tests = ["**/*_test.go"]
ignore = ["vendor/", "tools/"]
```

## Build Metrics

Track release binary sizes and build times. Override target auto-detection
with an explicit list.

```toml
[golang]
binary_size = true                      # Track release binary sizes
build_time = true                       # Track build times (cold and hot)
targets = ["cmd/myapp", "cmd/myserver"] # Override auto-detection
```

## CLOC Advice

```toml
[golang.cloc]
check = "error"
advice = "Custom advice for oversized Go files."
```

## Suppress Directives

Controls how `//nolint` directives are handled:

- `"forbid"` — never allowed
- `"comment"` — requires justification comment (default for source)
- `"allow"` — always allowed (default for tests)

```toml
[golang.suppress]
check = "comment"

[golang.suppress.test]
check = "allow"
```

## Suppress with Allowlist/Denylist

Exempt specific linters from the comment requirement, or forbid suppressing them entirely.
Require specific comments for certain linters.

```toml
[golang.suppress]
check = "comment"

[golang.suppress.source]
allow = ["unused"]     # No comment needed
forbid = ["govet"]     # Never suppress go vet

[golang.suppress.source.errcheck]
comment = "// OK:"

[golang.suppress.source.gosec]
comment = "// FALSE_POSITIVE:"

[golang.suppress.test]
check = "allow"
```

## Lint Config Policy

Require golangci-lint config changes in standalone PRs.

```toml
[golang.policy]
check = "error"
lint_changes = "standalone"
lint_config = [".golangci.yml", ".golangci.yaml", ".golangci.toml"]
```

## Escape Patterns

Go-specific escape hatches:

```toml
[[check.escapes.patterns]]
pattern = "unsafe\\.Pointer"
action = "comment"
comment = "// SAFETY:"
advice = "Add a // SAFETY: comment explaining pointer validity."

[[check.escapes.patterns]]
pattern = "//go:linkname"
action = "comment"
comment = "// LINKNAME:"
advice = "Add a // LINKNAME: comment explaining the external symbol dependency."

[[check.escapes.patterns]]
pattern = "//go:noescape"
action = "comment"
comment = "// NOESCAPE:"
advice = "Add a // NOESCAPE: comment explaining why escape analysis should be bypassed."
```

## Coverage

Go test runner provides built-in coverage:

```toml
[[check.tests.suite]]
runner = "go"
```

Integration tests can also instrument Go binaries:

```toml
[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
setup = "go build -cover ./cmd/myapp"
targets = ["cmd/myapp"]
```

## Complete Example

```toml
[golang]
source = ["**/*.go"]
tests = ["**/*_test.go"]
ignore = ["vendor/"]
targets = ["cmd/myapp", "cmd/myserver"]
binary_size = true
build_time = true

[golang.cloc]
check = "error"
advice = "Custom advice for Go files."

[golang.suppress]
check = "comment"

[golang.suppress.source]
allow = ["unused"]
forbid = ["govet"]

[golang.suppress.source.errcheck]
comment = "// OK:"

[golang.suppress.test]
check = "allow"

[golang.policy]
check = "error"
lint_changes = "standalone"
lint_config = [".golangci.yml"]

[[check.escapes.patterns]]
pattern = "unsafe\\.Pointer"
action = "comment"
comment = "// SAFETY:"

[[check.escapes.patterns]]
pattern = "//go:linkname"
action = "comment"
comment = "// LINKNAME:"

[[check.tests.suite]]
runner = "go"
```
