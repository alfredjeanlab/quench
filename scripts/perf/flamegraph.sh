#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Generate flame graphs for quench
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

FIXTURE="${1:-tests/fixtures/stress-monorepo}"
REPORT_DIR="reports/profiling"
mkdir -p "$REPORT_DIR"

# Check for flamegraph
if ! command -v flamegraph &>/dev/null; then
    echo "Installing flamegraph..."
    cargo install flamegraph
fi

# Build with debug symbols
echo "Building with debug symbols..."
RUSTFLAGS="-g" cargo build --release --quiet

# Generate fixtures if needed
if [ ! -d "$FIXTURE" ]; then
    echo "Generating stress fixtures..."
    ./scripts/fixtures/generate-stress-fixtures
fi

echo "=== Generating cold run flamegraph ==="
rm -rf "$FIXTURE/.quench"

# On macOS, flamegraph needs dtrace permissions
if [[ "$(uname)" == "Darwin" ]]; then
    echo "Note: On macOS, flamegraph may require running with sudo or dtrace permissions"
    echo "If this fails, try: sudo flamegraph -o $REPORT_DIR/cold-flamegraph.svg -- ./target/release/quench check $FIXTURE"
fi

flamegraph -o "$REPORT_DIR/cold-flamegraph.svg" \
    -- ./target/release/quench check "$FIXTURE" 2>/dev/null || {
    echo "Flamegraph failed. This may require elevated permissions on macOS."
    echo "Try running with sudo if needed."
}

if [ -f "$REPORT_DIR/cold-flamegraph.svg" ]; then
    echo "Cold flamegraph saved to: $REPORT_DIR/cold-flamegraph.svg"
fi

echo ""
echo "=== Generating warm run flamegraph ==="
# Warm the cache first
./target/release/quench check "$FIXTURE" >/dev/null 2>&1 || true

flamegraph -o "$REPORT_DIR/warm-flamegraph.svg" \
    -- ./target/release/quench check "$FIXTURE" 2>/dev/null || {
    echo "Flamegraph failed for warm run."
}

if [ -f "$REPORT_DIR/warm-flamegraph.svg" ]; then
    echo "Warm flamegraph saved to: $REPORT_DIR/warm-flamegraph.svg"
fi

echo ""
echo "=== Flamegraph generation complete ==="
echo "Reports in: $REPORT_DIR/"
ls -la "$REPORT_DIR/"*.svg 2>/dev/null || echo "No SVG files generated"
