# Checkpoint 15B: Init Command Dogfooding Validation

**Plan:** `checkpoint-15b-validate`
**Root Feature:** `quench-init`
**Depends On:** checkpoint-15a-precheck (all tests passing)

## Overview

Dogfood the `quench init` command on various project types to validate real-world behavior. Document results in `reports/checkpoint-15-init.md` including any behavioral gaps discovered during manual testing.

**Expected from checkpoint-15a:** All formatting and tests passing (411 tests, 0 ignored)

## Project Structure

```
reports/
└── checkpoint-15-init.md    # Validation report (to create)

tests/fixtures/
├── go-simple/               # Go project (has go.mod)
├── js-simple/               # JavaScript project (has package.json)
├── rust-simple/             # Rust project (has Cargo.toml)
├── shell/                   # Shell project (has *.sh files)
└── mixed/                   # Multi-language project
```

## Dependencies

- Built quench binary (`cargo build --release`)
- `jq` for JSON parsing (optional, for readable output)
- Temporary directories for clean testing

## Implementation Phases

### Phase 1: Validate Empty Directory Init

Test `quench init` on an empty directory produces the full default template.

**Commands:**
```bash
TEMP=$(mktemp -d)
cd "$TEMP"
cargo run -p quench -- init
cat quench.toml
rm -rf "$TEMP"
```

**Expected Output:**
- Creates `quench.toml` file
- Contains `version = 1`
- Contains `[check.cloc]`, `[check.escapes]`, `[check.agents]`, `[check.docs]`
- Contains `# Supported Languages:` comment
- No language sections (no detection markers present)

**Verification Checklist:**
- [ ] File created successfully
- [ ] `version = 1` present
- [ ] All check sections present
- [ ] Template matches `cli::default_template()` output
- [ ] Output message shows "Created quench.toml"

---

### Phase 2: Validate Rust Project Detection

Test `quench init` on a Rust project detects and adds `[rust]` section.

**Commands:**
```bash
TEMP=$(mktemp -d)
cd "$TEMP"
echo '[package]\nname = "test"' > Cargo.toml
cargo run -p quench -- init
cat quench.toml
rm -rf "$TEMP"
```

**Expected Output:**
- `[rust]` section present
- `rust.cloc.check = "error"` present
- `rust.policy.check = "error"` present
- `rust.suppress.check = "comment"` present
- Output message mentions "rust"

**Verification Checklist:**
- [ ] Rust detection works from Cargo.toml
- [ ] Dotted key format used (`rust.cloc.check`)
- [ ] All three rust sub-settings present
- [ ] No other languages detected

---

### Phase 3: Validate Go Project Detection

Test `quench init` on a Go project detects and adds `[golang]` section.

**Commands:**
```bash
TEMP=$(mktemp -d)
cd "$TEMP"
echo 'module test' > go.mod
cargo run -p quench -- init
cat quench.toml
rm -rf "$TEMP"
```

**Expected Output:**
- `[golang]` section present
- `golang.cloc.check = "error"` present
- `golang.policy.check = "error"` present
- `golang.suppress.check = "comment"` present

**Verification Checklist:**
- [ ] Go detection works from go.mod
- [ ] Dotted key format used
- [ ] All three golang sub-settings present

---

### Phase 4: Validate JavaScript Project Detection

Test `quench init` on a JavaScript project detects and adds `[javascript]` section.

**Commands:**
```bash
TEMP=$(mktemp -d)
cd "$TEMP"
echo '{"name": "test"}' > package.json
cargo run -p quench -- init
cat quench.toml
rm -rf "$TEMP"
```

**Variant - TypeScript:**
```bash
TEMP=$(mktemp -d)
cd "$TEMP"
echo '{}' > tsconfig.json
cargo run -p quench -- init
cat quench.toml
rm -rf "$TEMP"
```

**Expected Output:**
- `[javascript]` section present
- `javascript.cloc.check = "error"` present
- `javascript.policy.check = "error"` present
- `javascript.suppress.check = "comment"` present

**Verification Checklist:**
- [ ] JS detection works from package.json
- [ ] JS detection works from tsconfig.json
- [ ] JS detection works from jsconfig.json

---

### Phase 5: Validate Shell Project Detection

Test `quench init` on a Shell project detects and adds `[shell]` section.

**Commands:**
```bash
# Root directory shell files
TEMP=$(mktemp -d)
cd "$TEMP"
echo '#!/bin/bash' > build.sh
cargo run -p quench -- init
cat quench.toml
rm -rf "$TEMP"

# scripts/ directory
TEMP=$(mktemp -d)
cd "$TEMP"
mkdir scripts
echo '#!/bin/bash' > scripts/deploy.sh
cargo run -p quench -- init
cat quench.toml
rm -rf "$TEMP"

# bin/ directory
TEMP=$(mktemp -d)
cd "$TEMP"
mkdir bin
echo '#!/bin/bash' > bin/run.sh
cargo run -p quench -- init
cat quench.toml
rm -rf "$TEMP"
```

**Expected Output:**
- `[shell]` section present
- `shell.cloc.check = "error"` present
- `shell.policy.check = "error"` present
- `shell.suppress.check = "forbid"` (note: shell uses forbid by default)

**Verification Checklist:**
- [ ] Shell detection works from root *.sh files
- [ ] Shell detection works from scripts/*.sh
- [ ] Shell detection works from bin/*.sh
- [ ] Uses `forbid` for suppress (different from other languages)

---

### Phase 6: Validate Agent Detection (Claude)

Test `quench init` on a project with CLAUDE.md updates `[check.agents]`.

**Commands:**
```bash
TEMP=$(mktemp -d)
cd "$TEMP"
echo '# Project' > CLAUDE.md
cargo run -p quench -- init
cat quench.toml
rm -rf "$TEMP"
```

**Expected Output:**
- `[check.agents]` section present
- `required = ["CLAUDE.md"]` present

**Verification Checklist:**
- [ ] Claude detection works from CLAUDE.md
- [ ] Required array includes "CLAUDE.md"
- [ ] Output message mentions "claude" (if applicable)

---

### Phase 7: Validate Agent Detection (Cursor)

Test `quench init` on a project with Cursor markers.

**Commands:**
```bash
# .cursorrules
TEMP=$(mktemp -d)
cd "$TEMP"
echo '# Rules' > .cursorrules
cargo run -p quench -- init
cat quench.toml
rm -rf "$TEMP"

# .cursor/rules/*.mdc
TEMP=$(mktemp -d)
cd "$TEMP"
mkdir -p .cursor/rules
echo '# Rules' > .cursor/rules/project.mdc
cargo run -p quench -- init
cat quench.toml
rm -rf "$TEMP"
```

**Expected Output:**
- `required = [".cursorrules"]` present (for .cursorrules variant)

**Verification Checklist:**
- [ ] Cursor detection works from .cursorrules
- [ ] Cursor detection works from .cursor/rules/*.mdc
- [ ] Cursor detection works from .cursor/rules/*.md

---

### Phase 8: Validate --with Flag (Single Profile)

Test `quench init --with shell` creates shell-only config.

**Commands:**
```bash
TEMP=$(mktemp -d)
cd "$TEMP"
# Create markers for other languages (should be ignored)
echo '[package]' > Cargo.toml
echo 'module test' > go.mod
cargo run -p quench -- init --with shell
cat quench.toml
rm -rf "$TEMP"
```

**Expected Output:**
- `[shell]` section present with full profile
- `[shell.suppress]` section present
- `[shell.policy]` section present
- `[[check.escapes.patterns]]` for shell patterns (set +e, eval, rm -rf)
- NO `[rust]` or `[golang]` sections

**Verification Checklist:**
- [ ] Shell profile fully included (not just minimal section)
- [ ] Auto-detection skipped (Rust/Go markers ignored)
- [ ] Shell escape patterns included
- [ ] Output message shows "shell"

---

### Phase 9: Validate --with Flag (Combined Profiles)

Test `quench init --with rust,claude` creates combined config.

**Commands:**
```bash
TEMP=$(mktemp -d)
cd "$TEMP"
cargo run -p quench -- init --with rust,claude
cat quench.toml
rm -rf "$TEMP"
```

**Expected Output:**
- `[rust]` section with full profile
- `[check.agents]` with `required = ["CLAUDE.md"]`
- Rust escape patterns present (unsafe, unwrap, expect, transmute)
- Output message shows "rust, claude"

**Verification Checklist:**
- [ ] Multiple profiles combined correctly
- [ ] Both language and agent profiles work
- [ ] Escape patterns from rust profile included
- [ ] Agent required file set correctly

---

### Phase 10: Validate Multi-Language Detection

Test `quench init` on a project with multiple language markers.

**Commands:**
```bash
TEMP=$(mktemp -d)
cd "$TEMP"
echo '[package]' > Cargo.toml
mkdir scripts
echo '#!/bin/bash' > scripts/build.sh
echo '# Project' > CLAUDE.md
cargo run -p quench -- init
cat quench.toml
rm -rf "$TEMP"
```

**Expected Output:**
- `[rust]` section present
- `[shell]` section present
- `[check.agents]` with `required = ["CLAUDE.md"]`

**Verification Checklist:**
- [ ] All detected languages included
- [ ] All detected agents included
- [ ] Sections in correct order

---

### Phase 11: Document Results

Create `reports/checkpoint-15-init.md` with validation results.

**Report Structure:**
```markdown
# Checkpoint 15: Init Command Complete - Validation Report

Generated: [DATE]

## Summary

| Test | Status | Notes |
|------|--------|-------|
| Empty directory init | ? | |
| Rust project detection | ? | |
| Go project detection | ? | |
| JavaScript project detection | ? | |
| Shell project detection | ? | |
| Claude agent detection | ? | |
| Cursor agent detection | ? | |
| --with single profile | ? | |
| --with combined profiles | ? | |
| Multi-language detection | ? | |

**Overall Status: ?**

## Detailed Results

[Include actual command outputs and observations]

## Behavioral Gaps

[Any unexpected behaviors or deviations from spec]

## Conclusion

[Summary of init command status]
```

## Key Implementation Details

### Profile vs Detection Output

When `--with` is used:
- Full profile content is written (from `cli::rust_profile_defaults()` etc.)
- Includes escape patterns, suppress config, policy config
- Auto-detection is completely skipped

When auto-detection runs:
- Minimal sections are written (from `cli::rust_detected_section()` etc.)
- Uses dotted key format: `rust.cloc.check = "error"`
- No escape patterns (user customizes later)

### Expected Output Format

Default template structure:
```toml
# Quench configuration
version = 1

[check.cloc]
check = "error"

[check.escapes]
check = "error"

[check.agents]
check = "error"

[check.docs]
check = "error"

[check.tests]
check = "off"

[check.license]
check = "off"

[git.commit]
check = "off"

# Supported Languages:
# [rust], [golang], [javascript], [shell]
```

### Detection Markers

| Language | Detection Markers |
|----------|-------------------|
| Rust | `Cargo.toml` |
| Go | `go.mod` |
| JavaScript | `package.json`, `tsconfig.json`, `jsconfig.json` |
| Shell | `*.sh` in root, `bin/`, or `scripts/` |
| Claude | `CLAUDE.md` |
| Cursor | `.cursorrules`, `.cursor/rules/*.md[c]` |

## Verification Plan

### Quick Test Script

```bash
#!/bin/bash
# Run all validation tests
set -e

cargo build -p quench --release

echo "=== Phase 1: Empty dir ==="
TEMP=$(mktemp -d) && cd "$TEMP"
cargo run -p quench -- init
cat quench.toml
cd - && rm -rf "$TEMP"

echo "=== Phase 2: Rust project ==="
TEMP=$(mktemp -d) && cd "$TEMP"
echo '[package]\nname = "test"' > Cargo.toml
cargo run -p quench -- init
grep -q '\[rust\]' quench.toml && echo "PASS: [rust] found"
cd - && rm -rf "$TEMP"

# Continue for all phases...
```

### Automated Test Coverage

All scenarios should also have behavioral specs in `tests/specs/cli/init.rs`:
- `init_output_matches_template_format`
- `init_detects_rust_from_cargo_toml`
- `init_detects_golang_from_go_mod`
- `init_detects_javascript_from_package_json`
- `init_detects_shell_from_scripts_dir`
- `init_detects_claude_from_claude_md`
- `init_detects_cursor_from_cursorrules`
- `init_with_skips_auto_detection`
- `init_combined_profiles_generates_both`
- `init_detection_is_additive`

### Final Checklist

From `plans/.3-roadmap-init.md` checkpoint items:
- [ ] `quench init` on empty dir creates full template
- [ ] `quench init` on Rust project detects and adds `[rust]` section
- [ ] `quench init` on project with CLAUDE.md updates `[check.agents]`
- [ ] `quench init --with shell` creates shell-only config
- [ ] `quench init --with rust,claude` creates combined config
- [ ] All existing init tests updated and passing
