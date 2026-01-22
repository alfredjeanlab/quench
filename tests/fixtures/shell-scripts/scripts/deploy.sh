#!/bin/bash
# Deploy script for the project

set -euo pipefail

TARGET="${1:-production}"
echo "Deploying to $TARGET..."
echo "Deploy complete"
