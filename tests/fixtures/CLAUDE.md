# Test Fixtures

Test fixtures for quench behavioral specs. Each fixture is a self-contained mini-project.

## Fixture Index

| Fixture | Description | Primary Checks |
|---------|-------------|----------------|
| `minimal/` | Empty project, no config | Default behavior |
| `rust-simple/` | Small Rust library | cloc, tests |
| `rust-workspace/` | Multi-package workspace | Package metrics |
| `go-simple/` | Small Go project | cloc, tests |
| `go-multi/` | Multi-package Go project | Package metrics |
| `js-simple/` | Small JS/TS project | cloc, tests |
| `js-monorepo/` | Multi-package pnpm workspace | Package metrics |
| `shell-scripts/` | Shell scripts with bats | Shell escapes |
| `mixed/` | Rust CLI + shell scripts | Multi-language |
| `violations/` | Intentional violations | All checks |
| `docs-project/` | Proper docs structure | docs |
| `agents-project/` | Agent context files | agents |

## Usage in Specs

```rust
use crate::prelude::*;

#[test]
fn cloc_passes_on_simple_project() {
    check("cloc").on("rust-simple").passes();
}

#[test]
fn escapes_fails_on_unwrap() {
    check("escapes")
        .on("violations")
        .fails()
        .stdout_has("escapes.rs");
}
```

## Fixture Details

### minimal/

Bare project with no configuration. Tests that quench works with defaults and doesn't fail on empty projects.

- No `quench.toml`
- No source files
- Just `.gitkeep` to preserve directory

### rust-simple/

A minimal Rust library that passes all checks. Good baseline for testing default behavior.

- `quench.toml` with version 1
- `src/lib.rs` with simple function
- `src/lib_tests.rs` with unit test
- Under 750 lines (passes cloc)
- Proper test coverage (passes tests)

### rust-workspace/

Multi-package Rust workspace for testing package-level metrics and breakdown.

- Workspace with `crates/core/` and `crates/cli/`
- Integration tests at workspace root
- Package-specific metrics collection

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

### js-simple/

A minimal JavaScript/TypeScript project with idiomatic structure. Good baseline for testing JS detection and default behavior.

- `package.json` with module type and test script
- `tsconfig.json` with strict TypeScript config
- `src/index.ts` with main entry point
- `src/utils.ts` with utility functions
- `tests/index.test.ts` and `tests/utils.test.ts` with unit tests
- Under 750 lines (passes cloc)
- Proper test coverage (passes tests)

### js-monorepo/

Multi-package JavaScript/TypeScript monorepo for testing workspace detection and package enumeration.

- `pnpm-workspace.yaml` with `packages/*` glob
- Root `package.json` marked as private
- Root `tsconfig.json` with project references
- `packages/core/` - Core library with types and utilities
- `packages/cli/` - CLI package depending on core
- Each package has `src/` and `tests/` directories
- Tests pnpm workspace detection path

### shell-scripts/

Shell-only project for testing shell-specific checks.

- Shell scripts in `scripts/`
- Bats tests in `tests/`
- No Rust code

### mixed/

Combined Rust and shell project for testing multi-language detection.

- Rust CLI binary
- Shell install script
- Both bats and Rust tests

### violations/

Project with intentional violations for every check type. Essential for testing failure detection.

**Violations included:**

| Check | File | Violation |
|-------|------|-----------|
| cloc | `src/oversized.rs` | 800+ lines (max: 750) |
| escapes | `src/escapes.rs` | `.unwrap()`, `unsafe` without SAFETY |
| escapes | `scripts/bad.sh` | `shellcheck disable`, `set +e` |
| tests | `src/missing_tests.rs` | No corresponding test file |
| license | `src/no_license.rs` | Missing SPDX header |
| agents | `CLAUDE.md` | Table, missing "Landing the Plane" |
| docs | `docs/specs/CLAUDE.md` | Broken TOC path |
| docs | `docs/specs/broken-link.md` | Broken markdown link |
| escapes | `go/unsafe.go` | `unsafe.Pointer` without SAFETY |
| escapes | `go/linkname.go` | `//go:linkname` without LINKNAME |
| escapes | `go/noescape.go` | `//go:noescape` without NOESCAPE |
| suppress | `go/nolint.go` | `//nolint` without justification |
| escapes | `js/as-unknown.ts` | `as unknown` without CAST comment |
| escapes | `js/ts-ignore.ts` | `@ts-ignore` (forbidden in source) |
| suppress | `js/eslint-disable.ts` | `eslint-disable` without justification |

### docs-project/

Project with proper documentation structure for testing docs checks.

- `docs/specs/` with index and spec files
- Proper TOC with valid paths
- Working markdown links between files
- Required sections present

### agents-project/

Project with agent context files at multiple scopes.

- Root `CLAUDE.md` and `.cursorrules` (synced)
- Package-level `crates/api/CLAUDE.md`
- All required sections present
- No tables (forbidden)

## New Fixtures for File Walking

### bench-deep/

Deeply nested directory structure (120 levels) for testing depth limits.

- Generated by `scripts/fixtures/generate-bench-deep.sh`
- File at level 50: should be scanned with default limit
- File at level 120: should be skipped with default limit (100)
- Tests iterative traversal (recursive would stack overflow)

### gitignore-test/

Tests `.gitignore` file exclusion during file walking.

- Has standard `.gitignore` patterns
- `target/` and `vendor/` directories should be ignored
- `*.generated.rs` files should be ignored
- `src/lib.rs` should be scanned

### symlink-loop/

Tests symlink loop detection.

- Contains `loop -> .` symlink pointing to itself
- Created by `scripts/fixtures/setup-symlink-fixtures.sh`
- Should complete without hanging
- `src/lib.rs` should still be scanned

### custom-ignore/

Tests custom ignore patterns from `quench.toml`.

- Has `[project.ignore]` section with custom patterns
- Tests `*.snapshot`, `testdata/`, `**/fixtures/**` patterns
- `src/lib.rs` should be scanned, patterns should be ignored

## Regenerating Fixtures

Most fixtures are static. Some require generation:

```bash
# Generate oversized file
./scripts/fixtures/generate-oversized.sh > tests/fixtures/violations/src/oversized.rs

# Generate deeply nested structure
./scripts/fixtures/generate-bench-deep.sh

# Create symlinks (after clone)
./scripts/fixtures/setup-symlink-fixtures.sh

# Or run all fixture setup at once:
./scripts/fixtures/setup-test-fixtures.sh
```

## Adding New Fixtures

1. Create directory under `tests/fixtures/`
2. Add minimal `quench.toml` (or none for default behavior test)
3. Add source files appropriate for the test scenario
4. Document in this README
5. Add specs that use the fixture
