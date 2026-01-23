# Phase 451: Go Adapter Behavioral Specs

## Overview

Write behavioral specs for the Go language adapter, testing detection, default patterns, escape patterns, suppress directives, and lint policy enforcement. All specs will be marked with `#[ignore = "TODO: Phase 455+"]` for future implementation.

## Project Structure

```
tests/
├── specs/
│   └── adapters/
│       ├── mod.rs          # Add golang module
│       └── golang.rs       # NEW: Go adapter specs (~400 lines)
└── fixtures/
    └── golang/             # NEW: Go test fixtures
        ├── auto-detect/
        ├── vendor-ignore/
        ├── module-packages/
        ├── unsafe-pointer-fail/
        ├── unsafe-pointer-ok/
        ├── linkname-fail/
        ├── linkname-ok/
        ├── noescape-fail/
        ├── noescape-ok/
        ├── nolint-comment-fail/
        ├── nolint-comment-ok/
        └── lint-policy-fail/
```

## Dependencies

No new external dependencies. Uses existing test infrastructure:
- `tests/specs/prelude.rs` - Test helpers (`check()`, `cli()`, `temp_project()`)
- `tests/fixtures/` - Fixture organization patterns

## Implementation Phases

### Phase 1: Test Infrastructure Setup

**Goal**: Create golang module and basic test file structure.

**Tasks**:
1. Create `tests/specs/adapters/golang.rs` with module structure
2. Add `mod golang;` to `tests/specs/adapters/mod.rs`
3. Create basic fixture directory `tests/fixtures/golang/`

**Verification**: `cargo test --test specs` compiles without errors.

---

### Phase 2: Auto-Detection and Default Pattern Specs

**Goal**: Spec Go adapter detection and file patterns.

**Specs**:
- `auto_detected_when_go_mod_present` - go.mod triggers golang adapter
- `default_source_pattern_matches_go_files` - **/*.go applied
- `default_test_pattern_matches_test_files` - **/*_test.go applied
- `default_ignores_vendor_directory` - vendor/** excluded

**Fixtures**:
```
golang/auto-detect/
├── go.mod
├── main.go
└── pkg/
    └── lib.go

golang/vendor-ignore/
├── go.mod
├── main.go
└── vendor/
    └── dep/
        └── dep.go
```

**Test Pattern** (from rust.rs/shell.rs):
```rust
/// Spec: docs/specs/langs/golang.md#detection
///
/// > Go is detected when `go.mod` exists in the project root.
#[test]
#[ignore = "TODO: Phase 455+"]
fn auto_detected_when_go_mod_present() {
    cli().on("golang/auto-detect").passes();
}
```

---

### Phase 3: Module and Package Detection Specs

**Goal**: Spec module name and package structure detection.

**Specs**:
- `detects_module_name_from_go_mod` - Extracts module path
- `detects_packages_from_directory_structure` - Maps directories to packages

**Fixtures**:
```
golang/module-packages/
├── go.mod              # module example.com/myapp
├── main.go             # package main
├── internal/
│   └── config/
│       └── config.go   # package config
└── pkg/
    └── api/
        └── api.go      # package api
```

**Test Pattern**:
```rust
/// Spec: docs/specs/langs/golang.md#detection
///
/// > The module name is extracted from `go.mod`.
#[test]
#[ignore = "TODO: Phase 455+"]
fn detects_module_name_from_go_mod() {
    cli()
        .on("golang/module-packages")
        .json()
        .stdout_has("example.com/myapp");
}
```

---

### Phase 4: Escape Pattern Specs (unsafe.Pointer, go:linkname, go:noescape)

**Goal**: Spec escape patterns requiring justification comments.

**Specs**:
- `unsafe_pointer_without_safety_comment_fails` - Requires `// SAFETY:`
- `unsafe_pointer_with_safety_comment_passes` - Accepted with comment
- `go_linkname_without_linkname_comment_fails` - Requires `// LINKNAME:`
- `go_linkname_with_linkname_comment_passes` - Accepted with comment
- `go_noescape_without_noescape_comment_fails` - Requires `// NOESCAPE:`
- `go_noescape_with_noescape_comment_passes` - Accepted with comment

**Fixtures**:
```
golang/unsafe-pointer-fail/
├── go.mod
└── main.go             # unsafe.Pointer without SAFETY comment

golang/unsafe-pointer-ok/
├── go.mod
└── main.go             # // SAFETY: reason\n unsafe.Pointer

golang/linkname-fail/
├── go.mod
└── main.go             # //go:linkname without LINKNAME comment

golang/linkname-ok/
├── go.mod
└── main.go             # // LINKNAME: reason\n //go:linkname

golang/noescape-fail/
├── go.mod
└── main.go             # //go:noescape without NOESCAPE comment

golang/noescape-ok/
├── go.mod
└── main.go             # // NOESCAPE: reason\n //go:noescape
```

**Test Pattern** (from shell.rs escape patterns):
```rust
/// Spec: docs/specs/langs/golang.md#default-escape-patterns
///
/// > `unsafe.Pointer` requires `// SAFETY:` comment explaining why.
#[test]
#[ignore = "TODO: Phase 455+"]
fn unsafe_pointer_without_safety_comment_fails() {
    check("escapes").on("golang/unsafe-pointer-fail").fails();
}

#[test]
#[ignore = "TODO: Phase 455+"]
fn unsafe_pointer_with_safety_comment_passes() {
    check("escapes").on("golang/unsafe-pointer-ok").passes();
}
```

---

### Phase 5: Suppress and Policy Specs

**Goal**: Spec `//nolint` handling and lint config policy.

**Specs**:
- `nolint_without_comment_fails_when_comment_required` - `check = "comment"` mode
- `nolint_with_comment_passes` - Justification provided
- `lint_config_changes_with_source_fails_standalone_policy` - Mixed PR detection

**Fixtures**:
```
golang/nolint-comment-fail/
├── go.mod
├── quench.toml         # [golang.suppress]\n check = "comment"
└── main.go             # //nolint:errcheck without comment

golang/nolint-comment-ok/
├── go.mod
├── quench.toml         # [golang.suppress]\n check = "comment"
└── main.go             # //nolint:errcheck // reason: ...

golang/lint-policy-fail/
├── go.mod
├── quench.toml         # [golang.policy]\n lint_changes = "standalone"
├── .golangci.yml       # Modified in same changeset
└── main.go             # Source modified in same changeset
```

**Test Pattern** (from shell.rs suppress/policy):
```rust
/// Spec: docs/specs/langs/golang.md#suppress
///
/// > When `check = "comment"`, `//nolint` requires justification.
#[test]
#[ignore = "TODO: Phase 455+"]
fn nolint_without_comment_fails_when_comment_required() {
    check("suppress").on("golang/nolint-comment-fail").fails();
}

/// Spec: docs/specs/langs/golang.md#policy
///
/// > `lint_changes = "standalone"` requires lint config in separate PRs.
#[test]
#[ignore = "TODO: Phase 455+"]
fn lint_config_changes_with_source_fails_standalone_policy() {
    check("policy").on("golang/lint-policy-fail").fails();
}
```

---

## Key Implementation Details

### Fixture go.mod Format

Minimal valid `go.mod`:
```
module example.com/fixture

go 1.21
```

### Escape Pattern Comment Format

From `docs/specs/langs/golang.md`:
```go
// SAFETY: Converting pointer to access underlying memory layout
ptr := unsafe.Pointer(uintptr(0x1234))

// LINKNAME: Accessing runtime internal for profiling
//go:linkname runtimeNano runtime.nanotime

// NOESCAPE: Verified safe - pointer does not escape
//go:noescape
func fastHash(data []byte) uint64
```

### Suppress Directive Format

```go
//nolint:errcheck // reason: error intentionally ignored in tests
result, _ := riskyFunction()
```

### Test Naming Convention

Follow existing adapters:
- Descriptive snake_case function names
- Pattern: `{feature}_{condition}_{outcome}`
- Examples: `auto_detected_when_go_mod_present`, `unsafe_pointer_without_safety_comment_fails`

### Spec Documentation Format

Each test includes:
```rust
/// Spec: docs/specs/langs/golang.md#{section}
///
/// > Quoted text from specification
#[test]
#[ignore = "TODO: Phase 455+"]
fn descriptive_test_name() {
    // ...
}
```

## Verification Plan

### After Each Phase

1. **Compile check**: `cargo test --test specs --no-run`
2. **Ignored tests visible**: `cargo test --test specs -- --ignored --list 2>&1 | grep golang`

### Final Verification

```bash
# All golang specs compile
cargo test --test specs -- golang --ignored --list

# Fixtures are valid
ls tests/fixtures/golang/*/go.mod

# Full test suite passes
make check
```

### Expected Output

```
test specs::adapters::golang::auto_detected_when_go_mod_present ... ignored
test specs::adapters::golang::default_source_pattern_matches_go_files ... ignored
test specs::adapters::golang::default_test_pattern_matches_test_files ... ignored
test specs::adapters::golang::default_ignores_vendor_directory ... ignored
test specs::adapters::golang::detects_module_name_from_go_mod ... ignored
test specs::adapters::golang::detects_packages_from_directory_structure ... ignored
test specs::adapters::golang::unsafe_pointer_without_safety_comment_fails ... ignored
test specs::adapters::golang::unsafe_pointer_with_safety_comment_passes ... ignored
test specs::adapters::golang::go_linkname_without_linkname_comment_fails ... ignored
test specs::adapters::golang::go_linkname_with_linkname_comment_passes ... ignored
test specs::adapters::golang::go_noescape_without_noescape_comment_fails ... ignored
test specs::adapters::golang::go_noescape_with_noescape_comment_passes ... ignored
test specs::adapters::golang::nolint_without_comment_fails_when_comment_required ... ignored
test specs::adapters::golang::nolint_with_comment_passes ... ignored
test specs::adapters::golang::lint_config_changes_with_source_fails_standalone_policy ... ignored
```

## Checklist

- [ ] Phase 1: Create golang.rs module, add to mod.rs
- [ ] Phase 2: Auto-detection and default pattern specs + fixtures
- [ ] Phase 3: Module/package detection specs + fixtures
- [ ] Phase 4: Escape pattern specs (6 tests) + fixtures (6 directories)
- [ ] Phase 5: Suppress and policy specs + fixtures
- [ ] Final: `make check` passes
