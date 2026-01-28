# Package Manager Detection Implementation Plan

Implements phases 4994-4996 from the JavaScript roadmap: detect bun.lock and other lock files, centralize PackageManager module, and update test/build commands.

## Overview

Add automatic package manager detection based on lock files (`bun.lock`, `pnpm-lock.yaml`, `yarn.lock`, `package-lock.json`) to ensure quench uses the correct package manager for test and build commands. The `PackageManager` module becomes the single source of truth for all JS/TS package manager operations.

## Project Structure

```
crates/cli/src/
├── adapter/javascript/
│   ├── mod.rs              # Add `pub use package_manager::*`
│   ├── package_manager.rs  # NEW: PackageManager enum + detection
│   └── workspace.rs        # Existing workspace detection
├── checks/
│   ├── tests/runners/
│   │   ├── js_detect.rs    # Update to use PackageManager
│   │   ├── bun.rs          # Update command generation
│   │   ├── jest.rs         # Update command generation
│   │   └── vitest.rs       # Update command generation
│   └── build/
│       └── javascript.rs   # Update build command generation
└── profiles.rs             # Update Landing the Plane items
```

## Dependencies

No new external dependencies required. Uses existing:
- `std::fs` for lock file existence checks
- `std::path::Path` for path operations

## Implementation Phases

### Phase 1: PackageManager Module (Core Detection)

**Goal:** Create centralized `PackageManager` enum with lock file detection.

**File:** `crates/cli/src/adapter/javascript/package_manager.rs`

```rust
/// JavaScript package manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PackageManager {
    #[default]
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

impl PackageManager {
    /// Detect package manager from lock files in project root.
    ///
    /// Detection order (first match wins):
    /// 1. `bun.lock` (Bun 1.2+ text format)
    /// 2. `bun.lockb` (Bun binary format)
    /// 3. `pnpm-lock.yaml`
    /// 4. `yarn.lock`
    /// 5. `package-lock.json`
    /// 6. Fallback to npm
    pub fn detect(root: &Path) -> Self {
        if root.join("bun.lock").exists() || root.join("bun.lockb").exists() {
            return Self::Bun;
        }
        if root.join("pnpm-lock.yaml").exists() {
            return Self::Pnpm;
        }
        if root.join("yarn.lock").exists() {
            return Self::Yarn;
        }
        // package-lock.json or fallback
        Self::Npm
    }

    /// Command to run a package.json script (e.g., "build").
    pub fn run_command(&self, script: &str) -> Vec<&'static str> { ... }

    /// Command to run tests.
    pub fn test_command(&self) -> Vec<&'static str> { ... }

    /// Package manager executable name.
    pub fn executable(&self) -> &'static str { ... }
}
```

**Tasks:**
- [ ] Create `package_manager.rs` with `PackageManager` enum
- [ ] Implement `detect(root: &Path)` with lock file checks
- [ ] Implement `run_command()`, `test_command()`, `executable()`
- [ ] Add `pub mod package_manager` to `adapter/javascript/mod.rs`
- [ ] Add `pub use package_manager::{PackageManager, detect_package_manager}`
- [ ] Create `package_manager_tests.rs` with detection tests

**Verification:**
```bash
cargo test package_manager
```

---

### Phase 2: Test Runner Integration

**Goal:** Update JS test runners to use detected package manager for command generation.

**Files:**
- `crates/cli/src/checks/tests/runners/js_detect.rs`
- `crates/cli/src/checks/tests/runners/bun.rs`
- `crates/cli/src/checks/tests/runners/vitest.rs`
- `crates/cli/src/checks/tests/runners/jest.rs`

**Changes to `js_detect.rs`:**
```rust
use crate::adapter::javascript::PackageManager;

/// Detect test runner, preferring package manager's native runner.
pub fn detect_js_runner(root: &Path) -> Option<DetectionResult> {
    let pkg_mgr = PackageManager::detect(root);

    // If bun is detected and installed, prefer bun test
    if pkg_mgr == PackageManager::Bun && is_bun_installed() {
        return Some(DetectionResult {
            runner: JsRunner::Bun,
            source: DetectionSource::PackageManager,
        });
    }

    // Existing detection logic for config files, dependencies, scripts
    detect_from_config(root)
        .or_else(|| detect_from_dependencies(root))
        .or_else(|| detect_from_scripts(root))
}
```

**Changes to runner implementations:**
- Update `BunRunner::run()` to use `PackageManager::Bun.test_command()`
- Update `VitestRunner::run()` to use `pkg_mgr.run_command("test")` when vitest in scripts
- Update `JestRunner::run()` similarly

**Tasks:**
- [ ] Add `DetectionSource::PackageManager` variant
- [ ] Update `detect_js_runner()` to check package manager first
- [ ] Update `BunRunner` to use `PackageManager` for command generation
- [ ] Update `VitestRunner` to use detected package manager
- [ ] Update `JestRunner` to use detected package manager
- [ ] Add tests for each lock file triggering correct runner

**Verification:**
```bash
cargo test js_detect
cargo test bun
cargo test vitest
cargo test jest
```

---

### Phase 3: Build Command Integration

**Goal:** Update build metrics to use detected package manager.

**File:** `crates/cli/src/checks/build/javascript.rs`

**Current behavior:** Hardcodes `npm run build` or similar.

**New behavior:**
```rust
use crate::adapter::javascript::PackageManager;

fn build_command(root: &Path, script: &str) -> Command {
    let pkg_mgr = PackageManager::detect(root);
    let args = pkg_mgr.run_command(script);

    let mut cmd = Command::new(args[0]);
    cmd.args(&args[1..]);
    cmd
}
```

**Tasks:**
- [ ] Import `PackageManager` in `javascript.rs`
- [ ] Update cold build to use `pkg_mgr.run_command("build")`
- [ ] Update hot build similarly
- [ ] Add test for bun project using `bun run build`

**Verification:**
```bash
cargo test build::javascript
```

---

### Phase 4: Landing the Plane Integration

**Goal:** Update Landing the Plane checklist to use detected package manager.

**File:** `crates/cli/src/profiles.rs`

**Current items (hardcoded npm):**
- `npm run lint`
- `npm run typecheck`
- `npm test`
- `npm run build`

**New behavior:**
```rust
fn javascript_checklist_items(root: &Path) -> Vec<ChecklistItem> {
    let pkg_mgr = PackageManager::detect(root);
    let run = |script| format!("{} run {}", pkg_mgr.executable(), script);

    vec![
        ChecklistItem::new(&run("lint")),
        ChecklistItem::new(&run("typecheck")),
        ChecklistItem::new(&format!("{} test", pkg_mgr.executable())),
        ChecklistItem::new(&run("build")),
    ]
}
```

**Tasks:**
- [ ] Import `PackageManager` in `profiles.rs`
- [ ] Update `javascript_checklist_items()` to detect and use package manager
- [ ] Add test for bun.lock project checklist items

**Verification:**
```bash
cargo test profiles
```

---

### Phase 5: Test Fixtures

**Goal:** Create fixtures to verify detection with each lock file type.

**Structure:**
```
tests/fixtures/
└── js-pkg-managers/
    ├── bun-project/
    │   ├── package.json
    │   └── bun.lock
    ├── pnpm-project/
    │   ├── package.json
    │   └── pnpm-lock.yaml
    ├── yarn-project/
    │   ├── package.json
    │   └── yarn.lock
    └── npm-project/
        ├── package.json
        └── package-lock.json
```

**Tasks:**
- [ ] Create `js-pkg-managers/` fixture directory
- [ ] Add minimal `package.json` for each project
- [ ] Add empty lock files (detection only needs existence)
- [ ] Add integration test iterating fixtures
- [ ] Use `yare` for parameterized tests

**Verification:**
```bash
cargo test --test '*' pkg_manager
```

---

### Phase 6: Cleanup & Documentation

**Goal:** Remove duplication, ensure consistency, update docs.

**Tasks:**
- [ ] Search for hardcoded `yarn.lock` checks, remove duplicates
- [ ] Ensure all call sites use `PackageManager::detect()`
- [ ] Update `docs/specs/langs/javascript.md` Package Manager section
- [ ] Add `PackageManager` to module documentation
- [ ] Run `cargo clippy` and fix warnings
- [ ] Run full test suite

**Verification:**
```bash
make check
```

---

## Key Implementation Details

### Detection Order Rationale

Bun checked first because:
1. Bun is the newest package manager, users explicitly chose it
2. `bun.lock` (text) is Bun 1.2+, `bun.lockb` (binary) is legacy
3. Bun can run npm/yarn projects but the reverse isn't true

### Command Patterns

| Package Manager | Run Script | Run Tests | Install |
|-----------------|------------|-----------|---------|
| npm | `npm run <script>` | `npm test` | `npm install` |
| pnpm | `pnpm run <script>` | `pnpm test` | `pnpm install` |
| yarn | `yarn <script>` | `yarn test` | `yarn` |
| bun | `bun run <script>` | `bun test` | `bun install` |

Note: Yarn uses `yarn <script>` without `run` for conciseness.

### Caching Consideration

Package manager detection is fast (4 file existence checks). No caching needed. If workspace detection becomes slow, consider caching `PackageManager` in `JsWorkspace`.

### Fallback Behavior

When no lock file exists:
- Default to npm (most common, works everywhere)
- Don't warn—many projects run fine without lock files in development

## Verification Plan

### Unit Tests

Each phase adds unit tests in sibling `*_tests.rs` files:
- `package_manager_tests.rs` - detection logic
- `js_detect_tests.rs` - runner selection with package manager
- `profiles_tests.rs` - checklist generation

### Integration Tests

Test full workflows:
```rust
#[test]
fn bun_project_uses_bun_test() {
    let root = fixtures_path("js-pkg-managers/bun-project");
    // Run quench check --ci --tests and verify bun test was called
}
```

### Manual Verification

```bash
# Create test project
mkdir /tmp/bun-test && cd /tmp/bun-test
bun init -y
echo '{}' > quench.toml

# Verify detection
quench check --tests --ci
# Should show "bun test" in output
```

### CI Verification

```bash
make check  # Runs fmt, clippy, test, build, audit, deny
```

## Summary

| Phase | Deliverable | LOC Estimate |
|-------|-------------|--------------|
| 1 | PackageManager module | ~100 |
| 2 | Test runner integration | ~80 |
| 3 | Build command integration | ~30 |
| 4 | Landing the Plane integration | ~40 |
| 5 | Test fixtures | ~50 |
| 6 | Cleanup & docs | ~20 |

**Total:** ~320 lines of code
