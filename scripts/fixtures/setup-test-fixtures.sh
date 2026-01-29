#!/bin/bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Alfred Jean LLC
# Sets up test fixtures that require generation or special handling

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

cd "$PROJECT_ROOT"

echo "Setting up test fixtures..."

# Generate deeply nested fixture
if [ -x "./scripts/fixtures/generate-bench-deep.sh" ]; then
    echo "  Generating bench-deep fixture..."
    ./scripts/fixtures/generate-bench-deep.sh
fi

# Create symlinks
if [ -x "./scripts/fixtures/setup-symlink-fixtures.sh" ]; then
    echo "  Setting up symlink fixtures..."
    ./scripts/fixtures/setup-symlink-fixtures.sh
fi

# Install JS fixture dependencies
if [ -x "./scripts/fixtures/setup-js-fixtures.sh" ]; then
    echo "  Setting up JS fixtures..."
    ./scripts/fixtures/setup-js-fixtures.sh
fi

echo "Test fixtures ready."
