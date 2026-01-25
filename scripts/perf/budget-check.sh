#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Enforce performance budgets - fails CI on regression
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# Performance budgets (in milliseconds)
COLD_TARGET=500
COLD_ACCEPTABLE=1000
COLD_UNACCEPTABLE=2000

WARM_TARGET=100
WARM_ACCEPTABLE=200
WARM_UNACCEPTABLE=500

MEMORY_TARGET_MB=100
MEMORY_LIMIT_MB=500

FIXTURE="tests/fixtures/bench-medium"

# Build release
echo "Building release binary..."
cargo build --release --quiet

# Generate fixtures if needed
if [ ! -d "$FIXTURE" ]; then
    echo "Generating benchmark fixtures..."
    ./scripts/fixtures/generate-bench-fixtures
fi

echo "=== Performance Budget Check ==="
echo ""

# Cold run (average of 3)
echo "Cold run (3 runs, cache cleared each time):"
rm -rf "$FIXTURE/.quench"
COLD_TOTAL=0
for i in 1 2 3; do
    rm -rf "$FIXTURE/.quench"
    START=$(python3 -c 'import time; print(int(time.time() * 1000))')
    ./target/release/quench check "$FIXTURE" >/dev/null 2>&1 || true
    END=$(python3 -c 'import time; print(int(time.time() * 1000))')
    COLD_MS=$((END - START))
    COLD_TOTAL=$((COLD_TOTAL + COLD_MS))
    echo "  Run $i: ${COLD_MS}ms"
done
COLD_AVG=$((COLD_TOTAL / 3))
echo "  Average: ${COLD_AVG}ms (target: <${COLD_TARGET}ms, limit: <${COLD_ACCEPTABLE}ms)"

if [ "$COLD_AVG" -gt "$COLD_UNACCEPTABLE" ]; then
    echo "::error::FAIL: Cold run ${COLD_AVG}ms exceeds unacceptable threshold ${COLD_UNACCEPTABLE}ms"
    exit 1
elif [ "$COLD_AVG" -gt "$COLD_ACCEPTABLE" ]; then
    echo "::warning::WARN: Cold run ${COLD_AVG}ms exceeds acceptable threshold ${COLD_ACCEPTABLE}ms"
elif [ "$COLD_AVG" -gt "$COLD_TARGET" ]; then
    echo "  Note: Above target but within acceptable range"
else
    echo "  OK: Within target"
fi
echo ""

# Warm run (average of 5, after warmup)
echo "Warm run (5 runs, cache warm):"
./target/release/quench check "$FIXTURE" >/dev/null 2>&1 || true  # warmup
WARM_TOTAL=0
for i in 1 2 3 4 5; do
    START=$(python3 -c 'import time; print(int(time.time() * 1000))')
    ./target/release/quench check "$FIXTURE" >/dev/null 2>&1 || true
    END=$(python3 -c 'import time; print(int(time.time() * 1000))')
    WARM_MS=$((END - START))
    WARM_TOTAL=$((WARM_TOTAL + WARM_MS))
    echo "  Run $i: ${WARM_MS}ms"
done
WARM_AVG=$((WARM_TOTAL / 5))
echo "  Average: ${WARM_AVG}ms (target: <${WARM_TARGET}ms, limit: <${WARM_ACCEPTABLE}ms)"

if [ "$WARM_AVG" -gt "$WARM_UNACCEPTABLE" ]; then
    echo "::error::FAIL: Warm run ${WARM_AVG}ms exceeds unacceptable threshold ${WARM_UNACCEPTABLE}ms"
    exit 1
elif [ "$WARM_AVG" -gt "$WARM_ACCEPTABLE" ]; then
    echo "::warning::WARN: Warm run ${WARM_AVG}ms exceeds acceptable threshold ${WARM_ACCEPTABLE}ms"
elif [ "$WARM_AVG" -gt "$WARM_TARGET" ]; then
    echo "  Note: Above target but within acceptable range"
else
    echo "  OK: Within target"
fi
echo ""

# Memory check
echo "Memory usage:"
if command -v /usr/bin/time &>/dev/null; then
    if [[ "$(uname)" == "Linux" ]]; then
        MEM_OUTPUT=$(/usr/bin/time -v ./target/release/quench check "$FIXTURE" 2>&1 || true)
        MEM_KB=$(echo "$MEM_OUTPUT" | grep "Maximum resident" | awk -F': ' '{print $2}')
        if [ -n "$MEM_KB" ]; then
            MEM_MB=$((MEM_KB / 1024))
        else
            MEM_MB=0
        fi
    else
        # macOS
        MEM_OUTPUT=$(/usr/bin/time -l ./target/release/quench check "$FIXTURE" 2>&1 || true)
        MEM_BYTES=$(echo "$MEM_OUTPUT" | grep "peak memory footprint" | awk '{print $1}' | head -1)
        if [ -n "$MEM_BYTES" ] && [ "$MEM_BYTES" -gt 0 ] 2>/dev/null; then
            MEM_MB=$((MEM_BYTES / 1024 / 1024))
        else
            # Fallback to maximum resident set size
            MEM_BYTES=$(echo "$MEM_OUTPUT" | grep "maximum resident set size" | awk '{print $1}' | head -1)
            if [ -n "$MEM_BYTES" ] && [ "$MEM_BYTES" -gt 0 ] 2>/dev/null; then
                MEM_MB=$((MEM_BYTES / 1024 / 1024))
            else
                MEM_MB=0
            fi
        fi
    fi

    if [ "$MEM_MB" -gt 0 ]; then
        echo "  Peak memory: ${MEM_MB}MB (target: <${MEMORY_TARGET_MB}MB, limit: <${MEMORY_LIMIT_MB}MB)"

        if [ "$MEM_MB" -gt "$MEMORY_LIMIT_MB" ]; then
            echo "::error::FAIL: Memory ${MEM_MB}MB exceeds limit ${MEMORY_LIMIT_MB}MB"
            exit 1
        elif [ "$MEM_MB" -gt "$MEMORY_TARGET_MB" ]; then
            echo "::warning::WARN: Memory ${MEM_MB}MB exceeds target ${MEMORY_TARGET_MB}MB"
        else
            echo "  OK: Within target"
        fi
    else
        echo "  Could not measure memory (skipping check)"
    fi
else
    echo "  /usr/bin/time not available (skipping memory check)"
fi
echo ""

echo "=== All budgets passed ==="
