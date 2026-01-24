# Phase 615: Docs Check - Specs Directory

**Root Feature:** `quench-6204`

## Overview

Add specs directory validation to the docs check. This phase implements the foundation for validating specification documents by:
1. Configurable specs directory path (default: `docs/specs`)
2. Extension filtering for spec files (default: `.md`)
3. Index file auto-detection with configurable priority order
4. Basic `index = "exists"` mode to verify index file presence

This provides the scaffolding for future phases that will add linked reachability checking and section validation.

## Project Structure

```
crates/cli/src/checks/docs/
├── mod.rs          # Dispatcher (add specs::validate_specs call)
├── toc.rs          # Existing TOC validation
├── links.rs        # Existing link validation
└── specs.rs        # NEW: Specs directory validation
```

Key files to modify:
- `crates/cli/src/checks/docs/mod.rs` - Add `specs::validate_specs` call
- `crates/cli/src/config/checks.rs` - Add `SpecsConfig` struct

## Dependencies

No new external dependencies required. Uses:
- `globset` (already available)
- Standard path handling from `std::path`

## Implementation Phases

### Phase 1: Configuration Struct

Add `SpecsConfig` to `crates/cli/src/config/checks.rs` and integrate with `DocsConfig`.

```rust
/// Configuration for specs directory validation.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpecsConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,

    /// Specs directory path (default: "docs/specs").
    #[serde(default = "SpecsConfig::default_path")]
    pub path: String,

    /// File extension for spec files (default: ".md").
    #[serde(default = "SpecsConfig::default_extension")]
    pub extension: String,

    /// Index mode: "auto" | "toc" | "linked" | "exists" (default: "exists" for this phase).
    #[serde(default = "SpecsConfig::default_index")]
    pub index: String,

    /// Override index file path (auto-detect if not specified).
    pub index_file: Option<String>,
}

impl Default for SpecsConfig {
    fn default() -> Self {
        Self {
            check: None,
            path: Self::default_path(),
            extension: Self::default_extension(),
            index: Self::default_index(),
            index_file: None,
        }
    }
}

impl SpecsConfig {
    pub(super) fn default_path() -> String {
        "docs/specs".to_string()
    }

    pub(super) fn default_extension() -> String {
        ".md".to_string()
    }

    pub(super) fn default_index() -> String {
        "exists".to_string()
    }
}
```

Update `DocsConfig`:
```rust
pub struct DocsConfig {
    pub check: Option<String>,
    pub toc: TocConfig,
    pub links: LinksConfig,
    pub specs: SpecsConfig,  // NEW
}
```

**Milestone:** Configuration parses correctly with new fields.

### Phase 2: Index File Detection

Create `crates/cli/src/checks/docs/specs.rs` with index file detection logic.

**Detection order** (per spec):
1. `{path}/CLAUDE.md`
2. `docs/CLAUDE.md`
3. `{path}/00-overview.md`
4. `{path}/overview.md`
5. `{path}/00-summary.md`
6. `{path}/summary.md`
7. `{path}/00-index.md`
8. `{path}/index.md`
9. `docs/SPECIFICATIONS.md`
10. `docs/SPECS.md`

```rust
/// Index file detection candidates in priority order.
const INDEX_CANDIDATES: &[IndexCandidate] = &[
    IndexCandidate::InPath("CLAUDE.md"),
    IndexCandidate::Fixed("docs/CLAUDE.md"),
    IndexCandidate::InPath("00-overview.md"),
    IndexCandidate::InPath("overview.md"),
    IndexCandidate::InPath("00-summary.md"),
    IndexCandidate::InPath("summary.md"),
    IndexCandidate::InPath("00-index.md"),
    IndexCandidate::InPath("index.md"),
    IndexCandidate::Fixed("docs/SPECIFICATIONS.md"),
    IndexCandidate::Fixed("docs/SPECS.md"),
];

enum IndexCandidate {
    /// Relative to configured path (e.g., "{path}/CLAUDE.md")
    InPath(&'static str),
    /// Fixed path from project root
    Fixed(&'static str),
}

/// Detect index file using priority order.
fn detect_index_file(root: &Path, specs_path: &str) -> Option<PathBuf> {
    for candidate in INDEX_CANDIDATES {
        let path = match candidate {
            IndexCandidate::InPath(name) => root.join(specs_path).join(name),
            IndexCandidate::Fixed(path) => root.join(path),
        };
        if path.exists() && path.is_file() {
            return Some(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    None
}
```

**Milestone:** Index file detection works with priority order.

### Phase 3: Extension Filtering

Add helper to filter files by extension and validate specs directory existence.

```rust
/// Check if a file matches the configured extension.
fn matches_extension(path: &Path, extension: &str) -> bool {
    // Handle both ".md" and "md" formats
    let ext = extension.trim_start_matches('.');
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(ext))
        .unwrap_or(false)
}

/// Count spec files in the directory.
fn count_spec_files(root: &Path, specs_path: &str, extension: &str) -> usize {
    let specs_dir = root.join(specs_path);
    if !specs_dir.exists() || !specs_dir.is_dir() {
        return 0;
    }

    walkdir::WalkDir::new(&specs_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| matches_extension(e.path(), extension))
        .count()
}
```

**Milestone:** Extension filtering correctly identifies spec files.

### Phase 4: Exists Mode Validation

Implement the `index = "exists"` validation mode.

```rust
pub fn validate_specs(ctx: &CheckContext, violations: &mut Vec<Violation>) {
    let config = &ctx.config.check.docs.specs;

    // Check if specs validation is disabled
    let check_level = config
        .check
        .as_deref()
        .or(ctx.config.check.docs.check.as_deref())
        .unwrap_or("error");
    if check_level == "off" {
        return;
    }

    let specs_path = &config.path;
    let specs_dir = ctx.root.join(specs_path);

    // Skip if specs directory doesn't exist (not an error - project may not use specs)
    if !specs_dir.exists() || !specs_dir.is_dir() {
        return;
    }

    // Detect or use configured index file
    let index_file = config.index_file.as_ref().map(PathBuf::from).or_else(|| {
        detect_index_file(ctx.root, specs_path)
    });

    // Validate based on mode
    match config.index.as_str() {
        "exists" | "auto" => {
            // Check that index file exists
            if index_file.is_none() {
                violations.push(Violation::file(
                    specs_path,
                    0,
                    "missing_index",
                    "Specs directory has no index file.\n\
                     Create CLAUDE.md, overview.md, or index.md in the specs directory.",
                ));
            }
        }
        "toc" | "linked" => {
            // Future phases - skip for now
            // For now, fall back to exists mode
            if index_file.is_none() {
                violations.push(Violation::file(
                    specs_path,
                    0,
                    "missing_index",
                    "Specs directory has no index file.\n\
                     Create CLAUDE.md, overview.md, or index.md in the specs directory.",
                ));
            }
        }
        _ => {
            // Unknown mode - treat as exists
            if index_file.is_none() {
                violations.push(Violation::file(
                    specs_path,
                    0,
                    "missing_index",
                    "Specs directory has no index file.\n\
                     Create CLAUDE.md, overview.md, or index.md in the specs directory.",
                ));
            }
        }
    }
}
```

**Milestone:** Exists mode validation works correctly.

### Phase 5: Integration and Metrics

Integrate with `mod.rs` and add metrics output.

Update `crates/cli/src/checks/docs/mod.rs`:
```rust
mod links;
mod specs;
mod toc;

// In run():
toc::validate_toc(ctx, &mut violations);
links::validate_links(ctx, &mut violations);
specs::validate_specs(ctx, &mut violations);
```

Add metrics collection (for JSON output):
```rust
/// Collect specs metrics for reporting.
pub fn collect_metrics(ctx: &CheckContext) -> Option<SpecsMetrics> {
    let config = &ctx.config.check.docs.specs;
    let specs_dir = ctx.root.join(&config.path);

    if !specs_dir.exists() {
        return None;
    }

    let index_file = config.index_file.as_ref().map(PathBuf::from).or_else(|| {
        detect_index_file(ctx.root, &config.path)
    });

    Some(SpecsMetrics {
        index_file: index_file.map(|p| p.to_string_lossy().to_string()),
        spec_files: count_spec_files(ctx.root, &config.path, &config.extension),
    })
}

pub struct SpecsMetrics {
    pub index_file: Option<String>,
    pub spec_files: usize,
}
```

**Milestone:** Full integration with metrics in JSON output.

### Phase 6: Tests and Fixtures

Create tests and update specs.

**Unit tests** in `crates/cli/src/checks/docs/specs_tests.rs`:
```rust
#[test]
fn detects_claude_md_first() {
    let temp = tempdir();
    std::fs::create_dir_all(temp.path().join("docs/specs")).unwrap();
    std::fs::write(temp.path().join("docs/specs/CLAUDE.md"), "# Index").unwrap();
    std::fs::write(temp.path().join("docs/specs/overview.md"), "# Overview").unwrap();

    let result = detect_index_file(temp.path(), "docs/specs");
    assert_eq!(result, Some(PathBuf::from("docs/specs/CLAUDE.md")));
}

#[test]
fn matches_md_extension() {
    assert!(matches_extension(Path::new("foo.md"), ".md"));
    assert!(matches_extension(Path::new("foo.md"), "md"));
    assert!(!matches_extension(Path::new("foo.txt"), ".md"));
}

#[test]
fn detects_numbered_overview() {
    let temp = tempdir();
    std::fs::create_dir_all(temp.path().join("docs/specs")).unwrap();
    std::fs::write(temp.path().join("docs/specs/00-overview.md"), "# Overview").unwrap();

    let result = detect_index_file(temp.path(), "docs/specs");
    assert_eq!(result, Some(PathBuf::from("docs/specs/00-overview.md")));
}
```

**Test fixtures:**
- `tests/fixtures/docs/specs-ok/` - valid specs directory with index
- `tests/fixtures/docs/specs-no-index/` - specs directory without index (fails)

**Fixture: `tests/fixtures/docs/specs-ok/`**
```
quench.toml
docs/specs/CLAUDE.md
docs/specs/01-feature.md
```

**Fixture: `tests/fixtures/docs/specs-no-index/`**
```
quench.toml
docs/specs/orphan.md
```

**Update existing specs** in `tests/specs/checks/docs/index.rs`:
- Remove `#[ignore]` from `exists_mode_only_checks_index_exists` after implementation

**Milestone:** All specs pass, no regressions.

## Key Implementation Details

### Violation Output Format

Text output:
```
docs: FAIL
  docs/specs: missing_index
    Specs directory has no index file.
    Create CLAUDE.md, overview.md, or index.md in the specs directory.
```

JSON output:
```json
{
  "file": "docs/specs",
  "line": null,
  "type": "missing_index",
  "advice": "Specs directory has no index file.\nCreate CLAUDE.md, overview.md, or index.md in the specs directory."
}
```

### Configuration Examples

**Default behavior** (no config needed):
```toml
# Looks for docs/specs/ with .md files
# Auto-detects index file
```

**Custom specs path:**
```toml
[check.docs.specs]
path = "specifications"
extension = ".md"
```

**Explicit index file:**
```toml
[check.docs.specs]
index_file = "docs/SPECIFICATIONS.md"
```

**Disable specs validation:**
```toml
[check.docs.specs]
check = "off"
```

### Edge Cases

| Case | Behavior |
|------|----------|
| No specs directory | Skip silently (not all projects have specs) |
| Empty specs directory | Check for index file only |
| `index = "exists"` + no index | Generate `missing_index` violation |
| `index_file` configured + missing | Generate `missing_index` violation |
| Extension without dot (`.md` vs `md`) | Normalize both formats |

### Future Extensions (Not This Phase)

- `index = "linked"` - Verify all spec files reachable via links from index
- `index = "toc"` - Parse TOC in index, verify all entries exist
- `sections.required` - Validate required sections in spec files
- `max_lines` / `max_tokens` - Content size limits

## Verification Plan

### Unit Tests

Create `crates/cli/src/checks/docs/specs_tests.rs`:
- Index file detection priority order
- Extension matching (with/without dot)
- Spec file counting
- Metrics collection

### Spec Tests

Enable specs in `tests/specs/checks/docs/index.rs`:
```rust
#[test]
fn exists_mode_only_checks_index_exists() {
    let temp = Project::empty();
    temp.config(
        r#"[check.docs.specs]
path = "docs/specs"
index = "exists"
"#,
    );
    temp.file("docs/specs/CLAUDE.md", "# Specs Index\n");
    temp.file("docs/specs/orphan.md", "# Orphan (not linked)\n");

    // In exists mode, orphan.md is not flagged as unreachable
    check("docs").pwd(temp.path()).passes();
}
```

### Integration Verification

```bash
# Run all docs specs
cargo test --test specs docs

# Run full check suite
make check
```

## Checklist

- [ ] Add `SpecsConfig` to `crates/cli/src/config/checks.rs`
- [ ] Add `specs` field to `DocsConfig`
- [ ] Create `crates/cli/src/checks/docs/specs.rs`
- [ ] Create `crates/cli/src/checks/docs/specs_tests.rs`
- [ ] Implement index file detection with priority order
- [ ] Implement extension filtering
- [ ] Implement `index = "exists"` mode
- [ ] Add `specs::validate_specs` call in `mod.rs`
- [ ] Create test fixtures (`specs-ok`, `specs-no-index`)
- [ ] Update `tests/specs/checks/docs/index.rs` - remove `#[ignore]` from `exists_mode_only_checks_index_exists`
- [ ] Verify all specs pass
- [ ] Run `make check`
- [ ] Update `CACHE_VERSION` if check logic affects caching
