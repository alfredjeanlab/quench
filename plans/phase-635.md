# Phase 635: Docs Check - Area Mapping

**Plan:** `phase-635`
**Root Feature:** `quench-docs`
**Depends On:** Phase 630 (Docs Check - Commit Checking)

## Overview

Extend the docs check area mapping to support **source-based detection**. Currently, area mapping only works when commits have explicit scopes (e.g., `feat(api):`). This phase adds the ability to detect areas based on which source files were changed, even when commits don't have scopes.

**Key Behaviors:**
- Source changes trigger area requirements: `src/api/**` changes → require `docs/api/**`
- Scope-based matching takes priority over source-based matching
- Multiple area matches require docs for all matched areas
- Violations indicate why each area was triggered (scope vs source)

**Example Configuration:**
```toml
[check.docs.commit]
check = "error"

[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"

[check.docs.area.cli]
docs = "docs/cli/**"
source = "src/cli/**"
```

**Example Behavior:**
| Commit | Changed Files | Required Docs |
|--------|---------------|---------------|
| `feat(api): add endpoint` | `src/api/handler.rs` | `docs/api/**` (scope match) |
| `feat: add handler` | `src/api/handler.rs` | `docs/api/**` (source match) |
| `feat: refactor` | `src/api/x.rs`, `src/cli/y.rs` | `docs/api/**` AND `docs/cli/**` |

## Project Structure

```
crates/cli/src/
├── checks/docs/
│   ├── commit.rs         # MODIFY: Add source-based area detection
│   └── commit_tests.rs   # MODIFY: Add unit tests for source matching
└── check.rs              # MODIFY: Add area field to Violation
tests/
├── specs/checks/docs/
│   └── commit.rs         # MODIFY: Add behavioral specs for source matching
└── fixtures/docs/
    └── source-mapping/   # NEW: Fixture for source-based tests
```

## Dependencies

No new external crates. Uses existing:
- `globset` for pattern matching (already in deps)
- `std::process::Command` for git operations

## Implementation Phases

### Phase 1: Detect Areas from Source Changes

Add a function to find all areas that match changed source files.

**crates/cli/src/checks/docs/commit.rs:**

```rust
/// Find all areas that match the changed files based on source patterns.
fn find_areas_from_source(
    changed_files: &[String],
    areas: &HashMap<String, DocsAreaConfig>,
) -> Vec<(&str, &DocsAreaConfig)> {
    areas
        .iter()
        .filter_map(|(name, area)| {
            // Only consider areas with source patterns
            let source = area.source.as_ref()?;

            // Check if any changed file matches this area's source pattern
            if has_changes_matching(changed_files, source) {
                Some((name.as_str(), area))
            } else {
                None
            }
        })
        .collect()
}
```

**Unit Tests** (`commit_tests.rs`):

```rust
#[test]
fn finds_areas_from_source_changes() {
    let mut areas = HashMap::new();
    areas.insert(
        "api".to_string(),
        DocsAreaConfig {
            docs: "docs/api/**".to_string(),
            source: Some("src/api/**".to_string()),
        },
    );
    areas.insert(
        "cli".to_string(),
        DocsAreaConfig {
            docs: "docs/cli/**".to_string(),
            source: Some("src/cli/**".to_string()),
        },
    );

    let files = vec!["src/api/handler.rs".to_string()];
    let matched = find_areas_from_source(&files, &areas);

    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].0, "api");
}

#[test]
fn finds_multiple_areas_from_source_changes() {
    // Same setup...
    let files = vec![
        "src/api/handler.rs".to_string(),
        "src/cli/main.rs".to_string(),
    ];
    let matched = find_areas_from_source(&files, &areas);

    assert_eq!(matched.len(), 2);
}

#[test]
fn ignores_areas_without_source_pattern() {
    let mut areas = HashMap::new();
    areas.insert(
        "api".to_string(),
        DocsAreaConfig {
            docs: "docs/api/**".to_string(),
            source: None, // No source pattern
        },
    );

    let files = vec!["src/api/handler.rs".to_string()];
    let matched = find_areas_from_source(&files, &areas);

    assert!(matched.is_empty());
}
```

**Verification:**
- `cargo test -p quench commit` passes new unit tests
- `find_areas_from_source` correctly matches areas by source pattern

---

### Phase 2: Update Commit Doc Checking Logic

Modify `check_commit_has_docs` to use source-based matching when scope-based matching doesn't apply.

**crates/cli/src/checks/docs/commit.rs:**

```rust
/// Result of checking if a commit has required docs.
pub struct DocCheckResult {
    /// Whether the commit has all required documentation.
    pub has_docs: bool,
    /// Areas that matched (by scope or source).
    pub matched_areas: Vec<MatchedArea>,
}

/// An area that was matched for a commit.
#[derive(Debug, Clone)]
pub struct MatchedArea {
    /// Area name (e.g., "api").
    pub name: String,
    /// Required docs pattern (e.g., "docs/api/**").
    pub docs_pattern: String,
    /// How this area was matched.
    pub match_type: AreaMatchType,
    /// Whether docs were found for this area.
    pub has_docs: bool,
}

/// How an area was matched to a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaMatchType {
    /// Matched by commit scope (e.g., `feat(api):` → "api" area).
    Scope,
    /// Matched by source file changes (e.g., `src/api/**` changed).
    Source,
}

fn check_commit_has_docs(
    commit: &ConventionalCommit,
    changed_files: &[String],
    areas: &HashMap<String, DocsAreaConfig>,
) -> DocCheckResult {
    let mut matched_areas = Vec::new();

    // Priority 1: Check scope-based matching
    if let Some(scope) = &commit.scope {
        if let Some(area) = areas.get(scope) {
            let has_docs = has_changes_matching(changed_files, &area.docs);
            matched_areas.push(MatchedArea {
                name: scope.clone(),
                docs_pattern: area.docs.clone(),
                match_type: AreaMatchType::Scope,
                has_docs,
            });

            // Scope match takes priority - don't add source matches for same area
            return DocCheckResult {
                has_docs,
                matched_areas,
            };
        }
    }

    // Priority 2: Check source-based matching
    let source_matches = find_areas_from_source(changed_files, areas);
    if !source_matches.is_empty() {
        for (name, area) in source_matches {
            let has_docs = has_changes_matching(changed_files, &area.docs);
            matched_areas.push(MatchedArea {
                name: name.to_string(),
                docs_pattern: area.docs.clone(),
                match_type: AreaMatchType::Source,
                has_docs,
            });
        }

        let all_have_docs = matched_areas.iter().all(|a| a.has_docs);
        return DocCheckResult {
            has_docs: all_have_docs,
            matched_areas,
        };
    }

    // Fallback: No area matched, require generic docs/
    let has_docs = has_changes_matching(changed_files, "docs/**");
    DocCheckResult {
        has_docs,
        matched_areas, // Empty - no specific area
    }
}
```

**Verification:**
- Unit tests for scope priority over source
- Unit tests for multiple source areas requiring all docs
- Unit tests for fallback to generic docs/

---

### Phase 3: Add Area Field to Violations

Extend the `Violation` struct to include area information for better error messages.

**crates/cli/src/check.rs:**

```rust
#[derive(Debug, Clone, Serialize)]
pub struct Violation {
    // ... existing fields ...

    /// Area name that was matched (for area-specific violations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,

    /// How the area was matched ("scope" or "source").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area_match: Option<String>,
}

impl Violation {
    /// Add area information to the violation.
    pub fn with_area(mut self, name: &str, match_type: &str) -> Self {
        self.area = Some(name.to_string());
        self.area_match = Some(match_type.to_string());
        self
    }
}
```

**Verification:**
- JSON output includes `area` and `area_match` fields when present
- Fields are omitted when not applicable

---

### Phase 4: Update Violation Creation

Update the violation creation to include area information and more descriptive advice.

**crates/cli/src/checks/docs/commit.rs:**

```rust
fn create_area_violation(
    commit: &ConventionalCommit,
    area: &MatchedArea,
) -> Violation {
    let match_desc = match area.match_type {
        AreaMatchType::Scope => format!("feat({}):", area.name),
        AreaMatchType::Source => format!("changes in {} area", area.name),
    };

    let advice = format!(
        "Commit {} requires documentation update.\n\
         Update {} with the new functionality.",
        match_desc,
        area.docs_pattern
    );

    Violation::commit_violation(&commit.hash, &commit.message, "missing_docs", advice)
        .with_expected_docs(&area.docs_pattern)
        .with_area(&area.name, match area.match_type {
            AreaMatchType::Scope => "scope",
            AreaMatchType::Source => "source",
        })
}

fn create_violations_for_commit(
    commit: &ConventionalCommit,
    result: &DocCheckResult,
) -> Vec<Violation> {
    if result.has_docs {
        return Vec::new();
    }

    if result.matched_areas.is_empty() {
        // No specific area, generic docs/ violation
        return vec![create_violation(commit, None)];
    }

    // Create violation for each area missing docs
    result
        .matched_areas
        .iter()
        .filter(|a| !a.has_docs)
        .map(|a| create_area_violation(commit, a))
        .collect()
}
```

**Update `validate_commits`:**

```rust
pub fn validate_commits(
    root: &Path,
    base: &str,
    config: &DocsCommitConfig,
    areas: &HashMap<String, DocsAreaConfig>,
) -> CommitValidation {
    // ... existing setup ...

    // Check each feature commit
    for commit in &feature_commits {
        let result = check_commit_has_docs(commit, &changed_files, areas);
        if result.has_docs {
            validation.with_docs += 1;
        } else {
            // Create violations for each missing area
            let violations = create_violations_for_commit(commit, &result);
            validation.violations.extend(violations);
        }
    }

    validation
}
```

**Verification:**
- Violations include area name and match type
- Multiple violations generated when commit touches multiple areas without docs

---

### Phase 5: Add Behavioral Tests

Add behavioral specs to verify source-based matching works end-to-end.

**tests/specs/checks/docs/commit.rs:**

```rust
/// Spec: docs/specs/checks/docs.md#source-based-area-matching
///
/// > When source files matching an area's `source` pattern are changed,
/// > require documentation changes matching that area's `docs` pattern.
#[test]
fn source_change_triggers_area_doc_requirement() {
    let temp = Project::empty();
    temp.config(
        r#"[check.docs.commit]
check = "error"

[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"
"#,
    );

    // Initialize git repo
    init_git_repo(temp.path());

    // Feature branch with source change but no scope
    Command::new("git")
        .args(["checkout", "-b", "feature/api-change"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    temp.file("src/api/handler.rs", "pub fn handler() {}");
    git_add_commit(temp.path(), "feat: add api handler");

    check("docs")
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("docs/api/**")
        .stdout_has("source");
}

/// Spec: docs/specs/checks/docs.md#multiple-area-matching
///
/// > When source changes match multiple areas, require docs for all.
#[test]
fn multiple_source_areas_require_all_docs() {
    let temp = Project::empty();
    temp.config(
        r#"[check.docs.commit]
check = "error"

[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"

[check.docs.area.cli]
docs = "docs/cli/**"
source = "src/cli/**"
"#,
    );

    init_git_repo(temp.path());

    Command::new("git")
        .args(["checkout", "-b", "feature/multi-area"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    temp.file("src/api/handler.rs", "pub fn api() {}");
    temp.file("src/cli/main.rs", "pub fn cli() {}");
    git_add_commit(temp.path(), "feat: add api and cli");

    check("docs")
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("docs/api/**")
        .stdout_has("docs/cli/**");
}

/// Spec: docs/specs/checks/docs.md#scope-priority
///
/// > Scope-based matching takes priority over source-based matching.
#[test]
fn scope_takes_priority_over_source() {
    let temp = Project::empty();
    temp.config(
        r#"[check.docs.commit]
check = "error"

[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"
"#,
    );

    init_git_repo(temp.path());

    Command::new("git")
        .args(["checkout", "-b", "feature/scoped"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    temp.file("src/api/handler.rs", "pub fn api() {}");
    temp.file("docs/api/handler.md", "# Handler");
    git_add_commit(temp.path(), "feat(api): add handler with docs");

    // Should pass - scope matched and docs exist
    check("docs").pwd(temp.path()).args(&["--ci"]).passes();
}

/// Spec: docs/specs/checks/docs.md#source-with-docs
///
/// > Source-matched areas pass when corresponding docs exist.
#[test]
fn source_match_passes_with_docs() {
    let temp = Project::empty();
    temp.config(
        r#"[check.docs.commit]
check = "error"

[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"
"#,
    );

    init_git_repo(temp.path());

    Command::new("git")
        .args(["checkout", "-b", "feature/with-docs"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    temp.file("src/api/handler.rs", "pub fn api() {}");
    temp.file("docs/api/handler.md", "# Handler");
    git_add_commit(temp.path(), "feat: add handler with docs");

    // Should pass - source matched and docs exist
    check("docs").pwd(temp.path()).args(&["--ci"]).passes();
}

// Helper functions
fn init_git_repo(path: &Path) {
    Command::new("git").args(["init"]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial"])
        .current_dir(path)
        .output()
        .unwrap();
}

fn git_add_commit(path: &Path, msg: &str) {
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(path)
        .output()
        .unwrap();
}
```

**Verification:**
- All behavioral specs pass
- `cargo test --test specs commit` succeeds

---

### Phase 6: Polish and Documentation

1. **Update output formatting:**
   ```
   docs: FAIL
     Branch has feature commits without documentation:
       abc123: feat: add api handler
         Changes in api area require docs/api/**
   ```

2. **Bump `CACHE_VERSION`** in `crates/cli/src/cache.rs`

3. **Run full test suite:**
   ```bash
   make check
   ```

4. **Update spec doc** if needed in `docs/specs/checks/docs.md`

## Key Implementation Details

### Area Matching Priority

1. **Scope match** (highest priority): If commit has `feat(api):`, look up "api" area
2. **Source match**: If changed files match any area's `source` pattern
3. **Default**: Require any change in `docs/**`

### Multiple Area Handling

When source changes match multiple areas:
- All matched areas must have corresponding doc changes
- One violation per area missing docs
- Areas are independent (satisfying one doesn't affect others)

### Glob Pattern Matching

Uses existing `globset` crate patterns:
- `src/api/**` matches `src/api/handler.rs`, `src/api/nested/file.rs`
- `docs/api/**` matches any file under `docs/api/`

### Scope vs Source Priority

When a commit has a scope that matches an area, source matching is skipped for that area. This prevents duplicate violations and ensures explicit intent (via scope) takes precedence.

```
feat(api): change       -> Only check api area (scope)
feat: change api/x.rs   -> Check api area (source)
feat(cli): change api/  -> Check cli area (scope), api area (source)
```

## Verification Plan

### Unit Tests

**crates/cli/src/checks/docs/commit_tests.rs:**
- `find_areas_from_source` - Pattern matching
- `check_commit_has_docs` - Integration of scope and source
- `create_area_violation` - Correct advice and fields
- Edge cases: empty areas, no source patterns, multiple matches

### Behavioral Tests

**tests/specs/checks/docs/commit.rs:**
- `source_change_triggers_area_doc_requirement`
- `multiple_source_areas_require_all_docs`
- `scope_takes_priority_over_source`
- `source_match_passes_with_docs`

### Integration

```bash
# Full test suite
make check

# Specific tests
cargo test --test specs commit
cargo test -p quench commit

# Manual testing
cargo run -- check --ci --docs
```

### Manual Verification

1. Create test repo with area config
2. Make feature commit touching area source files (without scope)
3. Run `quench check --ci` → should fail
4. Add matching docs → should pass
5. Test multiple area scenario
