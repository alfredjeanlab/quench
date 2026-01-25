# Review: Default Behavior Choices

**Date:** 2026-01-24
**Scope:** Checkpoint 17H, Git Check (801-820), Report Command (16), Timing (1398-1401)

## Summary

The recent work introduced several new default behaviors. This review analyzes each choice for consistency, user impact, and potential issues.

**Overall Assessment:** Defaults follow a conservative, opt-in pattern. No blocking issues found.

---

## New Defaults Introduced

### 1. Git Check - Disabled by Default

**File:** `crates/cli/src/checks/git/mod.rs:92-94`

```rust
fn default_enabled(&self) -> bool {
    false
}
```

| Setting | Default | Configurable |
|---------|---------|--------------|
| Check enabled | `false` | Yes, via `[check.git]` |
| Commit format | `"conventional"` | Yes, via `git.commit.format` |
| Agent docs check | `true` | Yes, via `git.commit.agents` |
| Template creation | `true` | Yes, via `git.commit.template` |

**Rationale:** Not all projects use conventional commits. Opt-in is safer than breaking existing workflows.

**Verdict:** Good choice.

---

### 2. Template Filename: `.gitmessage`

**File:** `crates/cli/src/checks/git/template.rs`

```rust
pub const TEMPLATE_PATH: &str = ".gitmessage";
```

**Concern:** Hardcoded filename could conflict with existing user templates.

**Alternatives Considered:**
- `quench.gitmessage` - Namespaced but non-standard
- `.quench/gitmessage` - Hidden in cache dir
- Configurable via `git.commit.template_path`

**Recommendation:** Consider making configurable in future. Low priority - `.gitmessage` is the standard git convention.

**Verdict:** Acceptable, but note for future enhancement.

---

### 3. Agent Files: Fixed List

**File:** `crates/cli/src/checks/git/docs.rs`

```rust
const AGENT_FILES: &[&str] = &["CLAUDE.md", "AGENTS.md", ".cursorrules"];
```

**Concern:** Adding support for new AI tools (e.g., Copilot, Cody) requires code changes.

**Current Behavior:**
- Checks these specific files for commit format documentation
- No way to add custom agent files via config

**Recommendation:** Add `git.commit.agent_files` config option in future phase.

**Verdict:** Acceptable for now. Track as tech debt.

---

### 4. Conventional Commit Types: 10 Default Types

**File:** `crates/cli/src/checks/git/parse.rs`

```rust
pub const DEFAULT_TYPES: &[&str] = &[
    "feat", "fix", "chore", "docs", "test",
    "refactor", "perf", "ci", "build", "style"
];
```

**Status:** Already configurable via `git.commit.types` in quench.toml.

**Verdict:** Good - sensible defaults with override option.

---

### 5. Memory-Mapped I/O Threshold: 64KB

**File:** `crates/cli/src/file_size.rs`

```rust
pub const MMAP_THRESHOLD: u64 = 64 * 1024; // 64KB
```

**Rationale:**
- Below 64KB: Direct read is faster (no mmap setup overhead)
- Above 64KB: Mmap avoids copying large buffers

**Research:** 64KB is a common threshold in similar tools (ripgrep uses similar heuristics).

**Verdict:** Good choice. No configuration needed.

---

### 6. Report Output Format: Text

**File:** `crates/cli/src/cli.rs`

```rust
#[arg(long, short = 'f', default_value = "text")]
pub format: OutputFormat,
```

**Rationale:** Text is human-readable for interactive use. JSON/HTML for CI/tooling.

**Verdict:** Good - matches user expectations.

---

### 7. Baseline Path: `.quench/baseline.json`

**File:** `crates/cli/src/config/mod.rs`

```rust
pub fn baseline_path(&self) -> PathBuf {
    self.root.join(".quench/baseline.json")
}
```

**Rationale:** Keeps quench artifacts in `.quench/` directory, consistent with cache.

**Verdict:** Good - consistent with project structure.

---

### 8. Timing Flag: Off by Default

**File:** `crates/cli/src/cli.rs`

```rust
#[arg(long)]
pub timing: bool,  // defaults to false
```

**Rationale:** Timing info is for debugging/profiling, not everyday use.

**Verdict:** Good - opt-in for developer tooling.

---

### 9. Violations Display Limit: 15

**File:** `crates/cli/src/cli.rs`

```rust
#[arg(long, default_value = "15")]
pub limit: usize,
```

**Rationale:** Reasonable terminal output without overwhelming. Configurable via `--limit`.

**Verdict:** Good - sensible default with override.

---

## Action Items

| Priority | Item | Recommendation |
|----------|------|----------------|
| P3 | Template filename | Consider `git.commit.template_path` config |
| P3 | Agent files list | Consider `git.commit.agent_files` config |
| None | Other defaults | No changes needed |

---

## Consistency Check

All new defaults follow established patterns:

| Pattern | Examples | Status |
|---------|----------|--------|
| Opt-in for breaking changes | Git check disabled | Consistent |
| Sensible defaults with overrides | Types, limit, format | Consistent |
| Standard filenames | `.gitmessage`, `baseline.json` | Consistent |
| Performance thresholds as constants | `MMAP_THRESHOLD` | Consistent |

---

## Conclusion

The default behavior choices are well-considered:

1. **Conservative:** New features are opt-in when they could break existing workflows
2. **Configurable:** Most defaults can be overridden via quench.toml
3. **Consistent:** Follows patterns established elsewhere in the codebase
4. **Documented:** Thresholds have clear rationale

Two minor enhancements tracked for future:
- Configurable template path (P3)
- Configurable agent files list (P3)

No blocking issues. Ready to proceed with implementation.
