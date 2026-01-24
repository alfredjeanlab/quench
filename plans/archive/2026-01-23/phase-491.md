# Phase 491: JavaScript Adapter - Specs

**Plan:** `phase-491`
**Root Feature:** `quench-35f2`
**Blocked By:** Phase 201 (Generic Language Adapter)

## Overview

Write behavioral specs for JavaScript/TypeScript language detection and escape patterns. These specs define the expected behavior of the JavaScript adapter before implementation, following the spec-first development approach.

The specs will cover:
- Auto-detection via package.json/tsconfig.json/jsconfig.json
- Default source patterns for JS/TS files
- Default test patterns (*.test.*, *.spec.*, __tests__/)
- Default ignores (node_modules/, dist/, build/)
- Workspace detection (npm/yarn/pnpm workspaces)
- TypeScript escape patterns (`as unknown`, `@ts-ignore`)
- Lint suppress detection (eslint-disable, biome-ignore)

Reference: `docs/specs/langs/javascript.md`

## Project Structure

Files to create/modify:

```
tests/
├── specs/
│   └── adapters/
│       ├── mod.rs              # Add: pub mod javascript;
│       └── javascript.rs       # NEW: ~250 lines of behavioral specs
└── fixtures/
    └── javascript/             # NEW: Test fixtures
        ├── auto-detect/        # package.json only
        ├── tsconfig-detect/    # tsconfig.json detection
        ├── jsconfig-detect/    # jsconfig.json detection
        ├── default-patterns/   # Verify source/test patterns
        ├── node-modules-ignore/ # Verify node_modules ignored
        ├── workspace-npm/      # npm workspaces detection
        ├── workspace-pnpm/     # pnpm workspaces detection
        ├── as-unknown-fail/    # as unknown without // CAST:
        ├── as-unknown-ok/      # as unknown with // CAST:
        ├── ts-ignore-fail/     # @ts-ignore in source
        ├── ts-ignore-test-ok/  # @ts-ignore in test (allowed)
        ├── eslint-disable-fail/ # eslint-disable without comment
        ├── eslint-disable-ok/  # eslint-disable with comment
        ├── biome-ignore-fail/  # biome-ignore without explanation
        └── biome-ignore-ok/    # biome-ignore with explanation
```

## Dependencies

- Existing spec infrastructure (`tests/specs/prelude.rs`)
- Go/Rust adapter specs as reference patterns
- `docs/specs/langs/javascript.md` as source of truth
- Phase 201 Generic Language Adapter (for implementation, not specs)

## Implementation Phases

### Phase 1: Create Fixture Directory Structure

Create minimal fixture projects for JavaScript adapter testing.

**Fixtures to create:**

#### `javascript/auto-detect/`
```
package.json
src/index.js
src/utils.js
```

```json
// package.json
{
  "name": "test-project",
  "version": "1.0.0"
}
```

```javascript
// src/index.js
export function main() {
  return 'hello';
}
```

#### `javascript/tsconfig-detect/`
```
tsconfig.json
src/index.ts
```

```json
// tsconfig.json
{
  "compilerOptions": {
    "target": "ES2020"
  }
}
```

#### `javascript/jsconfig-detect/`
```
jsconfig.json
src/index.js
```

#### `javascript/default-patterns/`
```
package.json
src/app.ts
src/app.test.ts
src/utils.spec.js
src/__tests__/helpers.ts
tests/integration.test.ts
```

#### `javascript/node-modules-ignore/`
```
package.json
src/index.js
node_modules/lodash/index.js  # Should be ignored
dist/bundle.js                 # Should be ignored
build/output.js               # Should be ignored
```

#### `javascript/workspace-npm/`
```
package.json        # with "workspaces": ["packages/*"]
packages/core/package.json
packages/core/src/index.ts
packages/cli/package.json
packages/cli/src/main.ts
```

#### `javascript/workspace-pnpm/`
```
package.json
pnpm-workspace.yaml
packages/lib/package.json
packages/lib/src/index.ts
```

**Verification:**
- [ ] All fixtures have valid structure
- [ ] package.json files are valid JSON
- [ ] tsconfig.json/jsconfig.json files are valid

### Phase 2: Write Auto-Detection Specs

Create `tests/specs/adapters/javascript.rs` with detection specs.

```rust
//! Behavioral specs for the JavaScript/TypeScript language adapter.
//!
//! Tests that quench correctly:
//! - Detects JS/TS projects via package.json, tsconfig.json, jsconfig.json
//! - Applies default source/test patterns
//! - Ignores node_modules, dist, build directories
//! - Applies JS/TS-specific escape patterns
//!
//! Reference: docs/specs/langs/javascript.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// AUTO-DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#detection
///
/// > Detected when any of these exist in project root:
/// > - `package.json`
#[test]
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
fn auto_detected_when_package_json_present() {
    let result = cli().on("javascript/auto-detect").json().passes();
    let checks = result.checks();

    // escapes check should have JS-specific patterns active
    assert!(
        checks.iter().any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/langs/javascript.md#detection
///
/// > - `tsconfig.json`
#[test]
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
fn auto_detected_when_tsconfig_json_present() {
    let result = cli().on("javascript/tsconfig-detect").json().passes();
    let checks = result.checks();

    assert!(
        checks.iter().any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/langs/javascript.md#detection
///
/// > - `jsconfig.json`
#[test]
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
fn auto_detected_when_jsconfig_json_present() {
    let result = cli().on("javascript/jsconfig-detect").json().passes();
    let checks = result.checks();

    assert!(
        checks.iter().any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}
```

**Verification:**
- [ ] Specs compile with `cargo test --test specs -- --ignored`
- [ ] All detection specs use `#[ignore = "TODO: Phase 493"]`

### Phase 3: Write Default Pattern Specs

Add source/test/ignore pattern specs.

```rust
// =============================================================================
// DEFAULT PATTERN SPECS
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#default-patterns
///
/// > source = ["**/*.js", "**/*.jsx", "**/*.ts", "**/*.tsx", "**/*.mjs", "**/*.mts"]
#[test]
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
fn default_source_pattern_matches_js_ts_files() {
    let cloc = check("cloc").on("javascript/default-patterns").json().passes();
    let metrics = cloc.require("metrics");

    let source_lines = metrics.get("source_lines").and_then(|v| v.as_u64()).unwrap_or(0);
    assert!(source_lines > 0, "should count .js/.ts files as source");
}

/// Spec: docs/specs/langs/javascript.md#default-patterns
///
/// > tests = [
/// >   "**/*.test.js", "**/*.test.ts", "**/*.test.jsx", "**/*.test.tsx",
/// >   "**/*.spec.js", "**/*.spec.ts", "**/*.spec.jsx", "**/*.spec.tsx",
/// >   "**/__tests__/**",
/// >   "test/**", "tests/**"
/// > ]
#[test]
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
fn default_test_pattern_matches_test_files() {
    let cloc = check("cloc").on("javascript/default-patterns").json().passes();
    let metrics = cloc.require("metrics");

    let test_lines = metrics.get("test_lines").and_then(|v| v.as_u64()).unwrap_or(0);
    assert!(test_lines > 0, "should count *.test.*, *.spec.*, __tests__/** as test");
}

/// Spec: docs/specs/langs/javascript.md#default-patterns
///
/// > ignore = ["node_modules/", "dist/", "build/", ".next/", "coverage/"]
#[test]
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
fn default_ignores_node_modules_directory() {
    let cloc = check("cloc").on("javascript/node-modules-ignore").json().passes();
    let metrics = cloc.require("metrics");

    // Only src/index.js should be counted, not node_modules or dist
    let source_lines = metrics.get("source_lines").and_then(|v| v.as_u64()).unwrap_or(0);
    assert!(source_lines < 50, "node_modules/, dist/, build/ should be ignored");
}
```

**Verification:**
- [ ] Pattern specs cover all default patterns from javascript.md
- [ ] Specs use appropriate fixtures

### Phase 4: Write Workspace Detection Specs

Add npm/pnpm workspace detection specs.

```rust
// =============================================================================
// WORKSPACE DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/langs/javascript.md (implied by Default Patterns)
///
/// > Detects workspaces from package.json or pnpm-workspace.yaml
#[test]
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
fn detects_npm_workspaces_from_package_json() {
    let cloc = check("cloc").on("javascript/workspace-npm").json().passes();
    let by_package = cloc.get("by_package");

    assert!(by_package.is_some(), "should have by_package breakdown");
    let by_package = by_package.unwrap();

    assert!(by_package.get("core").is_some(), "should detect 'core' package");
    assert!(by_package.get("cli").is_some(), "should detect 'cli' package");
}

/// Spec: docs/specs/langs/javascript.md (implied by Default Patterns)
///
/// > Detects pnpm workspaces from pnpm-workspace.yaml
#[test]
#[ignore = "TODO: Phase 493 - JavaScript Adapter Detection"]
fn detects_pnpm_workspaces() {
    let cloc = check("cloc").on("javascript/workspace-pnpm").json().passes();
    let by_package = cloc.get("by_package");

    assert!(by_package.is_some(), "should have by_package breakdown");
}
```

**Verification:**
- [ ] Workspace specs cover npm and pnpm patterns
- [ ] Fixtures demonstrate multi-package structure

### Phase 5: Write TypeScript Escape Pattern Specs

Add `as unknown` and `@ts-ignore` escape pattern specs.

```rust
// =============================================================================
// ESCAPE PATTERN SPECS - as unknown
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#default-escape-patterns
///
/// > `as unknown` requires `// CAST:` comment explaining why.
#[test]
#[ignore = "TODO: Phase 495 - JavaScript Adapter Escapes"]
fn as_unknown_without_cast_comment_fails() {
    check("escapes")
        .on("javascript/as-unknown-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("// CAST:");
}

/// Spec: docs/specs/langs/javascript.md#default-escape-patterns
///
/// > `as unknown` with `// CAST:` comment passes.
#[test]
#[ignore = "TODO: Phase 495 - JavaScript Adapter Escapes"]
fn as_unknown_with_cast_comment_passes() {
    check("escapes").on("javascript/as-unknown-ok").passes();
}

// =============================================================================
// ESCAPE PATTERN SPECS - @ts-ignore
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#default-escape-patterns
///
/// > `@ts-ignore` is forbidden in source code.
#[test]
#[ignore = "TODO: Phase 495 - JavaScript Adapter Escapes"]
fn ts_ignore_forbidden_in_source() {
    check("escapes")
        .on("javascript/ts-ignore-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("@ts-ignore")
        .stdout_has("forbidden");
}

/// Spec: docs/specs/langs/javascript.md#escapes-in-test-code
///
/// > Escape patterns are allowed in test code.
#[test]
#[ignore = "TODO: Phase 495 - JavaScript Adapter Escapes"]
fn ts_ignore_allowed_in_test_code() {
    check("escapes").on("javascript/ts-ignore-test-ok").passes();
}
```

**Verification:**
- [ ] Escape specs reference javascript.md sections
- [ ] Fixtures demonstrate both pass and fail cases

### Phase 6: Write Lint Suppress Specs

Add ESLint and Biome suppress directive specs.

```rust
// =============================================================================
// SUPPRESS DIRECTIVE SPECS - ESLint
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#suppress
///
/// > When `check = "comment"`, `eslint-disable` requires justification.
#[test]
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
fn eslint_disable_without_comment_fails_when_comment_required() {
    check("escapes")
        .on("javascript/eslint-disable-fail")
        .fails()
        .stdout_has("eslint-disable");
}

/// Spec: docs/specs/langs/javascript.md#suppress
///
/// > `eslint-disable` with justification comment passes.
#[test]
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
fn eslint_disable_with_comment_passes() {
    check("escapes").on("javascript/eslint-disable-ok").passes();
}

/// Spec: docs/specs/langs/javascript.md#supported-patterns
///
/// > eslint-disable-next-line no-unused-vars
#[test]
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
fn eslint_disable_next_line_with_comment_passes() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[javascript.suppress]
check = "comment"
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"name": "test", "version": "1.0.0"}"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/index.ts"),
        r#"
// Legacy code requires this pattern
// eslint-disable-next-line no-console
console.log('debug');
"#,
    )
    .unwrap();

    check("escapes").pwd(dir.path()).passes();
}

// =============================================================================
// SUPPRESS DIRECTIVE SPECS - Biome
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#supported-patterns
///
/// > biome-ignore lint/suspicious/noExplicitAny: explanation required
#[test]
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
fn biome_ignore_without_explanation_fails() {
    check("escapes")
        .on("javascript/biome-ignore-fail")
        .fails()
        .stdout_has("biome-ignore");
}

/// Spec: docs/specs/langs/javascript.md#supported-patterns
///
/// > biome-ignore with explanation passes
#[test]
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
fn biome_ignore_with_explanation_passes() {
    check("escapes").on("javascript/biome-ignore-ok").passes();
}

/// Spec: docs/specs/langs/javascript.md#suppress
///
/// > Default: "comment" for source, "allow" for test code.
#[test]
#[ignore = "TODO: Phase 496 - JavaScript Adapter Suppress"]
fn eslint_disable_in_test_file_passes_without_comment() {
    check("escapes").on("javascript/eslint-test-ok").passes();
}
```

**Verification:**
- [ ] Suppress specs cover ESLint and Biome patterns
- [ ] Test code exemption is verified

## Key Implementation Details

### Fixture Configuration Files

Each fixture needs a `quench.toml` to configure the expected behavior:

```toml
# javascript/as-unknown-fail/quench.toml
version = 1

[[check.escapes.patterns]]
name = "as_unknown"
pattern = "as unknown"
action = "comment"
comment = "// CAST:"
source = ["**/*.ts", "**/*.tsx"]
advice = "Add a // CAST: comment explaining why the type assertion is necessary."
```

```toml
# javascript/ts-ignore-fail/quench.toml
version = 1

[[check.escapes.patterns]]
name = "ts_ignore"
pattern = "@ts-ignore"
action = "forbid"
in_tests = "allow"
source = ["**/*.ts", "**/*.tsx"]
advice = "Use @ts-expect-error instead, which fails if the error is resolved."
```

```toml
# javascript/eslint-disable-fail/quench.toml
version = 1

[javascript.suppress]
check = "comment"
```

### TypeScript Source Examples

```typescript
// javascript/as-unknown-fail/src/index.ts
export function convert(data: string): number {
  return data as unknown as number;  // Missing // CAST: comment
}
```

```typescript
// javascript/as-unknown-ok/src/index.ts
// CAST: JSON parse guarantees numeric string
export function convert(data: string): number {
  return data as unknown as number;
}
```

```typescript
// javascript/ts-ignore-fail/src/index.ts
// @ts-ignore
const x: number = "string";  // Forbidden in source
```

```typescript
// javascript/ts-ignore-test-ok/src/index.test.ts
describe('test', () => {
  // @ts-ignore - testing error case
  const invalid: number = "string";
});
```

### ESLint/Biome Examples

```typescript
// javascript/eslint-disable-fail/src/index.ts
// eslint-disable-next-line no-console
console.log('debug');  // Missing justification comment
```

```typescript
// javascript/eslint-disable-ok/src/index.ts
// Legacy API requires console output
// eslint-disable-next-line no-console
console.log('debug');
```

```typescript
// javascript/biome-ignore-fail/src/index.ts
// biome-ignore lint/suspicious/noExplicitAny
function legacy(data: any) {}  // Missing explanation after colon
```

```typescript
// javascript/biome-ignore-ok/src/index.ts
// biome-ignore lint/suspicious/noExplicitAny: legacy API requires any
function legacy(data: any) {}
```

### Spec Organization

The spec file should be organized by category with clear section headers:

1. **AUTO-DETECTION SPECS** - package.json, tsconfig.json, jsconfig.json
2. **DEFAULT PATTERN SPECS** - source, test, ignore patterns
3. **WORKSPACE DETECTION SPECS** - npm, pnpm workspaces
4. **ESCAPE PATTERN SPECS** - `as unknown`, `@ts-ignore`
5. **SUPPRESS DIRECTIVE SPECS** - eslint-disable, biome-ignore
6. **LINT CONFIG POLICY SPECS** - standalone policy for lint config changes

### Module Registration

Update `tests/specs/adapters/mod.rs`:

```rust
//! Behavioral specs for language adapters.

pub mod golang;
pub mod javascript;  // NEW
pub mod rust;
pub mod shell;
```

## Verification Plan

### Phase Completion Checklist

1. **Fixture Creation** (Phase 1)
   - [ ] Run `ls tests/fixtures/javascript/` - all directories exist
   - [ ] Validate JSON files: `jq . tests/fixtures/javascript/*/package.json`
   - [ ] Each fixture has quench.toml if needed

2. **Spec Compilation** (Phases 2-6)
   - [ ] Run `cargo test --test specs -- javascript --ignored` - compiles without errors
   - [ ] All specs have `#[ignore = "TODO: Phase NNN"]` annotations
   - [ ] Doc comments reference `docs/specs/langs/javascript.md`

3. **Spec Coverage**
   - [ ] Detection: 3 specs (package.json, tsconfig.json, jsconfig.json)
   - [ ] Patterns: 3 specs (source, test, ignore)
   - [ ] Workspaces: 2 specs (npm, pnpm)
   - [ ] Escapes: 4 specs (as unknown fail/ok, ts-ignore fail/test-ok)
   - [ ] Suppress: 6+ specs (eslint-disable, biome-ignore, test exemption)
   - [ ] **Total: ~18+ behavioral specs**

4. **Roadmap Alignment**
   - [ ] All items from `plans/.2-roadmap-javascript.md` Phase 491 covered:
     - [ ] auto-detected when package.json, tsconfig.json, or jsconfig.json present
     - [ ] default source patterns **/*.js, **/*.ts, **/*.jsx, **/*.tsx, **/*.mjs, **/*.mts
     - [ ] default test patterns **/*.test.*, **/*.spec.*, **/__tests__/**
     - [ ] default ignores node_modules/, dist/, build/, .next/, coverage/
     - [ ] detects package name from package.json
     - [ ] detects workspaces from package.json or pnpm-workspace.yaml
     - [ ] `as unknown` without // CAST: comment fails
     - [ ] `@ts-ignore` forbidden in source code
     - [ ] eslint-disable without comment fails (when configured)
     - [ ] biome-ignore without explanation fails (when configured)
     - [ ] lint config changes with source changes fails standalone policy

5. **Code Quality**
   - [ ] Run `cargo fmt --all`
   - [ ] Run `cargo clippy --all-targets`
   - [ ] No warnings in spec code

### Final Verification Commands

```bash
# Verify fixtures exist
ls -la tests/fixtures/javascript/

# Verify specs compile
cargo test --test specs -- javascript --ignored 2>&1 | head -20

# Count specs
grep -c '#\[test\]' tests/specs/adapters/javascript.rs

# Verify module registration
grep 'pub mod javascript' tests/specs/adapters/mod.rs
```
