# Phase 460: Go Adapter - Escape Patterns

**Root Feature:** `quench-c294`

## Overview

Implement Go-specific escape hatch patterns that require justification comments. This phase adds detection for `unsafe.Pointer`, `//go:linkname`, and `//go:noescape` patterns with corresponding comment requirements.

## Status: Implementation Complete

The core implementation was completed in prior phases. All behavioral specs pass.

## Project Structure

```
crates/cli/src/
├── adapter/
│   ├── go/
│   │   ├── mod.rs           # GO_ESCAPE_PATTERNS defined (lines 31-53)
│   │   ├── suppress.rs      # //nolint directive parsing
│   │   └── policy.rs        # Lint policy checking
│   └── common/
│       └── suppress.rs      # CommentStyle::GO defined (lines 21-25)
├── checks/
│   └── escapes/
│       ├── patterns.rs      # get_adapter_escape_patterns() (lines 46-48)
│       ├── comment.rs       # has_justification_comment() + is_go_directive()
│       └── mod.rs           # is_go_file() classification (lines 587-592)
tests/
├── fixtures/
│   └── golang/
│       ├── unsafe-pointer-ok/    # SAFETY comment passes
│       ├── unsafe-pointer-fail/  # Missing SAFETY fails
│       ├── linkname-ok/          # LINKNAME comment passes
│       ├── linkname-fail/        # Missing LINKNAME fails
│       ├── noescape-ok/          # NOESCAPE comment passes
│       └── noescape-fail/        # Missing NOESCAPE fails
└── specs/
    └── adapters/
        └── golang.rs        # Behavioral specs (14 passing)
```

## Dependencies

No new dependencies required. Uses existing:
- `globset` - File pattern matching
- `grep-regex` - Pattern matching via `CompiledPattern`

## Implementation Phases

### Phase 1: Verify Existing Implementation ✓

**Status: Complete**

Default escape patterns already defined in `crates/cli/src/adapter/go/mod.rs`:

```rust
const GO_ESCAPE_PATTERNS: &[EscapePattern] = &[
    EscapePattern {
        name: "unsafe_pointer",
        pattern: r"unsafe\.Pointer",
        action: EscapeAction::Comment,
        comment: Some("// SAFETY:"),
        advice: "Add a // SAFETY: comment explaining pointer validity.",
    },
    EscapePattern {
        name: "go_linkname",
        pattern: r"//go:linkname",
        action: EscapeAction::Comment,
        comment: Some("// LINKNAME:"),
        advice: "Add a // LINKNAME: comment explaining the external symbol dependency.",
    },
    EscapePattern {
        name: "go_noescape",
        pattern: r"//go:noescape",
        action: EscapeAction::Comment,
        comment: Some("// NOESCAPE:"),
        advice: "Add a // NOESCAPE: comment explaining why escape analysis should be bypassed.",
    },
];
```

### Phase 2: Verify Adapter Integration ✓

**Status: Complete**

- `GoAdapter::default_escapes()` returns `GO_ESCAPE_PATTERNS`
- `patterns.rs::get_adapter_escape_patterns()` calls `GoAdapter::default_escapes()` for Go projects
- Project language detection via `detect_language()` identifies `go.mod`

### Phase 3: Verify Comment Detection ✓

**Status: Complete**

The `comment.rs` module handles:
- `has_justification_comment()` - Searches upward for required comment pattern
- `is_go_directive()` - Identifies `//go:xxx` patterns as directives (not regular comments)
- Special case: Go directives at line start are detected as escape patterns, not skipped as comments

### Phase 4: Verify Test File Exemption ✓

**Status: Complete**

Test files (`*_test.go`) are classified as `FileKind::Test`:
- `GoAdapter::classify()` checks `test_patterns` (`**/*_test.go`) before `source_patterns`
- Escapes in test code are counted in metrics but never generate violations

### Phase 5: Add Missing Violation Fixture

**Status: Pending**

The `tests/fixtures/violations/go/` directory has:
- ✓ `unsafe.go` - unsafe.Pointer without SAFETY
- ✓ `linkname.go` - //go:linkname without LINKNAME
- ✓ `nolint.go` - //nolint without justification
- ✗ `noescape.go` - Missing (should have //go:noescape without NOESCAPE)

**Task:** Add `noescape.go` to violations fixture:

```go
// tests/fixtures/violations/go/noescape.go
package violations

// VIOLATION: //go:noescape without NOESCAPE comment
//go:noescape
func fastHash(data []byte) uint64
```

Also update `tests/fixtures/violations/quench.toml` to include go_noescape pattern.

### Phase 6: Unit Tests for Go Escape Patterns ✓

**Status: Complete**

Unit tests exist in `crates/cli/src/adapter/go_tests.rs`:
- `provides_default_escape_patterns` - Verifies all 3 patterns present with correct configs

## Key Implementation Details

### Pattern Detection Flow

1. `EscapesCheck::run()` calls `get_adapter_escape_patterns(ctx.root)`
2. `get_adapter_escape_patterns()` detects Go via `detect_language()` → `go.mod`
3. Creates `GoAdapter`, calls `default_escapes()` → `GO_ESCAPE_PATTERNS`
4. Converts adapter patterns to config format via `convert_adapter_patterns()`
5. Merges with user-configured patterns (user config takes precedence by name)
6. For each Go file, patterns are matched and `has_justification_comment()` is called

### Comment Search Algorithm

For `//go:noescape` on line N, the algorithm:
1. Checks line N for inline `// NOESCAPE:` comment (same line)
2. Walks backward from line N-1 looking for `// NOESCAPE:` at comment start
3. Skips blank lines and other comments
4. Stops at non-blank, non-comment lines (code)
5. The comment must START with the pattern (embedded mentions don't count)

### Go Directive Special Handling

In `is_match_in_comment()`:
```rust
// Special case: Go directive patterns at line start should be detected, not skipped.
// These look like comments but are actually compiler directives we want to check.
if match_offset_in_line == comment_start && is_go_directive(line_content) {
    return false;  // Not in comment, should be checked
}
```

## Verification Plan

### 1. Behavioral Specs (All Passing)

```bash
cargo test --test specs -- golang
```

Expected output:
```
test adapters::golang::unsafe_pointer_without_safety_comment_fails ... ok
test adapters::golang::unsafe_pointer_with_safety_comment_passes ... ok
test adapters::golang::go_linkname_without_linkname_comment_fails ... ok
test adapters::golang::go_linkname_with_linkname_comment_passes ... ok
test adapters::golang::go_noescape_without_noescape_comment_fails ... ok
test adapters::golang::go_noescape_with_noescape_comment_passes ... ok
```

### 2. Unit Tests

```bash
cargo test -p quench -- go
```

### 3. Manual Verification

```bash
# Test failing case
cd tests/fixtures/golang/unsafe-pointer-fail
quench check --escapes
# Expected: FAIL with "// SAFETY:" in advice

# Test passing case
cd tests/fixtures/golang/unsafe-pointer-ok
quench check --escapes
# Expected: PASS

# Repeat for linkname and noescape fixtures
```

### 4. Full CI Suite

```bash
make check
```

## Remaining Tasks

1. **Add noescape.go violation fixture** - Create `tests/fixtures/violations/go/noescape.go` with missing NOESCAPE comment
2. **Update violations quench.toml** - Add `go_noescape` pattern to match the fixture
3. **Verify complete test coverage** - Run `make check` to ensure all specs pass

## References

- `docs/specs/langs/golang.md` - Go escape patterns specification
- `docs/specs/checks/escape-hatches.md` - Escape framework specification
- `tests/specs/adapters/golang.rs` - Behavioral specs
