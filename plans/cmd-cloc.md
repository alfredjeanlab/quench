# Plan: `quench cloc` Command

## Status

**TODO**

## Overview

Add a standalone `quench cloc` subcommand that produces a `cloc`-like report with columns for **files**, **blank**, **comment**, and **code** — split by both **language** and **source vs test**. Unlike the traditional unix `cloc` tool, each language row is doubled: one for source files, one for test files (e.g. "Rust (source)", "Rust (tests)").

The command reuses the existing walker, exclude/ignore patterns, adapter registry, and file classification infrastructure from `quench check --cloc`, ensuring parity in which files are counted and how they're categorized.

## Project Structure

Files to create or modify:

```
crates/cli/src/
├── cli.rs                      # Add Cloc variant to Command enum + ClocArgs
├── main.rs                     # Add dispatch for Command::Cloc
├── cmd_cloc.rs                 # NEW: cloc command implementation
├── cloc/
│   ├── mod.rs                  # NEW: cloc counting logic (reusable)
│   ├── mod_tests.rs            # NEW: unit tests for counting
│   ├── comment.rs              # NEW: comment detection per language
│   └── comment_tests.rs        # NEW: comment detection tests
├── checks/cloc.rs              # Refactor: reuse shared counting from cloc/
└── lib.rs                      # pub mod cloc
tests/
└── specs/
    └── cloc.rs                 # NEW: behavioral tests
```

## Dependencies

No new external crates. Uses existing:
- `globset` — pattern matching
- `ignore` (via walker) — gitignore support
- Adapters from `crate::adapter` — language detection and file classification

## Implementation Phases

### Phase 1: Comment Detection Module

**Goal:** Add per-language comment line detection so we can split lines into blank / comment / code.

Create `crates/cli/src/cloc/comment.rs` with a `CommentStyle` struct and a function that maps file extensions to comment syntax:

```rust
/// Comment syntax for a language.
pub struct CommentStyle {
    /// Single-line comment prefixes (e.g. ["//", "#"])
    pub line: &'static [&'static str],
    /// Block comment delimiters (open, close) pairs
    pub block: &'static [(&'static str, &'static str)],
}

/// Return the comment style for a file extension.
pub fn comment_style(ext: &str) -> Option<CommentStyle> {
    match ext {
        "rs" => Some(CommentStyle {
            line: &["//"],
            block: &[("/*", "*/")],
        }),
        "go" => Some(CommentStyle {
            line: &["//"],
            block: &[("/*", "*/")],
        }),
        "py" | "rb" | "sh" | "bash" | "zsh" | "fish" | "bats" | "r" => Some(CommentStyle {
            line: &["#"],
            block: &[],
        }),
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "mts" | "cjs" | "cts"
        | "java" | "kt" | "scala" | "swift" | "c" | "cpp" | "h" | "hpp"
        | "cs" | "m" | "mm" => Some(CommentStyle {
            line: &["//"],
            block: &[("/*", "*/")],
        }),
        "lua" => Some(CommentStyle {
            line: &["--"],
            block: &[("--[[", "]]")],
        }),
        "sql" => Some(CommentStyle {
            line: &["--"],
            block: &[("/*", "*/")],
        }),
        "php" => Some(CommentStyle {
            line: &["//", "#"],
            block: &[("/*", "*/")],
        }),
        "vue" | "svelte" => Some(CommentStyle {
            line: &["//"],
            block: &[("/*", "*/"), ("<!--", "-->")],
        }),
        "pl" | "pm" => Some(CommentStyle {
            line: &["#"],
            block: &[("=pod", "=cut")],
        }),
        "ps1" => Some(CommentStyle {
            line: &["#"],
            block: &[("<#", "#>")],
        }),
        "bat" | "cmd" => Some(CommentStyle {
            line: &["REM ", ":: "],
            block: &[],
        }),
        _ => None,
    }
}
```

Add a `count_lines` function that takes file content and a `CommentStyle` and returns `(blank, comment, code)` counts. This uses a simple state machine: track whether we're inside a block comment, and for each line classify it as blank → comment → code (first match wins, consistent with how `cloc` works).

```rust
pub struct LineCounts {
    pub blank: usize,
    pub comment: usize,
    pub code: usize,
}

pub fn count_lines(content: &str, style: &CommentStyle) -> LineCounts { ... }
```

**Milestone:** `count_lines` unit tests pass for Rust, Python, Go, and mixed block-comment cases.

### Phase 2: Shared Counting Module

**Goal:** Extract file-reading and metrics logic into `crates/cli/src/cloc/mod.rs` so it can be reused by both `quench cloc` and `quench check --cloc`.

Create `crates/cli/src/cloc/mod.rs`:

```rust
pub mod comment;

/// Metrics for a single file (superset of existing FileMetrics).
pub struct FileMetrics {
    pub lines: usize,        // total lines (wc -l)
    pub blank: usize,        // blank lines
    pub comment: usize,      // comment lines
    pub code: usize,         // code lines (= lines - blank - comment)
    pub tokens: usize,       // chars/4
}

/// Count metrics from file content.
pub fn count_file_metrics(content: &str, ext: &str) -> FileMetrics { ... }
```

The function calls `comment::comment_style(ext)` to get the style, then `comment::count_lines(content, style)` for the breakdown. If no comment style is known for the extension, all non-blank lines are counted as code (matching `cloc` behavior for unknown languages).

Update `checks/cloc.rs` to call into `cloc::count_file_metrics` instead of its private `count_file_metrics`, keeping the check's existing behavior unchanged. The check only uses `lines`, `nonblank_lines` (which equals `code + comment`), and `tokens`, so we map accordingly.

**Milestone:** `make check` passes — existing check behavior unchanged, new module compiles.

### Phase 3: CLI Plumbing

**Goal:** Add the `quench cloc` subcommand with argument parsing and dispatch.

1. Add `Cloc(ClocArgs)` variant to `Command` enum in `cli.rs`:

```rust
/// Count lines of code by language
Cloc(ClocArgs),
```

2. Define `ClocArgs`:

```rust
#[derive(clap::Args)]
pub struct ClocArgs {
    /// Files or directories to count
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Maximum directory depth to traverse
    #[arg(long, default_value_t = 100)]
    pub max_depth: usize,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub output: OutputFormat,
}
```

3. Add dispatch in `main.rs`:

```rust
Some(Command::Cloc(args)) => cmd_cloc::run(args),
```

4. Add help text handling for the new subcommand.

**Milestone:** `quench cloc --help` prints usage. `quench cloc` runs (with placeholder output).

### Phase 4: Command Implementation

**Goal:** Implement `cmd_cloc.rs` — walk files, count metrics per language/kind, and output the report.

The command:
1. Loads config (same `discovery::find_config` + `config::load_with_warnings` path)
2. Detects language and builds exclude patterns (same logic as `cmd_check.rs` lines 96–252)
3. Creates `FileWalker` with those patterns
4. Builds `AdapterRegistry::for_project_with_config` for file classification
5. For each text file:
   - Reads content
   - Determines language from extension (using a `language_name(ext)` mapping)
   - Classifies as Source or Test via `registry.classify()`
   - For Rust source files with `cfg_test_split = Count`, uses the cfg_test splitter to attribute lines
   - Counts blank/comment/code using `cloc::count_file_metrics`
   - Accumulates into `HashMap<(language, kind), Totals>`
6. Sorts and prints the report

**Accumulator struct:**

```rust
#[derive(Default)]
struct LangStats {
    files: usize,
    blank: usize,
    comment: usize,
    code: usize,
}
```

**Key:** `(String /* language name */, FileKind /* Source or Test */)`

For the language name, map file extensions to human-readable names:
- `rs` → "Rust", `go` → "Go", `py` → "Python", `js`/`jsx` → "JavaScript", `ts`/`tsx` → "TypeScript", `sh`/`bash` → "Shell", etc.
- Group JS and JSX as "JavaScript"; TS and TSX as "TypeScript" (separate from JS).
- Extensions not recognized by any adapter use the extension itself uppercased.

**Milestone:** `quench cloc` produces a complete report for a real project.

### Phase 5: Output Formatting

**Goal:** Match the classic `cloc` table format, adapted for source/test split.

Text output format:

```
───────────────────────────────────────────────────────────
Language                 files      blank    comment       code
───────────────────────────────────────────────────────────
Rust (source)               42        580        320       4200
Rust (tests)                18        120         45       1800
Python (source)              3         40         25        280
Python (tests)               2         15          8        110
Shell (source)               5         30         10        150
───────────────────────────────────────────────────────────
Source total                50        650        355       4630
Test total                  20        135         53       1910
───────────────────────────────────────────────────────────
Total                       70        785        408       6540
───────────────────────────────────────────────────────────
```

Rules:
- Rows sorted by code descending (source rows first per language, then test rows)
- Omit rows with zero files
- Summary section shows source total, test total, and grand total
- JSON output (`--output json`) emits a structured object with the same data

**Milestone:** Output matches the format above. JSON output is valid and contains all data.

### Phase 6: Tests and Polish

**Goal:** Unit and behavioral tests, edge cases.

1. **Unit tests** in `cloc/comment_tests.rs`:
   - Blank/comment/code counting for each supported comment syntax
   - Block comments spanning multiple lines
   - Mixed line+block comments
   - Strings containing comment-like syntax (best-effort — not a full parser, same as `cloc`)

2. **Unit tests** in `cloc/mod_tests.rs`:
   - `count_file_metrics` with known content
   - Unknown extension falls back to all-code

3. **Behavioral tests** in `tests/specs/cloc.rs`:
   - `quench cloc` on a fixture project produces expected output
   - `--output json` produces valid JSON
   - Respects `project.exclude` from config
   - Respects `.gitignore`
   - Source/test split matches adapter classification

4. Polish:
   - Ensure Rust `cfg_test` split mode is respected (source lines from a file with inline `#[cfg(test)]` go to source, test lines go to test)
   - Verify exclude patterns from `check.cloc.exclude` are respected

**Milestone:** All tests pass. `make check` green.

## Key Implementation Details

### Comment counting is best-effort

Like `cloc` itself, comment detection uses simple line-by-line heuristics rather than full parsing. A line inside a block comment that also contains code after the closing delimiter is counted as code (first-wins: if the line has `*/` followed by code, it's code). This is consistent with `cloc`'s behavior and avoids the complexity of a full lexer per language.

### Language name mapping vs adapter name

Adapter names are lowercase identifiers (`"rust"`, `"go"`, `"javascript"`). For the cloc report, we use human-readable names ("Rust", "Go", "JavaScript") derived from the file extension. The adapter is used only for Source/Test classification. The language name comes from a simple `ext → display name` lookup (e.g. `"rs" → "Rust"`, `"ts" → "TypeScript"`, `"tsx" → "TypeScript"`).

This means TypeScript and JavaScript appear as separate rows even though they share the `"javascript"` adapter. This is the expected behavior — users want to see their TS and JS counts separately, just like `cloc` does.

### Shared infrastructure with check command

The `cmd_cloc` command replicates the same config loading, language detection, exclude pattern building, and walker setup as `cmd_check`. This is intentional duplication for now — the logic is straightforward and extracting a shared "project setup" helper would add abstraction without clear benefit until there are more commands sharing this pattern.

### No cache interaction

The `cloc` command does not read or write the check cache. It's a pure counting tool with no violations, ratcheting, or baselines. This keeps the command simple and always-fresh.

### CACHE_VERSION unchanged

This feature does not change any check logic. The existing `cloc` check in `checks/cloc.rs` continues to use `nonblank_lines` (code + comment) for its thresholds, which is unchanged. If Phase 2's refactor changes `checks/cloc.rs` to call into `cloc::count_file_metrics`, the returned values must preserve identical behavior — no CACHE_VERSION bump needed.

## Verification Plan

### Unit tests

- `cloc/comment_tests.rs`: Comment detection for all supported syntax families
- `cloc/mod_tests.rs`: File metrics counting end-to-end

### Behavioral tests

- `tests/specs/cloc.rs`: Command output format, JSON mode, exclude/ignore parity

### Manual verification

```bash
# Basic usage
quench cloc

# JSON output
quench cloc --output json

# Specific directory
quench cloc crates/cli

# Compare with system cloc (total lines should be similar,
# but quench splits source/test)
cloc crates/
quench cloc crates/
```
