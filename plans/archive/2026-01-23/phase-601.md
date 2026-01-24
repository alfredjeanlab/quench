# Phase 601: Docs Check - Specs

**Plan:** `phase-601`
**Root Feature:** `quench-docs`
**Blocked By:** None

## Overview

Write behavioral specs for the `docs` check. These specs define the expected behavior of documentation validation before implementation, following the spec-first development approach.

The specs will cover:
- TOC validation (directory trees in markdown reference existing files)
- Link validation (markdown links point to existing files)
- Specs validation (index detection, required/forbidden sections)
- Commit checking (CI mode: feature commits require doc updates)
- Area mapping (scoped documentation requirements)
- Violation type enumeration

This phase also retires two checks from `scripts/bootstrap` that are now superseded by quench checks (`cloc` and `escapes`).

Reference: `docs/specs/checks/docs.md`

## Project Structure

Files to create/modify:

```
tests/
├── specs/
│   └── checks/
│       ├── mod.rs              # Add: pub mod docs;
│       └── docs/
│           ├── mod.rs          # NEW: Module root
│           ├── toc.rs          # NEW: TOC validation specs
│           ├── links.rs        # NEW: Link validation specs
│           ├── index.rs        # NEW: Index detection specs
│           ├── sections.rs     # NEW: Section validation specs
│           ├── commit.rs       # NEW: Commit checking specs (CI mode)
│           └── output.rs       # NEW: JSON output format specs
└── fixtures/
    └── docs/                   # NEW: Test fixtures
        ├── toc-ok/             # Valid TOC entries
        ├── toc-broken/         # Broken TOC path
        ├── link-ok/            # Valid markdown links
        ├── link-broken/        # Broken markdown link
        ├── link-external/      # External URLs (not validated)
        ├── index-auto/         # Auto-detect index file
        ├── index-toc/          # TOC mode index
        ├── index-linked/       # Linked mode index
        ├── index-exists/       # Exists mode index
        ├── unreachable-spec/   # Spec file not linked from index
        ├── section-required/   # Missing required section
        ├── section-forbidden/  # Contains forbidden section
        ├── commit-feature/     # Feature commit without docs (CI mode)
        ├── commit-ok/          # Feature commit with docs (CI mode)
        └── area-mapping/       # Area-specific doc requirements
scripts/
└── bootstrap                   # MODIFY: Remove retired checks
```

## Dependencies

- Existing spec infrastructure (`tests/specs/prelude.rs`)
- `escapes` and `agents` spec patterns as reference
- `docs/specs/checks/docs.md` as source of truth
- Existing fixtures: `docs-project/`, `violations/docs/`

## Implementation Phases

### Phase 1: Retire Bootstrap Checks

The `scripts/bootstrap` script contains two checks now superseded by quench:
- **Dead code allowances** - replaced by `escapes` check with `[rust.suppress]` config
- **File size checks** - replaced by `cloc` check with `max_lines` config

Remove these sections from `scripts/bootstrap`, keeping only:
- Test file convention check (`_tests.rs` pattern)
- Performance smoke test

**File to modify:** `scripts/bootstrap`

```bash
#!/usr/bin/env bash
# scripts/bootstrap - Quality checks not yet implemented in quench

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

failed=0

# --- Test file convention ---
echo "Checking test file convention..."

# Find inline #[cfg(test)] mod tests { patterns (should use _tests.rs instead)
INLINE_TESTS=$(rg -n '#\[cfg\(test\)\]' --type rust crates \
    --glob '!*_tests.rs' 2>/dev/null | \
    rg -v '#\[path = "' || true)

if [[ -n "$INLINE_TESTS" ]]; then
    while IFS=: read -r file line _; do
        context=$(sed -n "${line},$((line+1))p" "$file")
        if echo "$context" | rg -q 'mod tests'; then
            echo -e "${RED}$file:$line: inline #[cfg(test)] mod tests - use _tests.rs file instead${NC}"
            failed=1
        fi
    done <<< "$INLINE_TESTS"
fi

if [ $failed -eq 0 ]; then
    echo -e "${GREEN}Test conventions OK${NC}"
fi

# --- Performance smoke test ---
echo ""
echo "Running performance smoke test..."

if [ -d "tests/fixtures/bench-rust" ] && [ -f "target/release/quench" ]; then
    if timeout 5s ./target/release/quench check tests/fixtures/bench-rust > /dev/null 2>&1; then
        echo -e "${GREEN}Performance smoke test passed${NC}"
    else
        echo -e "${RED}ERROR: Performance smoke test failed (timeout or error)${NC}"
        failed=1
    fi
else
    echo "Skipping performance smoke test (requires release build and bench-rust fixture)"
fi

# --- Summary ---
echo ""
if [ $failed -eq 0 ]; then
    echo -e "${GREEN}All bootstrap checks passed${NC}"
    exit 0
else
    echo -e "${RED}Bootstrap checks failed${NC}"
    exit 1
fi
```

**Verification:**
- [ ] `./scripts/bootstrap` runs successfully
- [ ] Dead code check removed (now in `escapes` via `[rust.suppress]`)
- [ ] File size check removed (now in `cloc` via `max_lines`)

### Phase 2: Create Fixture Directory Structure

Create minimal fixture projects for docs check testing.

**Fixtures to create:**

#### `docs/toc-ok/`
```
docs/
├── CLAUDE.md     # Valid TOC with existing files
└── specs/
    ├── overview.md
    └── config.md
quench.toml
```

```markdown
<!-- docs/CLAUDE.md -->
# Docs

## File Structure

\`\`\`
docs/
├── CLAUDE.md
└── specs/
    ├── overview.md
    └── config.md
\`\`\`
```

#### `docs/toc-broken/`
```
docs/
├── CLAUDE.md     # TOC references non-existent file
└── specs/
    └── overview.md
quench.toml
```

```markdown
<!-- docs/CLAUDE.md -->
# Docs

## File Structure

\`\`\`
docs/specs/
├── overview.md
└── missing.md    # Does not exist
\`\`\`
```

#### `docs/link-ok/`
```
docs/
├── README.md     # Link to existing file
└── guide.md
quench.toml
```

```markdown
<!-- docs/README.md -->
# Docs

See [the guide](guide.md) for details.
```

#### `docs/link-broken/`
```
docs/
└── README.md     # Link to non-existent file
quench.toml
```

```markdown
<!-- docs/README.md -->
# Docs

See [the missing file](nonexistent.md) for details.
```

#### `docs/link-external/`
```
docs/
└── README.md     # External URL (not validated)
quench.toml
```

```markdown
<!-- docs/README.md -->
# Docs

See [GitHub](https://github.com) for details.
```

#### `docs/index-auto/`
```
docs/specs/
├── CLAUDE.md     # First priority index
├── overview.md
└── config.md
quench.toml       # index = "auto"
```

#### `docs/index-toc/`
```
docs/specs/
├── 00-overview.md  # Contains TOC
├── 01-api.md
└── 02-config.md
quench.toml         # index = "toc", index_file = "docs/specs/00-overview.md"
```

#### `docs/index-linked/`
```
docs/specs/
├── CLAUDE.md       # Links to spec files
├── overview.md
└── config.md
quench.toml         # index = "linked"
```

```markdown
<!-- docs/specs/CLAUDE.md -->
# Specs

- [Overview](overview.md)
- [Config](config.md)
```

#### `docs/unreachable-spec/`
```
docs/specs/
├── CLAUDE.md       # Only links to overview.md
├── overview.md
└── orphan.md       # Not linked from anywhere
quench.toml         # index = "linked"
```

#### `docs/section-required/`
```
docs/specs/
├── CLAUDE.md
└── incomplete.md   # Missing "Purpose" section
quench.toml         # sections.required = ["Purpose"]
```

#### `docs/section-forbidden/`
```
docs/specs/
├── CLAUDE.md
└── draft.md        # Contains "TODO" section
quench.toml         # sections.forbid = ["TODO"]
```

#### `docs/commit-feature/` and `docs/commit-ok/`

These fixtures require git repositories with feature commits. Create as temp directories in tests.

#### `docs/area-mapping/`
```
docs/
├── api/
│   └── endpoints.md
└── cli/
    └── commands.md
src/
├── api/
│   └── lib.rs
└── cli/
    └── main.rs
quench.toml        # area.api.docs = "docs/api/**", area.api.source = "src/api/**"
```

**Verification:**
- [ ] All fixtures have valid structure
- [ ] `quench.toml` files are valid TOML
- [ ] TOC entries demonstrate both passing and failing cases

### Phase 3: Write TOC Validation Specs

Create `tests/specs/checks/docs/toc.rs` with TOC validation specs.

```rust
//! Behavioral specs for TOC (directory tree) validation in the docs check.
//!
//! Reference: docs/specs/checks/docs.md#fast-mode-toc-validation

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// TOC TREE VALIDATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#what-gets-validated
///
/// > Each file in the tree is checked for existence.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn toc_tree_entries_validated_against_filesystem() {
    // Valid TOC with all files existing should pass
    check("docs").on("docs/toc-ok").passes();
}

/// Spec: docs/specs/checks/docs.md#output
///
/// > CLAUDE.md:72: toc path not found: checks/coverage.md
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn broken_toc_path_generates_violation() {
    check("docs")
        .on("docs/toc-broken")
        .fails()
        .stdout_has("docs: FAIL")
        .stdout_has("toc path not found");
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated
///
/// > Both box-drawing format and indentation format are supported.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn toc_box_drawing_format_supported() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("docs/specs")).unwrap();
    std::fs::write(
        dir.path().join("docs/specs/overview.md"),
        "# Overview\n\n## Purpose\n\nTest.\n",
    ).unwrap();
    std::fs::write(
        dir.path().join("docs/CLAUDE.md"),
        r#"# Docs

## File Structure

```
docs/specs/
├── overview.md
└── config.md
```
"#,
    ).unwrap();
    // config.md doesn't exist - should fail
    check("docs").pwd(dir.path()).fails().stdout_has("config.md");
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated
///
/// > Indentation format (spaces or tabs) is supported.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn toc_indentation_format_supported() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("docs/specs")).unwrap();
    std::fs::write(
        dir.path().join("docs/specs/overview.md"),
        "# Overview\n\n## Purpose\n\nTest.\n",
    ).unwrap();
    std::fs::write(
        dir.path().join("docs/CLAUDE.md"),
        r#"# Docs

## File Structure

```
docs/specs/
  overview.md
  missing.md
```
"#,
    ).unwrap();
    // missing.md doesn't exist - should fail
    check("docs").pwd(dir.path()).fails().stdout_has("missing.md");
}

/// Spec: docs/specs/checks/docs.md#resolution
///
/// > Paths resolved in order: 1. Relative to markdown file's directory
/// > 2. Relative to docs/ directory 3. Relative to project root
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn toc_path_resolution_order() {
    let dir = temp_project();
    // Create file at project root
    std::fs::write(dir.path().join("README.md"), "# README\n").unwrap();
    std::fs::create_dir_all(dir.path().join("docs")).unwrap();
    std::fs::write(
        dir.path().join("docs/CLAUDE.md"),
        r#"# Docs

## File Structure

```
README.md
```
"#,
    ).unwrap();
    // Should resolve README.md from project root
    check("docs").pwd(dir.path()).passes();
}
```

**Verification:**
- [ ] Specs compile with `cargo test --test specs -- docs --ignored`
- [ ] All TOC specs use `#[ignore = "TODO: Phase 602"]`

### Phase 4: Write Link Validation Specs

Create `tests/specs/checks/docs/links.rs` with link validation specs.

```rust
//! Behavioral specs for markdown link validation in the docs check.
//!
//! Reference: docs/specs/checks/docs.md#fast-mode-link-validation

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// LINK VALIDATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#what-gets-validated-1
///
/// > Markdown links to local files are validated.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn markdown_link_to_missing_file_generates_violation() {
    check("docs")
        .on("docs/link-broken")
        .fails()
        .stdout_has("docs: FAIL")
        .stdout_has("broken link");
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated-1
///
/// > External URLs (http/https) are not validated.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn external_urls_not_validated() {
    check("docs").on("docs/link-external").passes();
}

/// Spec: docs/specs/checks/docs.md#output-1
///
/// > README.md:45: broken link: docs/old-guide.md
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn broken_link_includes_file_and_line() {
    let docs = check("docs").on("docs/link-broken").json().fails();
    let violations = docs.require("violations").as_array().unwrap();

    let link_violation = violations.iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("broken_link"))
        .expect("should have broken_link violation");

    assert!(link_violation.get("file").is_some(), "should have file");
    assert!(link_violation.get("line").is_some(), "should have line");
    assert!(link_violation.get("target").is_some(), "should have target");
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated-1
///
/// > Check [configuration](../02-config.md) for options.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn relative_path_links_validated() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("docs/specs")).unwrap();
    std::fs::write(
        dir.path().join("docs/specs/overview.md"),
        "See [config](../config.md) for details.\n",
    ).unwrap();
    // ../config.md doesn't exist relative to docs/specs/overview.md
    check("docs").pwd(dir.path()).fails().stdout_has("config.md");
}
```

**Verification:**
- [ ] Link specs cover local file links
- [ ] External URL exemption is verified

### Phase 5: Write Index Detection and Specs Validation Specs

Create `tests/specs/checks/docs/index.rs` and `tests/specs/checks/docs/sections.rs`.

```rust
// tests/specs/checks/docs/index.rs
//! Behavioral specs for specs index file detection.
//!
//! Reference: docs/specs/checks/docs.md#index-file

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// INDEX FILE DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#index-file
///
/// > Detection order: 1. {path}/CLAUDE.md 2. docs/CLAUDE.md
/// > 3. {path}/[00-]{overview,summary,index}.md 4. docs/SPECIFICATIONS.md
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn specs_directory_index_file_detected() {
    let docs = check("docs").on("docs/index-auto").json().passes();
    let metrics = docs.require("metrics");

    assert!(
        metrics.get("index_file").is_some(),
        "should have index_file in metrics"
    );
}

/// Spec: docs/specs/checks/docs.md#toc-format
///
/// > `linked` mode: All spec files must be reachable via markdown links.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn unreachable_spec_file_generates_violation_linked_mode() {
    check("docs")
        .on("docs/unreachable-spec")
        .fails()
        .stdout_has("unreachable from index");
}

/// Spec: docs/specs/checks/docs.md#index-file
///
/// > `exists` mode: Index file must exist, no reachability check.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn exists_mode_only_checks_index_exists() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[check.docs]
path = "docs/specs"
index = "exists"
"#,
    ).unwrap();
    std::fs::create_dir_all(dir.path().join("docs/specs")).unwrap();
    std::fs::write(
        dir.path().join("docs/specs/CLAUDE.md"),
        "# Specs Index\n",
    ).unwrap();
    std::fs::write(
        dir.path().join("docs/specs/orphan.md"),
        "# Orphan (not linked)\n",
    ).unwrap();

    // In exists mode, orphan.md is not flagged as unreachable
    check("docs").pwd(dir.path()).passes();
}
```

```rust
// tests/specs/checks/docs/sections.rs
//! Behavioral specs for section validation in spec files.
//!
//! Reference: docs/specs/checks/docs.md#section-validation

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// SECTION VALIDATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#section-validation
///
/// > sections.required = ["Purpose", "Configuration"]
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn missing_required_section_in_spec_generates_violation() {
    check("docs")
        .on("docs/section-required")
        .fails()
        .stdout_has("missing required section")
        .stdout_has("Purpose");
}

/// Spec: docs/specs/checks/docs.md#section-validation
///
/// > sections.forbid = ["TODO", "Draft*"]
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn forbidden_section_in_spec_generates_violation() {
    check("docs")
        .on("docs/section-forbidden")
        .fails()
        .stdout_has("forbidden section")
        .stdout_has("TODO");
}

/// Spec: docs/specs/checks/docs.md#section-validation
///
/// > Case-insensitive matching for section names.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn section_matching_is_case_insensitive() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[check.docs]
path = "docs/specs"
sections.required = ["purpose"]
"#,
    ).unwrap();
    std::fs::create_dir_all(dir.path().join("docs/specs")).unwrap();
    std::fs::write(
        dir.path().join("docs/specs/CLAUDE.md"),
        "# Specs\n",
    ).unwrap();
    std::fs::write(
        dir.path().join("docs/specs/feature.md"),
        "# Feature\n\n## PURPOSE\n\nThis is the purpose.\n",
    ).unwrap();

    // "PURPOSE" should match required "purpose"
    check("docs").pwd(dir.path()).passes();
}
```

**Verification:**
- [ ] Index detection specs cover all detection modes
- [ ] Section validation specs cover required and forbidden sections

### Phase 6: Write CI Mode Commit Checking Specs

Create `tests/specs/checks/docs/commit.rs` with CI mode specs.

```rust
//! Behavioral specs for commit checking in CI mode.
//!
//! Reference: docs/specs/checks/docs.md#ci-mode-commit-checking

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;
use std::process::Command;

// =============================================================================
// CI MODE COMMIT CHECKING SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#how-it-works
///
/// > Identify commits with `feat:` or `feat(area):` prefixes.
/// > Report when feature commits lack corresponding doc changes.
#[test]
#[ignore = "TODO: Phase 603 - Docs Check CI Mode"]
fn feature_commit_without_doc_change_generates_violation_ci_mode() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[check.docs.commit]
check = "error"
"#,
    ).unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create initial commit on main
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial commit"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create feature branch
    Command::new("git")
        .args(["checkout", "-b", "feature/new-thing"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Add feature commit without docs
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/feature.rs"), "pub fn new_feature() {}").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feat: add new feature"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    check("docs")
        .pwd(dir.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("feature commits without documentation");
}

/// Spec: docs/specs/checks/docs.md#area-mapping
///
/// > Use area mappings to require specific documentation for scoped commits.
#[test]
#[ignore = "TODO: Phase 603 - Docs Check CI Mode"]
fn area_mapping_restricts_doc_requirement_to_specific_paths() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[check.docs.commit]
check = "error"

[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"
"#,
    ).unwrap();

    // Initialize git repo with main branch
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Initial commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Feature branch with api scope
    Command::new("git")
        .args(["checkout", "-b", "feature/api-endpoint"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::fs::create_dir_all(dir.path().join("src/api")).unwrap();
    std::fs::write(dir.path().join("src/api/endpoint.rs"), "pub fn endpoint() {}").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feat(api): add endpoint"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    check("docs")
        .pwd(dir.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("feat(api)")
        .stdout_has("docs/api/**");
}

/// Spec: docs/specs/checks/docs.md#check-levels
///
/// > `off` - Disable commit checking (default).
#[test]
#[ignore = "TODO: Phase 603 - Docs Check CI Mode"]
fn commit_checking_disabled_by_default() {
    let dir = temp_project();
    // No [check.docs.commit] section - should be disabled

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["checkout", "-b", "feature/thing"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    std::fs::write(dir.path().join("new.rs"), "fn new() {}").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feat: new thing"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // With commit checking disabled, should pass even without docs
    check("docs").pwd(dir.path()).args(&["--ci"]).passes();
}
```

**Verification:**
- [ ] CI mode specs cover feature commit detection
- [ ] Area mapping specs demonstrate scoped requirements
- [ ] Default disabled behavior is verified

### Phase 7: Write JSON Output Format Specs

Create `tests/specs/checks/docs/output.rs` with JSON output specs.

```rust
//! Behavioral specs for docs check JSON output format.
//!
//! Reference: docs/specs/checks/docs.md#json-output

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// JSON OUTPUT FORMAT SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#json-output
///
/// > Violation types: missing_section, forbidden_section, broken_toc, broken_link, missing_docs
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn docs_violation_type_is_one_of_expected_values() {
    let docs = check("docs").on("violations").json().fails();
    let violations = docs.require("violations").as_array().unwrap();

    let valid_types = [
        "missing_section",
        "forbidden_section",
        "broken_toc",
        "broken_link",
        "missing_docs",
    ];

    for violation in violations {
        let vtype = violation.get("type").and_then(|t| t.as_str()).unwrap();
        assert!(
            valid_types.contains(&vtype),
            "unexpected violation type: {}",
            vtype
        );
    }
}

/// Spec: docs/specs/checks/docs.md#json-output
///
/// > broken_toc violation includes file, line, path, advice
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn broken_toc_violation_structure() {
    let docs = check("docs").on("docs/toc-broken").json().fails();
    let violations = docs.require("violations").as_array().unwrap();

    let toc_violation = violations.iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("broken_toc"))
        .expect("should have broken_toc violation");

    assert!(toc_violation.get("file").is_some(), "missing file");
    assert!(toc_violation.get("line").is_some(), "missing line");
    assert!(toc_violation.get("path").is_some(), "missing path");
    assert!(toc_violation.get("advice").is_some(), "missing advice");
}

/// Spec: docs/specs/checks/docs.md#json-output
///
/// > broken_link violation includes file, line, target, advice
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn broken_link_violation_structure() {
    let docs = check("docs").on("docs/link-broken").json().fails();
    let violations = docs.require("violations").as_array().unwrap();

    let link_violation = violations.iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("broken_link"))
        .expect("should have broken_link violation");

    assert!(link_violation.get("file").is_some(), "missing file");
    assert!(link_violation.get("line").is_some(), "missing line");
    assert!(link_violation.get("target").is_some(), "missing target");
    assert!(link_violation.get("advice").is_some(), "missing advice");
}

/// Spec: docs/specs/checks/docs.md#json-output
///
/// > missing_section violation includes file, section, advice
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn missing_section_violation_structure() {
    let docs = check("docs").on("docs/section-required").json().fails();
    let violations = docs.require("violations").as_array().unwrap();

    let section_violation = violations.iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("missing_section"))
        .expect("should have missing_section violation");

    assert!(section_violation.get("file").is_some(), "missing file");
    assert!(section_violation.get("section").is_some(), "missing section");
    assert!(section_violation.get("advice").is_some(), "missing advice");
}

/// Spec: docs/specs/checks/docs.md#json-output
///
/// > missing_docs violation (CI mode) has file: null with commit field
#[test]
#[ignore = "TODO: Phase 603 - Docs Check CI Mode"]
fn missing_docs_violation_structure() {
    // This test requires git setup - use temp dir
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[check.docs.commit]
check = "error"
"#,
    ).unwrap();

    // Setup git repo with feature commit (abbreviated - full setup in commit.rs)
    // ...

    let docs = check("docs")
        .pwd(dir.path())
        .args(&["--ci"])
        .json()
        .fails();
    let violations = docs.require("violations").as_array().unwrap();

    let docs_violation = violations.iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("missing_docs"))
        .expect("should have missing_docs violation");

    assert!(docs_violation.get("commit").is_some(), "missing commit");
    assert!(docs_violation.get("message").is_some(), "missing message");
}

/// Spec: docs/specs/checks/docs.md#json-output
///
/// > metrics: { index_file, spec_files, feature_commits, with_docs }
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn docs_json_metrics_structure() {
    let docs = check("docs").on("docs-project").json().passes();
    let metrics = docs.require("metrics");

    assert!(metrics.get("index_file").is_some(), "missing index_file");
    assert!(metrics.get("spec_files").is_some(), "missing spec_files");
}
```

**Verification:**
- [ ] Violation type enumeration is complete
- [ ] Each violation type has structure test
- [ ] Metrics structure is verified

### Phase 8: Module Registration and Final Touches

Create the module structure and register specs.

**`tests/specs/checks/docs/mod.rs`:**
```rust
//! Behavioral specs for the docs check.
//!
//! Tests that quench correctly:
//! - Validates TOC directory trees in markdown files
//! - Validates markdown links to local files
//! - Detects and validates specs index files
//! - Checks required/forbidden sections in spec files
//! - Checks feature commits have doc updates (CI mode)
//! - Generates correct violation types
//!
//! Reference: docs/specs/checks/docs.md

mod commit;
mod index;
mod links;
mod output;
mod sections;
mod toc;
```

**Update `tests/specs/checks/mod.rs`** (if exists) or create:
```rust
//! Behavioral specs for quench checks.

pub mod agents;
pub mod cloc;
pub mod docs;    // NEW
pub mod escapes;
```

**Verification:**
- [ ] Module registration compiles
- [ ] All specs are discoverable via `cargo test --test specs -- docs`

## Key Implementation Details

### Fixture Configuration Files

Each fixture needs a `quench.toml` to configure the expected behavior:

```toml
# docs/toc-broken/quench.toml
version = 1

[check.docs]
check = "error"

[check.docs.toc]
check = "error"
include = ["**/*.md"]
```

```toml
# docs/unreachable-spec/quench.toml
version = 1

[check.docs]
path = "docs/specs"
index = "linked"
```

```toml
# docs/section-required/quench.toml
version = 1

[check.docs]
path = "docs/specs"
sections.required = ["Purpose"]
```

### Spec Organization

The spec files are organized by feature area:

| File | Purpose |
|------|---------|
| `toc.rs` | TOC directory tree validation |
| `links.rs` | Markdown link validation |
| `index.rs` | Specs index file detection |
| `sections.rs` | Required/forbidden section checking |
| `commit.rs` | CI mode commit checking |
| `output.rs` | JSON output format validation |

### Ignore Annotations

All specs use phase-specific ignore annotations:

- `#[ignore = "TODO: Phase 602 - Docs Check Implementation"]` - Fast mode specs
- `#[ignore = "TODO: Phase 603 - Docs Check CI Mode"]` - CI mode specs

## Verification Plan

### Phase Completion Checklist

1. **Bootstrap Cleanup** (Phase 1)
   - [ ] `./scripts/bootstrap` runs without dead code check
   - [ ] `./scripts/bootstrap` runs without file size check
   - [ ] `make check` still passes

2. **Fixture Creation** (Phase 2)
   - [ ] Run `ls tests/fixtures/docs/` - all directories exist
   - [ ] Each fixture has valid `quench.toml`
   - [ ] TOC fixtures demonstrate pass/fail cases

3. **Spec Compilation** (Phases 3-7)
   - [ ] Run `cargo test --test specs -- docs --ignored` - compiles without errors
   - [ ] All specs have `#[ignore = "TODO: Phase NNN"]` annotations
   - [ ] Doc comments reference `docs/specs/checks/docs.md`

4. **Spec Coverage** (From outline)
   - [ ] TOC tree entries validated against filesystem
   - [ ] Broken TOC path generates violation
   - [ ] Markdown link to missing file generates violation
   - [ ] External URLs not validated
   - [ ] Specs directory index file detected
   - [ ] Unreachable spec file generates violation (linked mode)
   - [ ] Missing required section in spec generates violation
   - [ ] Feature commit without doc change generates violation (CI mode)
   - [ ] Area mapping restricts doc requirement to specific paths
   - [ ] Docs violation.type is one of expected values

5. **Code Quality**
   - [ ] Run `cargo fmt --all`
   - [ ] Run `cargo clippy --all-targets`
   - [ ] No warnings in spec code

### Final Verification Commands

```bash
# Verify bootstrap works
./scripts/bootstrap

# Verify fixtures exist
ls -la tests/fixtures/docs/

# Verify specs compile
cargo test --test specs -- docs --ignored 2>&1 | head -20

# Count specs
grep -r '#\[test\]' tests/specs/checks/docs/ | wc -l

# Verify module registration
grep 'pub mod docs' tests/specs/checks/mod.rs

# Full check
make check
```

### Spec Count Target

| Category | Specs |
|----------|-------|
| TOC validation | 5 |
| Link validation | 4 |
| Index detection | 3 |
| Section validation | 3 |
| Commit checking (CI) | 3 |
| JSON output | 6 |
| **Total** | **~24 behavioral specs** |
