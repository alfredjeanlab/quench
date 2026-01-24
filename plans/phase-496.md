# Phase 496: JavaScript Adapter - Suppress Detection

**Root Feature:** `quench-5c10`

## Overview

Implement lint suppression detection for JavaScript/TypeScript files, supporting both ESLint and Biome directive patterns. This enables enforcement of suppression policies (forbid/comment/allow) with per-rule configuration and separate source vs test file handling.

**Scope:**
- ESLint: `eslint-disable-next-line`, `eslint-disable` blocks
- Biome: `biome-ignore` directives
- Configurable check levels with allow/forbid lists
- Source vs test file policy separation

## Project Structure

```
crates/cli/src/
├── adapter/
│   ├── common/
│   │   └── suppress.rs              # Existing shared utilities
│   └── javascript/
│       ├── mod.rs                   # Update: export suppress module
│       ├── suppress.rs              # NEW: ESLint/Biome parsing
│       └── suppress_tests.rs        # NEW: Unit tests
├── checks/
│   └── escapes/
│       ├── mod.rs                   # Update: call JS suppress checker
│       └── javascript_suppress.rs   # NEW: Violation checking
└── config/
    └── javascript.rs                # Existing: SuppressConfig already defined

tests/
├── specs/
│   └── adapters/
│       └── javascript.rs            # Update: remove #[ignore] from suppress tests
└── fixtures/
    └── javascript/
        ├── eslint-disable-fail/     # Existing fixture
        ├── eslint-disable-ok/       # Existing fixture
        ├── biome-ignore-fail/       # Existing fixture
        ├── biome-ignore-ok/         # Existing fixture
        └── eslint-test-ok/          # Existing fixture
```

## Dependencies

No new external dependencies required. Uses existing:
- `regex` - Pattern matching for directives
- `lazy_static` - Compiled regex caching

## Implementation Phases

### Phase 1: ESLint Directive Parsing

**Goal:** Parse ESLint suppress directives from source files.

**Files:**
- `crates/cli/src/adapter/javascript/suppress.rs` (new)
- `crates/cli/src/adapter/javascript/suppress_tests.rs` (new)
- `crates/cli/src/adapter/javascript/mod.rs` (update)

**Structures:**

```rust
/// Represents a parsed ESLint suppress directive
pub struct EslintSuppress {
    pub line: usize,
    pub kind: EslintSuppressKind,
    pub codes: Vec<String>,      // Empty = all rules
    pub has_comment: bool,
    pub comment_text: Option<String>,
}

pub enum EslintSuppressKind {
    DisableNextLine,  // eslint-disable-next-line
    DisableBlock,     // /* eslint-disable */ ... /* eslint-enable */
    DisableFile,      // /* eslint-disable */ at file top (no enable)
}
```

**Parsing patterns:**

```rust
// Single-line: // eslint-disable-next-line [rules] [-- reason]
static ref ESLINT_NEXT_LINE: Regex = Regex::new(
    r"//\s*eslint-disable-next-line(?:\s+([^\n]+))?"
).unwrap();

// Block start: /* eslint-disable [rules] */
static ref ESLINT_DISABLE_BLOCK: Regex = Regex::new(
    r"/\*\s*eslint-disable(?:\s+([^*]+))?\s*\*/"
).unwrap();

// Block end: /* eslint-enable [rules] */
static ref ESLINT_ENABLE_BLOCK: Regex = Regex::new(
    r"/\*\s*eslint-enable(?:\s+([^*]+))?\s*\*/"
).unwrap();
```

**Functions:**

```rust
pub fn parse_eslint_suppresses(
    content: &str,
    comment_pattern: Option<&str>,
) -> Vec<EslintSuppress>
```

**Milestone:** Unit tests pass for ESLint directive parsing.

---

### Phase 2: Biome Directive Parsing

**Goal:** Parse Biome suppress directives from source files.

**Files:**
- `crates/cli/src/adapter/javascript/suppress.rs` (update)
- `crates/cli/src/adapter/javascript/suppress_tests.rs` (update)

**Structures:**

```rust
/// Represents a parsed Biome suppress directive
pub struct BiomeSuppress {
    pub line: usize,
    pub codes: Vec<String>,
    pub has_explanation: bool,    // Biome requires colon-separated explanation
    pub explanation_text: Option<String>,
    pub has_comment: bool,        // Additional comment above (for custom patterns)
    pub comment_text: Option<String>,
}
```

**Parsing patterns:**

```rust
// biome-ignore lint/category/rule: explanation
// biome-ignore lint/a lint/b: explanation for both
static ref BIOME_IGNORE: Regex = Regex::new(
    r"//\s*biome-ignore\s+((?:lint/\S+\s*)+)(?::\s*(.+))?"
).unwrap();
```

**Key difference from ESLint:** Biome requires explanations after the colon. An empty or missing explanation fails the `has_explanation` check.

**Functions:**

```rust
pub fn parse_biome_suppresses(
    content: &str,
    comment_pattern: Option<&str>,
) -> Vec<BiomeSuppress>
```

**Milestone:** Unit tests pass for Biome directive parsing.

---

### Phase 3: Unified JavaScript Suppress Types

**Goal:** Create a unified type for checking both ESLint and Biome suppressions.

**Files:**
- `crates/cli/src/adapter/javascript/suppress.rs` (update)

**Structures:**

```rust
/// Unified suppress directive for violation checking
pub struct JavaScriptSuppress {
    pub line: usize,
    pub tool: SuppressTool,
    pub codes: Vec<String>,
    pub has_comment: bool,
    pub comment_text: Option<String>,
}

pub enum SuppressTool {
    Eslint,
    Biome,
}

impl From<EslintSuppress> for JavaScriptSuppress { ... }
impl From<BiomeSuppress> for JavaScriptSuppress { ... }
```

**Functions:**

```rust
/// Parse all JavaScript suppress directives (ESLint + Biome)
pub fn parse_javascript_suppresses(
    content: &str,
    comment_pattern: Option<&str>,
) -> Vec<JavaScriptSuppress>
```

**Milestone:** Unified parsing with correct tool attribution.

---

### Phase 4: Violation Checking Logic

**Goal:** Implement suppress policy checking with allow/forbid lists.

**Files:**
- `crates/cli/src/checks/escapes/javascript_suppress.rs` (new)
- `crates/cli/src/checks/escapes/mod.rs` (update)

**Algorithm:**

```rust
pub fn check_javascript_suppress_violations(
    ctx: &CheckContext,
    path: &Path,
    content: &str,
    config: &SuppressConfig,
    is_test_file: bool,
    limit_reached: &mut bool,
) -> Vec<Violation> {
    // 1. Determine scope config (source vs test)
    let scope = if is_test_file { &config.test } else { &config.source };
    let level = scope.check.unwrap_or(config.check);

    // 2. Early return if allow-all
    if level == SuppressLevel::Allow {
        return vec![];
    }

    // 3. Parse all suppress directives
    let suppresses = parse_javascript_suppresses(content, config.comment.as_deref());

    // 4. Check each suppress against policy
    for suppress in suppresses {
        // Check forbid list first
        for code in &suppress.codes {
            if scope.forbid.contains(code) {
                violations.push(Violation::forbidden(code, suppress.line));
                continue;
            }
        }

        // Check allow list (any matching code skips comment check)
        if suppress.codes.iter().any(|c| scope.allow.contains(c)) {
            continue;
        }

        // Check level requirements
        match level {
            SuppressLevel::Forbid => {
                violations.push(Violation::all_forbidden(suppress.line));
            }
            SuppressLevel::Comment => {
                if !suppress.has_comment {
                    violations.push(Violation::missing_comment(suppress.line));
                }
            }
            SuppressLevel::Allow => unreachable!(),
        }
    }

    violations
}
```

**Milestone:** Violation checking works with configuration.

---

### Phase 5: Integration and Source/Test Separation

**Goal:** Integrate suppress checking into the escapes check with proper file classification.

**Files:**
- `crates/cli/src/checks/escapes/mod.rs` (update)
- `crates/cli/src/adapter/javascript/mod.rs` (update)

**Integration point in `escapes/mod.rs`:**

```rust
// In check_escapes function, after existing language-specific checks:
if adapter.name() == "javascript" {
    let is_test = adapter.classify(path) == FileKind::Test;
    let js_violations = check_javascript_suppress_violations(
        ctx, path, content, &config.javascript.suppress, is_test, &mut limit_reached
    );
    violations.extend(js_violations);
}
```

**Test file classification** uses existing `JavaScriptAdapter::classify()` which checks against configured test patterns.

**Milestone:** Full integration with source/test separation working.

---

### Phase 6: Behavioral Tests and Polish

**Goal:** Enable spec tests and verify end-to-end behavior.

**Files:**
- `tests/specs/adapters/javascript.rs` (update - remove `#[ignore]`)
- `tests/fixtures/javascript/*` (verify existing fixtures)

**Tests to enable:**

```rust
// Remove #[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
fn eslint_disable_without_comment_fails_when_comment_required()
fn eslint_disable_with_comment_passes()
fn eslint_disable_next_line_with_comment_passes()
fn biome_ignore_without_explanation_fails()
fn biome_ignore_with_explanation_passes()
fn eslint_disable_in_test_file_passes_without_comment()
```

**Milestone:** All spec tests pass, `make check` succeeds.

## Key Implementation Details

### ESLint Inline Reason Parsing

ESLint supports inline reasons with `--`:

```typescript
// eslint-disable-next-line no-magic-numbers -- pagination constants
const PAGE_SIZE = 20;
```

The parser should:
1. Check for `-- reason` suffix in the directive
2. If present, extract as `comment_text` and set `has_comment = true`
3. If not present, fall back to checking the line above

### Biome Explanation vs Comment

Biome has two comment sources:
1. **Explanation** (after colon): `// biome-ignore lint/rule: this is required`
2. **Comment above** (for custom patterns): Same as other adapters

When `config.comment` requires a specific pattern:
- First check explanation text
- Then check comment above
- Either matching satisfies the requirement

### Empty Codes Handling

When codes list is empty (e.g., `// eslint-disable-next-line` with no rules):
- Treat as "all rules" suppression
- Still subject to forbid/allow/comment policies
- Cannot match specific allow/forbid entries (no codes to compare)

### Comment Style for JavaScript

```rust
const JS_COMMENT_STYLE: CommentStyle = CommentStyle {
    prefix: "//",
    directive_patterns: &[
        "eslint-disable",
        "eslint-enable",
        "biome-ignore",
        "@ts-ignore",
        "@ts-expect-error",
    ],
};
```

## Verification Plan

### Unit Tests (`suppress_tests.rs`)

1. **ESLint parsing:**
   - `eslint-disable-next-line` with no rules
   - `eslint-disable-next-line` with single rule
   - `eslint-disable-next-line` with multiple comma-separated rules
   - `eslint-disable-next-line` with `-- reason` inline
   - `/* eslint-disable */` block start
   - `/* eslint-enable */` block end
   - Block with rules specified
   - Comment above directive detection

2. **Biome parsing:**
   - `biome-ignore` with single rule
   - `biome-ignore` with multiple space-separated rules
   - `biome-ignore` with explanation after colon
   - `biome-ignore` without explanation (empty after colon)
   - `biome-ignore` with no colon at all
   - Comment above directive detection

3. **Violation checking:**
   - Forbid list blocks specific codes
   - Allow list skips comment requirement
   - Comment check level enforces comments
   - Forbid check level blocks all suppressions
   - Allow check level permits everything

### Behavioral Tests (spec tests)

Use existing fixtures to verify:
- `eslint-disable-fail/` → Violation when comment required but missing
- `eslint-disable-ok/` → Pass when comment present
- `biome-ignore-fail/` → Violation when explanation missing
- `biome-ignore-ok/` → Pass when explanation present
- `eslint-test-ok/` → Pass in test files with `check = "allow"`

### Integration Verification

```bash
# Run full check suite
make check

# Specific test commands
cargo test --package quench -- javascript
cargo test --package quench -- suppress
```
