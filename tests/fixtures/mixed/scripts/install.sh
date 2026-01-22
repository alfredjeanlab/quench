#!/bin/bash
# Install the mixed CLI tool

set -euo pipefail

cargo build --release
cp target/release/mixed /usr/local/bin/
echo "Installed mixed to /usr/local/bin/"
