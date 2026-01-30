# Plan: Per-Package CLOC in `quench cloc`

## Status

**TODO**

## Overview

Add per-package (per-crate, per-workspace-member) line-of-code breakdown to the `quench cloc` standalone command. The `quench check` cloc check already tracks per-package metrics via `by_package` in its JSON output, but the standalone `quench cloc` command only aggregates by language — it doesn't report package-level data at all, despite already auto-detecting packages.

This plan adds:
1. Per-package stats collection in the `quench cloc` command
2. Per-package sections in both text and JSON output
3. Go package auto-detection (currently missing from `apply_language_defaults`)
4. Refactoring to eliminate duplicated config/detection logic between `cmd_cloc.rs` and `adapter/project.rs`

Supported monorepo types: Rust workspaces (`Cargo.toml` members), JavaScript/TypeScript workspaces (pnpm, npm, yarn), Go multi-package projects, and Python packages.

## Project Structure

Files to create or modify:

```
crates/cli/src/
├── cmd_cloc.rs                    # MODIFY: add per-package tracking + output
├── adapter/
│   ├── project.rs                 # MODIFY: add Go package auto-detection
│   └── go/mod.rs                  # (existing: enumerate_packages already exists)
tests/
├── specs/
│   └── cloc_cmd.rs                # MODIFY: add per-package behavioral specs
└── fixtures/
    ├── cloc-cmd-packages/         # NEW: fixture for per-package cloc command
    │   ├── quench.toml
    │   ├── Cargo.toml
    │   ├── cli/
    │   │   └── src/main.rs
    │   ├── core/
    │   │   ├── src/lib.rs
    │   │   └── tests/core_test.rs
    │   └── shared/
    │       └── src/lib.rs
    └── cloc-cmd-auto-detect/      # NEW: fixture for auto-detected workspace
        ├── Cargo.toml             # [workspace] members = ["crates/*"]
        └── crates/
            ├── alpha/
            │   ├── Cargo.toml
            │   └── src/lib.rs
            └── beta/
                ├── Cargo.toml
                └── src/lib.rs
```

## Dependencies

No new external crates. Uses existing:
- `adapter::project::apply_language_defaults` — consolidated language detection + package auto-detection
- `adapter::go::enumerate_packages` — Go package enumeration (already exists, not wired up)
- Existing walker, adapter registry, and config infrastructure

## Implementation Phases

### Phase 1: Refactor `cmd_cloc.rs` to use `apply_language_defaults`

**Goal:** Eliminate the ~80 lines of duplicated language detection and exclude pattern logic in `cmd_cloc.rs` (lines 57–135) by delegating to the shared `apply_language_defaults()` function from `adapter/project.rs`.

Currently `cmd_cloc.rs` manually detects the language, builds exclude patterns, and auto-detects workspace packages — duplicating the logic in `adapter/project.rs`. The cmd_cloc version is actually incomplete: it doesn't read package names from `Cargo.toml` (no `package_names` population), and it doesn't detect JS workspaces at all.

**Changes:**

1. In `cmd_cloc.rs::run()`, replace lines 54–135 with:

```rust
// Apply language defaults (excludes + package auto-detection)
let exclude_patterns = apply_language_defaults(&root, &mut config);

// Also add check.cloc.exclude patterns (parity with check command)
let mut exclude_patterns = exclude_patterns;
for pattern in &config.check.cloc.exclude {
    if !exclude_patterns.contains(pattern) {
        exclude_patterns.push(pattern.clone());
    }
}
```

2. Add import: `use quench::adapter::project::apply_language_defaults;`

3. Make `apply_language_defaults` public (it already is — `pub fn`).

**Milestone:** `make check` passes. `quench cloc` produces identical output to before. JS workspaces and Rust workspace package names are now correctly detected in the cloc command.

### Phase 2: Per-Package Data Collection

**Goal:** Track per-package stats alongside the existing per-language stats.

**Changes to `cmd_cloc.rs`:**

1. Add a `PackageStats` struct (mirrors the existing `LangStats` but keyed by package):

```rust
/// Accumulated statistics for a single package.
#[derive(Default)]
struct PackageStats {
    source: LangStats,
    test: LangStats,
}
```

2. Add a `HashMap<String, PackageStats>` alongside the existing `HashMap<(String, FileKind), LangStats>`:

```rust
let mut stats: HashMap<(String, FileKind), LangStats> = HashMap::new();
let mut package_stats: HashMap<String, PackageStats> = HashMap::new();
let packages = &config.project.packages;
```

3. In the file processing loop, after accumulating into `stats`, also assign the file to a package and accumulate:

```rust
// Per-package tracking
if !packages.is_empty() {
    if let Some(pkg) = file_package(relative_path, packages) {
        let entry = package_stats.entry(pkg).or_default();
        match file_kind {
            FileKind::Source => {
                entry.source.files += 1;
                entry.source.blank += metrics.blank;
                entry.source.comment += metrics.comment;
                entry.source.code += metrics.code;
            }
            FileKind::Test => {
                entry.test.files += 1;
                entry.test.blank += metrics.blank;
                entry.test.comment += metrics.comment;
                entry.test.code += metrics.code;
            }
            FileKind::Other => {}
        }
    }
}
```

4. Add the `file_package` helper (reuse the same logic as `checks/cloc.rs:412–426`):

```rust
fn file_package(path: &Path, packages: &[String]) -> Option<String> {
    for pkg in packages {
        if pkg == "." {
            return Some(pkg.clone());
        }
        if path.starts_with(pkg) {
            return Some(pkg.clone());
        }
    }
    None
}
```

5. Pass `package_stats` and `config.project.package_names` to the output functions.

**Note:** For Rust `cfg_test_split = Count`, the existing proportional splitting logic already produces separate source/test metrics — those split values should be accumulated into the package stats the same way they go into the language stats.

**Milestone:** Data is collected but not yet printed. Can verify with a debug log or by temporarily printing the HashMap.

### Phase 3: Per-Package Output Formatting

**Goal:** Add per-package sections to both text and JSON output.

#### Text Output

After the existing aggregate table, add a package summary table when packages are detected:

```
──────────────────────────────────────────────────────────────
Language                 files     blank   comment      code
──────────────────────────────────────────────────────────────
Rust (source)               42       580       320      4200
Rust (tests)                18       120        45      1800
──────────────────────────────────────────────────────────────
Source total                50       650       355      4630
Test total                  20       135        53      1910
──────────────────────────────────────────────────────────────
Total                       70       785       408      6540
──────────────────────────────────────────────────────────────

──────────────────────────────────────────────────────────────
Package                source     test     ratio
──────────────────────────────────────────────────────────────
cli                      3421     2890     0.84x
core                     9032     6031     0.67x
(unpackaged)              200      100     0.50x
──────────────────────────────────────────────────────────────
```

**Design decisions:**
- Package names come from `config.project.package_names` (display name), falling back to the path
- Sort packages alphabetically
- Show `(unpackaged)` row for files not belonging to any package (only if there are unpackaged files and at least one package is configured)
- `source` column = total code lines in source files for that package
- `test` column = total code lines in test files
- `ratio` = test / source (same as check's ratio)
- Use the same separator character (`\u{2500}`) and column widths

Update `print_text` signature:

```rust
fn print_text(
    stats: &HashMap<(String, FileKind), LangStats>,
    package_stats: &HashMap<String, PackageStats>,
    package_names: &HashMap<String, String>,
    total_source: &LangStats,
    total_test: &LangStats,
)
```

#### JSON Output

Add a `packages` object to the JSON when packages are present:

```json
{
  "languages": [...],
  "totals": {...},
  "packages": {
    "cli": {
      "source": { "files": 15, "blank": 200, "comment": 80, "code": 3421 },
      "test": { "files": 12, "blank": 90, "comment": 30, "code": 2890 },
      "ratio": 0.84
    },
    "core": {
      "source": { "files": 32, "blank": 450, "comment": 240, "code": 9032 },
      "test": { "files": 20, "blank": 180, "comment": 70, "code": 6031 },
      "ratio": 0.67
    }
  }
}
```

**Design decisions:**
- `packages` key only present when packages are configured/detected (same as check's `by_package`)
- Each package has `source` and `test` sub-objects with the same fields as `totals`
- Includes `ratio` at the package level
- Package keys use display names (from `package_names`)

Update `print_json` signature to accept the same additional parameters.

**Milestone:** `quench cloc` shows per-package breakdown on a Rust workspace. `quench cloc --output json` includes `packages` object.

### Phase 4: Go Package Auto-Detection

**Goal:** Wire up the existing `go::enumerate_packages()` function in `apply_language_defaults` so Go multi-package projects get automatic package detection.

Go projects don't have a workspace manifest, but `enumerate_packages()` (in `adapter/go/mod.rs:155–208`) already walks the directory tree to find directories containing `.go` files. This is equivalent to `go list ./...`.

**Changes to `adapter/project.rs`:**

```rust
ProjectLanguage::Go => {
    if !exclude_patterns.iter().any(|p| p.contains("vendor")) {
        exclude_patterns.push("vendor".to_string());
    }

    // Auto-detect Go packages if not configured
    if config.project.packages.is_empty() {
        let packages = super::go::enumerate_packages(root);
        // Only populate if there are multiple packages (single-package
        // projects don't benefit from per-package breakdown)
        if packages.len() > 1 {
            for pkg_path in packages {
                // Use directory name as display name
                let name = Path::new(&pkg_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&pkg_path)
                    .to_string();
                config.project.package_names.insert(pkg_path.clone(), name);
                config.project.packages.push(pkg_path);
            }
            config.project.packages.sort();
        }
    }
}
```

**Design decisions:**
- Only auto-detect if `config.project.packages` is empty (respect explicit config)
- Skip auto-detection for single-package projects (the breakdown would be trivially the same as the total)
- Use directory basename as display name (e.g., `cmd/server` → `server`, `pkg/api` → `api`)
- The `enumerate_packages` function already skips `vendor/`, `testdata/`, and hidden dirs

**Milestone:** `quench cloc` on a Go project with `cmd/` and `pkg/` directories shows per-package breakdown.

### Phase 5: Test Fixtures and Behavioral Specs

**Goal:** Add test coverage for per-package cloc command output.

#### New Fixtures

**`tests/fixtures/cloc-cmd-packages/`** — Multi-package project with explicit config:

```
cloc-cmd-packages/
├── quench.toml          # packages = ["cli", "core", "shared"]
├── Cargo.toml           # [workspace] members = ["cli", "core", "shared"]
├── cli/
│   ├── Cargo.toml       # [package] name = "my-cli"
│   └── src/main.rs      # ~5 lines
├── core/
│   ├── Cargo.toml       # [package] name = "my-core"
│   ├── src/lib.rs       # ~5 lines
│   └── tests/
│       └── core_test.rs # ~3 lines
└── shared/
    ├── Cargo.toml       # [package] name = "my-shared"
    └── src/lib.rs       # ~3 lines
```

#### New Specs (in `tests/specs/cloc_cmd.rs`)

```rust
// =============================================================================
// Per-package output
// =============================================================================

/// `quench cloc` shows per-package breakdown when packages are configured
#[test]
fn cloc_cmd_shows_package_breakdown() {
    let mut cmd = quench_cmd();
    cmd.arg("cloc");
    cmd.current_dir(fixture("cloc-cmd-packages"));
    let output = cmd.output().expect("command should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Package"), "should have Package header");
    assert!(stdout.contains("my-cli"), "should show cli package");
    assert!(stdout.contains("my-core"), "should show core package");
    assert!(stdout.contains("my-shared"), "should show shared package");
}

/// `quench cloc --output json` includes packages object
#[test]
fn cloc_cmd_json_includes_packages() {
    let mut cmd = quench_cmd();
    cmd.args(["cloc", "--output", "json"]);
    cmd.current_dir(fixture("cloc-cmd-packages"));
    let output = cmd.output().expect("command should run");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let packages = json.get("packages").expect("should have packages object");
    assert!(packages.get("my-cli").is_some(), "should have cli package");
    assert!(packages.get("my-core").is_some(), "should have core package");
    assert!(packages.get("my-shared").is_some(), "should have shared package");
}

/// Per-package JSON contains source, test, and ratio fields
#[test]
fn cloc_cmd_json_package_fields() {
    let mut cmd = quench_cmd();
    cmd.args(["cloc", "--output", "json"]);
    cmd.current_dir(fixture("cloc-cmd-packages"));
    let output = cmd.output().expect("command should run");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let core = &json["packages"]["my-core"];
    assert!(core.get("source").is_some(), "should have source stats");
    assert!(core.get("test").is_some(), "should have test stats");
    assert!(core.get("ratio").is_some(), "should have ratio");
    // core has tests, so test files > 0
    let test_files = core["test"]["files"].as_u64().unwrap();
    assert!(test_files >= 1, "core should have at least 1 test file");
}

/// Packages are omitted from JSON when no packages configured
#[test]
fn cloc_cmd_json_omits_packages_when_unconfigured() {
    let mut cmd = quench_cmd();
    cmd.args(["cloc", "--output", "json"]);
    cmd.current_dir(fixture("cloc-cmd"));
    let output = cmd.output().expect("command should run");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    // cloc-cmd fixture has no packages configured and is a single crate
    assert!(json.get("packages").is_none(),
        "should not have packages when unconfigured");
}
```

**Milestone:** All new specs pass. `make check` green.

## Key Implementation Details

### Package name resolution

Package names follow the same convention as the check command: `config.project.package_names` maps from path → display name. When auto-detecting Rust workspaces, the display name comes from `[package] name` in each crate's `Cargo.toml`. For JS workspaces, it comes from `package.json` `name` field. For Go, it uses the directory basename. If no name is found, the path itself is used.

### Unpackaged files

Files that don't belong to any configured package are tracked as an `(unpackaged)` category. This accounts for files at the workspace root (e.g., `build.rs`, root-level tests, scripts). This row is only shown if there are both packaged and unpackaged files — a project with no packages configured shows no package breakdown at all.

### Rust cfg_test splitting in per-package context

When `cfg_test_split = "count"`, a single Rust source file may contribute to both source and test line counts. For per-package tracking, the same proportional split that already applies to language-level stats also applies to package-level stats. The file is assigned to exactly one package (the first matching package prefix), and its split source/test counts go into that package's stats.

### No `--by-package` flag

The per-package breakdown is shown automatically when packages are configured or auto-detected. There is no flag to toggle it. This follows the convention-over-configuration principle: if the project is a monorepo, the user probably wants per-package data.

### Consistency with check command

The cloc command's per-package JSON uses a slightly different structure than the check's `by_package` — the cloc command includes full `source`/`test` sub-objects with `files`, `blank`, `comment`, `code` (matching the `totals` structure), while the check's `by_package` has flat `source_lines`, `source_files`, etc. This is intentional: the cloc command provides richer line-type breakdowns (blank/comment/code), while the check only tracks aggregate nonblank lines.

### No cache interaction

The `quench cloc` command does not use the check cache. Per-package data is computed fresh on every run. No `CACHE_VERSION` bump needed.

## Verification Plan

### Unit tests

- `file_package()` function: test with various path/package combinations
- Package stats accumulation: verify source/test splitting per package

### Behavioral tests (specs)

- Per-package text output includes package table section
- Per-package JSON output has `packages` object with expected structure
- Package names use display names from `package_names` mapping
- Packages omitted when not configured
- Auto-detected Rust workspace shows package breakdown
- Go project with multiple packages shows package breakdown

### Manual verification

```bash
# Rust workspace
cd /path/to/rust-workspace
quench cloc
quench cloc --output json | jq '.packages'

# JS monorepo
cd /path/to/js-monorepo
quench cloc

# Go project
cd /path/to/go-project
quench cloc

# Single-package project (no package breakdown)
cd /path/to/single-crate
quench cloc --output json | jq 'has("packages")'  # false
```

### Regression

```bash
make check  # fmt, clippy, test, build, audit, deny
```
