# QA Runbook: Checkpoint 17H and Related Features

Generated from comprehensive code review on 2026-01-24.

## 1. Performance Infrastructure

### 1.1 Cache Behavior
```bash
# Cold run (no cache)
rm -rf .quench/cache && time cargo run --release -- check
# Expected: <200ms, cache directory created

# Warm run (cached)
time cargo run --release -- check
# Expected: <50ms

# Cache invalidation on config change
echo "# comment" >> quench.toml && time cargo run --release -- check
# Expected: Full recheck, cache rebuilt
```

### 1.2 Memory-Mapped I/O
```bash
# Create large file (>64KB threshold)
dd if=/dev/urandom of=tests/fixtures/large-file.rs bs=1024 count=100

# Run check on large file
cargo run --release -- check tests/fixtures/large-file.rs

# Clean up
rm tests/fixtures/large-file.rs
```

### 1.3 Pattern Matching
```bash
# Verify literal patterns work
cargo run --release -- check --pattern "TODO"

# Verify regex patterns work
cargo run --release -- check --pattern "TODO.*FIXME"
```

## 2. Git Check Feature

### 2.1 Conventional Commits
```bash
# Valid commit message
echo "feat(scope): add feature" | cargo run -- check-git-message -

# Invalid commit message (missing type)
echo "add feature" | cargo run -- check-git-message -
# Expected: Error with suggestion

# Invalid scope format
echo "feat[scope]: add feature" | cargo run -- check-git-message -
# Expected: Error about scope format
```

### 2.2 Template Generation
```bash
# Generate .gitmessage template
cargo run -- git init-template

# Verify template created
cat .gitmessage
# Expected: Template with type prefixes

# Verify git config set
git config commit.template
# Expected: .gitmessage path
```

### 2.3 Git Hook Integration
```bash
# Install commit-msg hook
cargo run -- git install-hook

# Verify hook exists and is executable
ls -la .git/hooks/commit-msg

# Test hook with invalid message
echo "bad message" > /tmp/msg && .git/hooks/commit-msg /tmp/msg
# Expected: Non-zero exit, error message
```

## 3. Report Command

### 3.1 Output Formats
```bash
# Text format (default)
cargo run -- check --format text
# Expected: Human-readable violations

# JSON format
cargo run -- check --format json | jq .
# Expected: Valid JSON with violations array

# Compact JSON
cargo run -- check --format json --compact
# Expected: Single-line JSON

# Markdown format
cargo run -- check --format markdown
# Expected: Markdown table with violations

# HTML format
cargo run -- check --format html > report.html
# Expected: Standalone HTML file, open in browser
```

### 3.2 Human-Readable Bytes
```bash
# Check file size display
cargo run -- check --verbose
# Expected: "14.5 KB" not "14848 bytes"
```

### 3.3 Summary Statistics
```bash
# Verify summary at end of output
cargo run -- check
# Expected: "X violations in Y files (Z ms)"
```

## 4. Timing and Profiling

### 4.1 Timing Flag
```bash
# Enable timing output
cargo run -- check --timing
# Expected: Phase breakdown (discovery, checking, output)

# Timing with JSON
cargo run -- check --timing --format json | jq .timing
# Expected: timing object in JSON output
```

### 4.2 Performance Budgets
```bash
# Run benchmark suite
./scripts/perf/benchmark.sh

# Check against budgets
./scripts/perf/check-budget.sh
# Expected: All budgets pass
```

### 4.3 Profiling Workflow
```bash
# Generate flamegraph (requires cargo-flamegraph)
cargo flamegraph --bin quench -- check tests/fixtures/bench-medium

# Check memory usage
/usr/bin/time -v cargo run --release -- check 2>&1 | grep "Maximum resident"
# Expected: <50MB
```

## 5. Edge Cases

### 5.1 Empty Project
```bash
mkdir /tmp/empty-project && cd /tmp/empty-project
cargo run -- check
# Expected: "No files to check" or similar

rm -rf /tmp/empty-project
```

### 5.2 Binary Files
```bash
# Verify binary files are skipped
cargo run -- check --verbose 2>&1 | grep -i binary
# Expected: Binary files not processed
```

### 5.3 Symlink Loops
```bash
cargo run -- check tests/fixtures/symlink-loop
# Expected: No infinite loop, graceful handling
```

### 5.4 Non-UTF-8 Files
```bash
# Create non-UTF-8 file
printf '\x80\x81\x82' > /tmp/bad-utf8.rs
cargo run -- check /tmp/bad-utf8.rs
# Expected: Graceful skip or error message

rm /tmp/bad-utf8.rs
```

## 6. Integration Scenarios

### 6.1 CI Pipeline Simulation
```bash
# Full CI check
make check
# Expected: All checks pass

# With timing for performance regression detection
cargo run --release -- check --timing --format json > ci-results.json
```

### 6.2 Dogfooding
```bash
# Run quench on itself
cargo run --release -- check
# Expected: Clean or known violations only
```

## Verification Checklist

- [ ] All automated tests pass: `cargo test --all`
- [ ] Clippy clean: `cargo clippy --all-targets -- -D warnings`
- [ ] Formatted: `cargo fmt --all -- --check`
- [ ] Audit clean: `cargo audit`
- [ ] Dependencies OK: `cargo deny check`
- [ ] Performance within budget
- [ ] Manual scenarios above completed

## Related Plans

Gaps identified during review are tracked in:
- **2026-01-24-close-gaps.md** - Closes all test coverage and API gaps
