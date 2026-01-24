# Checkpoint 15D: Benchmark - Ratcheting

**Plan:** `checkpoint-15d-benchmark`
**Root Feature:** `quench-ratchet`
**Depends On:** checkpoint-15c-refactor (ratcheting config complete)

## Overview

Implement the ratcheting system that prevents quality regressions while allowing gradual improvement. The config parsing infrastructure is already complete from checkpoint 15C. This checkpoint adds:

1. Baseline file I/O (read/write `.quench/baseline.json`)
2. Metrics aggregation from check results
3. Ratchet enforcement (current vs baseline comparison)
4. Baseline auto-update on improvement (`--fix`)

**Current State**: `RatchetConfig` parsing complete, escapes check collects metrics, no baseline enforcement yet.

**End State**:
- Baseline file created/updated by `quench check --fix`
- Ratchet violations reported when metrics regress
- Escapes metrics ratcheted by default
- `make check` passes

## Project Structure

Files to create/modify:

```
crates/cli/src/
├── baseline.rs           # NEW: Baseline file I/O and types
├── baseline_tests.rs     # NEW: Unit tests for baseline module
├── ratchet.rs            # NEW: Ratchet comparison and enforcement
├── ratchet_tests.rs      # NEW: Unit tests for ratchet module
├── main.rs               # Integrate ratchet check into run_check()
├── check.rs              # Add ratchet-related fields to CheckOutput
├── config/
│   └── ratchet.rs        # Minor updates for tolerance parsing
└── output/
    ├── text.rs           # Add ratchet section to text output
    └── json.rs           # Add ratchet section to JSON output

tests/specs/
├── ratchet.rs            # NEW: Behavioral tests for ratcheting
└── fixtures/
    └── ratchet/          # NEW: Test fixtures for ratchet scenarios
```

## Dependencies

No new external dependencies required. Uses existing:
- `serde` / `serde_json` for baseline serialization
- `chrono` for timestamps (already in Cargo.toml)

## Implementation Phases

### Phase 15D.1: Baseline File Types and I/O

**Goal**: Define baseline file format and implement read/write operations.

**Create `crates/cli/src/baseline.rs`:**

```rust
//! Baseline file I/O for ratcheting.

use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Current baseline format version.
pub const BASELINE_VERSION: u32 = 1;

/// Baseline file containing stored metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    /// Format version for forward compatibility.
    pub version: u32,

    /// Last update timestamp (ISO 8601).
    pub updated: DateTime<Utc>,

    /// Git commit hash when baseline was set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,

    /// Stored metrics.
    pub metrics: BaselineMetrics,
}

/// All tracked metrics in the baseline.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BaselineMetrics {
    /// Coverage percentage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<CoverageMetrics>,

    /// Escape hatch counts by pattern.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escapes: Option<EscapesMetrics>,

    /// Binary sizes in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_size: Option<HashMap<String, u64>>,

    /// Build times in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_time: Option<BuildTimeMetrics>,

    /// Test execution times in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_time: Option<TestTimeMetrics>,
}

/// Coverage metrics with optional per-package breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageMetrics {
    pub total: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_package: Option<HashMap<String, f64>>,
}

/// Escape hatch counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscapesMetrics {
    /// Source file escape counts by pattern name.
    pub source: HashMap<String, usize>,
    /// Test file escape counts (tracked but not ratcheted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<HashMap<String, usize>>,
}

/// Build time metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildTimeMetrics {
    pub cold: f64,
    pub hot: f64,
}

/// Test time metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTimeMetrics {
    pub total: f64,
    pub avg: f64,
    pub max: f64,
}

impl Baseline {
    /// Create a new baseline with current timestamp.
    pub fn new() -> Self {
        Self {
            version: BASELINE_VERSION,
            updated: Utc::now(),
            commit: None,
            metrics: BaselineMetrics::default(),
        }
    }

    /// Load baseline from file, returning None if not found.
    pub fn load(path: &Path) -> Result<Option<Self>, BaselineError> {
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| BaselineError::Read(e.to_string()))?;

        let baseline: Baseline = serde_json::from_str(&content)
            .map_err(|e| BaselineError::Parse(e.to_string()))?;

        // Version check for forward compatibility
        if baseline.version > BASELINE_VERSION {
            return Err(BaselineError::Version {
                found: baseline.version,
                supported: BASELINE_VERSION,
            });
        }

        Ok(Some(baseline))
    }

    /// Save baseline to file, creating parent directories if needed.
    pub fn save(&self, path: &Path) -> Result<(), BaselineError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| BaselineError::Write(e.to_string()))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| BaselineError::Serialize(e.to_string()))?;

        std::fs::write(path, content)
            .map_err(|e| BaselineError::Write(e.to_string()))?;

        Ok(())
    }

    /// Set git commit hash from current HEAD.
    pub fn with_commit(mut self, root: &Path) -> Self {
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(root)
            .output()
        {
            if output.status.success() {
                self.commit = Some(
                    String::from_utf8_lossy(&output.stdout).trim().to_string()
                );
            }
        }
        self
    }
}

/// Errors that can occur during baseline operations.
#[derive(Debug, thiserror::Error)]
pub enum BaselineError {
    #[error("failed to read baseline: {0}")]
    Read(String),

    #[error("failed to parse baseline: {0}")]
    Parse(String),

    #[error("baseline version {found} is newer than supported {supported}")]
    Version { found: u32, supported: u32 },

    #[error("failed to serialize baseline: {0}")]
    Serialize(String),

    #[error("failed to write baseline: {0}")]
    Write(String),
}
```

**Milestone**: Baseline file can be read/written with all metric types.

---

### Phase 15D.2: Metrics Extraction from Check Results

**Goal**: Extract ratchetable metrics from check results.

**Add to `crates/cli/src/ratchet.rs`:**

```rust
//! Ratchet enforcement and metrics comparison.

use std::collections::HashMap;

use crate::baseline::{Baseline, BaselineMetrics, EscapesMetrics as BaselineEscapes};
use crate::check::CheckOutput;
use crate::config::RatchetConfig;

/// Current metrics extracted from check results.
#[derive(Debug, Clone, Default)]
pub struct CurrentMetrics {
    pub escapes: Option<EscapesCurrent>,
    // Coverage and timing metrics added in future phases
}

/// Current escape metrics extracted from check output.
#[derive(Debug, Clone)]
pub struct EscapesCurrent {
    pub source: HashMap<String, usize>,
    pub test: HashMap<String, usize>,
}

impl CurrentMetrics {
    /// Extract metrics from check output.
    pub fn from_output(output: &CheckOutput) -> Self {
        let mut metrics = Self::default();

        // Find escapes check result and extract metrics
        if let Some(escapes_result) = output.checks.iter().find(|c| c.name == "escapes") {
            if let Some(ref metrics_json) = escapes_result.metrics {
                metrics.escapes = extract_escapes_metrics(metrics_json);
            }
        }

        metrics
    }
}

fn extract_escapes_metrics(json: &serde_json::Value) -> Option<EscapesCurrent> {
    let source = json.get("source")?.as_object()?;
    let test = json.get("test")?.as_object()?;

    let source_map: HashMap<String, usize> = source
        .iter()
        .filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n as usize)))
        .collect();

    let test_map: HashMap<String, usize> = test
        .iter()
        .filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n as usize)))
        .collect();

    Some(EscapesCurrent {
        source: source_map,
        test: test_map,
    })
}
```

**Milestone**: Metrics can be extracted from `CheckOutput` for comparison.

---

### Phase 15D.3: Ratchet Comparison Logic

**Goal**: Compare current metrics against baseline and detect regressions.

**Extend `crates/cli/src/ratchet.rs`:**

```rust
/// Result of ratchet comparison.
#[derive(Debug, Clone)]
pub struct RatchetResult {
    /// Whether all ratcheted metrics pass.
    pub passed: bool,

    /// Individual metric comparison results.
    pub comparisons: Vec<MetricComparison>,

    /// Metrics that improved (for baseline update).
    pub improvements: Vec<MetricImprovement>,
}

/// Comparison of a single metric.
#[derive(Debug, Clone)]
pub struct MetricComparison {
    pub name: String,
    pub current: f64,
    pub baseline: f64,
    pub tolerance: f64,
    pub min_allowed: f64,
    pub passed: bool,
    pub improved: bool,
}

/// A metric that improved from baseline.
#[derive(Debug, Clone)]
pub struct MetricImprovement {
    pub name: String,
    pub old_value: f64,
    pub new_value: f64,
}

/// Compare current metrics against baseline using ratchet config.
pub fn compare(
    current: &CurrentMetrics,
    baseline: &BaselineMetrics,
    config: &RatchetConfig,
) -> RatchetResult {
    let mut comparisons = Vec::new();
    let mut improvements = Vec::new();
    let mut passed = true;

    // Compare escapes if enabled
    if config.escapes {
        if let (Some(curr), Some(base)) = (&current.escapes, &baseline.escapes) {
            for (pattern, &curr_count) in &curr.source {
                let base_count = base.source.get(pattern).copied().unwrap_or(0);

                // Escapes ratchet down (lower is better)
                let comparison = MetricComparison {
                    name: format!("escapes.{}", pattern),
                    current: curr_count as f64,
                    baseline: base_count as f64,
                    tolerance: 0.0, // No tolerance for counts
                    min_allowed: base_count as f64, // Can't exceed baseline
                    passed: curr_count <= base_count,
                    improved: curr_count < base_count,
                };

                if !comparison.passed {
                    passed = false;
                }

                if comparison.improved {
                    improvements.push(MetricImprovement {
                        name: format!("escapes.{}", pattern),
                        old_value: base_count as f64,
                        new_value: curr_count as f64,
                    });
                }

                comparisons.push(comparison);
            }
        }
    }

    // Coverage comparison would go here (ratchets up - higher is better)
    // Binary size comparison would go here (ratchets down - smaller is better)
    // Build/test time comparisons would go here (ratchet down - faster is better)

    RatchetResult {
        passed,
        comparisons,
        improvements,
    }
}

/// Update baseline with current metrics where improved.
pub fn update_baseline(
    baseline: &mut Baseline,
    current: &CurrentMetrics,
    improvements: &[MetricImprovement],
) {
    // Update escapes metrics
    if let Some(curr_escapes) = &current.escapes {
        let base_escapes = baseline.metrics.escapes.get_or_insert_with(|| {
            BaselineEscapes {
                source: HashMap::new(),
                test: None,
            }
        });

        // Update all source counts (baseline is always current snapshot)
        for (pattern, &count) in &curr_escapes.source {
            base_escapes.source.insert(pattern.clone(), count);
        }

        // Optionally track test counts
        if !curr_escapes.test.is_empty() {
            base_escapes.test = Some(curr_escapes.test.clone());
        }
    }

    // Update timestamp
    baseline.updated = chrono::Utc::now();
}
```

**Milestone**: Ratchet comparison produces pass/fail with details.

---

### Phase 15D.4: Integration into Main Check Flow

**Goal**: Integrate ratchet enforcement into `run_check()`.

**Update `crates/cli/src/main.rs`:**

```rust
// After running checks and creating output:
let output = json::create_output(check_results);

// Load baseline if ratcheting is enabled
let baseline_path = root.join(&config.git.baseline);
let ratchet_result = if config.ratchet.check != CheckLevel::Off {
    match Baseline::load(&baseline_path)? {
        Some(baseline) => {
            let current = CurrentMetrics::from_output(&output);
            Some(ratchet::compare(&current, &baseline.metrics, &config.ratchet))
        }
        None => {
            // No baseline yet - pass but suggest creating one
            if args.verbose {
                eprintln!("No baseline found at {}. Run with --fix to create.",
                    baseline_path.display());
            }
            None
        }
    }
} else {
    None
};

// Handle --fix: update baseline when metrics improve
if args.fix {
    if let Some(ref result) = ratchet_result {
        if !result.improvements.is_empty() || !baseline_path.exists() {
            let mut baseline = Baseline::load(&baseline_path)?
                .unwrap_or_else(Baseline::new)
                .with_commit(&root);

            let current = CurrentMetrics::from_output(&output);
            ratchet::update_baseline(&mut baseline, &current, &result.improvements);
            baseline.save(&baseline_path)?;

            // Report what was updated
            println!("ratchet: updated baseline");
            for improvement in &result.improvements {
                println!("  {}: {} -> {} (new ceiling)",
                    improvement.name,
                    improvement.old_value as i64,
                    improvement.new_value as i64);
            }
        }
    } else if !baseline_path.exists() {
        // Create initial baseline
        let current = CurrentMetrics::from_output(&output);
        let mut baseline = Baseline::new().with_commit(&root);
        ratchet::update_baseline(&mut baseline, &current, &[]);
        baseline.save(&baseline_path)?;
        println!("ratchet: created initial baseline at {}", baseline_path.display());
    }
}

// Determine exit code considering ratchet result
let ratchet_failed = ratchet_result.as_ref().is_some_and(|r| !r.passed);
let exit_code = if args.dry_run {
    ExitCode::Success
} else if !output.passed || ratchet_failed {
    ExitCode::CheckFailed
} else {
    ExitCode::Success
};
```

**Milestone**: `quench check` loads baseline and reports ratchet status.

---

### Phase 15D.5: Output Formatting for Ratchet Results

**Goal**: Display ratchet status in text and JSON output.

**Update `crates/cli/src/output/text.rs`:**

```rust
impl TextFormatter {
    /// Write ratchet comparison results.
    pub fn write_ratchet(&mut self, result: &RatchetResult) -> std::io::Result<()> {
        if result.passed {
            // Only show ratchet section if there were comparisons
            if !result.comparisons.is_empty() {
                self.write_check_name("ratchet", true)?;
                for comp in &result.comparisons {
                    if comp.improved {
                        writeln!(self.writer, "  {}: {} (baseline: {}) improved",
                            comp.name, comp.current as i64, comp.baseline as i64)?;
                    }
                }
            }
        } else {
            self.write_check_name("ratchet", false)?;
            for comp in &result.comparisons {
                if !comp.passed {
                    writeln!(self.writer, "  {}: {} (max: {} from baseline)",
                        comp.name, comp.current as i64, comp.baseline as i64)?;
                    writeln!(self.writer, "    Escape hatch count increased. Clean up or update baseline.")?;
                }
            }
        }
        Ok(())
    }
}
```

**Update JSON output to include ratchet section in `CheckOutput`:**

```rust
// In check.rs or output/json.rs
#[derive(Debug, Clone, Serialize)]
pub struct CheckOutput {
    pub timestamp: String,
    pub passed: bool,
    pub checks: Vec<CheckResult>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ratchet: Option<RatchetOutput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RatchetOutput {
    pub passed: bool,
    pub comparisons: Vec<RatchetComparison>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RatchetComparison {
    pub name: String,
    pub current: f64,
    pub baseline: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f64>,
    pub min_allowed: f64,
    pub passed: bool,
    pub improved: bool,
}
```

**Milestone**: Ratchet results displayed in text and JSON formats.

---

### Phase 15D.6: Testing and Quality Gates

**Goal**: Add behavioral tests and ensure all quality gates pass.

**Create `tests/specs/ratchet.rs`:**

```rust
//! Behavioral tests for ratcheting.

use crate::prelude::*;

#[test]
fn no_baseline_passes_with_suggestion() {
    let project = TempProject::new()
        .file("quench.toml", r#"
version = 1

[ratchet]
check = "error"
escapes = true
"#)
        .file("src/lib.rs", "fn main() {}");

    cli()
        .on(&project)
        .arg("check")
        .succeeds()
        .stderr_has("No baseline found");
}

#[test]
fn fix_creates_baseline() {
    let project = TempProject::new()
        .file("quench.toml", r#"
version = 1

[ratchet]
check = "error"
escapes = true
"#)
        .file("src/lib.rs", "fn main() { unsafe {} }");

    cli()
        .on(&project)
        .arg("check")
        .arg("--fix")
        .succeeds()
        .stdout_has("created initial baseline");

    assert!(project.path(".quench/baseline.json").exists());
}

#[test]
fn regression_fails() {
    let project = TempProject::new()
        .file("quench.toml", r#"
version = 1

[ratchet]
check = "error"
escapes = true
"#)
        .file(".quench/baseline.json", r#"{
  "version": 1,
  "updated": "2026-01-20T00:00:00Z",
  "metrics": {
    "escapes": {
      "source": { "unsafe": 1 }
    }
  }
}"#)
        .file("src/lib.rs", "fn main() { unsafe {} unsafe {} }");

    cli()
        .on(&project)
        .arg("check")
        .fails()
        .stdout_has("escapes.unsafe: 2 (max: 1 from baseline)");
}

#[test]
fn improvement_updates_baseline() {
    let project = TempProject::new()
        .file("quench.toml", r#"
version = 1

[ratchet]
check = "error"
escapes = true
"#)
        .file(".quench/baseline.json", r#"{
  "version": 1,
  "updated": "2026-01-20T00:00:00Z",
  "metrics": {
    "escapes": {
      "source": { "unsafe": 5 }
    }
  }
}"#)
        .file("src/lib.rs", "fn main() { unsafe {} unsafe {} }");

    cli()
        .on(&project)
        .arg("check")
        .arg("--fix")
        .succeeds()
        .stdout_has("escapes.unsafe: 5 -> 2 (new ceiling)");
}
```

**Run quality gates:**

```bash
# Full test suite
cargo test --all

# Ratchet-specific tests
cargo test ratchet
cargo test --test specs ratchet

# Full quality check
make check
```

**Milestone**: All tests pass, `make check` passes.

---

## Key Implementation Details

### Escapes Metrics Flow

1. `EscapesCheck::run()` collects counts into `EscapesMetrics`
2. Metrics returned via `CheckResult::with_metrics()`
3. `CurrentMetrics::from_output()` extracts from JSON
4. `ratchet::compare()` compares against baseline
5. Regression triggers failure, improvement triggers update

### Baseline Update Rules

| Condition | Baseline Updated? | Exit Code |
|-----------|-------------------|-----------|
| No baseline, no --fix | No | Pass |
| No baseline, --fix | Yes (create) | Pass |
| Regression, no --fix | No | Fail |
| Regression, --fix | No | Fail |
| Improvement, no --fix | No | Pass |
| Improvement, --fix | Yes (update) | Pass |
| Same as baseline | No | Pass |

### Ratchet Direction by Metric

| Metric | Good Direction | Comparison |
|--------|----------------|------------|
| Coverage | Higher | `current >= baseline - tolerance` |
| Escapes | Lower | `current <= baseline` |
| Binary size | Smaller | `current <= baseline + tolerance` |
| Build time | Faster | `current <= baseline + tolerance` |
| Test time | Faster | `current <= baseline + tolerance` |

### Tolerance Parsing

```rust
// In config/ratchet.rs, add parsing helpers:
impl RatchetConfig {
    /// Parse binary size tolerance (e.g., "100KB" -> bytes).
    pub fn binary_size_tolerance_bytes(&self) -> Option<u64> {
        self.binary_size_tolerance.as_ref().map(|s| parse_size(s))
    }

    /// Parse time tolerance (e.g., "5s" -> seconds).
    pub fn time_tolerance_secs(&self) -> Option<f64> {
        self.build_time_tolerance.as_ref().map(|s| parse_duration(s))
    }
}
```

### Per-Package Ratcheting (Future)

The baseline format supports per-package metrics:

```json
{
  "metrics": {
    "escapes": {
      "source": { "unsafe": 10 },
      "by_package": {
        "core": { "source": { "unsafe": 3 } },
        "cli": { "source": { "unsafe": 7 } }
      }
    }
  }
}
```

Config allows per-package overrides:

```toml
[ratchet.package.core]
escapes = true

[ratchet.package.cli]
escapes = false  # Still experimental
```

This is prepared in the data structures but enforcement deferred to a future checkpoint.

## Verification Plan

### Automated Verification

```bash
# Unit tests
cargo test baseline
cargo test ratchet

# Behavioral tests
cargo test --test specs ratchet

# Full suite
cargo test --all

# Quality gates
make check
```

### Manual Verification

```bash
# Test no baseline scenario
TEMP=$(mktemp -d) && cd "$TEMP"
cargo init --lib
cat > quench.toml << 'EOF'
version = 1
[ratchet]
check = "error"
escapes = true
EOF
echo "fn main() { unsafe {} }" > src/lib.rs
quench check && echo "PASS: no baseline passes"
quench check --fix && ls -la .quench/baseline.json && echo "PASS: baseline created"
cd - && rm -rf "$TEMP"

# Test regression detection
TEMP=$(mktemp -d) && cd "$TEMP"
cargo init --lib
cat > quench.toml << 'EOF'
version = 1
[ratchet]
check = "error"
escapes = true
EOF
mkdir -p .quench
echo '{"version":1,"updated":"2026-01-20T00:00:00Z","metrics":{"escapes":{"source":{"unsafe":1}}}}' > .quench/baseline.json
echo "fn main() { unsafe {} unsafe {} }" > src/lib.rs
quench check || echo "PASS: regression detected"
cd - && rm -rf "$TEMP"

# Test improvement update
TEMP=$(mktemp -d) && cd "$TEMP"
cargo init --lib
cat > quench.toml << 'EOF'
version = 1
[ratchet]
check = "error"
escapes = true
EOF
mkdir -p .quench
echo '{"version":1,"updated":"2026-01-20T00:00:00Z","metrics":{"escapes":{"source":{"unsafe":5}}}}' > .quench/baseline.json
echo "fn main() { unsafe {} }" > src/lib.rs
quench check --fix && grep '"unsafe": 1' .quench/baseline.json && echo "PASS: baseline updated"
cd - && rm -rf "$TEMP"
```

### Success Criteria

- [ ] Baseline file format matches spec (`docs/specs/04-ratcheting.md`)
- [ ] `quench check` loads baseline and compares metrics
- [ ] Regression causes check failure with clear message
- [ ] `quench check --fix` creates baseline if missing
- [ ] `quench check --fix` updates baseline on improvement
- [ ] Escapes metrics ratcheted by default
- [ ] Ratchet disabled respects `check = "off"`
- [ ] JSON output includes ratchet section
- [ ] Text output shows ratchet violations
- [ ] All existing tests pass (411+)
- [ ] `make check` passes
