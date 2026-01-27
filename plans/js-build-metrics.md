# JavaScript Bundle Size and Build Time Metrics (Phase 499)

## Overview

Extend the existing `build` check to support JavaScript/TypeScript projects by:

1. Detecting the bundler in use (Vite, Webpack, esbuild, Rollup, Next.js, Parcel)
2. Measuring bundle sizes (raw and gzipped)
3. Measuring build times (cold and hot builds)
4. Supporting per-target thresholds for JavaScript outputs

The build check already works for Rust (cargo) and Go (go build). This phase adds JavaScript support by detecting bundler configuration, running `npm run build`, and measuring the output artifacts.

## Project Structure

```
crates/cli/src/
├── checks/
│   └── build/
│       ├── mod.rs              # Main build check (extend for JS)
│       ├── mod_tests.rs        # Unit tests
│       └── javascript.rs       # NEW: JS-specific logic
├── adapter/
│   └── javascript/
│       ├── mod.rs              # Existing adapter
│       └── bundler.rs          # NEW: Bundler detection
└── config/
    └── javascript.rs           # Extend with build config

tests/
├── fixtures/javascript/
│   ├── build-vite/             # NEW: Vite fixture
│   ├── build-webpack/          # NEW: Webpack fixture
│   ├── build-esbuild/          # NEW: esbuild fixture
│   └── build-custom-targets/   # NEW: Explicit targets
└── specs/checks/
    └── build.rs                # Add JS behavioral specs
```

## Dependencies

No new external crates required. Uses existing:
- `std::process::Command` for running npm scripts
- `flate2` (already available) for gzip compression measurement
- `serde_json` for parsing bundler configs

## Implementation Phases

### Phase 1: Bundler Detection

**Goal**: Detect which bundler a JavaScript project uses.

**Files to create/modify**:
- `crates/cli/src/adapter/javascript/bundler.rs` (new)
- `crates/cli/src/adapter/javascript/mod.rs` (re-export)

**Bundler detection order** (first match wins):
1. **Vite**: `vite.config.ts`, `vite.config.js`, `vite.config.mjs`
2. **Webpack**: `webpack.config.js`, `webpack.config.ts`, `webpack.config.cjs`
3. **esbuild**: `esbuild.config.js`, `esbuild.config.mjs`, or `package.json` with `esbuild` in scripts
4. **Rollup**: `rollup.config.js`, `rollup.config.ts`, `rollup.config.mjs`
5. **Next.js**: `next.config.js`, `next.config.mjs`, `next.config.ts`
6. **Parcel**: `.parcelrc` or `package.json` with `parcel` in devDependencies

**Output**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bundler {
    Vite,
    Webpack,
    Esbuild,
    Rollup,
    NextJs,
    Parcel,
    Unknown,
}

pub fn detect_bundler(root: &Path) -> Bundler { ... }
```

**Verification**:
- Unit tests for each bundler detection case
- Test fixture: `tests/fixtures/javascript/bundler-detect/`

---

### Phase 2: JavaScript Target Detection

**Goal**: Determine which output files to measure.

**Files to modify**:
- `crates/cli/src/checks/build/javascript.rs` (new)
- `crates/cli/src/config/javascript.rs` (extend)

**Target resolution priority**:
1. Explicit config: `[check.build] targets = ["dist/index.js"]`
2. Bundler-specific detection:
   - **Vite**: Parse `vite.config.*` for `build.rollupOptions.output` or default `dist/`
   - **Webpack**: Parse `webpack.config.*` for `output.filename` or default `dist/`
   - **Next.js**: `.next/static/chunks/` (skip, complex) or require explicit targets
   - **Generic**: Scan `dist/`, `build/`, `out/` for `.js` files
3. Fallback: All `.js` files in output directory (excluding source maps)

**Configuration extension**:
```toml
[javascript]
# Explicit targets override auto-detection
targets = ["dist/app.js", "dist/vendor.js"]

# Or specify output directory for auto-detection
output_dir = "dist"
```

**Verification**:
- Unit tests for target resolution
- Test fixture with explicit targets

---

### Phase 3: Bundle Size Measurement

**Goal**: Measure raw and gzipped size of JavaScript bundles.

**Files to modify**:
- `crates/cli/src/checks/build/javascript.rs`
- `crates/cli/src/checks/build/mod.rs`

**Implementation**:
```rust
pub struct BundleSize {
    pub raw: u64,
    pub gzipped: u64,
}

pub fn measure_bundle_size(path: &Path) -> io::Result<BundleSize> {
    let content = fs::read(path)?;
    let raw = content.len() as u64;

    // Gzip with default compression
    let mut encoder = flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::default()
    );
    encoder.write_all(&content)?;
    let gzipped = encoder.finish()?.len() as u64;

    Ok(BundleSize { raw, gzipped })
}
```

**Size threshold checking**:
- Config can specify `size_max` (raw) or `size_gzip_max` (gzipped)
- Default comparison uses raw size (consistent with Rust/Go)
- Source map files (`.js.map`) are excluded from measurement

**Metrics output**:
```json
{
  "size": {
    "dist/app.js": 524288,
    "dist/vendor.js": 1048576
  },
  "size_gzip": {
    "dist/app.js": 156789,
    "dist/vendor.js": 312456
  }
}
```

**Verification**:
- Unit test: create temp file, verify size calculation
- Test fixture with pre-built bundle

---

### Phase 4: Build Time Measurement

**Goal**: Measure cold and hot build times.

**Files to modify**:
- `crates/cli/src/checks/build/javascript.rs`
- `crates/cli/src/checks/build/mod.rs`

**Implementation**:

```rust
pub fn measure_cold_build(root: &Path, output_dir: &str) -> io::Result<Duration> {
    // Clean output directory
    let output_path = root.join(output_dir);
    if output_path.exists() {
        fs::remove_dir_all(&output_path)?;
    }

    // Run build
    let start = Instant::now();
    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir(root)
        .status()?;

    if !status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "build failed"));
    }

    Ok(start.elapsed())
}

pub fn measure_hot_build(root: &Path) -> io::Result<Duration> {
    // Build already exists, just re-run (bundler cache active)
    let start = Instant::now();
    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir(root)
        .status()?;

    if !status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "build failed"));
    }

    Ok(start.elapsed())
}
```

**Build script detection**:
- Check `package.json` for `scripts.build`
- If missing, skip build time measurement (size-only mode)

**Verification**:
- Test fixture with minimal Vite project
- Behavioral test: verify times are measured and reported

---

### Phase 5: Integration with Build Check

**Goal**: Wire JavaScript support into the existing build check.

**Files to modify**:
- `crates/cli/src/checks/build/mod.rs`

**Changes to `BuildCheck::run()`**:
```rust
fn run(&self, ctx: &CheckContext) -> CheckResult {
    if !ctx.ci_mode {
        return CheckResult::stub(self.name());
    }

    let language = detect_language(ctx.root);

    match language {
        ProjectLanguage::Rust => self.run_rust(ctx),
        ProjectLanguage::Go => self.run_go(ctx),
        ProjectLanguage::JavaScript => self.run_javascript(ctx),  // NEW
        _ => CheckResult::stub(self.name()),
    }
}

fn run_javascript(&self, ctx: &CheckContext) -> CheckResult {
    let bundler = detect_bundler(ctx.root);
    let targets = resolve_js_targets(ctx, bundler);

    let mut violations = Vec::new();
    let mut metrics = BuildMetrics::default();

    // Measure sizes
    for target in &targets {
        let path = ctx.root.join(target);
        if !path.exists() {
            violations.push(Violation::missing_target(target));
            continue;
        }

        let size = measure_bundle_size(&path)?;
        metrics.size.insert(target.clone(), size.raw);
        metrics.size_gzip.insert(target.clone(), size.gzipped);

        // Check threshold
        if let Some(max) = get_size_max(ctx, target) {
            if size.raw > max {
                violations.push(Violation::size_exceeded(target, size.raw, max));
            }
        }
    }

    // Measure build times (if build script exists)
    if has_build_script(ctx.root) {
        if should_measure_cold(ctx) {
            metrics.time_cold = Some(measure_cold_build(...)?);
        }
        if should_measure_hot(ctx) {
            metrics.time_hot = Some(measure_hot_build(...)?);
        }
    }

    CheckResult { passed: violations.is_empty(), violations, metrics }
}
```

**Verification**:
- Behavioral tests covering full JavaScript build flow
- Test different bundler configurations

---

### Phase 6: Test Fixtures and Behavioral Specs

**Goal**: Comprehensive test coverage.

**New fixtures to create**:

1. `tests/fixtures/javascript/build-vite/`
   - Minimal Vite project
   - Pre-built `dist/` for size tests
   - `quench.toml` with size thresholds

2. `tests/fixtures/javascript/build-custom-targets/`
   - Explicit `targets` configuration
   - Multiple output files

3. `tests/fixtures/javascript/build-size-exceeded/`
   - Bundle larger than configured threshold
   - Verify FAIL output

**Behavioral specs to add** (`tests/specs/checks/build.rs`):
```rust
#[test]
fn javascript_bundle_size_passes() {
    check("build")
        .on("javascript/build-vite")
        .ci()
        .passes()
        .stdout_has("dist/index.js");
}

#[test]
fn javascript_bundle_size_exceeded() {
    check("build")
        .on("javascript/build-size-exceeded")
        .ci()
        .fails()
        .stdout_has("size_exceeded");
}

#[test]
fn javascript_reports_gzip_size() {
    check("build")
        .on("javascript/build-vite")
        .ci()
        .json()
        .passes()
        .metrics_has("size_gzip");
}
```

## Key Implementation Details

### Gzip Measurement Strategy

Use `flate2` with default compression (level 6) to simulate typical CDN/server compression:

```rust
use flate2::write::GzEncoder;
use flate2::Compression;

fn gzip_size(content: &[u8]) -> usize {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(content).unwrap();
    encoder.finish().unwrap().len()
}
```

### Source Map Exclusion

When scanning for targets, exclude files matching `*.map`:

```rust
fn is_bundle_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str());
    matches!(ext, Some("js" | "mjs" | "cjs")) &&
        !path.to_string_lossy().ends_with(".map")
}
```

### Build Script Detection

Check for build script in `package.json`:

```rust
fn has_build_script(root: &Path) -> bool {
    let pkg_path = root.join("package.json");
    if let Ok(content) = fs::read_to_string(&pkg_path) {
        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
            return pkg.get("scripts")
                .and_then(|s| s.get("build"))
                .is_some();
        }
    }
    false
}
```

### Default Output Directories

```rust
const JS_OUTPUT_DIRS: &[&str] = &["dist", "build", "out", ".next/static"];

fn find_output_dir(root: &Path) -> Option<PathBuf> {
    for dir in JS_OUTPUT_DIRS {
        let path = root.join(dir);
        if path.is_dir() {
            return Some(path);
        }
    }
    None
}
```

## Verification Plan

### Unit Tests

| Test | Location | Verifies |
|------|----------|----------|
| Bundler detection (all types) | `adapter/javascript/bundler_tests.rs` | Each bundler config detected correctly |
| Target resolution | `checks/build/javascript_tests.rs` | Explicit > auto-detected > fallback |
| Size measurement | `checks/build/javascript_tests.rs` | Raw and gzip sizes calculated correctly |
| Source map exclusion | `checks/build/javascript_tests.rs` | `.map` files not counted |

### Behavioral Tests

| Test | Fixture | Verifies |
|------|---------|----------|
| `javascript_bundle_size_passes` | `build-vite` | PASS when under threshold |
| `javascript_bundle_size_exceeded` | `build-size-exceeded` | FAIL with violation |
| `javascript_missing_target` | `build-missing` | FAIL for non-existent files |
| `javascript_gzip_metrics` | `build-vite` | `size_gzip` in JSON output |
| `javascript_custom_targets` | `build-custom-targets` | Explicit targets respected |

### Manual Testing

1. Run on a real Vite project: `quench check --ci build`
2. Verify size output matches `du -b dist/*.js`
3. Verify gzip output matches `gzip -c dist/*.js | wc -c`

### CI Integration

Ensure build check works in CI mode with `--ci` flag:
```bash
quench check --ci build
```

## Summary

| Phase | Deliverable | Files Changed |
|-------|-------------|---------------|
| 1 | Bundler detection | `adapter/javascript/bundler.rs` (new) |
| 2 | Target detection | `checks/build/javascript.rs` (new), `config/javascript.rs` |
| 3 | Size measurement | `checks/build/javascript.rs` |
| 4 | Build timing | `checks/build/javascript.rs` |
| 5 | Integration | `checks/build/mod.rs` |
| 6 | Tests | `tests/fixtures/javascript/build-*`, `tests/specs/checks/build.rs` |
