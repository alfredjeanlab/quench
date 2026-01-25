#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Run profiling on quench and generate reports
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

FIXTURE="${1:-tests/fixtures/stress-monorepo}"
REPORT_DIR="reports/profiling"
mkdir -p "$REPORT_DIR"

# Build with debug symbols for profiling
echo "Building with debug symbols..."
RUSTFLAGS="-g" cargo build --release --quiet

# Generate fixtures if needed
if [ ! -d "$FIXTURE" ]; then
    echo "Generating stress fixtures..."
    ./scripts/fixtures/generate-stress-fixtures
fi

echo "=== Profiling cold run ==="
rm -rf "$FIXTURE/.quench"

if [[ "$(uname)" == "Darwin" ]]; then
    # macOS: Use sample for quick profiling
    echo "Using macOS sample profiler..."

    # Start sample in background waiting for our process
    sample ./target/release/quench 5 -file "$REPORT_DIR/cold-sample.txt" \
        -wait &
    SAMPLE_PID=$!
    sleep 0.5  # Let sample attach
    ./target/release/quench check "$FIXTURE" >/dev/null 2>&1 || true
    wait $SAMPLE_PID 2>/dev/null || true

    echo "Cold run sample saved to: $REPORT_DIR/cold-sample.txt"

    # Try xctrace if available (requires Xcode)
    if command -v xcrun &>/dev/null && xcrun xctrace help &>/dev/null; then
        echo "Generating Time Profiler trace..."
        rm -rf "$FIXTURE/.quench"
        xcrun xctrace record --template 'Time Profiler' \
            --output "$REPORT_DIR/cold-trace.trace" \
            --launch -- ./target/release/quench check "$FIXTURE" 2>/dev/null || true
        echo "Trace saved to: $REPORT_DIR/cold-trace.trace"
    fi
else
    # Linux: Use perf
    if command -v perf &>/dev/null; then
        echo "Using Linux perf profiler..."
        perf record -g -o "$REPORT_DIR/cold-perf.data" \
            ./target/release/quench check "$FIXTURE" 2>/dev/null || true
        perf report -i "$REPORT_DIR/cold-perf.data" --stdio > "$REPORT_DIR/cold-perf.txt" 2>/dev/null || true
        echo "Cold run perf report saved to: $REPORT_DIR/cold-perf.txt"
    else
        echo "Warning: perf not found. Install linux-perf for profiling."
    fi
fi

echo ""
echo "=== Profiling warm run ==="
# Warm the cache first
./target/release/quench check "$FIXTURE" >/dev/null 2>&1 || true

if [[ "$(uname)" == "Darwin" ]]; then
    sample ./target/release/quench 5 -file "$REPORT_DIR/warm-sample.txt" \
        -wait &
    SAMPLE_PID=$!
    sleep 0.5
    ./target/release/quench check "$FIXTURE" >/dev/null 2>&1 || true
    wait $SAMPLE_PID 2>/dev/null || true
    echo "Warm run sample saved to: $REPORT_DIR/warm-sample.txt"
else
    if command -v perf &>/dev/null; then
        perf record -g -o "$REPORT_DIR/warm-perf.data" \
            ./target/release/quench check "$FIXTURE" 2>/dev/null || true
        perf report -i "$REPORT_DIR/warm-perf.data" --stdio > "$REPORT_DIR/warm-perf.txt" 2>/dev/null || true
        echo "Warm run perf report saved to: $REPORT_DIR/warm-perf.txt"
    fi
fi

echo ""
echo "=== Profiling complete ==="
echo "Reports saved to: $REPORT_DIR/"
ls -la "$REPORT_DIR/"
