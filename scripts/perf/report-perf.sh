#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Generate performance report
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

QUENCH="${QUENCH:-./target/release/quench}"
FIXTURES_DIR="tests/fixtures"
REPORT_FILE="${1:-reports/perf-$(date +%Y%m%d).md}"

# Build release if needed
if [ ! -f "$QUENCH" ]; then
    echo "Building release binary..."
    cargo build --release --quiet
fi

# Generate fixtures if needed
if [ ! -d "$FIXTURES_DIR/bench-medium" ]; then
    echo "Generating benchmark fixtures..."
    ./scripts/fixtures/generate-bench-fixtures
fi

mkdir -p "$(dirname "$REPORT_FILE")"

# Get commit info
COMMIT_SHA=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
COMMIT_DATE=$(git log -1 --format=%ci 2>/dev/null || echo "unknown")

cat > "$REPORT_FILE" << EOF
# Performance Report - $(date +%Y-%m-%d)

## Environment
- Commit: $COMMIT_SHA ($COMMIT_DATE)
- Platform: $(uname -sm)
- Rust: $(rustc --version 2>/dev/null || echo "unknown")

## Results

| Fixture | Mode | Files | Time (ms) | Memory (MB) | Status |
|---------|------|-------|-----------|-------------|--------|
EOF

for fixture in bench-small bench-medium bench-large; do
    path="$FIXTURES_DIR/$fixture"
    [ -d "$path" ] || continue

    files=$(find "$path" -type f -name '*.rs' 2>/dev/null | wc -l | tr -d ' ')

    # Cold run - use /usr/bin/time for cross-platform timing
    rm -rf "$path/.quench"
    if [ "$(uname)" = "Darwin" ]; then
        cold_output=$(/usr/bin/time -p "$QUENCH" check "$path" 2>&1 || true)
        cold_sec=$(echo "$cold_output" | grep "^real" | awk '{print $2}')
        cold=$(echo "$cold_sec * 1000" | bc | cut -d. -f1)
    else
        cold_output=$(/usr/bin/time -f "%e" "$QUENCH" check "$path" 2>&1 || true)
        cold_sec=$(echo "$cold_output" | tail -1)
        cold=$(echo "$cold_sec * 1000" | bc | cut -d. -f1)
    fi

    # Warm run
    if [ "$(uname)" = "Darwin" ]; then
        warm_output=$(/usr/bin/time -p "$QUENCH" check "$path" 2>&1 || true)
        warm_sec=$(echo "$warm_output" | grep "^real" | awk '{print $2}')
        warm=$(echo "$warm_sec * 1000" | bc | cut -d. -f1)
    else
        warm_output=$(/usr/bin/time -f "%e" "$QUENCH" check "$path" 2>&1 || true)
        warm_sec=$(echo "$warm_output" | tail -1)
        warm=$(echo "$warm_sec * 1000" | bc | cut -d. -f1)
    fi

    # Measure memory (macOS specific)
    if [ "$(uname)" = "Darwin" ]; then
        mem_output=$(/usr/bin/time -l "$QUENCH" check "$path" 2>&1 || true)
        mem_bytes=$(echo "$mem_output" | grep "peak memory footprint" | awk '{print $1}' | head -1)
        if [ -n "$mem_bytes" ]; then
            mem_mb=$((mem_bytes / 1024 / 1024))
        else
            mem_mb="N/A"
        fi
    elif [ "$(uname)" = "Linux" ]; then
        mem_output=$(/usr/bin/time -v "$QUENCH" check "$path" 2>&1 || true)
        mem_kb=$(echo "$mem_output" | grep "Maximum resident" | awk -F': ' '{print $2}')
        if [ -n "$mem_kb" ]; then
            mem_mb=$((mem_kb / 1024))
        else
            mem_mb="N/A"
        fi
    else
        mem_mb="N/A"
    fi

    # Determine status based on spec thresholds
    cold_status="OK"
    if [ "$cold" -gt 2000 ]; then
        cold_status="FAIL"
    elif [ "$cold" -gt 1000 ]; then
        cold_status="WARN"
    fi

    warm_status="OK"
    if [ "$warm" -gt 500 ]; then
        warm_status="FAIL"
    elif [ "$warm" -gt 200 ]; then
        warm_status="WARN"
    fi

    echo "| $fixture | cold | $files | $cold | $mem_mb | $cold_status |" >> "$REPORT_FILE"
    echo "| $fixture | warm | $files | $warm | - | $warm_status |" >> "$REPORT_FILE"
done

cat >> "$REPORT_FILE" << 'EOF'

## Thresholds (from docs/specs/20-performance.md)

| Mode | Target | Acceptable | Unacceptable |
|------|--------|------------|--------------|
| Cold | < 500ms | < 1s | > 2s |
| Warm | < 100ms | < 200ms | > 500ms |
| Memory | < 100MB | < 500MB | > 2GB |

## Legend

- **OK**: Within target or acceptable range
- **WARN**: Approaching unacceptable threshold
- **FAIL**: Exceeded unacceptable threshold

## Notes

Performance measured on local machine. CI may vary. Cold runs include cache
generation time. Warm runs benefit from in-memory cache.
EOF

echo ""
echo "Report written to: $REPORT_FILE"
echo ""
cat "$REPORT_FILE"
