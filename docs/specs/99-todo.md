# Future Work

Features discussed but not yet fully specified. To be designed in later phases.

## Report Command

The `quench report` command generates human-readable reports from stored metrics.

### Formats

- **Markdown** (default): Summary tables, metric values, trend indicators
- **JSON**: Machine-readable for external tools
- **HTML**: Static dashboard page

### Data Sources

Reports read from:
- `.quench/baseline.json` (committed file)
- Git notes (`git notes --ref=quench`)

History is derived from git notes history or git log/blame (for committed baseline).

### Weekly Summary (Future)

Generate trending reports over configurable period:
- Summary table with deltas from previous period
- Pass/fail status per metric
- Trend direction indicators (↑ ↓ →)

## GitHub Pages Dashboard

Auto-publish metrics to GitHub Pages. Based on pattern from wok project.

### Components

- `docs/reports/index.html` - Static dashboard with JavaScript
- `docs/reports/quality/latest.json` - Current metrics
- `docs/reports/quality/latest.md` - Human-readable summary

### CI Integration

```yaml
- name: Generate reports
  run: |
    quench check --ci --save .quench/baseline.json
    quench report -o json > docs/reports/quality/latest.json
    quench report -o html > docs/reports/index.html

- name: Deploy to GitHub Pages
  uses: actions/deploy-pages@v2
```

### Dashboard Features

- Metric cards with current values
- Color coding (green/yellow/red based on thresholds)
- Links to CI runs
- Responsive design

## Future Adapters

| Adapter | Detection | Notes |
|---------|-----------|-------|
| `typescript` | `tsconfig.json` | `as unknown`, `@ts-ignore`, `any` escapes |
| `python` | `pyproject.toml` | `# type: ignore`, `# noqa` escapes |
| `go` | `go.mod` | `unsafe.Pointer`, `//nolint` escapes |

### TypeScript/JavaScript Build Support

The `typescript` adapter will include build/bundle metrics:

- **Bundler detection**: Auto-detect from `vite.config.ts`, `webpack.config.js`, `esbuild.config.js`, `rollup.config.js`
- **Bundle size tracking**: Raw and gzipped sizes, chunk breakdown
- **Build time**: Cold and cached builds
- **Bundle analysis**: Large dependency warnings, forbidden imports (e.g., `moment` → suggest `date-fns`)
- **Source map handling**: Exclude from size calculations

See [checks/build.md](checks/build.md) for the build check specification.

## Rust `cfg_test_split` Modes

The `[rust].cfg_test_split` option controls how `#[cfg(test)]` blocks are handled for LOC counting:

```toml
[rust]
cfg_test_split = "count"  # default
```

| Mode | Behavior |
|------|----------|
| `"count"` | Split `#[cfg(test)]` blocks into test LOC (current behavior) |
| `"require"` | Fail if source files contain inline `#[cfg(test)]` blocks; require separate `_tests.rs` files |
| `"off"` | Count all lines as source LOC, don't parse for `#[cfg(test)]` |

### `require` Mode

Projects using `require` mode enforce the sibling test file convention:

```
src/parser.rs       # source only, no #[cfg(test)]
src/parser_tests.rs # all tests here
```

Violations would report:
```
src/parser.rs:150: inline_cfg_test
  Move tests to a sibling _tests.rs file.
```

This pairs with the existing convention documented in CLAUDE.md for using `#[path]` attributes.

## Doc Code Style

Controls what kind of code appears in documentation. Prevents real implementation code in docs (favoring pseudocode or signatures), and provides language-specific rules.

### Purpose

- Keep docs focused on concepts, not implementation details
- Prevent copy-paste of production code that may become stale
- Encourage pseudocode that explains intent over syntax
- Allow type signatures and function prototypes for API documentation

### Configuration

```toml
[check.docs.code]
check = "error"                        # error | warn | off

# Global defaults
style = "pseudocode"                   # pseudocode | signatures | any
```

| Style | Behavior |
|-------|----------|
| `pseudocode` | Forbid language-tagged blocks with real implementation code |
| `signatures` | Allow type/function signatures, forbid function bodies |
| `any` | No restrictions on code blocks (default) |

### Language-Specific Detection

Fine-grained control over which language constructs are allowed:

```toml
[check.docs.code.rust]
allow = ["type", "trait", "struct", "enum", "use", "mod", "const"]
forbid = ["fn", "impl"]              # No functions or impl blocks at all

[check.docs.code.typescript]
allow = ["type", "interface", "import", "export"]
forbid = ["function", "class", "const", "let"]

[check.docs.code.go]
allow = ["type", "interface", "import"]
forbid = ["func", "var"]
```

**Construct categories** (per-language):

| Rust | TypeScript | Go | Description |
|------|------------|-----|-------------|
| `type` | `type` | `type` | Type aliases |
| `struct` | - | `struct` | Struct definitions |
| `enum` | `enum` | - | Enum definitions |
| `trait` | `interface` | `interface` | Trait/interface definitions |
| `fn` | `function` | `func` | Function definitions (any) |
| `fn_signature` | `function_signature` | `func_signature` | Signature only (no body or `...` body) |
| `fn_body` | `function_body` | `func_body` | Function with implementation |
| `impl` | `class` | - | Implementation blocks |
| `use` | `import` | `import` | Import statements |
| `mod` | `export` | - | Module declarations |
| `const` | `const` | `const` | Constant definitions |
| `macro` | - | - | Macro definitions/invocations |

**Granularity**: `fn` forbids all functions; use `fn_signature`/`fn_body` for finer control.

### Real Code vs Pseudocode

The key distinction: does the code block contain language-specific syntax or describe an algorithm?

**Real code** (implementation details leak through):
~~~markdown
```rust
fn connect_or_start() -> Result<DaemonClient> {
    match DaemonClient::connect() {
        Ok(client) => Ok(client),
        Err(ClientError::DaemonNotRunning) => {
            start_daemon_background()?;
            retry_connect(Duration::from_secs(5))
        }
        Err(e) => Err(e),
    }
}
```
~~~

**Pseudocode** (describes intent, not syntax):
~~~markdown
```
connect_or_start():
    if can connect to socket:
        return connection
    else:
        start daemon in background
        retry connect with 5s timeout
        return connection
```
~~~

**Signature only** (API surface without implementation):
~~~markdown
```rust
fn connect_or_start() -> Result<DaemonClient>;
```
~~~

### Pseudocode Detection

When `style = "pseudocode"`, code blocks are analyzed for:

```toml
[check.docs.code]
style = "pseudocode"

# Indicators that code is pseudocode (any match = ok)
pseudocode_markers = [
  "...",                               # Ellipsis placeholder
  "// ...",                            # Comment ellipsis
  "/* ... */",
  "todo!()",                           # Rust placeholder
  "pass",                              # Python placeholder
  "???",                               # Kotlin/other placeholder
]

# Language tags treated as pseudocode (never checked)
pseudocode_langs = ["pseudo", "pseudocode", "algorithm", "text", ""]
```

### Per-Section Overrides

Different sections may have different needs:

```toml
# API reference sections can have full signatures
[[check.docs.code.section]]
pattern = "## API*"
style = "signatures"

# Examples section can have real code
[[check.docs.code.section]]
pattern = "## Example*"
style = "any"

# Everything else: pseudocode only
[[check.docs.code.section]]
pattern = "*"
style = "pseudocode"
```

### Output

```
docs: FAIL
  docs/specs/parser.md:45: code block contains implementation
    ```rust
    fn parse(input: &str) -> Result<Ast> {
        let tokens = lexer::tokenize(input)?;  // ← implementation
        ...
    }
    ```
    Use pseudocode or function signature only. Replace body with `...` or `todo!()`.
```

### JSON Output

```json
{
  "file": "docs/specs/parser.md",
  "line": 45,
  "type": "code_style",
  "lang": "rust",
  "style": "pseudocode",
  "reason": "function_body",
  "advice": "Use pseudocode or function signature only. Replace body with `...` or `todo!()`."
}
```

**Violation types**: `code_style`

**Reasons**: `function_body`, `impl_body`, `executable_statement`, `non_pseudocode`

## Spec Link Validation

Configurable validation for spec references in code comments:

```rust
/// Spec: docs/specs/checks/cloc.md#file-size-limits
```

Would verify:
- File exists
- Anchor (heading) exists in the markdown file
- Report stale references when specs are reorganized

Could be a new check (`docs` or `specs`) or integrated into existing tooling.

## Notes from Interview

- Primary users are AI agents ("landing the plane")
- Performance target: sub-second for fast checks
- All output should be agent-friendly (token-efficient)
- Progressive disclosure: only surface failures
- `--fix` should be explicit about what it can/cannot fix
