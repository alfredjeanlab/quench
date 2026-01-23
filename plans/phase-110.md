# Phase 110: CLOC Check - Token Limits Implementation

**Root Feature:** `quench-515d`

## Overview

Add token counting and limit enforcement to the CLOC check. Tokens are approximated as `characters / 4` (standard LLM tokenization heuristic). Files exceeding `max_tokens` generate violations, enabling AI agents to stay within context window constraints. This phase builds on the existing line counting infrastructure from Phase 105.

## Project Structure

```
crates/cli/src/
├── checks/
│   ├── cloc.rs          # Add token counting and limit enforcement
│   └── cloc_tests.rs    # Unit tests for token counting
├── config.rs            # Add max_tokens config field
tests/
├── fixtures/cloc/
│   └── high-tokens/     # Already exists (max_tokens=100, long identifiers)
└── specs/cloc.rs        # Remove #[ignore] from token spec
```

## Dependencies

No new external dependencies. Uses existing:
- `std::fs` - file reading (already used for line counting)
- `serde_json` - metrics output (already used)

## Implementation Phases

### Phase 1: Add Token Config Field

**Goal**: Parse `max_tokens` from config with default value of 20000.

**Changes in `crates/cli/src/config.rs`**:

1. Add `max_tokens` field to `ClocConfig`:

```rust
pub struct ClocConfig {
    pub max_lines: usize,
    pub max_lines_test: usize,
    pub check: CheckLevel,
    pub test_patterns: Vec<String>,
    pub exclude: Vec<String>,

    /// Maximum tokens per file (default: 20000, None = disabled).
    #[serde(default = "ClocConfig::default_max_tokens")]
    pub max_tokens: Option<usize>,
}

impl ClocConfig {
    fn default_max_tokens() -> Option<usize> {
        Some(20000)
    }
}
```

2. Update `Default` impl to include `max_tokens`.

3. Update `parse_with_warnings` to parse `max_tokens`:

```rust
let max_tokens = cloc_table
    .get("max_tokens")
    .map(|v| {
        if v.as_bool() == Some(false) {
            None  // max_tokens = false disables the check
        } else {
            v.as_integer().map(|n| n as usize)
        }
    })
    .unwrap_or_else(ClocConfig::default_max_tokens);
```

**Verification**: Unit test for parsing `max_tokens` in `config_tests.rs`.

### Phase 2: Implement Token Counting

**Goal**: Count tokens using `chars / 4` approximation.

**Changes in `crates/cli/src/checks/cloc.rs`**:

1. Add a simple token counting function:

```rust
/// Count tokens in a file using chars/4 approximation.
/// This matches typical LLM tokenization behavior.
fn count_tokens(path: &Path) -> std::io::Result<usize> {
    let content = std::fs::read(path)?;
    let text = String::from_utf8(content)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());

    // chars / 4 approximation (standard LLM heuristic)
    Ok(text.chars().count() / 4)
}
```

2. Add unit tests for token counting edge cases:
   - Empty file → 0 tokens
   - Short file (< 4 chars) → 0 tokens
   - File with 100 chars → 25 tokens
   - Unicode handling (chars, not bytes)

**Verification**: Unit tests pass in `cloc_tests.rs`.

### Phase 3: Token Limit Enforcement

**Goal**: Generate violations when files exceed `max_tokens`.

**Changes in `crates/cli/src/checks/cloc.rs`**:

1. In `ClocCheck::run()`, after line counting, add token check:

```rust
// Token limit check (skip excluded files)
if !is_excluded {
    if let Some(max_tokens) = cloc_config.max_tokens {
        let token_count = count_tokens(&file.path).unwrap_or(0);

        if token_count > max_tokens {
            let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
            if let Some(limit) = ctx.limit && current >= limit {
                break;
            }

            let display_path = file.path.strip_prefix(ctx.root).unwrap_or(&file.path);
            violations.push(
                Violation::file_only(
                    display_path,
                    "file_too_large",
                    format!(
                        "Split into smaller modules. {} tokens exceeds {} token limit.",
                        token_count, max_tokens
                    ),
                )
                .with_threshold(token_count as i64, max_tokens as i64),
            );
        }
    }
}
```

2. Track token counts for metrics:

```rust
let mut source_tokens: usize = 0;
let mut test_tokens: usize = 0;

// In the file loop:
let token_count = count_tokens(&file.path).unwrap_or(0);
if is_test {
    test_tokens += token_count;
} else {
    source_tokens += token_count;
}
```

**Note**: Use same violation type `"file_too_large"` as line violations. The `advice` message distinguishes between line vs token violations.

**Verification**: `cloc_fails_on_file_over_max_tokens` spec passes.

### Phase 4: Add Tokens to Metrics Output

**Goal**: Include token counts in JSON metrics.

**Changes in `crates/cli/src/checks/cloc.rs`**:

1. Update metrics JSON:

```rust
let result = result.with_metrics(json!({
    "source_lines": source_lines,
    "source_files": source_files,
    "source_tokens": source_tokens,
    "test_lines": test_lines,
    "test_files": test_files,
    "test_tokens": test_tokens,
    "ratio": (ratio * 100.0).round() / 100.0,
}));
```

2. Update per-package metrics to include tokens:

```rust
struct PackageMetrics {
    source_lines: usize,
    source_files: usize,
    source_tokens: usize,
    test_lines: usize,
    test_files: usize,
    test_tokens: usize,
}
```

**Verification**: JSON output includes `source_tokens` and `test_tokens`.

### Phase 5: Update Fixtures and Specs

**Goal**: Verify the high-tokens fixture triggers a violation and remove `#[ignore]`.

**Fixture review** (`tests/fixtures/cloc/high-tokens/`):

The existing fixture has:
- `quench.toml` with `max_tokens = 100`
- `src/tokens.rs` with 4 lines but long identifiers (~260 chars = ~65 tokens)

The fixture needs adjustment to actually exceed 100 tokens. Update `src/tokens.rs`:

```rust
// Each line has ~100+ chars to exceed 100 tokens total
pub fn very_long_function_name_with_many_words_in_it_to_increase_token_count_and_exceed_limits() -> i32 { 1 }
pub fn another_extremely_long_function_name_designed_to_maximize_tokens_in_this_file() -> i32 { 2 }
pub fn yet_another_incredibly_verbose_function_name_for_testing_purposes_here() -> i32 { 3 }
pub fn one_more_ridiculously_long_function_name_to_ensure_we_hit_the_token_limit() -> i32 { 4 }
pub fn final_extremely_verbose_function_declaration_to_push_us_over_the_edge() -> i32 { 5 }
```

**Spec change** (`tests/specs/cloc.rs`):

Remove `#[ignore]` from the token spec:

```rust
/// Spec: docs/specs/checks/cloc.md#file-size-limits
///
/// > max_tokens = 20000 (default)
#[test]
fn cloc_fails_on_file_over_max_tokens() {
    let json = check_json(&fixture("cloc/high-tokens"));
    let cloc = find_check(&json, "cloc");

    assert_eq!(cloc.get("passed").and_then(|v| v.as_bool()), Some(false));

    let violations = cloc.get("violations").and_then(|v| v.as_array()).unwrap();
    assert!(violations.iter().any(|v| {
        v.get("advice").and_then(|a| a.as_str()).unwrap().contains("token")
    }), "violation should mention tokens");
}
```

**Verification**: Spec passes without `#[ignore]`.

### Phase 6: Add Token Metrics Spec

**Goal**: Verify token metrics appear in JSON output.

**Add new spec** (`tests/specs/cloc.rs`):

```rust
/// Spec: docs/specs/checks/cloc.md#json-output
///
/// > JSON metrics include: source_tokens, test_tokens
#[test]
fn cloc_json_includes_token_metrics() {
    let json = check_json(&fixture("cloc/basic"));
    let cloc = find_check(&json, "cloc");
    let metrics = cloc.get("metrics").expect("cloc should have metrics");

    assert!(metrics.get("source_tokens").is_some(), "missing source_tokens");
    assert!(metrics.get("test_tokens").is_some(), "missing test_tokens");
}
```

**Verification**: New spec passes.

## Key Implementation Details

### Token Counting Algorithm

```rust
// Simple chars/4 approximation
fn count_tokens(text: &str) -> usize {
    text.chars().count() / 4
}
```

This matches typical LLM tokenization behavior where ~4 characters ≈ 1 token. It's intentionally simple and fast.

### Disabling Token Limits

Per spec, `max_tokens = false` disables token checking entirely:

```toml
[check.cloc]
max_tokens = false  # Disable token limit checking
```

This is handled by making `max_tokens` an `Option<usize>`:
- `Some(n)` - enforce n token limit
- `None` - skip token checking

### Violation Message Format

For consistency with line violations:
- **Line violation**: `"Split into smaller modules. 923 lines exceeds 750 line limit."`
- **Token violation**: `"Split into smaller modules. 25000 tokens exceeds 20000 token limit."`

Both use `violation_type = "file_too_large"`.

### Performance Consideration

Token counting reads the entire file content (already done for line counting). To avoid reading twice, refactor to count both lines and tokens in a single pass:

```rust
/// Count non-blank lines and tokens in a file.
fn count_file_metrics(path: &Path) -> std::io::Result<(usize, usize)> {
    let content = std::fs::read(path)?;
    let text = String::from_utf8(content)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());

    let lines = text.lines().filter(|l| !l.trim().is_empty()).count();
    let tokens = text.chars().count() / 4;

    Ok((lines, tokens))
}
```

## Verification Plan

### Unit Tests (`crates/cli/src/checks/cloc_tests.rs`)

Add tests for:

1. `count_tokens_empty_file` - returns 0
2. `count_tokens_short_content` - < 4 chars returns 0
3. `count_tokens_exact_math` - 100 chars = 25 tokens
4. `count_tokens_unicode` - counts Unicode chars correctly (not bytes)
5. `count_file_metrics_combined` - returns both lines and tokens

### Config Tests (`crates/cli/src/config_tests.rs`)

Add tests for:

1. `parse_max_tokens_default` - 20000 when not specified
2. `parse_max_tokens_custom` - respects custom value
3. `parse_max_tokens_false` - None when `max_tokens = false`

### Behavioral Specs (`tests/specs/cloc.rs`)

Remove `#[ignore]` from:
1. `cloc_fails_on_file_over_max_tokens` (Phase 110)

Add new spec:
2. `cloc_json_includes_token_metrics`

### Manual Verification

```bash
# Run unit tests
cargo test -p quench-cli cloc
cargo test -p quench-cli config

# Run behavioral specs
cargo test --test specs cloc

# Test on fixture
cargo run -- check --cloc -o json tests/fixtures/cloc/high-tokens | jq .

# Test on real project
cargo run -- check --cloc -o json | jq '.checks[] | select(.name == "cloc") | .metrics'
```

### Checklist Before Commit

- [ ] Unit tests for token counting in `cloc_tests.rs`
- [ ] Config tests for `max_tokens` in `config_tests.rs`
- [ ] `make check` passes (fmt, clippy, test, build, bootstrap, audit, deny)
- [ ] All Phase 110 specs pass (no `#[ignore]` remaining for this phase)
- [ ] Commit message lists passing specs

## Summary

| Task | Status |
|------|--------|
| File size limit checking (max_lines) | Done (Phase 105) |
| Test file size limit checking (max_lines_test) | Done (Phase 105) |
| Token counting (chars / 4) | **This phase** |
| Token limit checking (max_tokens) | **This phase** |
| Per-file violation generation | Done (Phase 105) |
| Exclude patterns for size limits | Done (Phase 105) |
| Per-package LOC breakdown | Done (Phase 105) |
| JSON output with metrics and by_package | Done (Phase 105) |
| Token metrics in JSON output | **This phase** |
