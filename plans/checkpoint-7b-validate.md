# Checkpoint 7B: Docs Check Complete - Validation Plan

## Overview

Validate that the docs check feature works end-to-end on real projects. This includes running `quench check --docs` on the docs-project fixture, configuring quench to validate its own docs/specs/ directory, and adding exact output tests for docs check output format.

## Checkpoint Criteria

- [ ] `quench check --docs` on fixtures/docs-project validates TOC and links
- [ ] `quench check --docs` on quench itself validates docs/specs/
- [ ] Snapshot tests for docs output

## Project Structure

No new files created. Changes to existing files:

```
quench/
├── quench.toml                        # Add [check.docs] configuration
├── docs/specs/
│   └── CLAUDE.md                      # Already serves as index file
├── tests/specs/checks/docs/
│   └── output.rs                      # Add text output format tests
└── reports/
    └── checkpoint-7-docs-check.md     # Validation report (NEW)
```

## Dependencies

None - all required functionality is already implemented.

## Implementation Phases

### Phase 1: Validate docs-project Fixture

**Goal**: Confirm `quench check --docs` works on the existing docs-project fixture.

1. Run `quench check --docs` on `tests/fixtures/docs-project`
2. Verify it passes (exit code 0)
3. Verify JSON output contains expected metrics:
   - `index_file`: detected index file path
   - `spec_files`: count of spec files found
4. Document results in checkpoint report

**Verification**:
```bash
cargo build && ./target/debug/quench check --docs tests/fixtures/docs-project
./target/debug/quench check --docs --format json tests/fixtures/docs-project
```

### Phase 2: Configure quench Self-Validation

**Goal**: Add docs check configuration so quench validates its own docs/specs/.

1. Add `[check.docs]` section to root `quench.toml`:

```toml
[check.docs]
check = "error"
path = "docs/specs"
index = "auto"  # Will detect CLAUDE.md as index

[check.docs.toc]
include = ["docs/**/*.md"]
exclude = ["plans/**"]

[check.docs.links]
include = ["docs/**/*.md"]
exclude = ["plans/**"]
```

2. Ensure docs/specs/CLAUDE.md has valid TOC entries
3. Run `quench check --docs` on quench repo and verify it passes

**Verification**:
```bash
cargo build && ./target/debug/quench check --docs
```

### Phase 3: Add Text Output Snapshot Tests

**Goal**: Add exact output comparison tests for docs check text format.

Add to `tests/specs/checks/docs/output.rs`:

```rust
/// Spec: docs/specs/checks/docs.md#text-output
///
/// > On pass: "docs: PASS" with metrics
#[test]
fn docs_text_output_on_pass() {
    cli()
        .on("docs/toc-ok")
        .args(&["--docs"])
        .passes()
        .stdout_has("docs: PASS");
}

/// Spec: docs/specs/checks/docs.md#text-output
///
/// > On fail: "docs: FAIL" with violations
#[test]
fn docs_text_output_on_broken_toc() {
    let result = cli()
        .on("docs/toc-broken")
        .args(&["--docs"])
        .fails();

    result.stdout_has("docs: FAIL");
    result.stdout_has("broken_toc");
}

/// Spec: docs/specs/checks/docs.md#text-output
///
/// > broken_link violations show target path and advice
#[test]
fn docs_text_output_on_broken_link() {
    let result = cli()
        .on("docs/link-broken")
        .args(&["--docs"])
        .fails();

    result.stdout_has("docs: FAIL");
    result.stdout_has("broken_link");
}
```

**Verification**:
```bash
cargo test --test specs docs_text_output
```

### Phase 4: Create Validation Report

**Goal**: Document validation results in checkpoint report.

Create `reports/checkpoint-7-docs-check.md` with:

1. Summary table of checkpoint criteria
2. Command outputs for docs-project fixture
3. Command outputs for quench self-validation
4. List of all passing docs specs (54 specs)
5. Any issues found and resolutions

## Key Implementation Details

### Index File Detection

The docs check auto-detects index files in this priority order:
1. `{path}/CLAUDE.md` (matches quench's docs/specs/CLAUDE.md)
2. `docs/CLAUDE.md`
3. `{path}/00-overview.md`
4. `{path}/overview.md`
5. ... and more

### TOC Validation Strategy

The TOC validator uses three resolution strategies:
1. Relative to markdown file's directory
2. Relative to project root
3. Strip parent directory name prefix

This handles common documentation patterns where paths may be written different ways.

### Test Patterns

The project uses `stdout_has()` and `stdout_eq()` for output testing rather than snapshot libraries like insta. The pattern is:

```rust
cli().on("fixture").args(&["--docs"]).passes().stdout_has("expected");
```

## Verification Plan

### Automated Tests

```bash
# Run all docs specs
cargo test --test specs checks_docs

# Run new output tests
cargo test --test specs docs_text_output
```

### Manual Validation

```bash
# Build quench
cargo build

# Test on docs-project fixture
./target/debug/quench check --docs tests/fixtures/docs-project
./target/debug/quench check --docs --format json tests/fixtures/docs-project

# Test on quench itself
./target/debug/quench check --docs
./target/debug/quench check --docs --format json

# Full check suite
make check
```

### Expected Results

| Criterion | Expected Outcome |
|-----------|------------------|
| docs-project fixture | PASS - all TOC paths valid, all links valid |
| quench self-validation | PASS - docs/specs/ structure validated |
| Text output tests | 3+ new specs passing |
| All docs specs | 54+ specs passing |

## Completion Checklist

- [ ] Phase 1: Verify docs-project fixture passes
- [ ] Phase 2: Add [check.docs] to quench.toml
- [ ] Phase 3: Add text output format tests
- [ ] Phase 4: Create validation report
- [ ] `cargo test --test specs checks_docs` passes
- [ ] `make check` passes
