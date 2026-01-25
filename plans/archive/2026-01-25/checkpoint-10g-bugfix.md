# Checkpoint 10G: Bug Fixes - Dogfooding Milestone 2

**Root Feature:** `quench-10g`
**Follows:** checkpoint-10f-quickwins (Quick Wins)

## Overview

This checkpoint addresses bugs and edge cases discovered during Dogfooding Milestone 2. With the core features complete and test coverage strong (530 passing tests), this phase focuses on:

1. **Error message clarity** - Improve confusing CLI error messages
2. **Edge case handling** - Fix subtle bugs in boundary conditions
3. **Output consistency** - Ensure all output formats behave consistently
4. **Robustness** - Handle uncommon but valid inputs gracefully

These fixes are low-risk improvements that enhance the developer experience before Milestone 3 (full CI integration).

## Project Structure

Changes will touch:

```
crates/cli/src/
├── cli.rs                      # CLI argument validation
├── cmd_check.rs                # Error handling improvements
├── output/
│   ├── text.rs                 # Output consistency fixes
│   └── json.rs                 # Schema compliance
├── checks/
│   ├── git/mod.rs              # Git edge case handling
│   └── docs/toc/resolve.rs     # TOC resolution edge cases
└── cache.rs                    # Cache edge case handling
tests/specs/
├── cli/flags.rs                # New edge case tests
├── output/format.rs            # Output consistency tests
└── modes/cache.rs              # Cache edge case tests
```

## Dependencies

No new external dependencies. Uses existing crates only.

## Implementation Phases

### Phase 1: CLI Error Message Improvements

**Goal**: Make error messages more actionable and less confusing.

**Issues Identified**:

1. `--dry-run` without `--fix` shows cryptic message: `--dry-run requires --fix`
2. Missing config file message lacks path suggestions
3. Invalid config TOML errors don't suggest common fixes

**Files**:
- `crates/cli/src/cli.rs` - Argument validation
- `crates/cli/src/cmd_check.rs` - Error formatting

**Changes**:

```rust
// Current: "--dry-run requires --fix"
// Better: "--dry-run only works with --fix. Use 'quench check --fix --dry-run' to preview changes."

// Current: "config file not found: foo.toml"
// Better: "config file not found: foo.toml
//         Expected quench.toml in current directory or specify with -C <path>"
```

**Tests**:
```rust
#[test]
fn dry_run_without_fix_shows_helpful_error() {
    cli()
        .args(&["check", "--dry-run"])
        .fails()
        .stderr_has("--fix")
        .stderr_has("preview");
}

#[test]
fn missing_config_suggests_alternatives() {
    cli()
        .args(&["-C", "nonexistent.toml", "check"])
        .fails()
        .stderr_has("quench.toml");
}
```

**Verification**:
- [ ] `quench check --dry-run` shows helpful error
- [ ] Missing config shows path suggestion
- [ ] All error tests pass

---

### Phase 2: Git Check Edge Cases

**Goal**: Handle edge cases in git repository detection and commit message validation.

**Issues Identified**:

1. Git check on bare repo may panic or show confusing error
2. Git check in worktree may not detect parent repo correctly
3. Empty commit message handling could be clearer

**Files**:
- `crates/cli/src/checks/git/mod.rs` - Git detection logic
- `crates/cli/src/checks/git/parse.rs` - Message parsing

**Changes**:

```rust
// Handle bare repository gracefully
fn detect_git_root(path: &Path) -> Option<PathBuf> {
    // Check for .git file (worktree) or .git directory
    let git_path = path.join(".git");
    if git_path.is_file() {
        // Worktree: read gitdir from .git file
        // ...
    } else if git_path.is_dir() {
        // Normal repo
        // ...
    }
    // Return None for bare repos (no working tree)
    None
}
```

**Tests**:
```rust
#[test]
fn git_check_skips_bare_repository() {
    let temp = Project::empty_bare_repo();
    check("git")
        .pwd(temp.path())
        .passes()
        .stdout_has("SKIP: git");
}

#[test]
fn git_check_works_in_worktree() {
    let (main, worktree) = Project::with_worktree();
    check("git")
        .pwd(worktree.path())
        .passes();
}
```

**Verification**:
- [ ] Bare repos handled gracefully (skip, not error)
- [ ] Worktrees work correctly
- [ ] Empty commit message shows clear violation

---

### Phase 3: Cache Edge Cases

**Goal**: Fix subtle issues with cache invalidation and persistence.

**Issues Identified**:

1. Cache with very old mtime (1970) may cause issues on some systems
2. Cache file permissions on read-only filesystem shows confusing error
3. Race condition possible when multiple processes access cache

**Files**:
- `crates/cli/src/cache.rs` - Cache implementation
- `crates/cli/src/cache_tests.rs` - Additional tests

**Changes**:

```rust
// Handle epoch mtime gracefully
fn mtime_key(path: &Path) -> Option<u64> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    // Clamp very old times to avoid overflow
    let dur = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    Some(dur.as_secs())
}

// Handle read-only cache directory
fn persist_cache(&self) -> Result<()> {
    let cache_dir = self.path.parent().unwrap_or(&self.path);
    if let Err(e) = fs::create_dir_all(cache_dir) {
        tracing::debug!("cache directory not writable: {}", e);
        return Ok(()); // Gracefully skip cache on read-only fs
    }
    // ... rest of persist logic
}
```

**Tests**:
```rust
#[test]
fn cache_handles_epoch_mtime() {
    let file = temp_file_with_mtime(0); // 1970-01-01
    let cache = FileCache::new();
    // Should not panic
    cache.check_file(&file);
}

#[test]
fn cache_skips_on_readonly_filesystem() {
    let temp = Project::readonly_dir();
    check("cloc")
        .pwd(temp.path())
        .passes(); // Should work, just without caching
}
```

**Verification**:
- [ ] Epoch mtime files don't crash
- [ ] Read-only directories work (no cache)
- [ ] Concurrent access doesn't corrupt cache

---

### Phase 4: TOC Resolution Edge Cases

**Goal**: Fix false positives in table-of-contents validation for documentation.

**Issues Identified**:

1. TOC entries with trailing slashes not handled consistently
2. TOC entries with Windows path separators on Unix fail incorrectly
3. TOC entries with encoded characters (e.g., `%20`) not decoded

**Files**:
- `crates/cli/src/checks/docs/toc/resolve.rs` - Path resolution
- `crates/cli/src/checks/docs/toc/parse.rs` - Path parsing

**Changes**:

```rust
// Normalize path separators
fn normalize_toc_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_end_matches('/')
        .to_string()
}

// Handle URL-encoded paths
fn decode_toc_path(path: &str) -> String {
    percent_encoding::percent_decode_str(path)
        .decode_utf8_lossy()
        .to_string()
}
```

**Tests**:
```rust
#[test]
fn toc_handles_trailing_slash() {
    let temp = default_project();
    temp.file("docs/CLAUDE.md", "```\ndocs/specs/\n```");
    temp.mkdir("docs/specs");
    check("docs").pwd(temp.path()).passes();
}

#[test]
fn toc_handles_windows_separators() {
    let temp = default_project();
    temp.file("docs/CLAUDE.md", "```\ndocs\\specs\\file.md\n```");
    temp.file("docs/specs/file.md", "# File");
    check("docs").pwd(temp.path()).passes();
}
```

**Verification**:
- [ ] Trailing slashes handled correctly
- [ ] Windows separators normalized
- [ ] URL-encoded paths decoded

---

### Phase 5: Output Consistency

**Goal**: Ensure consistent behavior across all output formats.

**Issues Identified**:

1. Timing output format differs between text and JSON
2. Check names in summary may differ from JSON check names
3. Empty violations array omitted in some cases, included in others

**Files**:
- `crates/cli/src/output/text.rs` - Text formatting
- `crates/cli/src/output/json.rs` - JSON formatting

**Changes**:

```rust
// Ensure consistent timing format (always milliseconds)
fn format_duration_ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

// Ensure check names match between text and JSON
const CHECK_NAMES: &[&str] = &[
    "cloc", "escapes", "agents", "docs", "tests", "git", "build", "license", "placeholders"
];

// Always include violations array (empty if no violations)
fn format_check(check: &CheckResult) -> serde_json::Value {
    json!({
        "name": check.name,
        "passed": check.passed,
        "violations": check.violations, // Always present
        // ...
    })
}
```

**Tests**:
```rust
#[test]
fn json_always_includes_violations_array() {
    let result = check("cloc").on("clean").json().passes();
    assert!(result.require("violations").is_array());
}

#[test]
fn check_names_match_between_text_and_json() {
    let text = cli().on("violations").text();
    let json = cli().on("violations").json();

    // Extract check names from both formats
    // Assert they match
}
```

**Verification**:
- [ ] Timing format consistent across outputs
- [ ] Check names match between text and JSON
- [ ] violations array always present in JSON

---

### Phase 6: Documentation and Cleanup

**Goal**: Document fixes and clean up any remaining issues.

**Tasks**:

1. Update docs if any behavior changed
2. Remove any temporary workarounds from previous phases
3. Verify `make check` passes
4. Run full dogfooding verification

**Verification**:
```bash
# Full test suite
make check

# Dogfooding verification
cargo run --release -- check --timing

# Specific edge case tests
cargo test cache_handles
cargo test toc_handles
cargo test json_always
```

---

## Key Implementation Details

### Error Message Design

Error messages should follow this structure:
1. **What went wrong** (brief description)
2. **Why it matters** (context, if not obvious)
3. **How to fix it** (actionable suggestion)

```
Error: --dry-run requires --fix
       The --dry-run flag shows what --fix would change without applying changes.
       Use: quench check --fix --dry-run
```

### Backward Compatibility

All fixes maintain backward compatibility:
- No command-line interface changes
- No configuration format changes
- No output format breaking changes (only additions)

### Testing Strategy

Each phase includes:
1. **Unit tests** in sibling `_tests.rs` files
2. **Spec tests** in `tests/specs/` for behavior
3. **Edge case tests** that document the fixed issue

---

## Verification Plan

### Per-Phase Verification

Each phase has inline checkboxes. Complete before moving to next phase.

### Final Verification

```bash
# Full test suite
make check

# Dogfooding
cargo run --release -- check --timing

# Edge case manual testing
./target/release/quench check --dry-run 2>&1 | grep -q "fix"
./target/release/quench check -C nonexistent.toml 2>&1 | grep -q "quench.toml"
```

### Success Criteria

1. **All tests pass**: `cargo test --all` exits 0
2. **No new warnings**: `cargo clippy` clean
3. **Dogfooding passes**: `quench check` on quench reports 0 violations
4. **Error messages improved**: Verified manually
5. **Edge cases handled**: All identified cases have tests

---

## Risk Assessment

| Phase | Risk | Mitigation |
|-------|------|------------|
| 1. CLI Errors | Very Low | Text changes only, no logic changes |
| 2. Git Edge Cases | Low | Add checks without changing success paths |
| 3. Cache Edge Cases | Low | Graceful degradation approach |
| 4. TOC Edge Cases | Low | Normalization before comparison |
| 5. Output Consistency | Low | Additive changes only |
| 6. Cleanup | Very Low | Documentation only |

---

## Summary

| Phase | Deliverable | Purpose |
|-------|-------------|---------|
| 1 | Better error messages | Improved developer experience |
| 2 | Git edge case handling | Robustness for unusual repos |
| 3 | Cache edge case handling | Reliability on edge-case filesystems |
| 4 | TOC edge case handling | Fewer false positives in docs check |
| 5 | Output consistency | Predictable behavior across formats |
| 6 | Documentation | Clean handoff for next milestone |

---

## Completion Criteria

- [ ] Phase 1: Error messages improved
- [ ] Phase 2: Git edge cases tested
- [ ] Phase 3: Cache edge cases tested
- [ ] Phase 4: TOC edge cases tested
- [ ] Phase 5: Output consistency verified
- [ ] Phase 6: `make check` passes
- [ ] `./done` executed successfully
