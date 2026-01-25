# Go Language Support

Go-specific behavior for quench checks.

## Detection

Detected when `go.mod` exists in project root.

## Profile Defaults

When using [`quench init --with golang`](../01-cli.md#explicit-profiles) (or `--with go`), the following opinionated defaults are configured:

```toml
[golang]
binary_size = true
build_time = true

[golang.suppress]
check = "comment"

[golang.suppress.test]
check = "allow"

[golang.policy]
lint_changes = "standalone"
lint_config = [".golangci.yml", ".golangci.yaml", ".golangci.toml"]

[[check.escapes.patterns]]
pattern = "unsafe.Pointer"
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

**Landing the Plane items** (added to agent files when combined with `claude` or `cursor` profile):
- `go fmt ./...`
- `go vet ./...`
- `golangci-lint run` (if `.golangci.yml` exists)
- `go test ./...`
- `go build ./...`

## Default Patterns

```toml
[golang]
source = ["**/*.go"]
tests = ["**/*_test.go"]
ignore = ["vendor/"]
```

When `[golang].tests` is not configured, patterns fall back to `[project].tests`, then to these defaults. See [Pattern Resolution](../02-config.md#pattern-resolution).

## Test Code Detection

**Test files** (entire file is test code):
- `*_test.go` files (Go's standard test convention)

Go uses a simple file-based convention: all `*_test.go` files are test code. No inline test detection is needed.

```go
// math.go       ← source LOC
package math

func Add(a, b int) int {
    return a + b
}
```

```go
// math_test.go  ← test LOC (entire file)
package math

import "testing"

func TestAdd(t *testing.T) {
    if Add(1, 2) != 3 {
        t.Error("expected 3")
    }
}
```

### Escapes in Test Code

Escape patterns (`unsafe.Pointer`, etc.) are allowed in test code:
- **Test files**: Any `*_test.go` file

## Default Escape Patterns

| Pattern | Action | Comment Required |
|---------|--------|------------------|
| `unsafe.Pointer` | comment | `// SAFETY:` |
| `//go:linkname` | comment | `// LINKNAME:` |
| `//go:noescape` | comment | `// NOESCAPE:` |

Quench does not forbid usage directly, and assumes you are already running `go vet` and `golangci-lint`. Instead it ensures escapes and suppressions are commented.

- **`unsafe.Pointer`**: Bypasses Go's type safety and memory guarantees
- **`//go:linkname`**: Links to unexported symbols in other packages; breaks between Go versions
- **`//go:noescape`**: Lies to compiler about escape analysis; misuse causes memory corruption

## Suppress

Controls `//nolint` directives (used by golangci-lint).

| Setting | Behavior |
|---------|----------|
| `"forbid"` | Never allowed |
| `"comment"` | Requires justification comment (default) |
| `"allow"` | Always allowed |

Default: `"comment"` for source, `"allow"` for test code.

```go
// OK: This field is intentionally unused for JSON marshaling
//nolint:unused
type Config struct {
    internal string
}

//nolint:errcheck  // ← Missing justification comment → violation
func riskyCall() {
    os.Remove("temp.txt")
}
```

### Configuration

```toml
[golang.suppress]
check = "comment"              # forbid | comment | allow
# comment = "// OK:"           # optional: require specific pattern (default: any)

[golang.suppress.source]
allow = ["unused"]             # no comment needed for these
forbid = ["govet"]             # never suppress go vet findings

[golang.suppress.test]
check = "allow"                # tests can suppress freely

# Per-lint patterns (optional)
[golang.suppress.source.errcheck]
comment = "// OK:"             # require specific pattern for errcheck

[golang.suppress.source.gosec]
comment = "// FALSE_POSITIVE:" # require specific pattern for gosec
```

### Violation Messages

When a suppression is missing a required comment, the error message provides:
1. A general statement that justification is required
2. Lint-specific guidance (future: will be tailored per common nolint codes)
3. The list of acceptable comment patterns (when configured)

**Example outputs:**

```
pkg/client/client.go:45: suppress_missing_comment: //nolint:errcheck
  Lint suppression requires justification.
  Is this error handling necessary to skip?
  Add a comment above the directive or inline (//nolint:code // reason).

pkg/client/client.go:67: suppress_missing_comment: //nolint:gosec
  Lint suppression requires justification.
  Is this security finding a false positive?
  If so, add:
    // FALSE_POSITIVE: ...
```

The first example shows the default behavior (no specific pattern required).
The second example shows when a specific pattern is configured.

**Note**: Per-lint guidance for common nolint codes (errcheck, gosec, etc.) will be added in a future update.

### Supported Patterns

```go
// Single linter
//nolint:errcheck

// Multiple linters
//nolint:errcheck,gosec

// All linters (discouraged)
//nolint

// With reason (golangci-lint convention)
//nolint:errcheck // reason here
```

## Policy

Enforce lint configuration hygiene.

```toml
[golang.policy]
lint_changes = "standalone"    # lint config changes must be standalone PRs
lint_config = [                # files that trigger standalone requirement
  ".golangci.yml",
  ".golangci.yaml",
  ".golangci.toml",
]
```

When `lint_changes = "standalone"`, changing any `lint_config` files alongside source/test changes fails:

```
golang: FAIL
  lint config changes must be standalone
    Changed: .golangci.yml
    Also changed: pkg/parser/parse.go, cmd/app/main.go
  Submit lint config changes in a separate PR.
```

## Build Metrics

Go build metrics are part of the `build` check. See [checks/build.md](../checks/build.md) for full details.

### Targets

Build targets for binary size tracking. Auto-detected from `main` packages in `cmd/` directory.

```toml
[golang]
targets = ["cmd/myapp", "cmd/myserver"]    # Override auto-detection
```

### Binary Size

Track release binary sizes (CI mode).

```
build: size
  myapp: 8.2 MB
  myserver: 12.4 MB
```

With threshold:
```
build: FAIL
  myapp: 9.1 MB (max: 8 MB)
```

### Build Time

Track build times (CI mode):

- **Cold**: `go clean -cache && go build -ldflags="-s -w"`
- **Hot**: Incremental rebuild

```
build: time
  cold (release): 18.4s
  hot: 0.9s
```

## Coverage

The `go` runner provides implicit Go coverage via `go test -cover`. Coverage data is in Go's native format and merges automatically across packages.

```toml
[[check.tests.suite]]
runner = "go"
# Implicit: covers Go code via built-in coverage

[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
setup = "go build -cover ./cmd/myapp"
targets = ["cmd/myapp"]             # Instrument Go binary
```

### Cover Profile

For CI mode, quench uses:

```bash
go test -coverprofile=coverage.out ./...
```

Coverage is aggregated across all packages and test runs.

## Configuration

```toml
[golang]
# Source/test patterns (falls back to [project].tests if not set)
# source = ["**/*.go"]
# tests = ["**/*_test.go"]
# ignore = ["vendor/"]

# Build targets (default: auto-detect from cmd/**/main.go)
# targets = ["cmd/myapp", "cmd/myserver"]

# Build metrics (CI mode) - see [check.build] for thresholds
binary_size = true
build_time = true

[golang.cloc]
check = "error"                  # error | warn | off
# advice = "..."                 # Custom advice for oversized Go files

[golang.suppress]
check = "comment"

[golang.suppress.test]
check = "allow"

[golang.policy]
lint_changes = "standalone"
lint_config = [".golangci.yml", ".golangci.yaml", ".golangci.toml"]
```

Test suites and coverage thresholds are configured in `[check.tests]`.
