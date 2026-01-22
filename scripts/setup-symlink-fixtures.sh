#!/bin/bash
# Creates symlinks for testing (run before tests)

set -euo pipefail

FIXTURE_DIR="tests/fixtures/symlink-loop"

# Create symlink loop (directory pointing to itself)
if [ ! -L "$FIXTURE_DIR/loop" ]; then
    ln -s . "$FIXTURE_DIR/loop"
    echo "Created symlink loop: $FIXTURE_DIR/loop -> ."
fi

# Create indirect loop (a -> b -> a)
mkdir -p "$FIXTURE_DIR/indirect"
if [ ! -L "$FIXTURE_DIR/indirect/a" ]; then
    mkdir -p "$FIXTURE_DIR/indirect"
    ln -s ../indirect "$FIXTURE_DIR/indirect/a" 2>/dev/null || true
    echo "Created indirect symlink loop"
fi
