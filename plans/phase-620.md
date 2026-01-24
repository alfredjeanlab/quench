# Phase 620: Docs Check - Specs Index Modes

**Root Feature:** `quench-355e`

## Overview

Implement three index validation modes for the specs directory check: `toc`, `linked`, and `auto`. These modes determine how the system verifies that spec files are discoverable from the index file (e.g., `CLAUDE.md`). Unreachable specs generate violations, ensuring documentation remains navigable.

**Current state**: The `index = "exists"` mode only checks that an index file exists. It performs no reachability analysis.

**Target state**: Three new modes that trace spec file reachability:
- `toc` - Specs must appear in directory trees within the index
- `linked` - Specs must be reachable via markdown link chains
- `auto` - Try `toc` first, fallback to `linked` if no trees found

## Project Structure

```
crates/cli/src/checks/docs/
├── mod.rs         # Orchestrates toc, links, specs validators
├── specs.rs       # Specs directory validation (MODIFY)
├── toc.rs         # Directory tree parsing (REUSE)
└── links.rs       # Markdown link extraction (REUSE)

tests/specs/checks/docs/
└── index.rs       # Specs index mode tests (UPDATE)

tests/fixtures/docs/
├── index-toc/     # TOC mode fixture (EXISTS)
├── index-linked/  # Linked mode fixture (EXISTS)
└── unreachable-spec/  # Unreachable spec fixture (EXISTS)
```

## Dependencies

No new external dependencies. Reuses:
- Existing `toc.rs` tree parsing
- Existing `links.rs` link extraction
- Existing `walkdir` for spec file enumeration

## Implementation Phases

### Phase 1: Collect Spec Files

Add a function to enumerate all spec files in the specs directory.

**File**: `crates/cli/src/checks/docs/specs.rs`

```rust
/// Collect all spec files in the specs directory.
fn collect_spec_files(
    specs_path: &Path,
    extension: &str,
) -> io::Result<HashSet<PathBuf>> {
    let mut spec_files = HashSet::new();
    for entry in WalkDir::new(specs_path)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| {
                format!(".{}", ext.to_string_lossy()) == extension
            }) {
                spec_files.insert(path.to_path_buf());
            }
        }
    }
    Ok(spec_files)
}
```

**Verification**: Unit test that collects files from a fixture directory.

### Phase 2: Implement `toc` Mode

Parse directory trees in the index file and collect referenced spec files. Compare against all spec files to find unreachable ones.

**File**: `crates/cli/src/checks/docs/specs.rs`

```rust
fn validate_toc_mode(
    index_file: &Path,
    specs_path: &Path,
    all_specs: &HashSet<PathBuf>,
    violations: &mut Vec<Violation>,
    limit: usize,
) -> io::Result<()> {
    // 1. Read index file content
    let content = fs::read_to_string(index_file)?;

    // 2. Extract tree blocks using toc::extract_tree_blocks()
    let blocks = toc::extract_tree_blocks(&content);

    // 3. For each block, parse entries and resolve paths
    let mut reachable: HashSet<PathBuf> = HashSet::new();
    for block in &blocks {
        let entries = toc::parse_tree_block(block);
        for entry in entries {
            // Resolve relative to specs_path
            let resolved = specs_path.join(&entry);
            if resolved.exists() && all_specs.contains(&resolved) {
                reachable.insert(resolved);
            }
        }
    }

    // 4. Generate violations for unreachable specs
    for spec in all_specs.difference(&reachable) {
        if violations.len() >= limit { break; }
        violations.push(Violation::file_only(
            spec,
            "unreachable_spec",
            "Spec file not referenced in index directory tree",
        ));
    }
    Ok(())
}
```

**Key details**:
- Reuse `toc::extract_tree_blocks()` and `toc::parse_tree_block()`
- May need to expose these as `pub(super)` in `toc.rs`
- Resolve entries relative to specs directory, not index file location

**Verification**:
- Enable `tests/specs/checks/docs/index.rs::unreachable_spec_file_generates_violation_toc_mode`
- Fixture: `tests/fixtures/docs/index-toc/`

### Phase 3: Implement `linked` Mode

Trace markdown links starting from the index file. Perform a breadth-first traversal to find all reachable spec files.

**File**: `crates/cli/src/checks/docs/specs.rs`

```rust
fn validate_linked_mode(
    index_file: &Path,
    specs_path: &Path,
    all_specs: &HashSet<PathBuf>,
    violations: &mut Vec<Violation>,
    limit: usize,
) -> io::Result<()> {
    // BFS from index file
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    let mut reachable: HashSet<PathBuf> = HashSet::new();

    queue.push_back(index_file.to_path_buf());
    visited.insert(index_file.to_path_buf());

    while let Some(current) = queue.pop_front() {
        let content = match fs::read_to_string(&current) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Extract links using links::extract_links()
        let links = links::extract_links(&content);
        for (link_target, _line) in links {
            // Resolve relative to current file's directory
            let resolved = current.parent()
                .unwrap_or(Path::new("."))
                .join(&link_target)
                .canonicalize()
                .ok();

            if let Some(resolved) = resolved {
                // Track if it's a spec file
                if all_specs.contains(&resolved) {
                    reachable.insert(resolved.clone());
                }

                // Queue for traversal if markdown and not yet visited
                if resolved.extension().map_or(false, |e| e == "md")
                    && visited.insert(resolved.clone())
                {
                    queue.push_back(resolved);
                }
            }
        }
    }

    // Generate violations for unreachable specs
    for spec in all_specs.difference(&reachable) {
        if violations.len() >= limit { break; }
        violations.push(Violation::file_only(
            spec,
            "unreachable_spec",
            "Spec file not reachable via markdown links from index",
        ));
    }
    Ok(())
}
```

**Key details**:
- Expose `links::extract_links()` as `pub(super)`
- BFS traversal follows links to other `.md` files
- Only files within specs directory count as spec files
- Canonicalize paths for consistent comparison

**Verification**:
- Enable `tests/specs/checks/docs/index.rs::unreachable_spec_file_generates_violation_linked_mode`
- Fixture: `tests/fixtures/docs/index-linked/`

### Phase 4: Implement `auto` Mode

Try `toc` mode first. If the index file contains no directory trees, fallback to `linked` mode.

**File**: `crates/cli/src/checks/docs/specs.rs`

```rust
fn validate_auto_mode(
    index_file: &Path,
    specs_path: &Path,
    all_specs: &HashSet<PathBuf>,
    violations: &mut Vec<Violation>,
    limit: usize,
) -> io::Result<()> {
    let content = fs::read_to_string(index_file)?;
    let blocks = toc::extract_tree_blocks(&content);

    if blocks.is_empty() {
        // No trees found, use linked mode
        validate_linked_mode(index_file, specs_path, all_specs, violations, limit)
    } else {
        // Trees found, use toc mode
        validate_toc_mode(index_file, specs_path, all_specs, violations, limit)
    }
}
```

**Verification**:
- Test with fixture containing trees → uses toc mode
- Test with fixture containing only links → uses linked mode

### Phase 5: Wire Up and Update Tests

Update `validate_specs()` to dispatch based on index mode:

```rust
pub fn validate_specs(
    ctx: &Context,
    violations: &mut Vec<Violation>,
) -> io::Result<()> {
    let specs_config = &ctx.config.check.docs.specs;
    let specs_path = ctx.root.join(&specs_config.path);

    if !specs_path.exists() {
        return Ok(());
    }

    // Find index file
    let index_file = match find_index_file(&specs_path, &ctx.root, specs_config) {
        Some(f) => f,
        None => {
            violations.push(Violation::file_only(
                &specs_path,
                "missing_index",
                "Specs directory exists but no index file found",
            ));
            return Ok(());
        }
    };

    // Dispatch based on mode
    match specs_config.index.as_str() {
        "exists" => Ok(()), // Current behavior - just check index exists
        "toc" => {
            let all_specs = collect_spec_files(&specs_path, &specs_config.extension)?;
            validate_toc_mode(&index_file, &specs_path, &all_specs, violations, ctx.limit)
        }
        "linked" => {
            let all_specs = collect_spec_files(&specs_path, &specs_config.extension)?;
            validate_linked_mode(&index_file, &specs_path, &all_specs, violations, ctx.limit)
        }
        "auto" => {
            let all_specs = collect_spec_files(&specs_path, &specs_config.extension)?;
            validate_auto_mode(&index_file, &specs_path, &all_specs, violations, ctx.limit)
        }
        _ => Ok(()), // Unknown mode, skip
    }
}
```

**Update tests**:
- Remove `#[ignore]` from index mode tests
- Add tests for each mode
- Add test for auto mode fallback behavior

## Key Implementation Details

### Reusing Existing Parsers

The `toc.rs` and `links.rs` modules already have the core parsing logic. Expose these functions:

```rust
// In toc.rs, make public to sibling modules
pub(super) fn extract_tree_blocks(content: &str) -> Vec<&str>
pub(super) fn parse_tree_block(block: &str) -> Vec<String>

// In links.rs
pub(super) fn extract_links(content: &str) -> Vec<(String, usize)>
```

### Path Resolution

- **toc mode**: Resolve tree entries relative to `specs_path` (not index file)
- **linked mode**: Resolve links relative to the containing markdown file
- Use `canonicalize()` for consistent path comparison

### Violation Type

Use a single violation type `unreachable_spec` for all modes:
- `violation_type`: `"unreachable_spec"`
- `advice`: Mode-specific message explaining how to fix
- `pattern`: None (the file path is sufficient)

### Performance Considerations

- Collect all spec files once per check run
- Use `HashSet` for O(1) reachability lookup
- BFS in linked mode has visited set to avoid cycles
- Respect `ctx.limit` to cap violations

## Verification Plan

### Unit Tests

Add to `crates/cli/src/checks/docs/specs_tests.rs`:
1. `collect_spec_files_finds_markdown_files`
2. `collect_spec_files_respects_extension_config`
3. `toc_mode_finds_referenced_specs`
4. `linked_mode_follows_link_chains`
5. `auto_mode_prefers_toc_when_trees_exist`
6. `auto_mode_falls_back_to_linked`

### Spec Tests

Enable in `tests/specs/checks/docs/index.rs`:
1. `unreachable_spec_file_generates_violation_linked_mode` (existing, currently ignored)
2. Add `unreachable_spec_file_generates_violation_toc_mode`
3. Add `auto_mode_uses_toc_when_trees_present`
4. Add `auto_mode_uses_linked_when_no_trees`
5. Add `exists_mode_ignores_unreachable_specs`

### Integration

Run `make check` to verify:
- All tests pass
- No clippy warnings
- Bootstrap scripts succeed
