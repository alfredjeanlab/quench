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

## Workspace Mode

Run quench across monorepos with nested `quench.toml` files.

When a root config defines workspace members, `quench check` runs each member with its own config and aggregates results. Members may inherit defaults from the workspace root.

**Open questions:**
- Explicit members list vs auto-discovery of nested configs
- Inheritance: opt-in or default?

## Language Server Protocol (LSP)

Expose Quench checks as an LSP server for IDE integration (e.g. vscode, cursor).

### Use Cases

- Real-time violation feedback in editor (alongside linters and type checkers)
- Hover diagnostics for metrics and advice
- Inline code actions for auto-fixes
- Integration with VSCode, Neovim, Emacs, etc.

### Implementation

**Core LSP features:**
- `initialize`: Load `quench.toml` and detect check configuration
- `textDocument/didOpen`: Initial analysis of opened file
- `textDocument/didChange`: Incremental re-analysis on edits
- `textDocument/publishDiagnostics`: Stream violations as they're detected
- `textDocument/codeAction`: Provide fixes from `--fix` mode
- `workspace/didChangeConfiguration`: Reload config when `quench.toml` changes

**Performance considerations:**
- Debounce on text changes (avoid analysis on every keystroke)
- Cache analysis results per file
- Lazy-load checks (only run requested checks)
- Use same parallelization as CLI

### Diagnostics Format

Maps Quench violations to LSP `Diagnostic` objects:

```json
{
  "range": {"start": {"line": 10, "character": 5}, "end": {"line": 10, "character": 20}},
  "severity": 1,  // Error
  "code": "cloc:file_too_large",
  "source": "quench",
  "message": "file_too_large (lines: 150 vs 100)",
  "relatedInformation": [
    {
      "location": {"uri": "file:///...", "range": {...}},
      "message": "Split into smaller modules."
    }
  ]
}
```

### Code Actions

Support LSP `codeAction` requests for violations that can be auto-fixed:

```json
{
  "title": "Apply quench fix",
  "kind": "quickfix",
  "command": {
    "title": "Fix",
    "command": "quench.fix",
    "arguments": ["file:///path/to/file.rs", 10, 5]
  }
}
```

## Import Dependency Rules

Enforce layered architecture by validating import/dependency relationships between modules.

### Motivation

Layered architectures prevent spaghetti dependencies, but violations are easy to introduce and hard to detect in review. Quench can enforce these rules statically by parsing imports.

### Architecture Example

```
                    ┌─────────────────────┐
                    │        cli          │  Layer 4: Entry points
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │       engine        │  Layer 3: Orchestration
                    └──────────┬──────────┘
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                    │
┌─────────▼─────────┐ ┌────────▼────────┐ ┌─────────▼───────┐
│     adapters      │ │     storage     │ │     runbook     │  Layer 2
└───────────────────┘ └─────────────────┘ └─────────────────┘
          │                    │                    │
          └────────────────────┼────────────────────┘
                               │
                    ┌──────────▼──────────┐
                    │        core         │  Layer 1: Pure logic
                    └─────────────────────┘
```

Rules: Higher layers may import from lower layers. Same-layer imports allowed within a module but not across modules. Lower layers may never import from higher layers.

### Configuration

```toml
[check.imports]
check = "error"                          # error | warn | off

[[check.imports.layer]]
name = "core"
paths = ["crate::core", "src/core/**"]
self = true                              # submodules can import siblings (default)
layers = false                           # no other layers allowed
builtin = true                           # std/builtins allowed
external = ["serde", "thiserror"]        # only listed externals

[[check.imports.layer]]
name = "adapters"
paths = ["crate::adapters", "src/adapters/**"]
self = false                             # submodules can't import siblings
layers = ["core"]                        # only core
builtin = true
external = true                          # all externals allowed

[[check.imports.layer]]
name = "storage"
paths = ["crate::storage", "src/storage/**"]
layers = ["core"]
external = ["sled", "serde"]

[[check.imports.layer]]
name = "engine"
paths = ["crate::engine", "src/engine/**"]
layers = ["core", "adapters", "storage"]
external = true

[[check.imports.layer]]
name = "cli"
paths = ["crate::cli", "src/cli/**", "src/main.rs"]
layers = "*"                             # any layer (equivalent to true)
builtin = "*"
external = "*"
```

**Field values:**
| Value | Meaning |
|-------|---------|
| `false` | none allowed |
| `true` or `"*"` | all allowed |
| `[...]` | only listed items allowed |

### Language Support

| Language | Import Detection |
|----------|------------------|
| `rust` | `use crate::`, `mod`, `extern crate` |
| `golang` | `import "..."`, `import (...)` |
| `typescript` | `import ... from`, `require()` |
| `python` | `import`, `from ... import` |

### Detection

For each source file:
1. Determine which layer the file belongs to (by path matching)
2. Parse imports/use statements
3. Resolve import targets to layers
4. Flag violations: importing from higher or same-level cross-module

### Output

```
imports: FAIL
  src/core/parser.rs:5: layer violation (core -> engine)
    use crate::engine::Config;
    Core layer cannot import from engine layer. Move shared types to core or pass via parameter.

  src/adapters/rust.rs:12: cross-module import (adapters -> storage)
    use crate::storage::Cache;
    Same-level modules should not depend on each other. Extract shared interface to core.
```

### JSON Output

```json
{
  "file": "src/core/parser.rs",
  "line": 5,
  "type": "layer_violation",
  "layer": "core",
  "import": "crate::engine::Config",
  "target": "engine",
  "allowed": [],
  "advice": "Core layer cannot import from engine."
}
```

```json
{
  "file": "src/core/utils.rs",
  "line": 12,
  "type": "external_violation",
  "layer": "core",
  "import": "reqwest",
  "allowed": ["serde", "thiserror"],
  "advice": "External crate 'reqwest' not allowed in core layer."
}
```

### Rust-Specific: Workspace Crates

For Rust workspaces, layers can be defined at crate granularity:

```toml
[[check.imports.layer]]
name = "core"
paths = ["quench_core"]               # Crate name from Cargo.toml
layers = false
external = ["serde"]

[[check.imports.layer]]
name = "cli"
paths = ["quench_cli"]
layers = "*"
external = "*"
```

Violations detected via `Cargo.toml` dependencies and `use` statements.

### Go-Specific: Package Paths

```toml
[[check.imports.layer]]
name = "core"
paths = ["github.com/org/project/internal/core"]
layers = false

[[check.imports.layer]]
name = "cmd"
paths = ["github.com/org/project/cmd/**"]
layers = "*"
```

### TypeScript-Specific: Path Aliases

Respects `tsconfig.json` path aliases:

```toml
[[check.imports.layer]]
name = "core"
paths = ["@/core/**", "src/core/**"]
layers = false
external = ["zod", "date-fns"]
```

## Build Scripts Validation

Validate that commands referenced in documentation (e.g., "Landing the Plane" sections in `CLAUDE.md`) actually exist in the project's build system.

### Motivation

AI agents and developers follow instructions in `CLAUDE.md` files, but those instructions can become stale. If a `make check` command is documented but the `check` target was removed or renamed, the instructions fail silently or confusingly.

### Detection Flow

1. Parse `CLAUDE.md` (or configured files) for command references
2. Identify the build system from the command prefix (`make`, `just`, `task`, `npm run`, `rake`)
3. Verify the target/script exists in the corresponding build file
4. Report missing or mismatched commands

### Build Systems

| Build System | Detection | Config File | Command Pattern |
|--------------|-----------|-------------|-----------------|
| `make` | `Makefile`, `GNUmakefile` | `Makefile` | `make <target>` |
| `just` | `justfile`, `Justfile` | `justfile` | `just <recipe>` |
| `task` | `Taskfile.yml`, `Taskfile.yaml` | `Taskfile.yml` | `task <task>` |
| `npm` | `package.json` | `package.json` | `npm run <script>` |
| `rake` | `Rakefile` | `Rakefile` | `rake <task>` |

### Configuration

```toml
[check.build_scripts]
check = "error"                         # error | warn | off

# Files to scan for command references
sources = ["CLAUDE.md", "README.md", "CONTRIBUTING.md"]

# Sections to parse (heading patterns)
sections = ["*Landing*", "*Development*", "*Build*", "*Setup*"]

# Build systems to validate against (auto-detect if not specified)
systems = ["make", "just", "npm"]
```

### Command Extraction

Extract commands from markdown code blocks and inline code:

~~~markdown
```bash
make check
npm run build
```

Run `just test` to execute tests.
~~~

**Extraction rules:**
- Fenced code blocks with `bash`, `sh`, `shell`, or no language tag
- Inline code matching known command patterns
- Commands chained with `&&` are split and validated individually

### Output

```
build_scripts: FAIL
  CLAUDE.md:45: missing target (make → Makefile)
    `make check` referenced but `check` target not found in Makefile
    Available targets: build, test, clean, lint

  CLAUDE.md:52: missing script (npm → package.json)
    `npm run deploy` referenced but `deploy` script not found
    Available scripts: build, test, start, lint
```

### JSON Output

```json
{
  "file": "CLAUDE.md",
  "line": 45,
  "type": "missing_target",
  "system": "make",
  "config_file": "Makefile",
  "command": "make check",
  "target": "check",
  "available": ["build", "test", "clean", "lint"],
  "advice": "`make check` referenced but `check` target not found in Makefile"
}
```

### Parsing Build Files

**Makefile**: Extract targets from lines matching `^target:` (excluding `.PHONY`, pattern rules)

**justfile**: Extract recipes from lines matching `^recipe-name:` or `^[recipe-name]:`

**Taskfile.yml**: Parse YAML, extract keys from `tasks:` mapping

**package.json**: Parse JSON, extract keys from `scripts` object

**Rakefile**: Extract tasks from `task :name` or `desc`/`task` pairs

### Edge Cases

- **Compound commands**: `make clean && make build` validates both targets
- **Variable targets**: `make $(TARGET)` skipped (dynamic)
- **Conditional commands**: `[ -f Makefile ] && make check` still validates
- **Subshells**: Commands in `$()` or backticks are extracted and validated

## Notes from Interview

- Primary users are AI agents ("landing the plane")
- Performance target: sub-second for fast checks
- All output should be agent-friendly (token-efficient)
- Progressive disclosure: only surface failures
- `--fix` should be explicit about what it can/cannot fix
