#!/bin/bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Alfred Jean LLC
# Sets up test fixtures that require generation or special handling

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "Setting up test fixtures..."

# Generate deeply nested fixture
if [ -x "./scripts/generate-bench-deep.sh" ]; then
    echo "  Generating bench-deep fixture..."
    ./scripts/generate-bench-deep.sh
fi

# Create symlinks
if [ -x "./scripts/setup-symlink-fixtures.sh" ]; then
    echo "  Setting up symlink fixtures..."
    ./scripts/setup-symlink-fixtures.sh
fi

echo "Test fixtures ready."
