# Phase 465: Go Adapter - Suppress

**Root Feature:** `quench-1034`

Implement Go `//nolint` directive detection and policies for the escapes check.

## Overview

This phase completes the Go suppress directive implementation by adding comprehensive behavioral specs, additional test fixtures, and ensuring all edge cases are properly handled. The core implementation (`adapter/go/suppress.rs`, `checks/escapes/go_suppress.rs`, `config/go.rs`) is already in place.

## Project Structure

```
crates/cli/src/
├── adapter/go/
│   ├── mod.rs              # Go adapter with file classification
│   ├── suppress.rs         # ✓ //nolint directive parsing
│   └── suppress_tests.rs   # ✓ Unit tests for parsing
├── checks/escapes/
│   ├── go_suppress.rs      # ✓ Violation checking logic
│   └── mod.rs              # ✓ Integration point
└── config/
    └── go.rs               # ✓ GoSuppressConfig struct

tests/
├── fixtures/golang/
│   ├── nolint-comment-fail/      # ✓ Exists
│   ├── nolint-comment-ok/        # ✓ Exists
│   ├── nolint-forbid-fail/       # NEW: Test forbid list
│   ├── nolint-allow-ok/          # NEW: Test allow list bypass
│   ├── nolint-multiple-codes/    # NEW: Test multiple linter codes
│   ├── nolint-all-linters/       # NEW: Test //nolint without codes
│   ├── nolint-custom-pattern/    # NEW: Test custom comment pattern
│   └── nolint-test-file-ok/      # NEW: Test file allows suppression
└── specs/adapters/
    └── golang.rs           # ✓ Behavioral specs (extend)
```

## Dependencies

No new external dependencies required. Uses existing:
- `serde` for config deserialization
- Common suppress infrastructure from `adapter/common/suppress.rs`
- Shared `SuppressScopeConfig` and `SuppressLevel` types

## Implementation Phases

### Phase 1: Verify Existing Implementation

**Goal:** Confirm all core functionality works correctly.

**Tasks:**
1. Run existing specs: `cargo test --test specs golang`
2. Verify `nolint-comment-fail` fixture produces violation
3. Verify `nolint-comment-ok` fixture passes
4. Review `go_suppress.rs` logic for correctness

**Files touched:**
- None (verification only)

**Verification:**
```bash
cargo test --test specs golang
cargo test adapter::go::suppress
```

---

### Phase 2: Add Multiple Linters Fixture and Spec

**Goal:** Test `//nolint:linter1,linter2` syntax.

**Tasks:**
1. Create `tests/fixtures/golang/nolint-multiple-codes/` fixture
2. Add behavioral spec to `tests/specs/adapters/golang.rs`

**Fixture structure:**
```
nolint-multiple-codes/
├── go.mod
├── quench.toml
└── main.go
```

**main.go:**
```go
package main

func main() {
    //nolint:errcheck,gosec // reason: both errors safely ignored
    riskyMultiple()
}

func riskyMultiple() error { return nil }
```

**Spec:**
```rust
/// Spec: docs/specs/langs/golang.md#supported-patterns
///
/// > //nolint:errcheck,gosec (multiple linters)
#[test]
fn nolint_with_multiple_codes_and_comment_passes() {
    check("escapes").on("golang/nolint-multiple-codes").passes();
}
```

**Verification:**
```bash
cargo test nolint_with_multiple_codes
```

---

### Phase 3: Add Forbid List Fixture and Spec

**Goal:** Test that `forbid` list prevents specific linter suppression.

**Tasks:**
1. Create `tests/fixtures/golang/nolint-forbid-fail/` fixture
2. Add behavioral spec for forbid list enforcement

**quench.toml:**
```toml
version = 1

[golang.suppress]
check = "comment"

[golang.suppress.source]
forbid = ["govet"]
```

**main.go:**
```go
package main

func main() {
    // OK: This comment exists but govet is forbidden
    //nolint:govet
    riskyFunction()
}

func riskyFunction() {}
```

**Spec:**
```rust
/// Spec: docs/specs/langs/golang.md#configuration
///
/// > forbid = ["govet"]             # never suppress go vet findings
#[test]
fn nolint_with_forbidden_code_fails() {
    check("escapes")
        .on("golang/nolint-forbid-fail")
        .fails()
        .stdout_has("govet")
        .stdout_has("forbidden");
}
```

**Verification:**
```bash
cargo test nolint_with_forbidden_code
```

---

### Phase 4: Add Allow List Fixture and Spec

**Goal:** Test that `allow` list skips comment requirement.

**Tasks:**
1. Create `tests/fixtures/golang/nolint-allow-ok/` fixture
2. Add behavioral spec for allow list bypass

**quench.toml:**
```toml
version = 1

[golang.suppress]
check = "comment"

[golang.suppress.source]
allow = ["unused"]
```

**main.go:**
```go
package main

// No comment needed for "unused" - it's in allow list
//nolint:unused
var unusedVar = "test"

func main() {}
```

**Spec:**
```rust
/// Spec: docs/specs/langs/golang.md#configuration
///
/// > allow = ["unused"]             # no comment needed for these
#[test]
fn nolint_with_allowed_code_passes_without_comment() {
    check("escapes").on("golang/nolint-allow-ok").passes();
}
```

**Verification:**
```bash
cargo test nolint_with_allowed_code
```

---

### Phase 5: Add Test File Behavior Fixture and Spec

**Goal:** Test that `*_test.go` files allow suppression by default.

**Tasks:**
1. Create `tests/fixtures/golang/nolint-test-file-ok/` fixture
2. Add behavioral spec for test file permissive behavior

**Fixture structure:**
```
nolint-test-file-ok/
├── go.mod
├── quench.toml
├── main.go
└── main_test.go
```

**main.go:**
```go
package main

func main() {}
```

**main_test.go:**
```go
package main

import "testing"

// No comment needed in test files (default: check = "allow")
//nolint:errcheck
func TestSomething(t *testing.T) {
    riskyTestFunction()
}

func riskyTestFunction() error { return nil }
```

**quench.toml:**
```toml
version = 1

[golang.suppress]
check = "comment"
# test.check defaults to "allow"
```

**Spec:**
```rust
/// Spec: docs/specs/langs/golang.md#suppress
///
/// > Default: "comment" for source, "allow" for test code.
#[test]
fn nolint_in_test_file_passes_without_comment() {
    check("escapes").on("golang/nolint-test-file-ok").passes();
}
```

**Verification:**
```bash
cargo test nolint_in_test_file
```

---

### Phase 6: Add All-Linters and Custom Pattern Fixtures

**Goal:** Test `//nolint` (all linters) and custom comment patterns.

**Tasks:**
1. Create `tests/fixtures/golang/nolint-all-linters/` fixture
2. Create `tests/fixtures/golang/nolint-custom-pattern/` fixture
3. Add behavioral specs for both cases

**nolint-all-linters/main.go:**
```go
package main

func main() {
    //nolint // reason: suppressing all linters temporarily
    riskyFunction()
}

func riskyFunction() error { return nil }
```

**nolint-custom-pattern/quench.toml:**
```toml
version = 1

[golang.suppress]
check = "comment"
comment = "// OK:"
```

**nolint-custom-pattern/main.go:**
```go
package main

func main() {
    // OK: Intentionally ignoring error in startup
    //nolint:errcheck
    riskyFunction()
}

func riskyFunction() error { return nil }
```

**Specs:**
```rust
/// Spec: docs/specs/langs/golang.md#supported-patterns
///
/// > //nolint (all linters, discouraged)
#[test]
fn nolint_all_linters_with_comment_passes() {
    check("escapes").on("golang/nolint-all-linters").passes();
}

/// Spec: docs/specs/langs/golang.md#configuration
///
/// > comment = "// OK:"           # optional: require specific pattern
#[test]
fn nolint_with_custom_pattern_passes() {
    check("escapes").on("golang/nolint-custom-pattern").passes();
}

/// Spec: docs/specs/langs/golang.md#configuration
///
/// > comment = "// OK:" requires that specific pattern
#[test]
fn nolint_without_custom_pattern_fails() {
    check("escapes")
        .on("golang/nolint-custom-pattern-fail")
        .fails()
        .stdout_has("// OK:");
}
```

**Verification:**
```bash
cargo test nolint_all_linters
cargo test nolint.*custom_pattern
```

## Key Implementation Details

### Nolint Directive Parsing

The parser in `adapter/go/suppress.rs` handles:

```go
//nolint                    // All linters (codes = [])
//nolint:errcheck           // Single linter (codes = ["errcheck"])
//nolint:errcheck,gosec     // Multiple linters (codes = ["errcheck", "gosec"])
//nolint:errcheck // reason // Inline comment counts as justification
```

Justification comments can appear:
1. **Inline**: `//nolint:errcheck // reason here`
2. **Previous line**: Comment directly above the directive

### Check Level Hierarchy

```
GoSuppressConfig
├── check: SuppressLevel     # Default level (comment)
├── comment: Option<String>  # Custom pattern (e.g., "// OK:")
├── source: SuppressScopeConfig
│   ├── check: Option<SuppressLevel>  # Override for source
│   ├── allow: Vec<String>            # Skip comment check
│   └── forbid: Vec<String>           # Never allowed
└── test: SuppressScopeConfig
    ├── check: Option<SuppressLevel>  # Default: Allow
    └── ...
```

### Violation Check Flow

```rust
// Pseudocode from go_suppress.rs
for directive in parse_nolint_directives(content) {
    let scope = if is_test_file { &config.test } else { &config.source };

    for code in directive.codes {
        // 1. Check forbid list - always fails
        if scope.forbid.contains(code) {
            emit_violation("suppress_forbidden");
            continue;
        }

        // 2. Check allow list - skip remaining checks
        if scope.allow.contains(code) {
            continue;
        }

        // 3. Check comment requirement
        if scope_check == Comment && !directive.has_comment {
            emit_violation("suppress_missing_comment");
        }

        // 4. Check forbid level
        if scope_check == Forbid {
            emit_violation("suppress_forbidden");
        }
    }
}
```

## Verification Plan

### Unit Tests
```bash
# Parser tests
cargo test adapter::go::suppress

# Config parsing tests
cargo test config::go
```

### Behavioral Specs
```bash
# All Go adapter specs
cargo test --test specs golang

# Specific suppress specs
cargo test --test specs nolint
```

### Full Check
```bash
make check
```

### Manual Verification
```bash
# Test failing fixture
cd tests/fixtures/golang/nolint-comment-fail
cargo run -- check --escapes

# Test passing fixture
cd tests/fixtures/golang/nolint-comment-ok
cargo run -- check --escapes
```

## Commit Message Template

```
Implement Go //nolint suppress detection (Phase 465)

Add comprehensive specs and fixtures for Go //nolint directive policies:
- Multiple linter codes (//nolint:errcheck,gosec)
- Forbid list enforcement ([golang.suppress.source.forbid])
- Allow list bypass ([golang.suppress.source.allow])
- Test file permissive behavior (default: allow)
- Custom comment patterns ([golang.suppress.comment])
- All-linters syntax (//nolint without codes)

Passing specs:
- nolint_with_multiple_codes_and_comment_passes
- nolint_with_forbidden_code_fails
- nolint_with_allowed_code_passes_without_comment
- nolint_in_test_file_passes_without_comment
- nolint_all_linters_with_comment_passes
- nolint_with_custom_pattern_passes
- nolint_without_custom_pattern_fails

Reference: docs/specs/langs/golang.md#suppress

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
