# Quench Implementation Outline

## Phase 1a: Project Foundation - Setup

- [ ] Project scaffolding (Cargo.toml workspace, crates/cli, directory structure, dependencies)
- [ ] Error types and Result aliases
- [ ] Unit test setup (cargo test)
- [ ] Integration test harness (CLI invocation via assert_cmd)
- [ ] Snapshot testing setup (insta crate)

## Phase 1b: Project Foundation - Implementation

- [ ] CLI skeleton with clap (quench, quench help, quench check, quench report, quench init)
- [ ] Global flags (--help, --version, --config)
- [ ] Config file discovery (current dir, parent dirs, up to git root)
- [ ] Config parsing with serde/toml
- [ ] Config version validation (version = 1)
- [ ] Unknown key warnings (forward compatibility)

## Phase 2: Test Fixtures

- [ ] fixtures/minimal/ - bare project, no config, no source files
- [ ] fixtures/rust-simple/ - small Rust project with Cargo.toml, src/, tests/
- [ ] fixtures/rust-workspace/ - multi-package Rust workspace
- [ ] fixtures/shell-scripts/ - shell scripts with bats tests
- [ ] fixtures/mixed/ - Rust CLI + shell scripts combination
- [ ] fixtures/violations/ - project with intentional violations for each check type
- [ ] fixtures/docs-project/ - project with docs/, specs, TOC trees, markdown links
- [ ] fixtures/agents-project/ - project with CLAUDE.md, .cursorrules, sections
- [ ] Fixture README documenting purpose of each fixture

## Phase 3a: File Walking - Specs

- [ ] Spec: file walking respects .gitignore
- [ ] Spec: file walking respects custom ignore patterns
- [ ] Spec: symlink loops don't cause infinite recursion
- [ ] Spec: deeply nested directories work (up to depth limit)

## Phase 3b: File Walking - Implementation

- [ ] Parallel file walking with ignore crate
- [ ] Gitignore integration (.gitignore, .ignore, global ignores)
- [ ] Custom ignore patterns from config
- [ ] Symlink loop detection
- [ ] Directory depth limiting (max 100)
- [ ] File metadata reading (size, mtime)
- [ ] Unit tests for walker with temp directories

## Phase 4a: Output Infrastructure - Specs

- [ ] Spec: text output format matches docs/specs/03-output.md
- [ ] Spec: JSON output validates against output.schema.json
- [ ] Spec: color disabled when CLAUDE_CODE env var set
- [ ] Spec: color disabled when not a TTY
- [ ] Spec: --no-color flag disables color
- [ ] Spec: exit code 0 when all checks pass
- [ ] Spec: exit code 1 when any check fails
- [ ] Spec: exit code 2 on config error
- [ ] Spec: violation limit defaults to 15
- [ ] Spec: --no-limit shows all violations
- [ ] Spec: --limit N shows N violations

## Phase 4b: Output Infrastructure - Implementation

- [ ] Text output formatter (check: FAIL format)
- [ ] JSON output formatter (top-level schema)
- [ ] TTY detection for color
- [ ] Agent environment detection (CLAUDE_CODE, CODEX, CURSOR)
- [ ] Color scheme (bold check names, red FAIL, cyan paths, yellow line numbers)
- [ ] --color/--no-color flag handling
- [ ] Exit codes (0 pass, 1 fail, 2 config error, 3 internal error)
- [ ] Violation limiting (default 15, --limit N, --no-limit)
- [ ] Streaming output (default) vs buffered (JSON)

## Phase 5a: Check Framework - Specs

- [ ] Spec: --cloc flag enables only cloc check
- [ ] Spec: --no-cloc flag disables cloc check
- [ ] Spec: multiple check flags combine correctly
- [ ] Spec: check failure doesn't prevent other checks from running
- [ ] Spec: skipped check shows error but continues

## Phase 5b: Check Framework - Implementation

- [ ] Check trait definition (name, run, fixable)
- [ ] Check result type (passed, violations, metrics, by_package)
- [ ] Violation type (file, line, type, advice, extra fields)
- [ ] Check registry and discovery
- [ ] Check runner (parallel execution across checks)
- [ ] Check toggle flags (--[no-]cloc, --[no-]escapes, etc.)
- [ ] Per-package metrics aggregation infrastructure
- [ ] Error recovery (continue on check failure, skip on error)
- [ ] Unit tests for check runner with mock checks

### Checkpoint: CLI Runs
- [ ] `quench check` on fixtures/minimal runs without panic
- [ ] `quench check --help` shows all flags
- [ ] `quench check -o json` produces valid JSON structure
- [ ] Exit code 0 when no checks enabled

## Phase 6a: CLOC Check - Specs

- [ ] Spec: counts non-blank lines as LOC
- [ ] Spec: blank lines not counted
- [ ] Spec: separates source and test files by pattern
- [ ] Spec: calculates source-to-test ratio
- [ ] Spec: JSON output includes source_lines, test_lines, ratio
- [ ] Spec: files over max_lines (750) generate violation
- [ ] Spec: test files over max_lines_test (1100) generate violation
- [ ] Spec: files over max_tokens generate violation
- [ ] Spec: excluded patterns don't generate violations
- [ ] Spec: per-package breakdown in JSON when packages configured

## Phase 6b: CLOC Check - Basic Implementation

- [ ] Line counting (non-whitespace lines)
- [ ] Source pattern matching from config
- [ ] Test pattern matching from config
- [ ] Source vs test file classification
- [ ] Total source/test line metrics
- [ ] Source-to-test ratio calculation
- [ ] Unit tests for line counting edge cases

## Phase 7: CLOC Check - Limits Implementation

- [ ] File size limit checking (max_lines, default 750)
- [ ] Test file size limit checking (max_lines_test, default 1100)
- [ ] Token counting (chars / 4 approximation)
- [ ] Token limit checking (max_tokens, default 20000)
- [ ] Per-file violation generation for oversized files
- [ ] Exclude patterns for size limits
- [ ] Per-package LOC breakdown
- [ ] JSON output with metrics and by_package

### Checkpoint: CLOC Works
- [ ] `quench check --cloc` on fixtures/rust-simple produces correct line counts
- [ ] `quench check --cloc` on fixtures/violations detects oversized file
- [ ] Snapshot test for CLOC text output
- [ ] Snapshot test for CLOC JSON output

## Phase 8: Generic Language Adapter

- [ ] Adapter trait definition
- [ ] Pattern-based source detection from [project] config
- [ ] Pattern-based test detection from [project] config
- [ ] Language-agnostic escape patterns (none by default)
- [ ] Adapter selection based on file extension
- [ ] Unit tests for pattern matching

## Phase 9a: Escapes Check - Specs

- [ ] Spec: detects pattern matches in source files
- [ ] Spec: reports line number of match
- [ ] Spec: count action counts occurrences
- [ ] Spec: count action fails when threshold exceeded
- [ ] Spec: comment action passes when comment present on same line
- [ ] Spec: comment action passes when comment present on preceding line
- [ ] Spec: comment action fails when no comment found
- [ ] Spec: forbid action always fails in source code
- [ ] Spec: forbid action allowed in test code
- [ ] Spec: test code escapes counted separately in metrics
- [ ] Spec: per-pattern advice shown in violation
- [ ] Spec: JSON includes source/test breakdown per pattern

## Phase 9b: Escapes Check - Pattern Matching

- [ ] Pattern configuration parsing ([[check.escapes.patterns]])
- [ ] Regex pattern compilation
- [ ] Literal pattern detection and memchr optimization
- [ ] Multi-literal pattern detection and aho-corasick optimization
- [ ] Pattern matching across file contents
- [ ] Line number extraction for matches
- [ ] Unit tests for each pattern type

## Phase 10: Escapes Check - Actions

- [ ] Count action implementation
- [ ] Count threshold checking (default 0)
- [ ] Comment action implementation
- [ ] Upward comment search (same line, preceding lines)
- [ ] Custom comment pattern matching (// SAFETY:, etc.)
- [ ] Forbid action implementation
- [ ] Source vs test code separation for actions
- [ ] Unit tests for comment search algorithm

## Phase 11: Escapes Check - Output

- [ ] Missing comment violation generation
- [ ] Forbidden pattern violation generation
- [ ] Threshold exceeded violation generation
- [ ] Per-pattern configurable advice
- [ ] Per-package escape counts (source/test breakdown)
- [ ] JSON output with metrics and by_package
- [ ] Early termination when limit reached (non-CI)

### Checkpoint: Escapes Works
- [ ] `quench check --escapes` on fixtures/violations detects all escape types
- [ ] Snapshot test for escapes text output (missing comment, forbidden, threshold)
- [ ] Snapshot test for escapes JSON output

## Phase 12a: Rust Adapter - Specs

- [ ] Spec: auto-detected when Cargo.toml present
- [ ] Spec: default source pattern **/*.rs
- [ ] Spec: default ignores target/
- [ ] Spec: detects workspace packages from Cargo.toml
- [ ] Spec: #[cfg(test)] blocks counted as test LOC
- [ ] Spec: unsafe without // SAFETY: comment fails
- [ ] Spec: .unwrap() in source code fails (forbid)
- [ ] Spec: .unwrap() in test code allowed
- [ ] Spec: #[allow(...)] without comment fails (when configured)
- [ ] Spec: lint config changes with source changes fails standalone policy

## Phase 12b: Rust Adapter - Detection

- [ ] Cargo.toml detection
- [ ] Default source patterns (**/*.rs)
- [ ] Default test patterns (tests/**, *_test.rs, *_tests.rs)
- [ ] Default ignore patterns (target/)
- [ ] Workspace detection and package enumeration
- [ ] Integration test: detect packages in fixtures/rust-workspace

## Phase 13: Rust Adapter - Test Code

- [ ] #[cfg(test)] block parsing
- [ ] Inline test LOC separation (split_cfg_test option)
- [ ] Test module detection within source files
- [ ] Integration with CLOC check for accurate counts
- [ ] Unit tests for #[cfg(test)] parsing edge cases

## Phase 14: Rust Adapter - Escapes

- [ ] Default unsafe pattern (unsafe { })
- [ ] Default unwrap pattern (.unwrap())
- [ ] Default expect pattern (.expect()
- [ ] Default transmute pattern (mem::transmute)
- [ ] SAFETY comment requirement for unsafe

## Phase 15: Rust Adapter - Suppress

- [ ] #[allow(...)] detection
- [ ] #[expect(...)] detection
- [ ] Suppress check levels (forbid/comment/allow)
- [ ] Custom comment pattern for suppress (// JUSTIFIED:)
- [ ] Per-code allow list (no comment needed)
- [ ] Per-code forbid list (never allowed)
- [ ] Separate source vs test suppress policies

## Phase 16: Rust Adapter - Policy

- [ ] lint_changes = "standalone" enforcement
- [ ] Lint config file detection (rustfmt.toml, clippy.toml)
- [ ] Mixed change detection (lint config + source in same branch)
- [ ] Standalone PR requirement violation

### Checkpoint: Rust Adapter Complete
- [ ] `quench check` on fixtures/rust-simple with no config produces useful output
- [ ] `quench check` on fixtures/rust-workspace detects all packages
- [ ] Rust-specific escapes detected in fixtures/violations
- [ ] #[cfg(test)] LOC counted separately

## Phase 17a: Shell Adapter - Specs

- [ ] Spec: auto-detected when *.sh files in root, bin/, or scripts/
- [ ] Spec: default source pattern **/*.sh, **/*.bash
- [ ] Spec: default test pattern tests/**/*.bats
- [ ] Spec: set +e without # OK: comment fails
- [ ] Spec: eval without # OK: comment fails
- [ ] Spec: # shellcheck disable= forbidden by default

## Phase 17b: Shell Adapter - Detection

- [ ] Shell file detection (*.sh in root, bin/, scripts/)
- [ ] Default source patterns (**/*.sh, **/*.bash)
- [ ] Default test patterns (tests/**/*.bats, *_test.sh)

## Phase 18: Shell Adapter - Escapes

- [ ] Default set +e pattern
- [ ] Default eval pattern
- [ ] OK comment requirement

## Phase 19: Shell Adapter - Suppress

- [ ] # shellcheck disable= detection
- [ ] Suppress check levels (forbid default for shell)
- [ ] Per-code allow/forbid lists
- [ ] Separate source vs test policies

## Phase 20: Shell Adapter - Policy

- [ ] lint_changes = "standalone" for shell
- [ ] .shellcheckrc detection

### Checkpoint: Shell Adapter Complete
- [ ] `quench check` on fixtures/shell-scripts produces useful output
- [ ] Shell-specific escapes detected in fixtures/violations

## Phase 21a: Agents Check - Specs

- [ ] Spec: detects CLAUDE.md at project root
- [ ] Spec: detects .cursorrules at project root
- [ ] Spec: missing required file generates violation
- [ ] Spec: files out of sync generates violation
- [ ] Spec: --fix syncs files from sync_source
- [ ] Spec: missing required section generates violation with advice
- [ ] Spec: forbidden section generates violation
- [ ] Spec: markdown table generates violation (default forbid)
- [ ] Spec: file over max_lines generates violation
- [ ] Spec: file over max_tokens generates violation
- [ ] Spec: JSON includes files_found, in_sync metrics

## Phase 21b: Agents Check - File Detection

- [ ] Agent file recognition (CLAUDE.md, AGENTS.md, .cursorrules, .cursor/rules/*.md)
- [ ] Configurable files list
- [ ] File existence checking
- [ ] Required/optional/forbid file configuration
- [ ] Scope detection (root vs package vs module)

## Phase 22: Agents Check - Sync

- [ ] Multi-file sync detection
- [ ] Section-level diff comparison
- [ ] sync_source configuration
- [ ] Sync violation generation
- [ ] --fix: sync from source file

## Phase 23: Agents Check - Sections

- [ ] Markdown heading parsing
- [ ] Required section validation (case-insensitive)
- [ ] Section advice configuration (extended form)
- [ ] Forbidden section validation
- [ ] Glob pattern matching for forbidden sections

## Phase 24: Agents Check - Content

- [ ] Markdown table detection
- [ ] tables = forbid enforcement (default)
- [ ] Box diagram detection (┌─┐ style)
- [ ] Mermaid block detection
- [ ] Size limits (max_lines, max_tokens per scope)

## Phase 25: Agents Check - Output

- [ ] Missing file violations
- [ ] Out of sync violations
- [ ] Missing section violations (with advice)
- [ ] Forbidden content violations
- [ ] File too large violations
- [ ] JSON output with metrics
- [ ] --fix output (FIXED status)

### Checkpoint: Agents Check Complete
- [ ] `quench check --agents` on fixtures/agents-project detects all violation types
- [ ] `quench check --agents --fix` syncs files correctly
- [ ] Snapshot tests for agents output

### Dogfooding Milestone 1
- [ ] Run `quench check` on quench itself (cloc, escapes, agents)
- [ ] Fix any violations found
- [ ] Add quench.toml to quench project

## Phase 26a: Docs Check - Specs

- [ ] Spec: TOC tree entries validated against filesystem
- [ ] Spec: broken TOC path generates violation
- [ ] Spec: markdown link to missing file generates violation
- [ ] Spec: external URLs not validated
- [ ] Spec: specs directory index file detected
- [ ] Spec: unreachable spec file generates violation (linked mode)
- [ ] Spec: missing required section in spec generates violation
- [ ] Spec: feature commit without doc change generates violation (CI mode)
- [ ] Spec: area mapping restricts doc requirement to specific paths

## Phase 26b: Docs Check - TOC Validation

- [ ] Fenced code block detection in markdown
- [ ] Directory tree structure parsing
- [ ] Tree entry extraction (files and directories)
- [ ] Comment stripping (after #)
- [ ] Path resolution (relative to file, docs/, root)
- [ ] File existence validation
- [ ] Exclude patterns (plans/**, plan.md, etc.)
- [ ] Broken TOC violation generation
- [ ] Unit tests for tree parsing

## Phase 27: Docs Check - Link Validation

- [ ] Markdown link parsing ([text](path))
- [ ] Local file link detection (vs http/https)
- [ ] Relative path resolution
- [ ] File existence validation
- [ ] Exclude patterns
- [ ] Broken link violation generation

## Phase 28: Docs Check - Specs Directory

- [ ] Specs path configuration (default: docs/specs)
- [ ] Extension filtering (.md default)
- [ ] Index file detection order (CLAUDE.md, overview.md, etc.)
- [ ] index = "exists" mode (just check index exists)

## Phase 29: Docs Check - Specs Index Modes

- [ ] index = "toc" mode (parse directory tree in index)
- [ ] index = "linked" mode (reachability via markdown links)
- [ ] index = "auto" mode (try toc, fallback to linked)
- [ ] Unreachable spec violation generation

## Phase 30: Docs Check - Specs Content

- [ ] Required sections in spec files
- [ ] Forbidden sections in spec files
- [ ] Content rules (tables, diagrams allowed by default)
- [ ] Size limits for spec files

## Phase 31: Docs Check - Commit Checking (CI)

- [ ] check.docs.commit configuration
- [ ] Commit type filtering (feat, breaking, etc.)
- [ ] Branch commit enumeration (vs base)
- [ ] Feature commit identification
- [ ] Doc change detection (any file in docs/)

## Phase 32: Docs Check - Area Mapping

- [ ] Area definition ([check.docs.area.*])
- [ ] Area docs path configuration
- [ ] Area source path configuration
- [ ] Commit scope to area matching (feat(api) -> api area)
- [ ] Source change to area matching
- [ ] Area-specific doc requirement violations

### Checkpoint: Docs Check Complete
- [ ] `quench check --docs` on fixtures/docs-project validates TOC and links
- [ ] `quench check --docs` on quench itself validates docs/specs/
- [ ] Snapshot tests for docs output

## Phase 33a: Tests Check - Specs (Correlation)

- [ ] Spec: --staged checks only staged files
- [ ] Spec: --base REF compares against git ref
- [ ] Spec: source change without test change generates violation
- [ ] Spec: test change without source change passes (TDD)
- [ ] Spec: inline #[cfg(test)] change satisfies test requirement
- [ ] Spec: placeholder test (#[ignore]) satisfies test requirement
- [ ] Spec: excluded files (mod.rs, main.rs) don't require tests
- [ ] Spec: JSON includes source_files_changed, with_test_changes metrics

## Phase 33b: Tests Check - Change Detection

- [ ] Git diff parsing (--staged, --base)
- [ ] Source file change detection
- [ ] Added/modified/deleted classification
- [ ] Lines changed counting
- [ ] Test pattern matching

## Phase 34: Tests Check - Correlation

- [ ] Test file matching for source files
- [ ] Multiple test location search (tests/, *_test.rs, etc.)
- [ ] Inline test change detection (Rust #[cfg(test)])
- [ ] Branch scope: aggregate all changes
- [ ] Commit scope: per-commit with asymmetric rules (tests-first OK)

## Phase 35: Tests Check - Placeholders

- [ ] Rust #[ignore] test detection
- [ ] Rust todo!() body detection
- [ ] JavaScript test.todo() detection
- [ ] JavaScript test.fixme() detection
- [ ] placeholders = "allow" configuration

## Phase 36: Tests Check - Output

- [ ] Missing tests violation generation
- [ ] change_type in violations (added/modified)
- [ ] lines_changed in violations
- [ ] Exclude patterns (mod.rs, main.rs, generated/)
- [ ] JSON output with metrics

### Checkpoint: Tests Correlation Complete
- [ ] `quench check --staged` works in fixtures with staged changes
- [ ] `quench check --base main` works in fixtures with branch changes
- [ ] Snapshot tests for tests correlation output

## Phase 37a: Git Check - Specs

- [ ] Spec: validates commit message format
- [ ] Spec: invalid type generates violation
- [ ] Spec: invalid scope generates violation (when scopes configured)
- [ ] Spec: missing format documentation in CLAUDE.md generates violation
- [ ] Spec: --fix creates .gitmessage template
- [ ] Spec: --fix configures git commit.template

## Phase 37b: Git Check - Message Parsing

- [ ] Commit message extraction (git log)
- [ ] Conventional commit regex parsing
- [ ] Type extraction
- [ ] Scope extraction (optional)
- [ ] Description extraction
- [ ] Unit tests for commit message parsing

## Phase 38: Git Check - Validation

- [ ] Format validation (type: or type(scope):)
- [ ] Type validation against allowed list
- [ ] Scope validation against allowed list (if configured)
- [ ] Invalid format violation generation
- [ ] Invalid type violation generation
- [ ] Invalid scope violation generation

## Phase 39: Git Check - Agent Documentation

- [ ] CLAUDE.md commit format section search
- [ ] Type prefix detection in docs (feat:, fix()
- [ ] "conventional commits" phrase detection
- [ ] Missing documentation violation

## Phase 40: Git Check - Template

- [ ] .gitmessage template generation
- [ ] Template content from config (types, scopes)
- [ ] git config commit.template setting
- [ ] --fix: create template if missing
- [ ] --fix: configure git if not set

### Checkpoint: Git Check Complete
- [ ] `quench check --git` validates commit messages
- [ ] `quench check --git --fix` creates .gitmessage template
- [ ] Snapshot tests for git output

### Dogfooding Milestone 2
- [ ] Use quench in pre-commit hook for quench development
- [ ] `quench check --staged` runs on every commit
- [ ] All fast checks pass on quench codebase

## Phase 41a: CI Mode - Specs

- [ ] Spec: --ci enables slow checks (build, license)
- [ ] Spec: --ci disables violation limit
- [ ] Spec: --ci auto-detects base branch
- [ ] Spec: --save FILE writes metrics to file
- [ ] Spec: --save-notes writes metrics to git notes

## Phase 41b: CI Mode Infrastructure

- [ ] --ci flag handling
- [ ] Base branch auto-detection (main > master > develop)
- [ ] Slow check enabling (build, license)
- [ ] Full violation counting (no limit)
- [ ] Metrics storage path configuration
- [ ] --save FILE flag
- [ ] --save-notes flag (git notes)

## Phase 42a: Test Runners - Specs

- [ ] Spec: cargo runner executes cargo test
- [ ] Spec: cargo runner extracts per-test timing
- [ ] Spec: bats runner executes bats with timing
- [ ] Spec: coverage collected for Rust code
- [ ] Spec: coverage collected for shell scripts via kcov
- [ ] Spec: multiple suite coverages merged

## Phase 42b: Test Runners - Framework

- [ ] Runner trait definition
- [ ] Suite configuration parsing ([[check.tests.suite]])
- [ ] Runner selection by name
- [ ] Setup command execution
- [ ] ci = true filtering (CI-only suites)

## Phase 43: Test Runners - Cargo

- [ ] cargo test --release -- --format json execution
- [ ] JSON output parsing
- [ ] Per-test timing extraction
- [ ] Pass/fail status extraction
- [ ] Test count metrics
- [ ] Integration test: run cargo tests on fixtures/rust-simple

## Phase 44: Test Runners - Cargo Coverage

- [ ] cargo llvm-cov integration
- [ ] Coverage report parsing
- [ ] Line coverage percentage extraction
- [ ] Per-file coverage data

## Phase 45: Test Runners - Bats

- [ ] bats --timing execution
- [ ] TAP output parsing
- [ ] Per-test timing extraction
- [ ] Pass/fail status extraction
- [ ] Integration test: run bats tests on fixtures/shell-scripts

## Phase 46: Test Runners - Other Runners

- [ ] pytest runner (--durations=0 -v)
- [ ] vitest runner (--reporter=json)
- [ ] jest runner (--json)
- [ ] bun runner (--reporter=json)
- [ ] go test runner (-json)
- [ ] Custom command runner (no per-test timing)

## Phase 47: Test Runners - Coverage Targets

- [ ] targets field parsing
- [ ] Build target name resolution (Rust binaries)
- [ ] Glob pattern resolution (shell scripts)
- [ ] Instrumented binary building
- [ ] kcov integration for shell scripts
- [ ] Coverage merging across suites

## Phase 48: Tests Check - CI Mode Metrics

- [ ] Test suite execution orchestration
- [ ] Total time aggregation
- [ ] Average time calculation
- [ ] Max test time tracking (with test name)
- [ ] Coverage aggregation by language
- [ ] Per-package coverage breakdown

## Phase 49a: Tests Check CI Thresholds - Specs

- [ ] Spec: coverage below min generates violation
- [ ] Spec: per-package coverage thresholds work
- [ ] Spec: test time over max_total generates violation
- [ ] Spec: slowest test over max_test generates violation

## Phase 49b: Tests Check - CI Mode Thresholds

- [ ] coverage.min threshold checking
- [ ] Per-package coverage.min
- [ ] time.max_total threshold (per suite)
- [ ] time.max_avg threshold (per suite)
- [ ] time.max_test threshold (per suite)
- [ ] check.tests.time check level (error/warn/off)
- [ ] check.tests.coverage check level

### Checkpoint: Tests CI Mode Complete
- [ ] `quench check --ci --tests` runs tests and collects coverage
- [ ] Coverage and timing metrics in JSON output
- [ ] Snapshot tests for CI tests output

## Phase 50a: Build Check - Specs

- [ ] Spec: detects binary targets from Cargo.toml
- [ ] Spec: measures binary size
- [ ] Spec: binary over size_max generates violation
- [ ] Spec: measures cold build time
- [ ] Spec: measures hot build time
- [ ] Spec: build time over threshold generates violation

## Phase 50b: Build Check - Targets

- [ ] Build target detection from language adapters
- [ ] Rust: [[bin]] entries from Cargo.toml
- [ ] Explicit targets configuration override
- [ ] Per-target configuration ([check.build.target.*])

## Phase 51: Build Check - Size

- [ ] Release build execution
- [ ] Binary file size measurement
- [ ] Strip handling (respect profile.release.strip)
- [ ] size_max threshold checking (global and per-target)
- [ ] Size violation generation

## Phase 52: Build Check - Time

- [ ] Cold build execution (cargo clean && cargo build --release)
- [ ] Cold build timing
- [ ] Hot build execution (touch && cargo build)
- [ ] Hot build timing
- [ ] time_cold_max threshold checking
- [ ] time_hot_max threshold checking
- [ ] Time violation generation

## Phase 53: Build Check - Output

- [ ] Build metrics output (size, time)
- [ ] JSON output with metrics
- [ ] Per-target breakdown

### Checkpoint: Build Check Complete
- [ ] `quench check --ci --build` measures binary size and build time
- [ ] Snapshot tests for build output

## Phase 54a: License Check - Specs

- [ ] Spec: detects SPDX-License-Identifier header
- [ ] Spec: missing header generates violation
- [ ] Spec: wrong license generates violation
- [ ] Spec: outdated year generates violation
- [ ] Spec: --fix adds missing headers
- [ ] Spec: --fix updates outdated years
- [ ] Spec: shebang preserved when adding header

## Phase 54b: License Check - Detection

- [ ] SPDX-License-Identifier line detection
- [ ] Copyright line detection
- [ ] License identifier extraction
- [ ] Copyright year extraction
- [ ] Copyright holder extraction

## Phase 55: License Check - Validation

- [ ] Missing header detection
- [ ] Wrong license detection (vs configured)
- [ ] Outdated year detection (vs current year)
- [ ] File pattern filtering by language
- [ ] Exclude patterns

## Phase 56: License Check - Comment Syntax

- [ ] Extension to comment style mapping
- [ ] // style (rs, ts, js, go, c, cpp, h)
- [ ] # style (sh, bash, py, rb, yaml)
- [ ] <!-- --> style (html, xml)
- [ ] Custom syntax configuration override

## Phase 57: License Check - Fix

- [ ] Header generation from config (license, copyright)
- [ ] Header insertion at file start
- [ ] Shebang preservation (insert after #!)
- [ ] Year update in existing headers
- [ ] --fix output (added/updated counts)

### Checkpoint: License Check Complete
- [ ] `quench check --ci --license` detects missing/wrong headers
- [ ] `quench check --ci --license --fix` adds headers correctly
- [ ] Snapshot tests for license output

### Dogfooding Milestone 3
- [ ] Full `quench check --ci` runs on quench itself
- [ ] All CI checks pass
- [ ] Coverage metrics collected for quench

## Phase 58a: Ratcheting - Specs

- [ ] Spec: baseline file read on check
- [ ] Spec: coverage below baseline generates violation
- [ ] Spec: escape count above baseline generates violation
- [ ] Spec: tolerance allows small regressions
- [ ] Spec: --fix updates baseline when metrics improve
- [ ] Spec: baseline not updated when metrics regress
- [ ] Spec: per-package ratcheting works

## Phase 58b: Ratcheting - Baseline

- [ ] Baseline file path configuration ([git].baseline)
- [ ] Baseline file reading
- [ ] Baseline file format (version, updated, commit, metrics)
- [ ] Baseline writing on --fix
- [ ] Git notes reading (alternative storage)
- [ ] Git notes writing (--save-notes)

## Phase 59: Ratcheting - Coverage

- [ ] Coverage floor tracking
- [ ] Coverage regression detection
- [ ] coverage_tolerance configuration
- [ ] Coverage improvement detection
- [ ] Floor update on improvement

## Phase 60: Ratcheting - Escapes

- [ ] Per-pattern count ceiling tracking
- [ ] Escape count regression detection
- [ ] Escape improvement detection
- [ ] Ceiling update on improvement

## Phase 61: Ratcheting - Build Metrics

- [ ] binary_size ceiling tracking (opt-in)
- [ ] build_time_cold ceiling tracking (opt-in)
- [ ] build_time_hot ceiling tracking (opt-in)
- [ ] Size/time tolerance configuration
- [ ] Regression detection and violation

## Phase 62: Ratcheting - Test Time

- [ ] test_time_total ceiling tracking (opt-in)
- [ ] test_time_avg ceiling tracking (opt-in)
- [ ] test_time_max ceiling tracking (opt-in)

## Phase 63: Ratcheting - Per-Package

- [ ] Per-package baseline storage
- [ ] Per-package ratchet configuration
- [ ] Package-level regression detection
- [ ] Package exclusion from ratcheting

## Phase 64: Ratcheting - Output

- [ ] Ratchet pass output (current vs baseline)
- [ ] Ratchet fail output (regression details)
- [ ] JSON ratchet section
- [ ] --fix baseline update output

### Checkpoint: Ratcheting Complete
- [ ] Baseline file created with `quench check --ci --fix --save .quench/baseline.json`
- [ ] Regression detected when metrics worsen
- [ ] Baseline updates when metrics improve
- [ ] Snapshot tests for ratchet output

## Phase 65a: Report Command - Specs

- [ ] Spec: quench report reads baseline file
- [ ] Spec: text format shows summary
- [ ] Spec: JSON format outputs metrics
- [ ] Spec: HTML format produces valid HTML
- [ ] Spec: -o report.html writes to file

## Phase 65b: Report Command - Basic

- [ ] quench report command
- [ ] Baseline file reading
- [ ] Check toggle flags (same as check)
- [ ] Text format output (default)

## Phase 66: Report Command - Formats

- [ ] JSON format output (-o json)
- [ ] HTML format output (-o html)
- [ ] File output (-o report.html)
- [ ] Metric cards and summary tables

### Checkpoint: Report Command Complete
- [ ] `quench report` produces readable summary
- [ ] `quench report -o json` produces valid JSON
- [ ] `quench report -o html` produces valid HTML

## Phase 67: Performance - Caching

- [ ] File cache structure (path -> mtime, size, result)
- [ ] Cache lookup before file processing
- [ ] Cache population after processing
- [ ] In-memory cache for single session
- [ ] Persistent cache (.quench/cache.bin)
- [ ] Cache invalidation (config change, version change)

## Phase 68: Performance - File Reading

- [ ] Size check before reading (from metadata)
- [ ] Direct read for files < 64KB
- [ ] Memory-mapped I/O for files 64KB - 10MB
- [ ] Skip with warning for files > 10MB
- [ ] Report as oversized for files > 1MB

## Phase 69: Performance - Timeouts

- [ ] Per-file processing timeout (5s default)
- [ ] Timeout handling (skip file, continue)
- [ ] Timeout warning in output

## Phase 70: Performance - Early Termination

- [ ] Violation limit tracking during scan
- [ ] Early file termination when limit reached
- [ ] Early check termination when limit reached
- [ ] Disable early termination in CI mode

### Checkpoint: Performance Complete
- [ ] Benchmark: cold run < 500ms on 50K LOC fixture
- [ ] Benchmark: warm run < 100ms on 50K LOC fixture
- [ ] Large file (>10MB) skipped with warning
- [ ] Cache invalidation works correctly

## Phase 71a: Init Command - Specs

- [ ] Spec: creates quench.toml in current directory
- [ ] Spec: detects Rust project and configures accordingly
- [ ] Spec: detects shell project and configures accordingly
- [ ] Spec: detects mixed project and configures both
- [ ] Spec: refuses to overwrite without --force
- [ ] Spec: --force overwrites existing config

## Phase 71b: Init Command - Implementation

- [ ] quench init command
- [ ] Existing config detection
- [ ] --force flag to overwrite
- [ ] Language detection (Cargo.toml, *.sh, etc.)
- [ ] Default config generation based on detected languages
- [ ] Config file writing

### Checkpoint: Init Command Complete
- [ ] `quench init` on fixtures/rust-simple creates appropriate config
- [ ] `quench init` on fixtures/shell-scripts creates appropriate config
- [ ] `quench init` on fixtures/mixed detects both languages

## Phase 72: Polish

- [ ] Comprehensive --help text
- [ ] Error message improvements
- [ ] Config validation error formatting
- [ ] Shell completions (bash, zsh, fish)
- [ ] Man page generation

### Final Validation

- [ ] All snapshot tests pass
- [ ] All integration tests pass
- [ ] `quench check` on quench itself passes
- [ ] `quench check --ci` on quench itself passes with metrics
- [ ] Pre-commit hook works reliably
- [ ] JSON output validates against output.schema.json
- [ ] Performance targets met (< 500ms cold, < 100ms warm)
