# Go Test Runner Coverage Collection and Reporting

Phase 477 - Implement Go coverage collection for the `go` test runner.

## Overview

Add coverage collection support to the existing Go test runner. The runner already executes `go test -json ./...` and parses test results. This plan adds:

1. Coverage profile generation using `go test -coverprofile`
2. Parsing of Go's coverage profile format
3. Per-file and per-package coverage aggregation
4. Integration with the existing `TestRunResult` coverage API

## Project Structure

```
crates/cli/src/checks/tests/runners/
├── mod.rs                 # MODIFY: export Go coverage, add to AggregatedCoverage
├── go.rs                  # MODIFY: integrate coverage collection
├── go_coverage.rs         # NEW: coverage profile parsing
├── go_coverage_tests.rs   # NEW: unit tests
└── coverage.rs            # REFERENCE: Rust coverage pattern

tests/fixtures/
├── go-simple/             # Existing: single-package Go project
└── go-multi/              # Existing: multi-package Go project
```

## Dependencies

No new external dependencies required. Uses:

- `std::process::Command` - execute `go test`
- `std::sync::OnceLock` - cache Go availability
- `std::collections::HashMap` - per-file/package coverage maps

## Implementation Phases

### Phase 1: Coverage Profile Parsing

**Goal**: Parse Go's coverage profile format into `CoverageResult`.

**Files**:
- Create `crates/cli/src/checks/tests/runners/go_coverage.rs`
- Create `crates/cli/src/checks/tests/runners/go_coverage_tests.rs`

**Key Implementation**:

Go coverage profile format:
```
mode: set
github.com/example/pkg/math/math.go:5.14,7.2 1 1
github.com/example/pkg/math/math.go:9.14,11.2 1 0
```

Format: `<file>:<startLine>.<startCol>,<endLine>.<endCol> <numStatements> <count>`

```rust
// go_coverage.rs

/// Parse Go coverage profile format.
/// Returns per-file coverage as (covered_statements / total_statements) * 100.
pub fn parse_cover_profile(content: &str) -> CoverageResult {
    // Skip mode line, aggregate by file
    let mut file_stats: HashMap<String, (u64, u64)> = HashMap::new(); // (covered, total)

    for line in content.lines().skip(1) {
        // Parse: file:start,end statements count
        if let Some((file, statements, count)) = parse_profile_line(line) {
            let entry = file_stats.entry(file).or_default();
            entry.1 += statements;
            if count > 0 {
                entry.0 += statements;
            }
        }
    }
    // Convert to percentages, group by package
}
```

**Verification**:
- Unit tests in `go_coverage_tests.rs`:
  - `test_parse_empty_profile`
  - `test_parse_single_file`
  - `test_parse_multi_file`
  - `test_parse_zero_coverage`
  - `test_parse_full_coverage`
  - `test_package_extraction`

### Phase 2: Coverage Collection

**Goal**: Execute `go test -coverprofile` and return coverage data.

**Files**:
- Extend `crates/cli/src/checks/tests/runners/go_coverage.rs`

**Key Implementation**:

```rust
// go_coverage.rs

static GO_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if Go is available (cached).
pub fn go_available() -> bool {
    *GO_AVAILABLE.get_or_init(|| {
        Command::new("go")
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

/// Collect Go coverage for a project.
pub fn collect_go_coverage(root: &Path, path: Option<&str>) -> CoverageResult {
    if !go_available() {
        return CoverageResult::skipped();
    }

    let start = Instant::now();

    // Create temp file for coverage profile
    let cover_file = root.join(".quench-coverage.out");

    let mut cmd = Command::new("go");
    cmd.args(["test", "-coverprofile"]);
    cmd.arg(&cover_file);
    cmd.arg(path.unwrap_or("./..."));
    cmd.current_dir(root);

    // Execute and parse profile
    let output = cmd.output()?;
    let content = std::fs::read_to_string(&cover_file)?;
    std::fs::remove_file(&cover_file).ok(); // Cleanup

    parse_cover_profile(&content)
}
```

**Verification**:
- Integration test with `go-simple` fixture
- Verify coverage file cleanup

### Phase 3: Runner Integration

**Goal**: Integrate coverage collection into `GoRunner::run()`.

**Files**:
- Modify `crates/cli/src/checks/tests/runners/go.rs`
- Modify `crates/cli/src/checks/tests/runners/mod.rs`

**Key Implementation**:

```rust
// go.rs - add to run() method after parsing test results

if ctx.collect_coverage {
    let coverage = collect_go_coverage(ctx.root, config.path.as_deref());
    if let Some(line_coverage) = coverage.line_coverage {
        let mut cov_map = HashMap::new();
        cov_map.insert("go".to_string(), line_coverage);
        result = result.with_coverage(cov_map);
    }
    if !coverage.packages.is_empty() {
        result = result.with_package_coverage(coverage.packages);
    }
}
```

```rust
// mod.rs - extend AggregatedCoverage

pub struct AggregatedCoverage {
    pub rust: Option<CoverageResult>,
    pub shell: Option<CoverageResult>,
    pub go: Option<CoverageResult>,  // NEW
}

impl AggregatedCoverage {
    pub fn merge_go(&mut self, result: CoverageResult) { ... }

    pub fn to_coverage_map(&self) -> HashMap<String, f64> {
        // Add Go to the map
        if let Some(ref go) = self.go && let Some(pct) = go.line_coverage {
            map.insert("go".to_string(), pct);
        }
    }
}
```

**Verification**:
- Run `cargo test` with all runner tests
- Verify `go-simple` fixture reports coverage

### Phase 4: Package Coverage Extraction

**Goal**: Extract per-package coverage for ratcheting support.

**Files**:
- Extend `crates/cli/src/checks/tests/runners/go_coverage.rs`

**Key Implementation**:

Go module paths use forward slashes. Extract package from file path:

```rust
/// Extract Go package name from file path.
///
/// Examples:
/// - "github.com/user/repo/pkg/math/math.go" -> "pkg/math"
/// - "github.com/user/repo/internal/core/core.go" -> "internal/core"
/// - "github.com/user/repo/main.go" -> "root"
fn extract_go_package(path: &str) -> String {
    // Find common patterns: pkg/, internal/, cmd/
    for marker in ["pkg/", "internal/", "cmd/"] {
        if let Some(idx) = path.find(marker) {
            // Extract up to filename
            let package_path = &path[idx..];
            if let Some(file_idx) = package_path.rfind('/') {
                return package_path[..file_idx].to_string();
            }
        }
    }
    "root".to_string()
}
```

**Verification**:
- Unit tests for package extraction
- `go-multi` fixture shows per-package coverage

### Phase 5: Behavioral Tests

**Goal**: Add spec tests for Go coverage behavior.

**Files**:
- Extend `tests/specs/tests_go.rs` (or create if needed)

**Test Cases**:

```rust
#[test]
fn go_runner_collects_coverage() {
    cli()
        .on("go-simple")
        .args(["check", "--ci", "--tests"])
        .succeeds()
        .stdout_has("coverage:")
        .stdout_has("go:");
}

#[test]
fn go_coverage_per_package() {
    cli()
        .on("go-multi")
        .args(["check", "--ci", "--tests"])
        .succeeds()
        .stdout_has("pkg/api:")
        .stdout_has("pkg/storage:")
        .stdout_has("internal/core:");
}

#[test]
fn go_coverage_respects_min_threshold() {
    cli()
        .on("go-simple")
        .with_config(r#"
            [[check.tests.suite]]
            runner = "go"

            [check.tests.coverage]
            check = "error"
            min = 100.0
        "#)
        .fails()
        .stdout_has("coverage: FAIL");
}
```

**Verification**:
- `cargo test --all` passes
- Behavioral tests match spec

## Key Implementation Details

### Coverage Profile Line Parsing

The Go coverage profile format requires careful parsing:

```rust
fn parse_profile_line(line: &str) -> Option<(String, u64, u64)> {
    // Format: file:startLine.startCol,endLine.endCol statements count
    // Example: github.com/user/repo/pkg/math.go:5.14,7.2 1 1

    let colon_idx = line.rfind(':')?;
    let file = &line[..colon_idx];
    let rest = &line[colon_idx + 1..];

    // Split "5.14,7.2 1 1" into parts
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() != 3 {
        return None;
    }

    let statements: u64 = parts[1].parse().ok()?;
    let count: u64 = parts[2].parse().ok()?;

    Some((file.to_string(), statements, count))
}
```

### Coverage File Cleanup

Use a dedicated file name to avoid conflicts:

```rust
let cover_file = root.join(".quench-coverage.out");
// ... execute tests ...
if cover_file.exists() {
    std::fs::remove_file(&cover_file).ok();
}
```

The file is excluded from git via `.gitignore` pattern `.quench-*`.

### Module Path Normalization

Go reports full module paths in coverage profiles. Normalize for display:

```rust
fn normalize_go_path(path: &str) -> String {
    // Remove module prefix: "github.com/user/repo/" -> ""
    // Keep relative path: "pkg/math/math.go"

    for marker in ["pkg/", "internal/", "cmd/", "src/"] {
        if let Some(idx) = path.find(marker) {
            return path[idx..].to_string();
        }
    }

    // Fallback: use filename
    path.rsplit('/').next().unwrap_or(path).to_string()
}
```

## Verification Plan

### Unit Tests

Run after each phase:
```bash
cargo test -p quench-cli -- go_coverage
```

### Integration Tests

After Phase 3:
```bash
cargo test -p quench-cli -- go_runner
```

### Behavioral Tests

After Phase 5:
```bash
cargo test --test specs -- tests_go
```

### Manual Verification

```bash
# Build quench
cargo build --release

# Test on go-simple fixture
./target/release/quench check --ci --tests -C tests/fixtures/go-simple

# Verify coverage output shows Go percentage
# Expected: "coverage: go: XX.X%"

# Test on go-multi fixture
./target/release/quench check --ci --tests -C tests/fixtures/go-multi

# Verify per-package coverage
# Expected: "coverage: go: XX.X% (pkg/api: XX%, internal/core: XX%)"
```

### Full CI Check

Before committing:
```bash
make check
```

Ensures:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `cargo audit`
- `cargo deny check`
