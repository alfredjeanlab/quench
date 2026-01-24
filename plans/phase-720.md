# Phase 720: Tests Check - Output

**Root Feature:** `quench-88fc`

## Overview

Enhance the tests check JSON output to include `change_type` and `lines_changed` fields in `missing_tests` violations. This provides agents and tooling with richer context about what kind of change triggered the violation and how significant it was.

Currently, the tests check generates violations with only a `lines` field (which is semantically for cloc violations). The spec requires dedicated `change_type` ("added" | "modified") and `lines_changed` fields in the JSON output.

**Expected output format:**
```json
{
  "file": "src/parser.rs",
  "type": "missing_tests",
  "change_type": "modified",
  "lines_changed": 79,
  "advice": "Add tests in tests/parser_tests.rs..."
}
```

## Project Structure

```
crates/cli/src/
├── check.rs                    # Add change_type, lines_changed fields
└── checks/tests/
    ├── mod.rs                  # Populate new fields in violations
    ├── mod_tests.rs            # Unit tests for new fields
    └── diff.rs                 # (existing) ChangeType enum

tests/specs/checks/tests/
└── correlation.rs              # Behavioral specs for JSON output
```

## Dependencies

No new external dependencies. Uses existing:
- `serde` - Serialization with skip_serializing_if
- `serde_json` - JSON output

## Implementation Phases

### Phase 1: Add Fields to Violation Struct

**Goal**: Add `change_type` and `lines_changed` fields to the Violation struct with a builder method.

**Files to modify**:
- `crates/cli/src/check.rs`

**Add fields to Violation struct** (after line 138):

```rust
/// Type of change for missing_tests violations ("added" | "modified").
#[serde(skip_serializing_if = "Option::is_none")]
pub change_type: Option<String>,

/// Lines changed (added + deleted) for missing_tests violations.
#[serde(skip_serializing_if = "Option::is_none")]
pub lines_changed: Option<i64>,
```

**Update all constructors** (file, file_only, commit_violation) to initialize:

```rust
change_type: None,
lines_changed: None,
```

**Add builder method** (after with_target):

```rust
/// Add change information for missing_tests violations.
pub fn with_change_info(
    mut self,
    change_type: impl Into<String>,
    lines_changed: i64,
) -> Self {
    self.change_type = Some(change_type.into());
    self.lines_changed = Some(lines_changed);
    self
}
```

**Verification**:
- Unit test: `violation_with_change_info_serializes_correctly()`
- Unit test: `violation_without_change_info_omits_fields()`

---

### Phase 2: Update Tests Check to Populate Fields

**Goal**: Modify the tests check to use the new builder method instead of directly setting `lines`.

**Files to modify**:
- `crates/cli/src/checks/tests/mod.rs`

**Import ChangeType**:

```rust
use self::diff::{ChangeType, get_base_changes, get_commits_since, get_staged_changes};
```

**Update run_branch_scope** (replace lines 154-158):

```rust
let mut v = Violation::file_only(path, "missing_tests", advice);

if let Some(c) = change {
    let change_type = match c.change_type {
        ChangeType::Added => "added",
        ChangeType::Modified => "modified",
        ChangeType::Deleted => "deleted",  // Won't occur for violations
    };
    v = v.with_change_info(change_type, c.lines_changed() as i64);
}

violations.push(v);
```

**Update run_commit_scope** (around line 260):

For commit scope, we need to look up the change from the commit's changes. Update the violation creation:

```rust
// Find the change info for this file in this commit
let change = commit.changes.iter().find(|c| {
    c.path.strip_prefix(ctx.root).unwrap_or(&c.path).eq(path)
});

let mut v = Violation::file_only(path, "missing_tests", advice);

if let Some(c) = change {
    let change_type = match c.change_type {
        ChangeType::Added => "added",
        ChangeType::Modified => "modified",
        ChangeType::Deleted => "deleted",
    };
    v = v.with_change_info(change_type, c.lines_changed() as i64);
}

violations.push(v);
```

**Verification**:
- Unit test: `branch_scope_includes_change_type()`
- Unit test: `commit_scope_includes_change_type()`

---

### Phase 3: Update Unit Tests

**Goal**: Add unit tests for the new violation fields.

**Files to modify**:
- `crates/cli/src/check_tests.rs`
- `crates/cli/src/checks/tests/mod_tests.rs`

**Add to check_tests.rs**:

```rust
#[test]
fn violation_with_change_info_serializes_correctly() {
    let v = Violation::file_only("src/foo.rs", "missing_tests", "Add tests")
        .with_change_info("modified", 42);

    let json = serde_json::to_value(&v).unwrap();

    assert_eq!(json["change_type"], "modified");
    assert_eq!(json["lines_changed"], 42);
}

#[test]
fn violation_without_change_info_omits_fields() {
    let v = Violation::file_only("src/foo.rs", "missing_tests", "Add tests");

    let json = serde_json::to_value(&v).unwrap();

    assert!(json.get("change_type").is_none());
    assert!(json.get("lines_changed").is_none());
}
```

**Add to mod_tests.rs** (if test helpers exist):

```rust
#[test]
fn missing_tests_violation_includes_change_type_added() {
    // Test that new files get change_type: "added"
}

#[test]
fn missing_tests_violation_includes_change_type_modified() {
    // Test that modified files get change_type: "modified"
}
```

**Verification**:
- `cargo test --package quench -- check_tests`
- `cargo test --package quench -- checks::tests::unit_tests`

---

### Phase 4: Behavioral Specs

**Goal**: Add behavioral specs that verify the JSON output format.

**Files to modify**:
- `tests/specs/checks/tests/correlation.rs`

**Add specs**:

```rust
/// Spec: JSON output includes change_type for modified files
#[test]
fn missing_tests_json_includes_change_type_modified() {
    let temp = Project::empty();
    temp.config(r#"[check.tests.commit]
check = "error"
"#);

    init_git_repo(temp.path());
    temp.file("src/existing.rs", "pub fn existing() {}");
    git_add_all(temp.path());
    git_commit(temp.path(), "initial");

    // Modify the file
    temp.file("src/existing.rs", "pub fn existing() {}\npub fn more() {}");
    git_stage(temp.path());

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--staged"])
        .json()
        .fails();

    let violations = result.violations_of_type("missing_tests");
    assert!(!violations.is_empty());

    let v = &violations[0];
    assert_eq!(v.get("change_type").and_then(|v| v.as_str()), Some("modified"));
    assert!(v.get("lines_changed").and_then(|v| v.as_i64()).is_some());
}

/// Spec: JSON output includes change_type for added files
#[test]
fn missing_tests_json_includes_change_type_added() {
    let temp = Project::empty();
    temp.config(r#"[check.tests.commit]
check = "error"
"#);

    init_git_repo(temp.path());
    temp.file("src/new_file.rs", "pub fn new_fn() {}");
    git_stage(temp.path());

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--staged"])
        .json()
        .fails();

    let violations = result.violations_of_type("missing_tests");
    assert!(!violations.is_empty());

    let v = &violations[0];
    assert_eq!(v.get("change_type").and_then(|v| v.as_str()), Some("added"));
}

/// Spec: lines_changed reflects actual diff size
#[test]
fn missing_tests_json_includes_lines_changed() {
    let temp = Project::empty();
    temp.config(r#"[check.tests.commit]
check = "error"
"#);

    init_git_repo(temp.path());

    // Create a file with known line count
    let content = (0..10).map(|i| format!("pub fn f{}() {{}}", i)).collect::<Vec<_>>().join("\n");
    temp.file("src/multi.rs", &content);
    git_stage(temp.path());

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--staged"])
        .json()
        .fails();

    let violations = result.violations_of_type("missing_tests");
    let v = &violations[0];

    // Should have 10 lines added
    assert_eq!(v.get("lines_changed").and_then(|v| v.as_i64()), Some(10));
}
```

**Verification**:
- `cargo test --test specs -- tests::correlation`

---

### Phase 5: Documentation and Cleanup

**Goal**: Ensure spec compliance and run final checks.

**Tasks**:
1. Verify JSON output matches spec in `docs/specs/checks/tests.md` (lines 157-188)
2. Bump `CACHE_VERSION` in `crates/cli/src/cache.rs` (check logic changed)
3. Run `make check` to verify all tests pass

**Spec compliance check**:

The output should match:
```json
{
  "file": "src/parser.rs",
  "line": null,
  "type": "missing_tests",
  "change_type": "modified",
  "lines_changed": 79,
  "advice": "Add tests in tests/parser_tests.rs or update inline #[cfg(test)] block"
}
```

**Verification**:
- `make check` passes
- JSON output matches spec format

---

## Key Implementation Details

### Field Naming

The spec uses `lines_changed` (not `lines`) for clarity:
- `lines`: existing field used by cloc check for total file lines
- `lines_changed`: new field for tests check, represents diff size (added + deleted)

Both fields can coexist on a Violation, but they serve different purposes.

### ChangeType Conversion

The `ChangeType` enum maps to strings for JSON output:
- `ChangeType::Added` → `"added"`
- `ChangeType::Modified` → `"modified"`
- `ChangeType::Deleted` → `"deleted"` (won't appear in violations since deleted files don't require tests)

### Exclude Patterns

Exclude patterns are already implemented in the correlation module (`crates/cli/src/checks/tests/correlation.rs`). Default excludes:
- `**/mod.rs` - Module declarations
- `**/lib.rs` - Library roots
- `**/main.rs` - Binary entry points
- `**/generated/**` - Generated code

These are configured via `[check.tests.commit].exclude` in quench.toml.

### Builder Pattern

The new `with_change_info()` method follows the existing builder pattern:
```rust
Violation::file_only(path, "missing_tests", advice)
    .with_change_info("modified", 42)
```

This keeps the API consistent with `with_threshold()`, `with_pattern()`, etc.

---

## Verification Plan

### Unit Tests

```bash
cargo test --package quench -- check_tests::violation_with_change_info
cargo test --package quench -- checks::tests::unit_tests
```

Tests:
- `violation_with_change_info_serializes_correctly()`
- `violation_without_change_info_omits_fields()`
- `branch_scope_includes_change_type()`
- `commit_scope_includes_change_type()`

### Behavioral Specs

```bash
cargo test --test specs -- tests::correlation
```

Specs:
- `missing_tests_json_includes_change_type_modified()`
- `missing_tests_json_includes_change_type_added()`
- `missing_tests_json_includes_lines_changed()`

### Integration

```bash
make check
```

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `cargo audit`
- `cargo deny check`
