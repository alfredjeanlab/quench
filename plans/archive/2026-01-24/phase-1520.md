# Phase 1520: Language Auto-Detection

**Root Feature:** `quench-init`

## Overview

Implement language auto-detection for `quench init`. When running `quench init` without `--with`, the command will detect languages present in the project (Rust, Go, JavaScript, Shell) and include their configuration sections in the generated `quench.toml`. Detection is additive, meaning projects with both `Cargo.toml` and `scripts/*.sh` will get both `[rust]` and `[shell]` sections.

## Project Structure

Files to create/modify:

```
crates/cli/src/
├── cli.rs          # Add minimal language output functions
├── init.rs         # NEW: Init detection logic module
└── main.rs         # Update run_init to use detection

tests/specs/cli/
└── init.rs         # Enable Phase 1520 specs
```

Reference files:

```
crates/cli/src/adapter/mod.rs    # Existing detect_language() and helpers
docs/specs/commands/quench-init.md
docs/specs/10-language-adapters.md
```

## Dependencies

No new dependencies. Reuses existing detection helpers from `adapter/mod.rs`.

## Implementation Phases

### Phase 1: Create Init Module with Detection Functions

Create `crates/cli/src/init.rs` with multi-language detection:

```rust
//! Init command detection and output.

use std::path::Path;

/// Languages that can be detected in a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DetectedLanguage {
    Rust,
    Golang,
    JavaScript,
    Shell,
}

/// Detect all languages present in a project.
///
/// Returns a list of detected languages. Detection is additive:
/// a project with Cargo.toml and scripts/*.sh returns both Rust and Shell.
pub fn detect_languages(root: &Path) -> Vec<DetectedLanguage> {
    let mut languages = Vec::new();

    // Rust: Cargo.toml exists
    if root.join("Cargo.toml").exists() {
        languages.push(DetectedLanguage::Rust);
    }

    // Go: go.mod exists
    if root.join("go.mod").exists() {
        languages.push(DetectedLanguage::Golang);
    }

    // JavaScript: package.json, tsconfig.json, or jsconfig.json exists
    if root.join("package.json").exists()
        || root.join("tsconfig.json").exists()
        || root.join("jsconfig.json").exists()
    {
        languages.push(DetectedLanguage::JavaScript);
    }

    // Shell: *.sh in root, bin/, or scripts/
    if has_shell_markers(root) {
        languages.push(DetectedLanguage::Shell);
    }

    languages
}

/// Check if project has Shell markers.
fn has_shell_markers(root: &Path) -> bool {
    has_sh_files(root)
        || root.join("bin").is_dir() && has_sh_files(&root.join("bin"))
        || root.join("scripts").is_dir() && has_sh_files(&root.join("scripts"))
}

/// Check if a directory contains *.sh files.
fn has_sh_files(dir: &Path) -> bool {
    dir.read_dir()
        .ok()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                let path = entry.path();
                path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sh")
            })
        })
        .unwrap_or(false)
}
```

Key design decisions:

| Decision | Rationale |
|----------|-----------|
| Separate enum from `ProjectLanguage` | Init detection returns multiple languages; check detection returns primary |
| Vec return type | Additive detection, order matches detection priority |
| Reuse shell detection logic | Consistent with existing `has_shell_markers` in adapter/mod.rs |

### Phase 2: Add Minimal Language Output Functions

Add functions in `crates/cli/src/cli.rs` for detected language output:

```rust
/// Minimal Rust section for auto-detection output.
///
/// Uses dotted keys per spec: docs/specs/commands/quench-init.md
pub fn rust_detected_section() -> &'static str {
    r#"[rust]
rust.cloc.check = "error"
rust.policy.check = "error"
rust.suppress.check = "comment"
"#
}

/// Minimal Go section for auto-detection output.
pub fn golang_detected_section() -> &'static str {
    r#"[golang]
golang.cloc.check = "error"
golang.policy.check = "error"
golang.suppress.check = "comment"
"#
}

/// Minimal JavaScript section for auto-detection output.
pub fn javascript_detected_section() -> &'static str {
    r#"[javascript]
javascript.cloc.check = "error"
javascript.policy.check = "error"
javascript.suppress.check = "comment"
"#
}

/// Minimal Shell section for auto-detection output.
///
/// Note: Shell uses "forbid" for suppress by default.
pub fn shell_detected_section() -> &'static str {
    r#"[shell]
shell.cloc.check = "error"
shell.policy.check = "error"
shell.suppress.check = "forbid"
"#
}
```

These minimal sections differ from `*_profile_defaults()`:
- No escape patterns (those come with `--with`)
- Dotted key format per spec
- Minimal config to enable language checks

### Phase 3: Update run_init with Detection Logic

Modify `run_init` in `crates/cli/src/main.rs`:

```rust
use quench::init::{DetectedLanguage, detect_languages};

fn run_init(_cli: &Cli, args: &InitArgs) -> anyhow::Result<ExitCode> {
    use quench::cli::{
        default_template,
        golang_profile_defaults, rust_profile_defaults, shell_profile_defaults,
        rust_detected_section, golang_detected_section,
        javascript_detected_section, shell_detected_section,
    };

    let cwd = std::env::current_dir()?;
    let config_path = cwd.join("quench.toml");

    if config_path.exists() && !args.force {
        eprintln!("quench.toml already exists. Use --force to overwrite.");
        return Ok(ExitCode::ConfigError);
    }

    // Determine what to include
    let (config, message) = if !args.with_profiles.is_empty() {
        // --with specified: use full profiles, skip detection
        let mut cfg = default_template().to_string();
        for profile in &args.with_profiles {
            match profile.as_str() {
                "rust" => {
                    cfg.push('\n');
                    cfg.push_str(&rust_profile_defaults());
                }
                "shell" => {
                    cfg.push('\n');
                    cfg.push_str(&shell_profile_defaults());
                }
                "golang" | "go" => {
                    cfg.push('\n');
                    cfg.push_str(&golang_profile_defaults());
                }
                other => {
                    eprintln!("quench: warning: unknown profile '{}', skipping", other);
                }
            }
        }
        let msg = format!("Created quench.toml with profile(s): {}",
                          args.with_profiles.join(", "));
        (cfg, msg)
    } else {
        // No --with: run auto-detection
        let detected = detect_languages(&cwd);

        let mut cfg = default_template().to_string();
        for lang in &detected {
            cfg.push('\n');
            match lang {
                DetectedLanguage::Rust => cfg.push_str(rust_detected_section()),
                DetectedLanguage::Golang => cfg.push_str(golang_detected_section()),
                DetectedLanguage::JavaScript => cfg.push_str(javascript_detected_section()),
                DetectedLanguage::Shell => cfg.push_str(shell_detected_section()),
            }
        }

        let msg = if detected.is_empty() {
            "Created quench.toml".to_string()
        } else {
            let names: Vec<_> = detected.iter().map(|l| match l {
                DetectedLanguage::Rust => "rust",
                DetectedLanguage::Golang => "golang",
                DetectedLanguage::JavaScript => "javascript",
                DetectedLanguage::Shell => "shell",
            }).collect();
            format!("Created quench.toml (detected: {})", names.join(", "))
        };
        (cfg, msg)
    };

    std::fs::write(&config_path, config)?;
    println!("{}", message);
    Ok(ExitCode::Success)
}
```

### Phase 4: Register Init Module

Add module to `crates/cli/src/lib.rs`:

```rust
pub mod init;
```

### Phase 5: Unit Tests

Add tests in `crates/cli/src/init_tests.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn detect_rust_from_cargo_toml() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Rust));
}

#[test]
fn detect_golang_from_go_mod() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("go.mod"), "module test").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Golang));
}

#[test]
fn detect_javascript_from_package_json() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::JavaScript));
}

#[test]
fn detect_javascript_from_tsconfig() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("tsconfig.json"), "{}").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::JavaScript));
}

#[test]
fn detect_shell_from_root_sh() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("build.sh"), "#!/bin/bash").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Shell));
}

#[test]
fn detect_shell_from_scripts_dir() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("scripts")).unwrap();
    fs::write(temp.path().join("scripts/deploy.sh"), "#!/bin/bash").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Shell));
}

#[test]
fn detect_shell_from_bin_dir() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("bin")).unwrap();
    fs::write(temp.path().join("bin/run.sh"), "#!/bin/bash").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Shell));
}

#[test]
fn detection_is_additive() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Cargo.toml"), "[package]").unwrap();
    fs::create_dir(temp.path().join("scripts")).unwrap();
    fs::write(temp.path().join("scripts/test.sh"), "#!/bin/bash").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Rust));
    assert!(detected.contains(&DetectedLanguage::Shell));
}

#[test]
fn no_markers_returns_empty() {
    let temp = TempDir::new().unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.is_empty());
}
```

### Phase 6: Enable Behavioral Specs

Remove `#[ignore]` from these tests in `tests/specs/cli/init.rs`:

| Test | Verification |
|------|-------------|
| `init_with_skips_auto_detection` | --with shell doesn't add [rust] or [golang] |
| `init_without_with_triggers_auto_detection` | Cargo.toml adds [rust] section |
| `init_detects_rust_from_cargo_toml` | [rust] and rust.cloc.check present |
| `init_detects_golang_from_go_mod` | [golang] present |
| `init_detects_javascript_from_package_json` | [javascript] present |
| `init_detects_shell_from_scripts_dir` | [shell] present |
| `init_detection_is_additive` | Both [rust] and [shell] present |

## Key Implementation Details

### Detection vs Profile Output

Two distinct output modes:

| Mode | Trigger | Output Format | Includes |
|------|---------|---------------|----------|
| Detection | No `--with` | Dotted keys | cloc, policy, suppress checks only |
| Profile | `--with rust` | Nested sections | Full config with escape patterns |

This design allows:
- Quick start with detection (minimal config)
- Full customization with `--with` (recommended patterns)

### Dotted Key Format

Per spec, detected language sections use dotted keys:

```toml
[rust]
rust.cloc.check = "error"
rust.policy.check = "error"
rust.suppress.check = "comment"
```

This is valid TOML and equivalent to nested sections, but more compact.

### Shell Default Differences

Shell uses `"forbid"` for suppress check (not `"comment"`), matching the convention that shell scripts should use explicit comments rather than inline suppressions.

### Order of Detection

Languages are detected in this order: Rust, Go, JavaScript, Shell. This matches the priority in `adapter/mod.rs` and ensures consistent output ordering.

## Verification Plan

### 1. Unit Tests

```bash
cargo test init::tests
```

Expected: All detection tests pass.

### 2. Behavioral Specs

```bash
cargo test --test specs init_with_skips
cargo test --test specs init_without_with
cargo test --test specs init_detects_rust
cargo test --test specs init_detects_golang
cargo test --test specs init_detects_javascript
cargo test --test specs init_detects_shell
cargo test --test specs init_detection_is_additive
```

Expected: All 7 Phase 1520 specs pass.

### 3. Manual Verification

```bash
# Detection: Rust project
cd /tmp && mkdir rust-test && cd rust-test
echo '[package]\nname = "test"' > Cargo.toml
quench init
cat quench.toml  # Should have [rust] section

# Detection: Multi-language
cd /tmp && mkdir multi-test && cd multi-test
echo '[package]\nname = "test"' > Cargo.toml
mkdir scripts && echo '#!/bin/bash' > scripts/build.sh
quench init
cat quench.toml  # Should have [rust] and [shell]

# --with skips detection
cd /tmp && mkdir skip-test && cd skip-test
echo '[package]\nname = "test"' > Cargo.toml
quench init --with shell
cat quench.toml  # Should have [shell] only, no [rust]
```

### 4. Full Check

```bash
make check
```

### 5. Spec Coverage

| Spec Requirement | Test Function | Status |
|-----------------|---------------|--------|
| --with skips auto-detection | `init_with_skips_auto_detection` | Enable |
| No --with triggers detection | `init_without_with_triggers_auto_detection` | Enable |
| Cargo.toml → rust | `init_detects_rust_from_cargo_toml` | Enable |
| go.mod → golang | `init_detects_golang_from_go_mod` | Enable |
| package.json → javascript | `init_detects_javascript_from_package_json` | Enable |
| *.sh in root/bin/scripts → shell | `init_detects_shell_from_scripts_dir` | Enable |
| Detection is additive | `init_detection_is_additive` | Enable |
