#!/bin/bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Alfred Jean LLC
# install.sh - Install quench from GitHub Releases
#
# Usage:
#   curl -fsSL https://github.com/alfredjeanlab/quench/releases/latest/download/install.sh | bash
#
# Environment variables:
#   QUENCH_VERSION - Version to install (default: latest)
#   QUENCH_INSTALL - Installation directory (default: ~/.local/bin)

set -e

QUENCH_VERSION="${QUENCH_VERSION:-latest}"
QUENCH_INSTALL="${QUENCH_INSTALL:-$HOME/.local/bin}"
QUENCH_REPO="alfredjeanlab/quench"
GITHUB_API="https://api.github.com"
GITHUB_RELEASES="https://github.com/${QUENCH_REPO}/releases"

# Colors (disabled if not a terminal)
if [ -t 1 ]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[0;33m'
  NC='\033[0m' # No Color
else
  RED=''
  GREEN=''
  YELLOW=''
  NC=''
fi

info() { echo -e "${GREEN}==>${NC} $1"; }
warn() { echo -e "${YELLOW}Warning:${NC} $1"; }
error() { echo -e "${RED}Error:${NC} $1" >&2; exit 1; }

# Check for required commands
for cmd in curl tar; do
  if ! command -v "$cmd" &> /dev/null; then
    error "$cmd is required but not installed"
  fi
done

# Detect platform
detect_platform() {
  local os arch
  os=$(uname -s)
  arch=$(uname -m)

  case "$os" in
    Linux)  os="linux" ;;
    Darwin) os="macos" ;;
    *)      error "Unsupported OS: $os" ;;
  esac

  case "$arch" in
    x86_64)  arch="x86_64" ;;
    aarch64) arch="aarch64" ;;
    arm64)   arch="aarch64" ;;
    *)       error "Unsupported architecture: $arch" ;;
  esac

  echo "quench-${os}-${arch}"
}

PLATFORM=$(detect_platform)
info "Detected platform: $PLATFORM"

# Resolve "latest" to actual version
if [ "$QUENCH_VERSION" = "latest" ]; then
  info "Fetching latest version..."
  QUENCH_VERSION=$(curl -fsSL "${GITHUB_API}/repos/${QUENCH_REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/')
  if [ -z "$QUENCH_VERSION" ]; then
    error "Could not determine latest version. Check your internet connection."
  fi
fi

info "Installing quench v${QUENCH_VERSION}..."

# Create temp directory
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

# Download tarball and checksum
TARBALL="${PLATFORM}.tar.gz"
CHECKSUM="${TARBALL}.sha256"
DOWNLOAD_URL="${GITHUB_RELEASES}/download/v${QUENCH_VERSION}"

info "Downloading ${TARBALL}..."
if ! curl -fsSL "${DOWNLOAD_URL}/${TARBALL}" -o "${TMPDIR}/${TARBALL}"; then
  error "Failed to download ${TARBALL}. Version v${QUENCH_VERSION} may not exist."
fi

info "Downloading checksum..."
if ! curl -fsSL "${DOWNLOAD_URL}/${CHECKSUM}" -o "${TMPDIR}/${CHECKSUM}"; then
  error "Failed to download checksum file"
fi

# Verify checksum
info "Verifying checksum..."
cd "$TMPDIR"
if command -v sha256sum &> /dev/null; then
  sha256sum -c "${CHECKSUM}" --quiet || error "Checksum verification failed!"
elif command -v shasum &> /dev/null; then
  shasum -a 256 -c "${CHECKSUM}" --quiet || error "Checksum verification failed!"
else
  warn "No sha256sum or shasum available, skipping checksum verification"
fi

# Extract tarball
info "Extracting..."
tar -xzf "${TARBALL}"

# Install binary
mkdir -p "$QUENCH_INSTALL"
info "Installing to ${QUENCH_INSTALL}..."
cp quench "$QUENCH_INSTALL/quench"
chmod +x "$QUENCH_INSTALL/quench"

echo ""
info "quench v${QUENCH_VERSION} installed successfully!"

# Check if install dir is in PATH
if [[ ":$PATH:" != *":$QUENCH_INSTALL:"* ]]; then
  echo ""
  warn "$QUENCH_INSTALL is not in your PATH"
  echo "Add this to your shell profile:"
  echo ""
  echo "  export PATH=\"$QUENCH_INSTALL:\$PATH\""
fi

# Install shell completions (idempotent)
install_completions() {
    local quench="${QUENCH_INSTALL}/quench"
    local marker="# quench-shell-completion"
    local data_dir="${XDG_DATA_HOME:-$HOME/.local/share}/quench/completions"

    # Bash
    if command -v bash &> /dev/null; then
        local rc=""
        if [ -f "$HOME/.bashrc" ]; then
            rc="$HOME/.bashrc"
        elif [ -f "$HOME/.bash_profile" ]; then
            rc="$HOME/.bash_profile"
        fi
        if [ -n "$rc" ] && ! grep -q "$marker" "$rc"; then
            mkdir -p "$data_dir"
            "$quench" completions bash > "$data_dir/quench.bash"
            printf '\n%s\n[ -f "%s" ] && source "%s"\n' \
                "$marker" "$data_dir/quench.bash" "$data_dir/quench.bash" >> "$rc"
            info "Installed bash completions (source: $rc)"
        fi
    fi

    # Zsh
    if command -v zsh &> /dev/null; then
        if [ -f "$HOME/.zshrc" ] && ! grep -q "$marker" "$HOME/.zshrc"; then
            mkdir -p "$data_dir"
            "$quench" completions zsh > "$data_dir/_quench"
            printf '\n%s\n[ -f "%s" ] && source "%s"\n' \
                "$marker" "$data_dir/_quench" "$data_dir/_quench" >> "$HOME/.zshrc"
            info "Installed zsh completions (source: ~/.zshrc)"
        fi
    fi

    # Fish
    if command -v fish &> /dev/null; then
        local fish_dir="${XDG_CONFIG_HOME:-$HOME/.config}/fish/completions"
        if [ ! -f "$fish_dir/quench.fish" ]; then
            mkdir -p "$fish_dir"
            "$quench" completions fish > "$fish_dir/quench.fish"
            info "Installed fish completions"
        fi
    fi
}

install_completions

echo ""
echo "To get started in a project:"
echo "  cd /path/to/your/project"
echo "  quench check"
