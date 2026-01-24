# Quench (Quality Benchmarks)

A fast linting tool for AI agents that measures quality signals.
Configured via `quench.toml`.

## Directory Structure

```
quench/
├── README.md         # Human-focused introduction
├── crates/           # Rust workspace
│   └── cli/          # CLI binary
├── docs/
│   ├── specs/        # Feature specifications (the "what")
│   └── arch/         # Architecture decisions
├── plans/            # Implementation plans (the "how" and "when")
├── tests/
│   ├── specs/        # Behavioral tests (black-box, see tests/specs/CLAUDE.md)
│   └── fixtures/     # Test projects
└── scripts/          # Build and utility scripts
```

## Unit Test Convention

Use sibling `_tests.rs` files instead of inline `#[cfg(test)]` modules:

```rust
// src/parser.rs
#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
```

```rust
// src/parser_tests.rs
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn parses_empty_input() { ... }
```

**Why separate files?**
- Shorter source files fit better in LLM context windows
- LOC metrics reflect implementation conciseness, not test volume
- Integration tests remain in `tests/` as usual

## Development Lifecycle

1. Specs are written from `docs/specs/`, not from implementation
2. Write specs first, mark unimplemented with `#[ignore = "TODO: Phase N"]`
3. Implement feature in `src/`
4. Remove `#[ignore]`, verify specs pass
5. List passing specs in commit message

## Commits

Use conventional commit format: `type(scope): description`

Types: feat, fix, chore, docs, test, refactor, perf, ci, build, style

## Landing the Plane

Before committing changes:

- [ ] Unit tests in sibling `_tests.rs` files
- [ ] Bump `CACHE_VERSION` in `crates/cli/src/cache.rs` if check logic changed
- [ ] Run `make check` which will
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all`
  - `cargo build --all`
  - `cargo audit`
  - `cargo deny check`
