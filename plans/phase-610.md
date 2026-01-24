# Phase 610: Docs Check - Link Validation

## Overview

Add markdown link validation to the existing docs check. This validates that local file links in markdown documents (`[text](path)`) point to existing files, reporting broken links as violations. External URLs (http/https) are skipped.

## Project Structure

```
crates/cli/src/checks/docs/
├── mod.rs          # Dispatcher (add links::validate_links call)
├── toc.rs          # Existing TOC validation
└── links.rs        # NEW: Link validation
```

Key files to modify:
- `crates/cli/src/checks/docs/mod.rs` - Add `validate_links` call
- `crates/cli/src/config/checks.rs` - Extend `LinksConfig` with include/exclude patterns

## Dependencies

No new external dependencies required. Uses:
- `globset` (already used by TOC validation)
- Standard `regex` for link extraction

## Implementation Phases

### Phase 1: Link Extraction

Create `crates/cli/src/checks/docs/links.rs` with markdown link parsing.

**Pattern to match:**
```
[text](url)
```

Regex pattern:
```rust
static LINK_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[(?:[^\[\]]|\[[^\]]*\])*\]\(([^)]+)\)").unwrap()
});
```

**Key considerations:**
- Must handle nested brackets in link text: `[[text]](url)`
- Must capture only the URL portion (group 1)
- Track line numbers for violation reporting

```rust
/// A markdown link extracted from content.
struct ExtractedLink {
    /// Line number (1-indexed) where the link appears.
    line: u32,
    /// The URL/path from the link.
    target: String,
}

/// Extract all markdown links from content.
fn extract_links(content: &str) -> Vec<ExtractedLink> {
    let mut links = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let line_num = idx as u32 + 1;
        for cap in LINK_PATTERN.captures_iter(line) {
            if let Some(target) = cap.get(1) {
                links.push(ExtractedLink {
                    line: line_num,
                    target: target.as_str().to_string(),
                });
            }
        }
    }
    links
}
```

**Milestone:** Unit tests pass for link extraction with various markdown formats.

### Phase 2: Local vs External Detection

Filter links to only validate local file paths.

```rust
/// Check if a link target is a local file path (not external URL).
fn is_local_link(target: &str) -> bool {
    // Skip external URLs
    if target.starts_with("http://") || target.starts_with("https://") {
        return false;
    }
    // Skip mailto: and other protocols
    if target.contains("://") {
        return false;
    }
    // Skip fragment-only links (#section)
    if target.starts_with('#') {
        return false;
    }
    true
}
```

**Edge cases:**
- `#section` - fragment-only, skip validation
- `page.md#section` - validate path, ignore fragment
- `mailto:foo@bar.com` - skip
- `//cdn.example.com/` - protocol-relative, skip

**Milestone:** External links are correctly identified and skipped.

### Phase 3: Path Resolution

Resolve relative paths to absolute paths for existence checking.

```rust
/// Strip fragment from link target.
fn strip_fragment(target: &str) -> &str {
    target.split('#').next().unwrap_or(target)
}

/// Resolve a link target relative to the markdown file.
fn resolve_link(md_file: &Path, root: &Path, target: &str) -> PathBuf {
    let target = strip_fragment(target);

    // Normalize `.`/`./` prefix
    let normalized = if target.starts_with("./") {
        &target[2..]
    } else if target == "." {
        ""
    } else {
        target
    };

    // Resolve relative to markdown file's directory
    if let Some(parent) = md_file.parent() {
        parent.join(normalized)
    } else {
        root.join(normalized)
    }
}
```

Resolution strategy (simpler than TOC - single pass):
1. Resolve relative to the markdown file's directory
2. Do NOT try project root fallback (links should be explicit)

**Milestone:** Paths resolve correctly for various relative link formats.

### Phase 4: Configuration Extension

Extend `LinksConfig` with include/exclude patterns (matching `TocConfig`).

```rust
// In crates/cli/src/config/checks.rs

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LinksConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,

    /// Include patterns for markdown files.
    #[serde(default = "LinksConfig::default_include")]
    pub include: Vec<String>,

    /// Exclude patterns (plans, etc.).
    #[serde(default = "LinksConfig::default_exclude")]
    pub exclude: Vec<String>,
}

impl Default for LinksConfig {
    fn default() -> Self {
        Self {
            check: None,
            include: Self::default_include(),
            exclude: Self::default_exclude(),
        }
    }
}

impl LinksConfig {
    pub(super) fn default_include() -> Vec<String> {
        vec!["**/*.md".to_string(), "**/*.mdc".to_string()]
    }

    pub(super) fn default_exclude() -> Vec<String> {
        vec![
            "plans/**".to_string(),
            "plan.md".to_string(),
            "*_plan.md".to_string(),
            "plan_*".to_string(),
            "**/fixtures/**".to_string(),
            "**/testdata/**".to_string(),
        ]
    }
}
```

**Milestone:** Configuration parsing works with new fields.

### Phase 5: Validation Integration

Add the main validation function and integrate with `mod.rs`.

```rust
// In crates/cli/src/checks/docs/links.rs

pub fn validate_links(ctx: &CheckContext, violations: &mut Vec<Violation>) {
    let config = &ctx.config.check.docs.links;

    // Check if link validation is disabled
    let check_level = config
        .check
        .as_deref()
        .or(ctx.config.check.docs.check.as_deref())
        .unwrap_or("error");
    if check_level == "off" {
        return;
    }

    // Build include/exclude matchers
    let include_set = build_glob_set(&config.include);
    let exclude_set = build_glob_set(&config.exclude);

    // Process each markdown file
    for walked in ctx.files {
        let relative_path = walked.path.strip_prefix(ctx.root).unwrap_or(&walked.path);
        let path_str = relative_path.to_string_lossy();

        // Check include patterns
        if !include_set.is_match(&*path_str) {
            continue;
        }

        // Check exclude patterns
        if exclude_set.is_match(&*path_str) {
            continue;
        }

        // Read file content
        let content = match std::fs::read_to_string(&walked.path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Extract and validate links
        validate_file_links(ctx, relative_path, &content, violations);
    }
}

fn validate_file_links(
    ctx: &CheckContext,
    relative_path: &Path,
    content: &str,
    violations: &mut Vec<Violation>,
) {
    let links = extract_links(content);
    let abs_file = ctx.root.join(relative_path);

    for link in links {
        // Skip external links
        if !is_local_link(&link.target) {
            continue;
        }

        // Resolve and check existence
        let resolved = resolve_link(&abs_file, ctx.root, &link.target);
        if !resolved.exists() {
            violations.push(
                Violation::file(
                    relative_path,
                    link.line,
                    "broken_link",
                    "Linked file does not exist. Update the link or create the file.",
                )
                .with_pattern(strip_fragment(&link.target)),
            );
        }
    }
}
```

Update `mod.rs` dispatcher:
```rust
mod links;
mod toc;

// In run():
toc::validate_toc(ctx, &mut violations);
links::validate_links(ctx, &mut violations);
```

**Milestone:** Full integration works end-to-end.

### Phase 6: Tests and Specs

Remove `#[ignore]` from existing specs and verify:

1. **Existing fixtures:**
   - `tests/fixtures/docs/link-ok/` - passes
   - `tests/fixtures/docs/link-broken/` - fails with broken_link
   - `tests/fixtures/docs/link-external/` - passes (external URLs skipped)

2. **Add new fixtures for edge cases:**
   - `tests/fixtures/docs/link-fragment/` - links with `#section` fragments
   - `tests/fixtures/docs/link-relative/` - `../` relative paths

3. **Update violation fixture:**
   - Add broken link to `tests/fixtures/violations/docs/specs/broken-link.md`

4. **Unit tests in `links_tests.rs`:**
   - Link extraction patterns
   - Local vs external detection
   - Fragment stripping
   - Path resolution

**Milestone:** All specs pass, no regressions.

## Key Implementation Details

### Violation Output Format

Text output (from spec):
```
docs: FAIL
  README.md:45: broken link: docs/old-guide.md
    Linked file does not exist. Update the link or create the file.
```

JSON output:
```json
{
  "file": "README.md",
  "line": 45,
  "type": "broken_link",
  "pattern": "docs/old-guide.md",
  "advice": "Linked file does not exist. Update the link or create the file."
}
```

Note: Use `pattern` field (not `target`) to match existing Violation struct.

### Edge Cases to Handle

| Case | Behavior |
|------|----------|
| `[text](file.md)` | Validate file exists |
| `[text](file.md#section)` | Validate file exists, ignore fragment |
| `[text](#section)` | Skip (fragment-only link) |
| `[text](https://example.com)` | Skip (external URL) |
| `[text](../sibling/file.md)` | Resolve relative to markdown file |
| `[text](./file.md)` | Same as `file.md` |
| `[text](dir/)` | Validate directory exists |
| Reference links `[1]: url` | Skip (different syntax, future enhancement) |
| Image links `![alt](img.png)` | Skip (different syntax, future enhancement) |

### Performance Considerations

- Reuse `build_glob_set` from TOC module (extract to shared utility)
- Single-pass regex over each line
- Respects `ctx.limit` for early termination

## Verification Plan

### Unit Tests

Create `crates/cli/src/checks/docs/links_tests.rs`:

```rust
#[test]
fn extracts_simple_link() {
    let links = extract_links("[text](file.md)");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "file.md");
}

#[test]
fn extracts_multiple_links_per_line() {
    let links = extract_links("[a](x.md) and [b](y.md)");
    assert_eq!(links.len(), 2);
}

#[test]
fn skips_external_urls() {
    assert!(!is_local_link("https://example.com"));
    assert!(!is_local_link("http://example.com"));
}

#[test]
fn handles_fragment_links() {
    assert!(!is_local_link("#section"));
    assert!(is_local_link("file.md#section"));
    assert_eq!(strip_fragment("file.md#section"), "file.md");
}
```

### Spec Tests

Enable ignored specs in `tests/specs/checks/docs/links.rs`:

```rust
#[test]
fn valid_markdown_link_passes() {
    check("docs").on("docs/link-ok").passes();
}

#[test]
fn markdown_link_to_missing_file_generates_violation() {
    check("docs")
        .on("docs/link-broken")
        .fails()
        .stdout_has("broken link");
}

#[test]
fn external_urls_not_validated() {
    check("docs").on("docs/link-external").passes();
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

- [ ] Create `crates/cli/src/checks/docs/links.rs`
- [ ] Create `crates/cli/src/checks/docs/links_tests.rs`
- [ ] Extend `LinksConfig` in `config/checks.rs`
- [ ] Add `links::validate_links` call in `mod.rs`
- [ ] Add test fixtures for edge cases
- [ ] Remove `#[ignore]` from link specs
- [ ] Verify all specs pass
- [ ] Run `make check`
- [ ] Update `CACHE_VERSION` if check logic affects caching
