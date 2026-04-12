#!/bin/bash
# tv-mcp installer
# Usage: curl -sSL https://raw.githubusercontent.com/FrontToBackCulture/tv-mcp/main/install.sh | bash

set -e

REPO="FrontToBackCulture/tv-mcp"
INSTALL_DIR="$HOME/.tv-mcp/bin"
BINARY_NAME="tv-mcp"

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)  PLATFORM="apple-darwin" ;;
  Linux)   PLATFORM="unknown-linux-gnu" ;;
  MINGW*|MSYS*|CYGWIN*) PLATFORM="pc-windows-msvc" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64)  TARGET="${ARCH}-${PLATFORM}" ;;
  arm64|aarch64) TARGET="aarch64-${PLATFORM}" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "tv-mcp installer"
echo "  Platform: ${OS} ${ARCH}"
echo "  Target:   ${TARGET}"
echo ""

# Check for GitHub release first
LATEST_URL="https://api.github.com/repos/${REPO}/releases/latest"
RELEASE_INFO=$(curl -sSf "$LATEST_URL" 2>/dev/null || echo "")

if [ -n "$RELEASE_INFO" ] && echo "$RELEASE_INFO" | grep -q "tag_name"; then
  VERSION=$(echo "$RELEASE_INFO" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
  ASSET_NAME="tv-mcp-${TARGET}"
  [ "$PLATFORM" = "pc-windows-msvc" ] && ASSET_NAME="${ASSET_NAME}.exe"

  DOWNLOAD_URL=$(echo "$RELEASE_INFO" | grep "browser_download_url" | grep "$ASSET_NAME" | head -1 | sed 's/.*"\(https[^"]*\)".*/\1/')

  if [ -n "$DOWNLOAD_URL" ]; then
    echo "Downloading ${VERSION} from release..."
    mkdir -p "$INSTALL_DIR"
    curl -sSfL "$DOWNLOAD_URL" -o "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    echo ""
    echo "Installed: ${INSTALL_DIR}/${BINARY_NAME}"
    "${INSTALL_DIR}/${BINARY_NAME}" --version
    echo ""
    echo "Done! tv-mcp will be auto-registered next time you open TV Client."
    echo "Or register manually: claude mcp add tv-mcp ${INSTALL_DIR}/${BINARY_NAME}"
    exit 0
  fi
fi

# No release available — build from source
echo "No prebuilt binary found. Building from source..."
echo ""

if ! command -v cargo &>/dev/null; then
  echo "Error: Rust is not installed. Install it from https://rustup.rs"
  exit 1
fi

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

echo "Cloning tv-mcp..."
git clone --depth 1 "https://github.com/${REPO}.git" "$TMPDIR/tv-mcp" 2>/dev/null

echo "Building (this takes ~1 minute)..."
cd "$TMPDIR/tv-mcp"
cargo build --release --quiet

mkdir -p "$INSTALL_DIR"
cp "target/release/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

echo ""
echo "Installed: ${INSTALL_DIR}/${BINARY_NAME}"
"${INSTALL_DIR}/${BINARY_NAME}" --version
echo ""
echo "Done! tv-mcp will be auto-registered next time you open TV Client."
echo "Or register manually: claude mcp add tv-mcp ${INSTALL_DIR}/${BINARY_NAME}"
