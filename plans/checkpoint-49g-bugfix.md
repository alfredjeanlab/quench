# Checkpoint 49g: JavaScript Adapter Bug Fixes

**Root Feature:** `quench-f592`

## Overview

Fix bugs and gaps from prior JavaScript adapter refactors (checkpoint-49f-quickwins). The recent refactor added `.cjs`/`.cts` extension support and consolidated suppress logic, but left documentation and test coverage out of sync with the implementation.

**Key Issues Identified:**
1. **Documentation mismatch**: Spec doc patterns don't include new `.cjs`/`.cts` extensions
2. **Missing test coverage**: No fixtures verify `.cjs`/`.cts` file handling
3. **Policy test incomplete**: Classifier doesn't recognize all JS/TS extensions
4. **Spec docstrings outdated**: Test docstrings reference old pattern lists

## Project Structure

```
docs/specs/langs/
└── javascript.md              # Update: add .cjs/.cts to patterns

tests/
├── fixtures/javascript/
│   └── default-patterns/      # Update: add .cjs/.cts test files
├── specs/adapters/
│   └── javascript.rs          # Update: fix docstrings

crates/cli/src/
├── adapter/javascript/
│   ├── mod.rs                 # (OK - already has .cjs/.cts)
│   └── policy_tests.rs        # Update: fix js_classifier
└── config/
    └── javascript.rs          # (OK - already has .cjs/.cts)
```

## Dependencies

No new dependencies required.

## Implementation Phases

### Phase 1: Documentation Sync

**Goal**: Update spec documentation to match implementation.

**Files**:
- `docs/specs/langs/javascript.md`

**Changes**:

1. Update `Default Patterns` section (line 54):
```toml
[javascript]
source = ["**/*.js", "**/*.jsx", "**/*.ts", "**/*.tsx", "**/*.mjs", "**/*.mts", "**/*.cjs", "**/*.cts"]
tests = [
  "**/*.test.*", "**/*.spec.*",
  "**/*_test.*", "**/*_tests.*", "**/test_*.*",
  "**/__tests__/**",
  "**/test/**", "**/tests/**"
]
ignore = ["node_modules/", "dist/", "build/", ".next/", "coverage/"]
```

2. Update `Test Code Detection` section (lines 67-71):
```markdown
**Test files** (entire file is test code):
- `*.test.*`, `*.spec.*` (any extension)
- `*_test.*`, `*_tests.*`, `test_*.*` (underscore variants)
- Files in `__tests__/` directories
- Files in `test/` or `tests/` directories
```

3. Update `Configuration` section (line 328-329):
```toml
# source = ["**/*.js", "**/*.jsx", "**/*.ts", "**/*.tsx", "**/*.mjs", "**/*.mts", "**/*.cjs", "**/*.cts"]
# tests = ["**/*.test.*", "**/*.spec.*", "**/*_test.*", "**/*_tests.*", "**/test_*.*", "**/__tests__/**", "**/test/**", "**/tests/**"]
```

**Verification**:
```bash
# Verify doc builds/renders correctly
cat docs/specs/langs/javascript.md | grep -E "\.cjs|\.cts"
```

---

### Phase 2: Unit Test Fixes

**Goal**: Ensure policy test classifier covers all extensions.

**Files**:
- `crates/cli/src/adapter/javascript/policy_tests.rs`

**Changes**:

1. Update `js_classifier` function to include all extensions:

```rust
fn js_classifier(path: &Path) -> FileKind {
    let path_str = path.to_string_lossy();
    if path_str.contains(".test.")
        || path_str.contains(".spec.")
        || path_str.contains("__tests__")
        || path_str.contains("_test.")
        || path_str.contains("_tests.")
        || path_str.starts_with("test_")
    {
        FileKind::Test
    } else if path_str.ends_with(".ts")
        || path_str.ends_with(".js")
        || path_str.ends_with(".tsx")
        || path_str.ends_with(".jsx")
        || path_str.ends_with(".mjs")
        || path_str.ends_with(".mts")
        || path_str.ends_with(".cjs")
        || path_str.ends_with(".cts")
    {
        FileKind::Source
    } else {
        FileKind::Other
    }
}
```

2. Add test for new extensions:

```rust
#[test]
fn recognizes_commonjs_extensions() {
    let policy = default_policy();
    let files = [
        Path::new("src/config.cjs"),
        Path::new("src/types.cts"),
        Path::new(".eslintrc"),
    ];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, js_classifier);
    assert!(result.standalone_violated);
    assert_eq!(result.changed_source.len(), 2);
}
```

**Verification**:
```bash
cargo test -p quench policy_tests -- --nocapture
```

---

### Phase 3: Fixture Coverage

**Goal**: Add `.cjs` and `.cts` test files to verify extension handling.

**Files**:
- `tests/fixtures/javascript/default-patterns/src/config.cjs`
- `tests/fixtures/javascript/default-patterns/src/types.cts`
- `tests/fixtures/javascript/default-patterns/tests/setup.cjs`

**Changes**:

1. Create `tests/fixtures/javascript/default-patterns/src/config.cjs`:
```javascript
// CommonJS configuration file
module.exports = {
  port: 3000,
  debug: false
};
```

2. Create `tests/fixtures/javascript/default-patterns/src/types.cts`:
```typescript
// CommonJS TypeScript definitions
export interface Config {
  port: number;
  debug: boolean;
}
```

3. Create `tests/fixtures/javascript/default-patterns/tests/setup.cjs`:
```javascript
// CommonJS test setup file
module.exports = {
  setup: () => console.log('test setup')
};
```

**Verification**:
```bash
# Verify files are classified correctly
cargo run -- check --escapes tests/fixtures/javascript/default-patterns --output json | jq '.checks[] | select(.name == "cloc") | .metrics'
```

---

### Phase 4: Spec Docstring Updates

**Goal**: Update spec test docstrings to match implementation.

**Files**:
- `tests/specs/adapters/javascript.rs`

**Changes**:

1. Update docstring on line 72-73:
```rust
/// Spec: docs/specs/langs/javascript.md#default-patterns
///
/// > source = ["**/*.js", "**/*.jsx", "**/*.ts", "**/*.tsx", "**/*.mjs", "**/*.mts", "**/*.cjs", "**/*.cts"]
```

2. Update docstring on lines 88-95:
```rust
/// Spec: docs/specs/langs/javascript.md#default-patterns
///
/// > tests = [
/// >   "**/*.test.*", "**/*.spec.*",
/// >   "**/*_test.*", "**/*_tests.*", "**/test_*.*",
/// >   "**/__tests__/**",
/// >   "**/test/**", "**/tests/**"
/// > ]
```

**Verification**:
```bash
cargo test --test specs adapters::javascript -- --nocapture
```

---

### Phase 5: Verification & Cleanup

**Goal**: Ensure all tests pass and no regressions.

**Commands**:
```bash
# Full test suite
make check

# Specific JavaScript adapter tests
cargo test -p quench javascript

# Spec tests
cargo test --test specs adapters::javascript
```

**Checklist**:
- [ ] All 351 tests pass
- [ ] No clippy warnings
- [ ] `make check` passes completely
- [ ] New fixtures are counted in cloc metrics

## Key Implementation Details

### Pattern Alignment

The adapter and config must use identical patterns:

| Component | Location | Status |
|-----------|----------|--------|
| `Adapter::extensions()` | `mod.rs:140` | Has cjs/cts |
| `is_js_extension()` | `mod.rs:65-70` | Has cjs/cts |
| `JavaScriptConfig::default_source()` | `config/javascript.rs:45-55` | Has cjs/cts |
| `JavaScriptConfig::default_tests()` | `config/javascript.rs:58-68` | Has wildcards |
| Spec doc | `docs/specs/langs/javascript.md` | **Needs update** |

### Test Pattern Wildcards

The refactor changed explicit patterns to wildcards:

**Before (explicit):**
```rust
"**/*.test.js", "**/*.test.ts", "**/*.test.jsx", "**/*.test.tsx"
```

**After (wildcard):**
```rust
"**/*.test.*"
```

This is more maintainable but must be documented correctly.

### CommonJS Extensions

`.cjs` and `.cts` are CommonJS equivalents:
- `.cjs` = CommonJS JavaScript (explicit CommonJS when `"type": "module"` in package.json)
- `.cts` = CommonJS TypeScript (TypeScript with CommonJS output)

These are increasingly common in modern Node.js projects that use ES modules by default.

## Verification Plan

### Unit Tests
```bash
cargo test -p quench javascript
cargo test -p quench policy_tests
cargo test -p quench suppress
```

### Spec Tests
```bash
cargo test --test specs adapters::javascript
```

### Integration
```bash
# Verify new extensions are counted
cargo run -- check tests/fixtures/javascript/default-patterns -o json | jq '.checks[] | select(.name == "cloc")'

# Full verification
make check
```

### Manual Verification
1. Create a temp project with `.cjs`/`.cts` files
2. Run `quench check` and verify they're classified as source
3. Verify test patterns work with new extensions

## Commit Message Template

```
fix(js): sync docs and tests with .cjs/.cts extension support

Updates:
- docs/specs/langs/javascript.md: add .cjs/.cts to source patterns
- policy_tests.rs: include all extensions in test classifier
- default-patterns fixture: add .cjs/.cts test files
- javascript.rs spec: fix docstrings to match implementation

This completes the work from checkpoint-49f-quickwins which added
.cjs/.cts extension support but left documentation out of sync.

Specs:
- tests/specs/adapters/javascript.rs: all pass
- tests/fixtures/javascript/default-patterns: includes new extensions

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
