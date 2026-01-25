#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Check for performance regressions against baseline
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

THRESHOLD_PCT="${THRESHOLD_PCT:-20}"  # 20% regression threshold

# Generate fixtures if needed
if [ ! -d "tests/fixtures/bench-medium" ]; then
    echo "Generating benchmark fixtures..."
    ./scripts/fixtures/generate-bench-fixtures
fi

# Build release binary
echo "Building release binary..."
cargo build --release --quiet

# Check if baseline exists
if [ ! -d "target/criterion" ]; then
    echo "No baseline found. Creating initial baseline..."
    cargo bench --bench check -- --save-baseline main --noplot
    cargo bench --bench cache -- --save-baseline main --noplot
    echo "Baseline created. Run this script again after making changes."
    exit 0
fi

# Run benchmarks comparing to baseline
echo "Running benchmarks and comparing to baseline..."
OUTPUT=$(cargo bench --bench check -- --baseline main 2>&1 || true)
echo "$OUTPUT"

# Check for regressions
if echo "$OUTPUT" | grep -q "Performance has regressed"; then
    REGRESSIONS=$(echo "$OUTPUT" | grep -B2 -A2 "Performance has regressed" || true)
    echo ""
    echo "====================================="
    echo "ERROR: Performance regression detected!"
    echo "====================================="
    echo "$REGRESSIONS"
    echo ""
    echo "Threshold: ${THRESHOLD_PCT}% slower than baseline"
    exit 1
fi

# Run cache benchmarks too
CACHE_OUTPUT=$(cargo bench --bench cache -- --baseline main 2>&1 || true)
echo "$CACHE_OUTPUT"

if echo "$CACHE_OUTPUT" | grep -q "Performance has regressed"; then
    REGRESSIONS=$(echo "$CACHE_OUTPUT" | grep -B2 -A2 "Performance has regressed" || true)
    echo ""
    echo "====================================="
    echo "ERROR: Cache performance regression detected!"
    echo "====================================="
    echo "$REGRESSIONS"
    exit 1
fi

echo ""
echo "====================================="
echo "Performance check passed!"
echo "====================================="
echo "No significant regressions detected (threshold: ${THRESHOLD_PCT}%)"
