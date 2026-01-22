# Test Fixtures

Test fixtures for quench behavioral specs. Each fixture is a self-contained mini-project.

## Fixture Index

| Fixture | Description | Primary Checks |
|---------|-------------|----------------|
| `minimal/` | Empty project, no config | Default behavior |
| `rust-simple/` | Small Rust library | cloc, tests |
| `rust-workspace/` | Multi-package workspace | Package metrics |
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
        .with_violation("escapes.rs");
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

## Regenerating Fixtures

Most fixtures are static. The oversized file is generated:

```bash
./scripts/generate-oversized.sh > tests/fixtures/violations/src/oversized.rs
```

## Adding New Fixtures

1. Create directory under `tests/fixtures/`
2. Add minimal `quench.toml` (or none for default behavior test)
3. Add source files appropriate for the test scenario
4. Document in this README
5. Add specs that use the fixture
