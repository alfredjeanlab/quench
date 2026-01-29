# Configuration Specification

Quench uses convention over configuration with a single optional `quench.toml` at project root.

## File Location

```text
project-root/
├── quench.toml              # Single config file (optional)
├── .quench/
│   └── baseline.json        # Metrics storage
├── crates/
│   ├── cli/                 # No config here
│   └── core/                # No config here
```

## Discovery

1. CLI flags (highest priority)
2. `quench.toml` in current directory or nearest parent (up to git root)
3. Built-in defaults (lowest priority)

## Config Sections

```toml
version = 1      # Config format version (required)
[project]        # Project identity and file patterns
[git]            # Git integration settings
[rust]           # Rust language config (optional, has defaults)
[golang]         # Go language config (optional, has defaults)
[javascript]     # JavaScript/TypeScript config (optional, has defaults)
[shell]          # Shell language config (optional, has defaults)
[check.*]        # Check-specific configuration
[ratchet]        # Regression prevention
```

## Minimal Config

Most projects need no config. Missing `quench.toml` uses all defaults.

When a config file exists, `version` is required:

```toml
version = 1
```

## Version

The `version` field is a required integer at the top of the config file.

- Bumped only on breaking changes (renamed/removed fields, changed semantics)
- Additions don't bump the version
- Quench validates version before parsing:
  ```
  quench: unsupported config version 2 (supported: 1)
    Upgrade quench to use this config.
  ```

Current version: **1**

## Pattern Resolution

When quench classifies files as source or test code, patterns are resolved in this hierarchy:

```
1. [<language>].tests   ← Language-specific override (most specific)
2. [project].tests      ← Project-wide patterns (applies to all languages)
3. Adapter defaults     ← Built-in convention (zero-config)
```

**Examples:**

```toml
# All shell tests are in packages/*/tests/
[shell]
tests = ["packages/*/tests/**/*.bats"]

# Other languages use project defaults
[project]
tests = ["**/tests/**", "**/*_test.*"]
```

In this setup:
- Shell files match against `packages/*/tests/**/*.bats`
- Rust files match against `**/tests/**` and `**/*_test.*` (from `[project]`)
- Go files match against `**/*_test.go` (built-in default, since no override)

The same hierarchy applies to `source` patterns.

## Full Schema

### [project]

Project identity and file patterns.

```toml
[project]
name = "my-project"                    # Optional, inferred from directory

# File patterns (applies to all languages unless overridden by [<lang>].tests)
source = ["**/*.rs", "**/*.sh"]
tests = ["**/tests/**", "**/*_test.*", "**/*.spec.*"]
ignore = ["target/", "node_modules/", "dist/", ".git/"]

# Packages (auto-detected for cargo workspaces)
[[project.packages]]
name = "cli"
path = "crates/cli"

[[project.packages]]
name = "core"
path = "crates/core"
```

### [git]

Git integration settings.

```toml
[git]
base = "main"                          # Default for --base (auto: main > master > develop)

# Baseline source (default: "notes")
#   "notes" - use git notes (refs/notes/quench)
#   "<path>" - use file at path (e.g., ".quench/baseline.json")
baseline = "notes"

[git.commit]
check = "error"                        # error | warn | off (enabled by default)
# format = "conventional"              # conventional | none (default: conventional)

# Optional: restrict to specific types (default: common conventional types)
# types = ["feat", "fix", "chore", "docs", "test", "refactor", "perf", "ci", "build", "style"]

# Optional: restrict to specific scopes (default: any)
# scopes = ["api", "cli", "core"]

# Check that commit format is documented in agent files
agents = true                          # default: true

# Create .gitmessage template with --fix
template = true                        # default: true
```

### [rust]

Rust language configuration. Auto-detected when `Cargo.toml` exists.

```toml
[rust]
# Source/test patterns (falls back to [project].tests if not set)
# source = ["**/*.rs"]
# tests = ["**/tests/**", "**/test/**/*.rs", "**/benches/**", "**/*_test.rs", "**/*_tests.rs"]
# ignore = ["target/"]

cfg_test_split = "count"               # count | require | off (default: "count")
                                       # Boolean still works: true="count", false="off"

# Build targets for coverage + binary size (default: all [[bin]] entries)
# targets = ["myapp", "myserver"]

# Build metrics (CI mode)
binary_size = true
build_time = true

# Thresholds (optional) - see [check.build] for details
binary_size_max = "5 MB"
build_time_cold_max = "60s"
build_time_hot_max = "2s"

# Lint suppression (#[allow(...)])
[rust.suppress]
check = "comment"                      # forbid | comment | allow
# comment = "// JUSTIFIED:"            # optional: require specific pattern

[rust.suppress.test]
check = "allow"                        # tests can suppress freely

# Per-language cloc settings (overrides [check.cloc])
[rust.cloc]
check = "error"                        # error | warn | off (inherits from [check.cloc].check)
advice = "Custom advice for Rust files."

# Policy
[rust.policy]
check = "error"                        # error | warn | off (default: error)
lint_changes = "standalone"
lint_config = ["rustfmt.toml", "clippy.toml"]
```

### [shell]

Shell language configuration. Auto-detected when `*.sh` files exist in root, `bin/`, or `scripts/`.

```toml
[shell]
# Source/test patterns (falls back to [project].tests if not set)
# source = ["**/*.sh", "**/*.bash"]
# tests = ["**/tests/**/*.bats", "**/test/**/*.bats", "**/*_test.sh"]

# Suppress (# shellcheck disable=)
[shell.suppress]
check = "forbid"                       # forbid | comment | allow

[shell.suppress.test]
check = "allow"

# Per-language cloc settings (overrides [check.cloc])
[shell.cloc]
check = "error"                        # error | warn | off (inherits from [check.cloc].check)
advice = "Custom advice for shell scripts."

[shell.policy]
check = "error"                        # error | warn | off (default: error)
lint_changes = "standalone"
lint_config = [".shellcheckrc"]
```

### [golang]

Go language configuration. Auto-detected when `go.mod` exists.

```toml
[golang]
# Source/test patterns
# source = ["**/*.go"]
# tests = ["**/*_test.go"]
# ignore = ["vendor/**"]

# Lint suppression (//nolint:)
[golang.suppress]
check = "comment"                      # forbid | comment | allow

[golang.suppress.test]
check = "allow"

# Per-language cloc settings (overrides [check.cloc])
[golang.cloc]
check = "error"                        # error | warn | off (inherits from [check.cloc].check)
advice = "Custom advice for Go files."

# Policy
[golang.policy]
check = "error"                        # error | warn | off (default: error)
lint_changes = "standalone"
lint_config = [".golangci.yml", ".golangci.yaml", ".golangci.toml"]
```

### [javascript]

JavaScript/TypeScript language configuration. Auto-detected when `package.json` exists.

```toml
[javascript]
# Source/test patterns
# source = ["**/*.js", "**/*.jsx", "**/*.ts", "**/*.tsx", "**/*.mjs", "**/*.mts"]
# tests = ["**/tests/**", "**/__tests__/**", "**/*.test.*", "**/*.spec.*"]
# ignore = ["node_modules/**", "dist/**", "build/**"]

# Lint suppression (eslint-disable)
[javascript.suppress]
check = "comment"                      # forbid | comment | allow

[javascript.suppress.test]
check = "allow"

# Per-language cloc settings (overrides [check.cloc])
[javascript.cloc]
check = "error"                        # error | warn | off (inherits from [check.cloc].check)
advice = "Custom advice for JS/TS files."

# Policy
[javascript.policy]
check = "error"                        # error | warn | off (default: error)
lint_changes = "standalone"
lint_config = [".eslintrc", ".eslintrc.js", ".eslintrc.json", "eslint.config.js", "tsconfig.json", "biome.json"]
```

### [check.*]

Each check has its own section. Common fields:

| Field | Type | Description |
|-------|------|-------------|
| `check` | string | `"error"` \| `"warn"` \| `"off"` (default: `"error"`, except license) |
| `exclude` | [string] | Patterns to skip |

#### [check.cloc]

Lines of code and file size limits.

```toml
[check.cloc]
check = "error"                        # error | warn | off
max_lines = 750                        # Source file limit
max_lines_test = 1100                  # Test file limit
max_tokens = 20000                     # Use false to disable
exclude = ["**/generated/**"]

# Custom advice for violations (defaults shown)
advice = "Can the code be made more concise? If not, split large source files into sibling modules or submodules in a folder; consider refactoring to be more unit testable."
advice_test = "Can tests be parameterized or use shared fixtures to be more concise? If not, split large test files into a folder."

# Per-package overrides
[check.cloc.package.cli]
max_lines = 500                        # Stricter for CLI

[check.cloc.package.generated]
check = "off"                          # Skip entirely
```

#### [check.escapes]

Escape hatch detection with configurable patterns.

```toml
[check.escapes]
check = "error"                        # error | warn | off

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"             # count | comment | forbid
comment = "// SAFETY:"
advice = "Add a // SAFETY: comment explaining the invariants."

[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"

[[check.escapes.patterns]]
name = "todo"
pattern = "TODO|FIXME|XXX"
action = "count"
threshold = 10
```

#### [check.agents]

Agent file validation (CLAUDE.md, .cursorrules). Supports scope hierarchy.

```toml
[check.agents]
check = "error"                        # error | warn | off
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"

# Root scope (project root)
[check.agents.root]
required = ["CLAUDE.md"]
optional = [".cursorrules"]
sections.required = ["Project Structure", "Development"]
max_lines = 500
max_tokens = 20000                     # Use false to disable
tables = "forbid"

# Package scope (each package directory)
[check.agents.package]
required = []
optional = ["CLAUDE.md"]
max_lines = 200
max_tokens = 800

# Module scope (subdirectories)
[check.agents.module]
required = []
max_lines = 100
max_tokens = 400
```

#### [check.docs]

TOC validation, link validation, spec files, and commit checking.

```toml
[check.docs]
check = "error"                            # error | warn | off

# TOC validation (directory trees in markdown)
[check.docs.toc]
check = "error"                            # error | warn | off
# include = ["**/*.md", "**/*.mdc"]        # optional, defaults shown
exclude = ["plans/**", "plan.md", "*_plan.md", "plan_*"]

# Link validation (markdown links)
[check.docs.links]
check = "error"                            # error | warn | off
# include = ["**/*.md", "**/*.mdc"]        # optional
exclude = ["plans/**"]

# Specs validation
[check.docs.specs]
check = "error"                            # error | warn | off
path = "docs/specs"
# extension = ".md"                        # optional
# index_file = "docs/specs/CLAUDE.md"      # optional, auto-detected
index = "auto"                             # auto | toc | linked | exists

# Commit checking (CI mode only)
[check.docs.commit]
check = "off"                              # error | warn | off (default: off)
# types = ["feat", "feature", "story", "breaking"]   # default

# Area mappings (reusable across features)
[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"
```

#### [check.tests]

Test correlation, execution, and metrics.

```toml
[check.tests]
check = "error"                        # error | warn | off

# Commit checking (source changes need test changes)
[check.tests.commit]
check = "error"                        # error | warn | off
# types = ["feat", "feature", "story", "breaking"] # default; only these commits require tests
scope = "branch"                       # branch | commit
placeholders = "allow"
exclude = ["**/mod.rs", "**/main.rs"]

# Test suites (time limits per-suite)
[[check.tests.suite]]
runner = "cargo"
# covers Rust automatically via llvm-cov
max_total = "30s"
max_test = "1s"

[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
setup = "cargo build"
targets = ["myapp"]                     # instrument Rust binary
max_total = "10s"
max_test = "500ms"

[[check.tests.suite]]
runner = "pytest"
path = "tests/integration/"
ci = true                              # only run in CI mode (slow)
targets = ["myserver"]                  # also instrument Rust binary
max_total = "60s"

[[check.tests.suite]]
runner = "bats"
path = "tests/scripts/"
targets = ["scripts/*.sh"]              # shell scripts via kcov

# Coverage settings
[check.tests.coverage]
check = "error"                        # error | warn | off
min = 75                               # minimum coverage %

# Per-package coverage thresholds
[check.tests.coverage.package.core]
min = 90

[check.tests.coverage.package.cli]
min = 60
exclude = ["src/main.rs"]

# Test time check level (thresholds are per-suite)
[check.tests.time]
check = "warn"                         # error | warn | off
```

#### [check.license]

License header validation (CI only, disabled by default).

```toml
[check.license]
check = "off"                          # error | warn | off (default: off)
license = "MIT"
copyright = "Your Organization"
exclude = ["**/generated/**"]
```

### [ratchet]

Prevent quality regressions.

```toml
[ratchet]
check = "error"                        # error | warn | off

# Metrics to ratchet (defaults shown)
coverage = true                        # Coverage can't drop
escapes = true                         # Escape counts can't increase
binary_size = false                    # Binary size can't grow
build_time_cold = false
build_time_hot = false
test_time_total = false
test_time_avg = false
test_time_max = false

# Tolerances
coverage_tolerance = 0.5               # Allow 0.5% drop

# Per-package
[ratchet.package.core]
coverage = true

[ratchet.package.experimental]
coverage = false                       # Don't ratchet experimental
```

## Language Detection

Quench auto-detects project languages:

| Detection | Language | Default Patterns |
|-----------|----------|------------------|
| `Cargo.toml` | Rust | `**/*.rs`, ignore `target/` |
| `*.sh` in root/bin/ | Shell | `**/*.sh`, `**/*.bash` |
| `package.json` | JS/TS | Future |

## Environment Variables

```bash
QUENCH_NO_COLOR=1              # Disable color
QUENCH_LOG=debug               # Enable tracing (off, error, warn, info, debug, trace)
QUENCH_DEBUG=1                 # Enable debug output (file stats, cache stats, etc.)
QUENCH_DEBUG_FILES=1           # List scanned files (for debugging file walking)
```

**QUENCH_LOG**: When set, quench emits tracing output to stderr via the tracing crate:
- `debug`: Shows file walking decisions, pattern matches, cache hits/misses
- `trace`: Extremely verbose, includes per-line processing

**QUENCH_DEBUG**: When set, quench prints additional diagnostic info to stderr:
- File scan statistics (files found, errors, symlink loops)
- Cache hit/miss statistics
- Warnings about skipped files

**QUENCH_DEBUG_FILES**: When set, quench lists all scanned files to stdout instead of running checks. Useful for debugging file walking and ignore patterns.

Log/debug output goes to stderr and doesn't affect stdout (safe to use with `-o json`).

## Validation

Invalid config produces clear errors:

```
quench: error in quench.toml
  check.escapes.patterns[0].action: invalid value "warn"
    expected one of: count, comment, forbid
```

Unknown keys are errors (provides immediate feedback on typos):

```
quench: error in quench.toml
  unknown field `check.unknown`, expected one of: ...
```

This ensures config correctness and catches typos immediately rather than silently ignoring them.
