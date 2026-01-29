# Auto-Install Shell Completions on Init

## Overview

Add automatic shell completion installation to `quench init`, following the proven pattern from the wok project. When users run `quench init`, completions will be automatically installed for detected shells (bash, zsh, fish), making CLI tab completion immediately available.

## Context

The wok project implements auto-install completions with these features:
- Detects installed shells using `which` command
- Generates completion scripts via `clap_complete::generate()`
- Stores scripts in `~/.local/share/wk/completions/` (XDG Base Directory spec)
- Fish gets special treatment: uses native `~/.config/fish/completions/` directory
- Adds sourcing lines to shell RC files with marker-based idempotency
- Gracefully handles errors: logs warnings but doesn't fail init

Quench already has:
- `completions` command that outputs to stdout
- `clap_complete` dependency
- Clean module structure with sibling `_tests.rs` files

## Implementation Steps

### 1. Add Dependencies

**File:** `crates/cli/Cargo.toml`

Add to `[dependencies]` section (alphabetically after `chrono`):
```toml
dirs = "6"
```

### 2. Create Completions Module

**File:** `crates/cli/src/completions.rs` (~230 lines)

Copy and adapt from `/Users/kestred/Developer/wok/crates/cli/src/completions.rs`:

**Key adaptations:**
- Change binary name: `"wk"` → `"quench"`
- Change marker: `"# wk-shell-completion"` → `"# quench-shell-completion"`
- Change data dir: `"wk/completions"` → `"quench/completions"`
- Change script filenames: `"wk.bash"` → `"quench.bash"`, `"_wk"` → `"_quench"`, `"wk.fish"` → `"quench.fish"`
- Update license header to match quench's license
- Import `crate::cli::Cli` and `crate::error::{Error, Result}`

**Module structure:**
```rust
// Constants
const QUENCH_COMPLETION_MARKER: &str = "# quench-shell-completion";

// Public types
pub enum ShellKind { Bash, Zsh, Fish }

// Public functions
pub fn detect_shells() -> Vec<ShellKind>
pub fn install_all() -> Result<()>

// Private functions
fn shell_exists(name: &str) -> bool
fn completions_dir() -> Option<PathBuf>
fn write_completion_script(shell: ShellKind) -> Result<PathBuf>
fn install_completion_source(shell: ShellKind, script_path: &Path) -> Result<()>
fn install_for_shell(shell: ShellKind) -> Result<()>
fn install_fish_completions() -> Result<()>

// ShellKind methods
impl ShellKind {
    pub fn rc_file(&self) -> Option<PathBuf>
    fn clap_shell(&self) -> clap_complete::Shell
    fn script_filename(&self) -> &'static str
}
```

**Key behaviors:**
- `install_all()` detects shells and installs for each, logs warnings on errors
- RC files: bash prefers `~/.bashrc` (falls back to `~/.bash_profile` on macOS), zsh uses `~/.zshrc`
- Fish writes directly to `~/.config/fish/completions/quench.fish` (native auto-load)
- Idempotency: checks for marker before adding sourcing lines
- Conditional sourcing: `[ -f "..." ] && source "..."` (bash/zsh) or `test -f "..." && source "..."` (fish)

### 3. Create Unit Tests

**File:** `crates/cli/src/completions_tests.rs` (~180 lines)

Adapt tests from wok's `completions_tests.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn test_detect_shells() { /* ... */ }

#[test]
fn test_shell_exists() { /* ... */ }

#[test]
fn test_shell_kind_script_filename() {
    assert_eq!(ShellKind::Bash.script_filename(), "quench.bash");
    assert_eq!(ShellKind::Zsh.script_filename(), "_quench");
    assert_eq!(ShellKind::Fish.script_filename(), "quench.fish");
}

#[test]
fn test_shell_kind_clap_shell() { /* ... */ }

#[test]
fn test_completions_dir() { /* ... */ }

#[test]
fn test_write_completion_script() { /* ... */ }

#[test]
fn test_install_completion_source_idempotent() { /* ... */ }

#[test]
fn test_install_all_no_shells() { /* ... */ }
```

Use `tempfile::TempDir` for isolated testing and override `HOME`/`XDG_DATA_HOME` environment variables.

### 4. Register Module

**File:** `crates/cli/src/lib.rs`

Add module declaration (after line ~15, alphabetically):
```rust
pub mod completions;
```

Add at bottom with other test modules:
```rust
#[cfg(test)]
#[path = "completions_tests.rs"]
mod completions_tests;
```

### 5. Integrate with Init Command

**File:** `crates/cli/src/cmd_init.rs`

**Add import** (top of file, with other imports):
```rust
use crate::completions;
```

**Add call after `.gitignore` update** (after line 192, before line 194):
```rust
// Ensure .quench/ is in .gitignore
if let Err(e) = ensure_gitignored(&cwd) {
    eprintln!("quench: warning: failed to update .gitignore: {}", e);
}

// Install shell completions
if let Err(e) = completions::install_all() {
    eprintln!("quench: warning: failed to install shell completions: {}", e);
}

println!("{}", message);
Ok(ExitCode::Success)
```

## Critical Files

Implementation involves these files:
- **New:** `crates/cli/src/completions.rs` (~230 lines)
- **New:** `crates/cli/src/completions_tests.rs` (~180 lines)
- **Modified:** `crates/cli/Cargo.toml` (+1 line: `dirs` dependency)
- **Modified:** `crates/cli/src/lib.rs` (+2 lines: module declaration)
- **Modified:** `crates/cli/src/cmd_init.rs` (+4 lines: import + install call)
- **Reference:** `/Users/kestred/Developer/wok/crates/cli/src/completions.rs` (copy and adapt)
- **Reference:** `/Users/kestred/Developer/wok/crates/cli/src/completions_tests.rs` (copy and adapt)

## Edge Cases

1. **No shells installed**: `install_all()` succeeds silently (no-op)
2. **No RC files exist**: Skips that shell, tries others, no warning
3. **RC file not writable**: Logs warning, continues with other shells
4. **Data directory creation fails**: Logs warning, init succeeds
5. **Completions already installed**: Marker found, skips silently (idempotent)
6. **Fish not configured**: Creates directory or logs warning if creation fails

All errors are non-blocking: init succeeds with warnings.

## Testing Strategy

### Unit Tests
- Shell detection (doesn't panic)
- Script filename generation (`quench.bash`, `_quench`, `quench.fish`)
- Directory path generation
- Script generation with temp HOME
- RC file idempotency (run twice, verify single marker)
- Graceful handling when no shells available

### Integration Tests (Manual)
1. Run `quench init` in temp directory
2. Verify scripts created in `~/.local/share/quench/completions/`
3. Verify RC files contain marker and sourcing lines
4. Verify Fish completion in `~/.config/fish/completions/quench.fish`
5. Re-run `quench init`, verify no duplicate lines
6. Test tab completion: `quench <TAB>`, `quench check <TAB>`

## Verification

Before committing:
- [ ] Unit tests in `completions_tests.rs` pass
- [ ] Run `quench init` in temp dir, verify completions installed
- [ ] Verify RC files contain marker and sourcing
- [ ] Re-run init, verify idempotency (no duplicates)
- [ ] `make check` passes (fmt, clippy, test, build, audit, deny)
- [ ] **No cache version bump needed** (no check logic changed)
- [ ] Commit with specs: Integration with init command

## Benefits

- **Immediate UX improvement**: Tab completion works right after init
- **Zero user action required**: No manual setup steps
- **Non-intrusive**: Errors don't break init
- **Idempotent**: Safe to run multiple times
- **Cross-platform**: Works on macOS and Linux
- **Proven pattern**: Same approach as wok (well-tested)
