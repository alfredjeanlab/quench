#!/bin/bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Alfred Jean LLC
# Generates deeply nested directory structure for depth limit testing

set -euo pipefail

FIXTURE_DIR="tests/fixtures/bench-deep"
DEPTH=120  # Exceeds default limit of 100

# Clean existing nested structure
rm -rf "$FIXTURE_DIR/deep"

# Build path incrementally using short names to avoid path length limits
# Use "d" (for directory) with the number to keep paths short
current_path="$FIXTURE_DIR/deep"
for i in $(seq 1 $DEPTH); do
    current_path="$current_path/d$i"
done

# Create the full path
mkdir -p "$current_path"

# Add source file at deepest level
cat > "$current_path/deep.rs" << 'EOF'
//! File at maximum nesting depth.
//! Used to test depth limit handling.

pub fn at_depth() -> &'static str {
    "reached maximum depth"
}
EOF

# Add file at level 50 (within default limit)
mid_path="$FIXTURE_DIR/deep"
for i in $(seq 1 50); do
    mid_path="$mid_path/d$i"
done
cat > "$mid_path/mid.rs" << 'EOF'
//! File at moderate depth (50 levels).
//! Should be reachable with default depth limit.

pub fn at_mid_depth() -> i32 {
    50
}
EOF

echo "Created bench-deep fixture with $DEPTH levels"
echo "  - File at level 50: $mid_path/mid.rs"
echo "  - File at level $DEPTH: $current_path/deep.rs"
