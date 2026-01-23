# Phase 457: Go Test Fixtures

**Root Feature:** `quench-333a` (Go Language Support)

## Overview

Create comprehensive Go test fixtures for adapter testing. Phase 455 implemented the Go adapter with escape patterns, suppress checking, and policy enforcement. That phase also created targeted fixtures under `tests/fixtures/golang/` for specific behavioral tests. This phase adds:

1. **`go-simple/`** - A realistic small Go project with idiomatic structure (`cmd/`, `pkg/`, `internal/`)
2. **`go-multi/`** - A multi-package Go project for testing package enumeration
3. **Go violations** - Add Go-specific violations to the existing `violations/` fixture
4. **Documentation** - Update fixture README with Go fixture index

## Project Structure

```
tests/fixtures/
├── go-simple/                    # NEW: Small Go project (analogous to rust-simple)
│   ├── go.mod
│   ├── quench.toml
│   ├── cmd/
│   │   └── app/
│   │       └── main.go
│   ├── pkg/
│   │   └── math/
│   │       ├── math.go
│   │       └── math_test.go
│   └── internal/
│       └── config/
│           └── config.go
├── go-multi/                     # NEW: Multi-package project (analogous to rust-workspace)
│   ├── go.mod
│   ├── quench.toml
│   ├── cmd/
│   │   ├── server/
│   │   │   └── main.go
│   │   └── cli/
│   │       └── main.go
│   ├── pkg/
│   │   ├── api/
│   │   │   ├── api.go
│   │   │   └── api_test.go
│   │   └── storage/
│   │       ├── storage.go
│   │       └── storage_test.go
│   └── internal/
│       └── core/
│           ├── core.go
│           └── core_test.go
├── violations/
│   ├── quench.toml               # MODIFY: Add Go escape patterns
│   └── go/                       # NEW: Go violations subdirectory
│       ├── unsafe.go             # unsafe.Pointer without SAFETY
│       ├── linkname.go           # //go:linkname without LINKNAME
│       └── nolint.go             # //nolint without justification
└── CLAUDE.md                     # MODIFY: Add Go fixture index
```

## Dependencies

No new external dependencies. All fixtures are static file content.

## Implementation Phases

### Phase 1: Create go-simple Fixture

**Goal**: Create a minimal but realistic Go project structure that passes all checks.

**Tasks**:
1. Create `tests/fixtures/go-simple/go.mod` with module declaration
2. Create `tests/fixtures/go-simple/quench.toml` with version 1
3. Create `cmd/app/main.go` with simple main function
4. Create `pkg/math/math.go` with exported function
5. Create `pkg/math/math_test.go` with unit test
6. Create `internal/config/config.go` with internal package

**Key Files**:

```go
// go.mod
module example.com/go-simple

go 1.21
```

```toml
# quench.toml
version = 1

[project]
name = "go-simple"
```

```go
// cmd/app/main.go
package main

import (
    "fmt"

    "example.com/go-simple/pkg/math"
)

func main() {
    fmt.Println("Sum:", math.Add(1, 2))
}
```

```go
// pkg/math/math.go
package math

// Add returns the sum of two integers.
func Add(a, b int) int {
    return a + b
}
```

```go
// pkg/math/math_test.go
package math

import "testing"

func TestAdd(t *testing.T) {
    if Add(1, 2) != 3 {
        t.Error("expected 3")
    }
}
```

```go
// internal/config/config.go
package config

// Config holds application configuration.
type Config struct {
    Port int
}

// Default returns default configuration.
func Default() *Config {
    return &Config{Port: 8080}
}
```

**Verification**:
- `quench check tests/fixtures/go-simple` passes
- `check("cloc").on("go-simple").passes()` works in specs

---

### Phase 2: Create go-multi Fixture

**Goal**: Create a multi-package Go project for testing package enumeration and multi-binary projects.

**Tasks**:
1. Create `tests/fixtures/go-multi/go.mod`
2. Create `tests/fixtures/go-multi/quench.toml`
3. Create two binaries in `cmd/server/` and `cmd/cli/`
4. Create reusable packages in `pkg/api/` and `pkg/storage/`
5. Create internal core package with tests
6. Add test files for each package

**Key Files**:

```go
// go.mod
module example.com/go-multi

go 1.21
```

```toml
# quench.toml
version = 1

[project]
name = "go-multi"

[golang]
targets = ["cmd/server", "cmd/cli"]
```

```go
// cmd/server/main.go
package main

import (
    "example.com/go-multi/pkg/api"
    "example.com/go-multi/pkg/storage"
)

func main() {
    store := storage.New()
    api.Serve(store)
}
```

```go
// cmd/cli/main.go
package main

import (
    "fmt"

    "example.com/go-multi/pkg/storage"
)

func main() {
    store := storage.New()
    fmt.Println("CLI connected to:", store.Name())
}
```

```go
// pkg/api/api.go
package api

import "example.com/go-multi/pkg/storage"

// Serve starts the API server.
func Serve(store *storage.Store) {
    // API server logic
}
```

```go
// pkg/storage/storage.go
package storage

import "example.com/go-multi/internal/core"

// Store represents a data store.
type Store struct {
    engine *core.Engine
}

// New creates a new store.
func New() *Store {
    return &Store{engine: core.NewEngine()}
}

// Name returns the store name.
func (s *Store) Name() string {
    return s.engine.Name()
}
```

```go
// internal/core/core.go
package core

// Engine is the internal storage engine.
type Engine struct{}

// NewEngine creates a new engine.
func NewEngine() *Engine {
    return &Engine{}
}

// Name returns the engine name.
func (e *Engine) Name() string {
    return "core-engine"
}
```

**Verification**:
- `quench check tests/fixtures/go-multi` passes
- Both `cmd/server` and `cmd/cli` targets detected
- Package enumeration includes all packages

---

### Phase 3: Add Go Violations to violations/ Fixture

**Goal**: Add Go-specific violations that exercise Go escape patterns and suppress checking.

**Tasks**:
1. Create `tests/fixtures/violations/go/` directory
2. Add `unsafe.go` with `unsafe.Pointer` without SAFETY comment
3. Add `linkname.go` with `//go:linkname` without LINKNAME comment
4. Add `nolint.go` with `//nolint` without justification comment
5. Update `tests/fixtures/violations/quench.toml` to include Go patterns

**Key Files**:

```go
// violations/go/unsafe.go
package violations

import "unsafe"

// VIOLATION: unsafe.Pointer without SAFETY comment
func UnsafeExample() uintptr {
    var x int = 42
    ptr := unsafe.Pointer(&x)
    return uintptr(ptr)
}
```

```go
// violations/go/linkname.go
package violations

import _ "unsafe"

// VIOLATION: //go:linkname without LINKNAME comment
//go:linkname runtimeNano runtime.nanotime
func runtimeNano() int64
```

```go
// violations/go/nolint.go
package violations

import "os"

// VIOLATION: //nolint without justification comment
//nolint:errcheck
func NolintExample() {
    os.Remove("temp.txt")
}
```

**Config Updates** (add to `violations/quench.toml`):

```toml
# Go-specific escape patterns
[[check.escapes.patterns]]
name = "go_unsafe_pointer"
pattern = "unsafe\\.Pointer"
action = "comment"
comment = "// SAFETY:"
source = ["**/*.go"]

[[check.escapes.patterns]]
name = "go_linkname"
pattern = "//go:linkname"
action = "comment"
comment = "// LINKNAME:"
source = ["**/*.go"]

[golang.suppress]
check = "comment"
```

**Verification**:
- `check("escapes").on("violations").fails().stdout_has("unsafe.go")`
- `check("escapes").on("violations").fails().stdout_has("linkname.go")`
- Suppress check fails on `nolint.go`

---

### Phase 4: Update Fixture Documentation

**Goal**: Update `tests/fixtures/CLAUDE.md` with Go fixture index and documentation.

**Tasks**:
1. Add `go-simple/` to fixture index table
2. Add `go-multi/` to fixture index table
3. Add detailed descriptions for both fixtures
4. Document Go violations in violations section

**README Updates**:

Add to fixture index table:
```markdown
| `go-simple/` | Small Go project | cloc, tests |
| `go-multi/` | Multi-package Go project | Package metrics |
```

Add new sections:

```markdown
### go-simple/

A minimal Go project with idiomatic structure. Good baseline for testing Go detection and default behavior.

- `go.mod` with module declaration
- `cmd/app/main.go` with main function
- `pkg/math/` with exported package and tests
- `internal/config/` with internal package
- Under 750 lines (passes cloc)
- Proper test coverage (passes tests)

### go-multi/

Multi-package Go project for testing package-level metrics and breakdown.

- Module with multiple binaries (`cmd/server/`, `cmd/cli/`)
- Reusable packages in `pkg/api/` and `pkg/storage/`
- Internal core package with tests
- Package enumeration testing
```

Update violations table:
```markdown
| escapes | `go/unsafe.go` | `unsafe.Pointer` without SAFETY |
| escapes | `go/linkname.go` | `//go:linkname` without LINKNAME |
| suppress | `go/nolint.go` | `//nolint` without justification |
```

**Verification**:
- README documents all Go fixtures
- Fixture purpose and structure is clear
- Violations table includes Go violations

---

## Key Implementation Details

### Go Project Structure Conventions

The fixtures follow standard Go project layout:

```
project/
├── go.mod           # Module declaration (required for detection)
├── cmd/             # Main applications
│   └── app/
│       └── main.go
├── pkg/             # Public library code
│   └── lib/
│       ├── lib.go
│       └── lib_test.go
├── internal/        # Private application code
│   └── core/
│       └── core.go
└── vendor/          # Dependencies (ignored by quench)
```

### Fixture Size Guidelines

Following the pattern from existing fixtures:

- **go-simple**: ~50 total lines across all files
- **go-multi**: ~100 total lines across all files
- Each file is minimal but syntactically complete
- Tests are simple single-assertion functions

### Go Module Naming

Use `example.com/` prefix for fixture modules (standard practice for examples):
- `example.com/go-simple`
- `example.com/go-multi`

### Violation Patterns

Go violations must be syntactically valid Go code that triggers specific checks:

| Check | Violation | Trigger |
|-------|-----------|---------|
| escapes | `unsafe.Pointer` without comment | Missing `// SAFETY:` on preceding line |
| escapes | `//go:linkname` without comment | Missing `// LINKNAME:` on preceding line |
| suppress | `//nolint` without justification | Missing comment when `check = "comment"` |

## Verification Plan

### Phase 1 Verification (go-simple)

```bash
# Fixture builds
cd tests/fixtures/go-simple && go build ./...

# Quench passes
cargo run -- check tests/fixtures/go-simple

# Specs pass
cargo test --test specs -- go_simple
```

### Phase 2 Verification (go-multi)

```bash
# Fixture builds
cd tests/fixtures/go-multi && go build ./...

# Quench passes
cargo run -- check tests/fixtures/go-multi

# Package enumeration works
cargo run -- check tests/fixtures/go-multi --json | jq '.checks'
```

### Phase 3 Verification (violations)

```bash
# Escapes check fails on Go violations
cargo run -- check tests/fixtures/violations --escapes 2>&1 | grep -E 'unsafe.go|linkname.go'

# Suppress check fails on nolint
cargo run -- check tests/fixtures/violations --escapes 2>&1 | grep 'nolint.go'
```

### Phase 4 Verification (docs)

```bash
# README is valid markdown
cat tests/fixtures/CLAUDE.md | head -50

# All Go fixtures documented
grep -E 'go-simple|go-multi' tests/fixtures/CLAUDE.md
```

### Full Verification

```bash
# All checks pass
make check

# Go fixtures properly detected
cargo run -- check tests/fixtures/go-simple --json | jq '.languages'
cargo run -- check tests/fixtures/go-multi --json | jq '.languages'
```

## Checklist

- [ ] Phase 1: Create go-simple fixture (go.mod, cmd/, pkg/, internal/)
- [ ] Phase 2: Create go-multi fixture (multiple binaries, packages)
- [ ] Phase 3: Add Go violations to violations/ fixture
- [ ] Phase 4: Update fixture README with Go documentation
- [ ] Final: `make check` passes, fixtures build with `go build`
