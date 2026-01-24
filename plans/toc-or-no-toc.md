# TOC-or-No-TOC: Explicit Code Block Annotations

**Root Feature:** `quench-51f9`

## Overview

Add explicit annotation support for TOC validation in code blocks. The `toc` language tag forces validation regardless of heuristics, while `no-toc` and `ignore` tags explicitly skip validation. This makes intent clear and provides better advice when violations are detected.

This addresses edge cases where:
- A code block looks like a directory tree but shouldn't be validated
- A code block is a valid tree but heuristics miss it
- Users want explicit control over validation

## Project Structure

Changes to existing files only:

```
quench/
└── crates/cli/src/checks/docs/
    ├── toc.rs                  # Add toc/no-toc handling, update advice
    └── toc_tests.rs            # Add tests for new annotations
```

## Dependencies

None - all required functionality is already implemented in the TOC validation module.

## Implementation Phases

### Phase 1: Add `no-toc` and `ignore` Skip Tags

**Goal**: Explicitly skip validation for blocks tagged with `no-toc` or `ignore`.

1. Add `no-toc` and `ignore` to the `NON_TREE_LANGUAGES` constant in `toc.rs`:

```rust
/// Language tags that indicate the block is NOT a directory tree.
#[rustfmt::skip]
const NON_TREE_LANGUAGES: &[&str] = &[
    // Explicit skip annotations
    "no-toc", "ignore",
    // Code languages
    "rust", "rs", "go", ...
];
```

2. Add tests in `toc_tests.rs`:

```rust
#[test]
fn no_toc_block_skipped() {
    let block = FencedBlock {
        start_line: 1,
        lines: vec!["src/".to_string(), "├── lib.rs".to_string()],
        language: Some("no-toc".to_string()),
    };
    assert!(!looks_like_tree(&block));
}

#[test]
fn ignore_block_skipped() {
    let block = FencedBlock {
        start_line: 1,
        lines: vec!["src/".to_string(), "├── lib.rs".to_string()],
        language: Some("ignore".to_string()),
    };
    assert!(!looks_like_tree(&block));
}
```

**Verification**:
```bash
cargo test --package quench -- checks::docs::toc::tests::no_toc
cargo test --package quench -- checks::docs::toc::tests::ignore
```

### Phase 2: Update Advice Messages

**Goal**: Recommend `no-toc` instead of `text` for illustrative trees.

1. Update the advice strings in `validate_file_toc` (around lines 688-724):

```rust
// Old advice:
"If this is illustrative, add a ```text language tag."

// New advice:
"If this is illustrative, add a ```no-toc language tag."
```

2. Update both advice locations:
   - Line ~692: "failed all strategies" path
   - Line ~721: "single best strategy" path

**Verification**:
```bash
cargo test --package quench -- checks::docs
```

### Phase 3: Add `toc` Forced Validation

**Goal**: Force validation when `toc` language tag is present.

1. Add a constant for explicit TOC tags:

```rust
/// Language tag that forces TOC validation.
const TOC_LANGUAGE: &str = "toc";
```

2. Modify `looks_like_tree` to always return `true` for `toc`-tagged blocks:

```rust
pub(super) fn looks_like_tree(block: &FencedBlock) -> bool {
    // Explicit toc tag forces validation
    if block.language.as_deref() == Some(TOC_LANGUAGE) {
        return true;
    }

    // Blocks with known non-tree language tags are skipped
    if let Some(ref lang) = block.language
        && NON_TREE_LANGUAGES.contains(&lang.as_str())
    {
        return false;
    }

    // ... rest of heuristic detection
}
```

3. Add tests:

```rust
#[test]
fn toc_tag_forces_validation() {
    let block = FencedBlock {
        start_line: 1,
        lines: vec!["just-a-file.txt".to_string()],
        language: Some("toc".to_string()),
    };
    // Single line without tree indicators would normally fail heuristics
    assert!(looks_like_tree(&block));
}

#[test]
fn toc_tag_with_box_drawing() {
    let block = FencedBlock {
        start_line: 1,
        lines: vec![
            "src/".to_string(),
            "├── lib.rs".to_string(),
        ],
        language: Some("toc".to_string()),
    };
    assert!(looks_like_tree(&block));
}
```

**Verification**:
```bash
cargo test --package quench -- checks::docs::toc::tests::toc_tag
```

### Phase 4: Add Format Validation for `toc` Blocks

**Goal**: Error if a `toc`-tagged block doesn't match box-drawing or indentation format.

1. Add a format validation function:

```rust
/// Check if a block matches a valid tree format (box-drawing or indentation).
/// Returns true if at least some entries can be parsed.
fn is_valid_tree_format(block: &FencedBlock) -> bool {
    // Empty block is not a valid tree
    if block.lines.is_empty() {
        return false;
    }

    // Try to parse entries - if we get any, format is valid
    let entries = parse_tree_block(block);
    !entries.is_empty()
}
```

2. Add format validation in `validate_file_toc` for `toc`-tagged blocks:

```rust
for block in blocks {
    // For explicit toc tag, validate format
    if block.language.as_deref() == Some(TOC_LANGUAGE) {
        if !is_valid_tree_format(&block) {
            violations.push(
                Violation::file(
                    relative_path,
                    block.start_line,
                    "invalid_toc_format",
                    "Code block marked as `toc` doesn't match box-drawing or indentation format.\n\
                     Use box-drawing (├──, └──, │) or consistent indentation.",
                )
            );
            continue;
        }
    }

    // Skip blocks that don't look like directory trees
    if !looks_like_tree(&block) {
        continue;
    }

    // ... rest of validation
}
```

3. Add tests:

```rust
#[test]
fn toc_tag_invalid_format_detected() {
    // Test that arbitrary text in a toc block is caught
    let block = FencedBlock {
        start_line: 1,
        lines: vec![
            "This is not a tree".to_string(),
            "Just some random text".to_string(),
        ],
        language: Some("toc".to_string()),
    };
    assert!(!is_valid_tree_format(&block));
}

#[test]
fn toc_tag_valid_indentation_format() {
    let block = FencedBlock {
        start_line: 1,
        lines: vec![
            "src/".to_string(),
            "  lib.rs".to_string(),
        ],
        language: Some("toc".to_string()),
    };
    assert!(is_valid_tree_format(&block));
}
```

**Verification**:
```bash
cargo test --package quench -- checks::docs::toc::tests::toc_tag_invalid
cargo test --package quench -- checks::docs::toc::tests::toc_tag_valid
```

### Phase 5: Integration Tests

**Goal**: Add end-to-end tests for the new annotations.

1. Create test fixture files or add inline tests using tempfile:

```rust
#[test]
fn toc_annotation_validates_when_explicit() {
    use tempfile::TempDir;

    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create a file referenced in the toc block
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), "").unwrap();

    // Create markdown with explicit toc block
    let content = r#"# Test

```toc
src/
├── lib.rs
```
"#;
    std::fs::write(root.join("README.md"), content).unwrap();

    // Validate - should pass
    // (implementation detail: call validate_toc or use CLI)
}

#[test]
fn no_toc_annotation_skips_validation() {
    use tempfile::TempDir;

    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create markdown with no-toc block (file doesn't need to exist)
    let content = r#"# Test

```no-toc
nonexistent/
├── fake.rs
```
"#;
    std::fs::write(root.join("README.md"), content).unwrap();

    // Validate - should pass (block skipped)
}
```

**Verification**:
```bash
cargo test --package quench -- checks::docs::toc
make check
```

## Key Implementation Details

### Language Tag Handling Order

The `looks_like_tree` function should check tags in this order:
1. `toc` → force validation (return true)
2. `no-toc`, `ignore`, or other NON_TREE_LANGUAGES → skip validation (return false)
3. No tag or unknown tag → apply heuristics

### Violation Types

| Violation Code | Condition |
|----------------|-----------|
| `broken_toc` | File path in TOC doesn't exist (existing) |
| `invalid_toc_format` | `toc`-tagged block doesn't parse as tree (new) |

### Advice Message Template

The updated advice for broken TOC entries:
```
File does not exist (N of M paths valid, K failed).
This check ensures directory trees in documentation stay up-to-date.
Update the table of contents or directory tree to match actual files.
If this is illustrative, add a ```no-toc language tag.

Tried: relative to markdown file, relative to project root, stripping parent directory prefix
```

## Verification Plan

### Unit Tests

```bash
# Run all TOC tests
cargo test --package quench -- checks::docs::toc::tests

# Run specific new tests
cargo test --package quench -- checks::docs::toc::tests::no_toc
cargo test --package quench -- checks::docs::toc::tests::ignore
cargo test --package quench -- checks::docs::toc::tests::toc_tag
```

### Full Check Suite

```bash
make check
```

### Expected Results

| Test Category | Expected Outcome |
|---------------|------------------|
| `no-toc` skip | Block with `no-toc` tag not validated |
| `ignore` skip | Block with `ignore` tag not validated |
| `toc` force | Block with `toc` tag always validated |
| `toc` format | Invalid format in `toc` block produces `invalid_toc_format` |
| Advice update | Violation advice suggests `no-toc` not `text` |

## Completion Checklist

- [ ] Phase 1: `no-toc` and `ignore` added to NON_TREE_LANGUAGES
- [ ] Phase 2: Advice messages updated to suggest `no-toc`
- [ ] Phase 3: `toc` tag forces validation in `looks_like_tree`
- [ ] Phase 4: Format validation added for `toc`-tagged blocks
- [ ] Phase 5: Integration tests pass
- [ ] All existing TOC tests still pass
- [ ] `make check` passes
