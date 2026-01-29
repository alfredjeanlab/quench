#!/bin/bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Alfred Jean LLC
# Installs npm modules for JavaScript test fixtures

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/../../tests/fixtures"

cd "$FIXTURES_DIR"

for fixture in js-simple js-monorepo; do
  if [ -f "$fixture/package.json" ]; then
    if [ -f "$fixture/package-lock.json" ]; then
      echo "  Installing npm modules for $fixture (using npm ci)..."
      (cd "$fixture" && npm ci --silent)
    else
      echo "  Installing npm modules for $fixture (using npm install)..."
      (cd "$fixture" && npm install --silent)
    fi
  fi
done
