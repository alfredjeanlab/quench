# Checkpoint 9A: Pre-Checkpoint Fix - Git Check Complete

**Plan:** `checkpoint-9a-precheck`
**Root Feature:** `quench-git`
**Depends On:** Phase 820 (Git Check - Template)

## Overview

Verify the git check feature is complete and all behavioral specs pass. Phases 810 and 820 implemented commit message validation, type/scope enforcement, agent documentation checking, and `.gitmessage` template generation. This checkpoint validates the implementation against all specs before proceeding to subsequent features.

**Current State:**
- Core implementation: `crates/cli/src/checks/git/`
  - `mod.rs` - Main check orchestration
  - `parse.rs` - Conventional commit parsing
  - `docs.rs` - Agent documentation checking
  - `template.rs` - `.gitmessage` generation
- Unit tests: All passing (`*_tests.rs` siblings)
- Behavioral specs: 21 specs in `tests/specs/checks/git.rs`
- Fixtures: `tests/fixtures/git/`

**Goal:** Confirm all git check deliverables are complete and passing.

## Project Structure

```
crates/cli/src/checks/git/
├── mod.rs              # GitCheck implementation, fix logic
├── mod_tests.rs        # Unit tests for main module
├── parse.rs            # Conventional commit parsing
├── parse_tests.rs      # Parser unit tests
├── docs.rs             # Agent documentation checking (CLAUDE.md/.cursorrules)
├── docs_tests.rs       # Documentation check unit tests
├── template.rs         # .gitmessage template generation
└── template_tests.rs   # Template generation unit tests

tests/specs/checks/git.rs    # 21 behavioral specs (all should pass)

tests/fixtures/git/
├── missing-docs/        # Fixture for missing documentation check
├── invalid-type/        # Fixture for invalid type check (if exists)
└── invalid-scope/       # Fixture for invalid scope check (if exists)
```

## Dependencies

No external dependencies beyond existing crate dependencies:
- `serde_json` for JSON output
- `std::process::Command` for git operations
- `std::fs` for file operations

## Implementation Phases

### Phase 1: Verify Unit Tests

**Goal:** Confirm all git check unit tests pass.

**Test files:**
- `mod_tests.rs` - Main check logic tests
- `parse_tests.rs` - Conventional commit parsing tests
- `docs_tests.rs` - Agent documentation detection tests
- `template_tests.rs` - Template generation tests

**Verification:**
```bash
cargo test -p quench checks::git
```

**Expected:** All unit tests pass.

---

### Phase 2: Verify Behavioral Specs - Format Validation

**Goal:** Confirm commit format validation specs pass.

**Specs (4 total):**
1. `git_validates_conventional_commit_format` - Valid format passes
2. `git_invalid_format_generates_violation` - Invalid format fails with `invalid_format`
3. `git_invalid_type_generates_violation` - Disallowed type fails with `invalid_type`
4. `git_invalid_scope_generates_violation_when_scopes_configured` - Disallowed scope fails

**Key behavior:**
- Format: `<type>(<scope>): <description>`
- Types: configurable via `types = [...]`
- Scopes: configurable via `scopes = [...]` (any scope allowed if not configured)

**Verification:**
```bash
cargo test --test specs git_validates
cargo test --test specs git_invalid
```

---

### Phase 3: Verify Behavioral Specs - Agent Documentation

**Goal:** Confirm agent documentation checking specs pass.

**Specs (4 total):**
1. `git_missing_format_documentation_generates_violation` - Missing docs fails
2. `git_detects_commit_format_via_type_prefixes` - Detects `feat:`, `fix(` patterns
3. `git_detects_commit_format_via_conventional_commits_phrase` - Detects phrase
4. `git_skips_docs_check_when_agents_disabled` - `agents = false` skips check

**Key behavior:**
- Searches CLAUDE.md, .cursorrules for commit format documentation
- Detection: type prefixes (`feat:`, `fix(`) or "conventional commits" phrase
- Disabled with `agents = false`

**Verification:**
```bash
cargo test --test specs git_missing_format
cargo test --test specs git_detects
cargo test --test specs git_skips
```

---

### Phase 4: Verify Behavioral Specs - Template Generation

**Goal:** Confirm `.gitmessage` template generation specs pass.

**Specs (3 total):**
1. `git_fix_creates_gitmessage_template` - `--fix` creates `.gitmessage`
2. `git_fix_configures_commit_template` - `--fix` runs `git config commit.template`
3. `git_fix_does_not_overwrite_existing_gitmessage` - Existing file preserved

**Key behavior:**
- Template contains types, optional scopes, examples
- Git config set to use `.gitmessage`
- Idempotent: never overwrites existing files

**Verification:**
```bash
cargo test --test specs git_fix
```

---

### Phase 5: Verify Behavioral Specs - JSON Output

**Goal:** Confirm JSON output format specs pass.

**Specs (5 total):**
1. `git_violation_type_is_one_of_expected_values` - Types: `invalid_format`, `invalid_type`, `invalid_scope`, `missing_docs`
2. `git_commit_violations_have_commit_field` - Commit violations have `commit` field, null `file`
3. `git_missing_docs_violation_references_file` - Doc violations reference agent file
4. `git_any_scope_allowed_when_not_configured` - No scope restriction when unconfigured

**Key JSON structure:**
```json
{
  "violations": [
    {
      "type": "invalid_format",
      "commit": "abc123",
      "file": null,
      "advice": "..."
    }
  ]
}
```

**Verification:**
```bash
cargo test --test specs git_violation
cargo test --test specs git_commit
cargo test --test specs git_any
```

---

### Phase 6: Full Integration Testing

**Goal:** Run complete test suite and verify no regressions.

**Actions:**
1. Run all git specs:
   ```bash
   cargo test --test specs git
   ```
2. Run full make check:
   ```bash
   make check
   ```
3. Verify no ignored specs remain:
   ```bash
   grep -r "#\[ignore" tests/specs/checks/git.rs
   # Should return empty
   ```

**Verification checklist:**
- [ ] All 21 git specs pass
- [ ] No regressions in other checks
- [ ] `make check` completes successfully (fmt, clippy, test, build, audit, deny)

---

### Phase 7: Final Verification and Documentation

**Goal:** Confirm completion and prepare for commit.

**Actions:**
1. Verify all checklist items are complete
2. Run final `make check`
3. Archive implementation plan if needed

**Commit message template:**
```
feat(git): complete git check implementation

Verify all git check deliverables are complete and passing:
- Commit format validation (conventional commits)
- Type and scope enforcement
- Agent documentation checking
- .gitmessage template generation with --fix

Passing specs (21 total):
- git_validates_conventional_commit_format
- git_invalid_format_generates_violation
- git_invalid_type_generates_violation
- git_invalid_scope_generates_violation_when_scopes_configured
- git_any_scope_allowed_when_not_configured
- git_missing_format_documentation_generates_violation
- git_detects_commit_format_via_type_prefixes
- git_detects_commit_format_via_conventional_commits_phrase
- git_skips_docs_check_when_agents_disabled
- git_fix_creates_gitmessage_template
- git_fix_configures_commit_template
- git_fix_does_not_overwrite_existing_gitmessage
- git_violation_type_is_one_of_expected_values
- git_commit_violations_have_commit_field
- git_missing_docs_violation_references_file
```

## Key Implementation Details

### Git Check Configuration

The git check is configured via `[git.commit]` in `quench.toml`:

```toml
[git.commit]
check = "error"           # "error" | "warn" | "off"
format = "conventional"   # "conventional" | "none"
types = ["feat", "fix", "chore", "docs", "test", "refactor"]
scopes = ["api", "cli"]   # Optional - any scope allowed if omitted
agents = true             # Check CLAUDE.md for format documentation
template = true           # Generate .gitmessage with --fix
```

### Violation Types

| Type | Description | Fields |
|------|-------------|--------|
| `invalid_format` | Commit message doesn't match format | `commit`, `advice` |
| `invalid_type` | Type not in allowed list | `commit`, `type`, `advice` |
| `invalid_scope` | Scope not in allowed list | `commit`, `scope`, `advice` |
| `missing_docs` | No commit format in agent files | `file`, `advice` |

### Template Generation

Generated `.gitmessage` follows this structure:

```
# <type>(<scope>): <description>
#
# Types: feat, fix, chore, docs, test, refactor
# Scope: optional (api, cli)
#
# Examples:
#   feat(api): add export endpoint
#   fix: handle edge case
```

### Idempotent Fix Behavior

| Scenario | Action |
|----------|--------|
| `.gitmessage` missing | Create it |
| `.gitmessage` exists | Leave it alone |
| `commit.template` not set | Set to `.gitmessage` |
| `commit.template` already set | Leave it alone |

## Verification Plan

### Unit Tests
```bash
# All git check unit tests
cargo test -p quench checks::git

# Specific modules
cargo test -p quench checks::git::parse
cargo test -p quench checks::git::docs
cargo test -p quench checks::git::template
```

### Behavioral Specs
```bash
# All git specs
cargo test --test specs git
# Expected: 21 specs pass, 0 ignored

# Show any ignored specs (should be none)
cargo test --test specs git -- --ignored
```

### Manual Testing
```bash
# Create test project
cd /tmp && mkdir git-test && cd git-test
git init
echo 'version = 1
[git.commit]
check = "error"
types = ["feat", "fix"]' > quench.toml
echo '# Test

## Commits

Use feat: format.

## Directory Structure

Minimal.

## Landing the Plane

- Done' > CLAUDE.md

# Valid commit should pass
git add . && git commit -m "feat: initial"
git checkout -b feature
quench check --git --ci
# Expected: PASS

# Invalid commit should fail
echo "test" > dummy.txt && git add . && git commit -m "update stuff"
quench check --git --ci
# Expected: FAIL with invalid_format

# Fix should create template
quench check --git --fix
cat .gitmessage
git config commit.template
```

### Full Suite
```bash
make check
# Expected: All checks pass
```

## Checklist

- [ ] Unit tests pass (`cargo test -p quench checks::git`)
- [ ] No `#[ignore]` in `tests/specs/checks/git.rs`
- [ ] Format validation specs pass (4)
- [ ] Agent documentation specs pass (4)
- [ ] Template generation specs pass (3)
- [ ] JSON output specs pass (5)
- [ ] Scope allowance spec passes (1)
- [ ] All 21 git specs pass
- [ ] `make check` passes
- [ ] No regressions in other checks
- [ ] Cache version appropriate (no logic changes needed)
