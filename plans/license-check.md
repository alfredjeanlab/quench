# License Check Implementation Plan

Implement license check: SPDX header detection, validation, comment syntax, and --fix.

## Overview

The license check validates that source files have proper SPDX license headers and copyright notices. It supports:
- SPDX-License-Identifier detection and validation
- Copyright year validation (including range formats)
- Language-specific comment syntax
- Auto-fix for missing/outdated headers

**Current state**: Detection is already implemented in `crates/cli/src/checks/license.rs`. The remaining work is implementing the `--fix` functionality (Phase 1120).

## Project Structure

```
crates/cli/src/
├── checks/
│   ├── license.rs          # Main check implementation (existing)
│   └── license_tests.rs    # Unit tests (existing)
├── config/mod.rs           # LicenseConfig (existing)
tests/
├── fixtures/license/       # Test fixtures (existing)
│   ├── missing-header/
│   ├── wrong-license/
│   ├── outdated-year/
│   ├── valid-headers/
│   ├── mixed-violations/
│   └── with-shebang/
└── specs/checks/license.rs # Behavioral specs (4 ignored for fix)
docs/specs/checks/
└── license-headers.md      # Specification (existing)
```

## Dependencies

Already available in the project:
- `chrono` - Date/year handling
- `regex` - Pattern matching for SPDX/copyright
- `globset` - File pattern matching
- `serde_json` - Fix summary serialization

## Implementation Phases

### Phase 1: Add Comment Syntax Helper (Phase 1115)

Add helper function to determine comment prefix by file extension.

**File**: `crates/cli/src/checks/license.rs`

```rust
/// Get comment prefix for a file based on extension.
fn comment_prefix_for_extension(ext: &str) -> &'static str {
    match ext {
        // Line comment languages
        "rs" | "ts" | "tsx" | "js" | "jsx" | "go" | "c" | "cpp" | "h" => "// ",
        // Hash comment languages
        "sh" | "bash" | "py" | "rb" | "yaml" | "yml" => "# ",
        // Default to line comments
        _ => "// ",
    }
}
```

**Milestone**: Unit test passes for comment prefix selection.

---

### Phase 2: Add Header Generation (Phase 1115)

Add function to generate license header content.

**File**: `crates/cli/src/checks/license.rs`

```rust
/// Generate a license header for a file.
fn generate_header(license: &str, copyright_holder: &str, year: i32, ext: &str) -> String {
    let prefix = comment_prefix_for_extension(ext);
    format!(
        "{prefix}SPDX-License-Identifier: {license}\n\
         {prefix}Copyright (c) {year} {copyright_holder}\n"
    )
}
```

**Milestone**: Unit test passes for header generation with various extensions.

---

### Phase 3: Implement Fix Logic (Phase 1120)

Add `--fix` handling to the check's `run` method. Three fix operations:

1. **Add missing headers** - Insert at file start (after shebang if present)
2. **Update outdated years** - Replace year in copyright line
3. **Fix wrong license** - Replace license identifier (optional, may skip)

**Pattern from agents check**:
```rust
if ctx.fix {
    if ctx.dry_run {
        // Preview: collect diffs without modifying files
        fixes.add_preview(...);
    } else {
        // Apply: write modified content to file
        std::fs::write(&file.path, &new_content).ok();
        fixes.add_fixed(...);
    }
}
```

**Key considerations**:
- Preserve shebang lines (detect `#!` at start)
- Use correct comment syntax per file extension
- Track fix counts for summary output
- Handle dry_run mode (preview without writing)

**Fix tracking structure**:
```rust
struct LicenseFixes {
    headers_added: Vec<String>,      // file paths
    years_updated: Vec<String>,      // file paths
    dry_run: bool,
}

impl LicenseFixes {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "headers_added": self.headers_added.len(),
            "years_updated": self.years_updated.len(),
            "files": {
                "added": self.headers_added,
                "updated": self.years_updated,
            }
        })
    }
}
```

**Milestone**: `license_fix_adds_missing_header` spec passes.

---

### Phase 4: Year Update Implementation (Phase 1120)

Implement year update in existing headers.

**Logic**:
```rust
/// Update copyright year in content.
fn update_copyright_year(content: &str, current_year: i32) -> String {
    // Find the copyright line
    // Replace year (single or range) with current year
    // If range ends before current year, extend range
    // If single year, create range or replace
}
```

**Year handling**:
- `2020` -> `2020-2026` (extend to range)
- `2020-2025` -> `2020-2026` (extend range end)
- `2026` -> no change needed

**Milestone**: `license_fix_updates_outdated_year` spec passes.

---

### Phase 5: Shebang Preservation (Phase 1120)

Ensure headers are inserted after shebangs.

**Logic**:
```rust
fn insert_header_preserving_shebang(content: &str, header: &str) -> String {
    if content.starts_with("#!") {
        // Find end of shebang line
        if let Some(newline_pos) = content.find('\n') {
            let shebang = &content[..=newline_pos];
            let rest = &content[newline_pos + 1..];
            return format!("{shebang}{header}\n{rest}");
        }
    }
    // No shebang, prepend header
    format!("{header}\n{content}")
}
```

**Milestone**: `license_fix_preserves_shebang` spec passes.

---

### Phase 6: Integration and Cleanup (Phase 1120)

1. Wire fix logic into main `run()` method
2. Return `CheckResult::fixed()` when fixes applied
3. Update cache version in `crates/cli/src/cache.rs` if check logic changed
4. Remove `#[ignore]` from spec tests
5. Run full test suite

**Final check flow**:
```rust
fn run(&self, ctx: &CheckContext) -> CheckResult {
    // ... existing detection logic ...

    // If --fix is enabled, apply fixes
    let mut fixes = LicenseFixes::new(ctx.dry_run);

    for file in ctx.files {
        // ... pattern matching ...

        if ctx.fix {
            match violation_type {
                "missing_header" => {
                    let header = generate_header(...);
                    let new_content = insert_header_preserving_shebang(&content, &header);
                    if !ctx.dry_run {
                        std::fs::write(&file.path, &new_content).ok();
                    }
                    fixes.headers_added.push(relative_path.display().to_string());
                }
                "outdated_year" => {
                    let new_content = update_copyright_year(&content, current_year);
                    if !ctx.dry_run {
                        std::fs::write(&file.path, &new_content).ok();
                    }
                    fixes.years_updated.push(relative_path.display().to_string());
                }
                _ => {}
            }
        } else {
            violations.push(...);
        }
    }

    // Return result
    if ctx.fix && !fixes.is_empty() {
        CheckResult::fixed(self.name(), fixes.to_json())
    } else if violations.is_empty() {
        CheckResult::passed(self.name())
    } else {
        CheckResult::failed(self.name(), violations)
    }
}
```

**Milestone**: All 4 ignored specs pass, `make check` passes.

## Key Implementation Details

### Regex Patterns (existing)

```rust
static SPDX_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"SPDX-License-Identifier:\s*(\S+)").expect("valid regex"));

static COPYRIGHT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Copyright\s+\([cC]\)\s+(\d{4}(?:-\d{4})?)\s+(.+)").expect("valid regex")
});
```

### Comment Syntax by Extension

| Extensions | Prefix |
|------------|--------|
| rs, ts, tsx, js, jsx, go, c, cpp, h | `// ` |
| sh, bash, py, rb, yaml, yml | `# ` |

### Fix Summary JSON Format

```json
{
  "headers_added": 3,
  "years_updated": 2,
  "files": {
    "added": ["src/lib.rs", "src/main.rs", "src/util.rs"],
    "updated": ["src/old.rs", "src/legacy.rs"]
  }
}
```

### Edge Cases to Handle

1. **Empty files** - Add header as only content
2. **Binary files** - Skip (handled by UTF-8 check)
3. **Shebang with blank line** - Insert header after shebang, preserve blank line if present
4. **Files with only shebang** - Add header after shebang
5. **UTF-8 BOM** - Preserve BOM if present (rare, but possible)

## Verification Plan

### Unit Tests (`crates/cli/src/checks/license_tests.rs`)

1. `comment_prefix_for_extension` returns correct prefix
2. `generate_header` produces valid headers
3. `insert_header_preserving_shebang` handles all cases
4. `update_copyright_year` handles single/range formats

### Behavioral Specs (`tests/specs/checks/license.rs`)

Remove `#[ignore]` from:
- `license_fix_adds_missing_header`
- `license_fix_updates_outdated_year`
- `license_fix_preserves_shebang`
- `exact_fix_output_text`

### Integration

```bash
# Run all license specs
cargo test --test specs license

# Run full check suite
make check
```

### Manual Testing

```bash
# Create test project
mkdir -p /tmp/license-test/src
echo 'pub fn hello() {}' > /tmp/license-test/src/lib.rs
cat > /tmp/license-test/quench.toml << 'EOF'
version = 1
[check.license]
check = "error"
license = "MIT"
copyright = "Test Org"
[check.license.patterns]
rust = ["**/*.rs"]
EOF

# Test detection
cargo run -- check --ci --license /tmp/license-test

# Test fix
cargo run -- check --ci --fix --license /tmp/license-test
cat /tmp/license-test/src/lib.rs  # Should have header
```

## Summary

| Phase | Description | Deliverable |
|-------|-------------|-------------|
| 1 | Comment syntax helper | `comment_prefix_for_extension()` |
| 2 | Header generation | `generate_header()` |
| 3 | Fix logic framework | Missing header fix |
| 4 | Year update | Outdated year fix |
| 5 | Shebang preservation | Shell script support |
| 6 | Integration | All specs pass, `make check` green |
