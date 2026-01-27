# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Commands
- Shell completions via `clap_complete` for bash, zsh, fish, and PowerShell

#### Behavior
- **Ratcheting**: Git notes as default baseline storage (replaces `.quench/baseline.json`)
- **Ratcheting**: Coverage ratcheting with per-package support
- **License check**: `--fix` functionality for automatic license file generation

#### Test Runners
- Ruby test runners: RSpec, Minitest, and Cucumber support
- JavaScript coverage collection for JS test runners
- JavaScript test runner auto-detection
- Go coverage collection for Go test runner

#### Adapters
- Ruby language adapter with project detection and escape patterns
- Ruby language profile support in `quench init`

#### Build Metrics
- JavaScript bundle size metrics

## [0.3.0]

### Added

#### Commands
- `quench init` command with language and agent auto-detection
- `quench report` command with JSON, HTML, and Markdown output formats
- `--timing` flag for performance breakdown analysis

#### Behavior
- **Ratcheting**: Quality metrics can improve but not regress. Compares current metrics against `.quench/baseline.json`. Use `--fix` to update baseline when metrics improve.

#### Checks
- **git check**: Conventional commit message validation with `.gitmessage` template generation
- **git check**: Agent documentation check for AI coding assistants
- **git check**: Skip merge commits option (`skip_merge`)
- **tests check**: Source/test correlation with commit-scope checking
- **tests check**: Placeholder metrics for Rust (`#[ignore]`, `todo!()`) and JS/TS (`test.todo()`, `it.skip()`)
- **docs check**: Table of contents validation with explicit `toc`/`no-toc` annotations
- **docs check**: Specs index validation modes (toc, linked, auto)
- **docs check**: Markdown link validation
- **docs check**: Source-based area detection for commit checking
- **cloc check**: Per-language check levels
- **policy check**: Per-language policy enforcement

#### Adapters
- JavaScript/TypeScript adapter with project detection and escape patterns
- Go adapter improvements with profile support

#### Configuration
- Agent profiles and ratcheting config in `quench init`
- Typo suggestions for unknown check names
- `cfg_test_split` modes (count, require, off)
- Scope field added to Violation output

### Changed

- Migrated git operations from subprocess calls to git2 library (6x speedup)
- Optimized warm cache performance with memory-mapped file reading
- Improved tests correlation check with O(1) lookups
- Enhanced docs check with lazy regex and parallel processing
- Replaced `--profile` with `--with` for `quench init`
- Removed `[workspace]` config namespace, consolidated into `[project]`

### Fixed

- Handle deleted files in staged/changed file detection
- Multi-line attribute parsing for `cfg(test)` and `allow`/`expect`
- Edge cases in tests correlation check
- Ratchet message output, decimal precision, and unused parameter handling
- `.cjs`/`.cts` extension support in JavaScript adapter

### Performance

- git2 migration provides 6x speedup over subprocess calls
- Memory-mapped file reading reduces allocations
- Lazy regex compilation in docs check
- Parallel processing for docs validation
- O(1) lookups in tests correlation
- Fast-path optimizations in JavaScript adapter
- Performance budget enforcement and profiling infrastructure

## [0.2.0]

Initial public release with core linting functionality.

## [0.1.0]

Internal preview release.

[Unreleased]: https://github.com/anthropics/quench/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/anthropics/quench/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/anthropics/quench/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/anthropics/quench/releases/tag/v0.1.0
