#!/bin/bash
# tv-mcp Mac release
# Usage: ./scripts/release-mac.sh v0.10.31 "Release notes here"
#
# Creates the GitHub release with the Mac binary attached.
# Run release-win.ps1 inside Parallels afterwards to add the Windows binary.

set -e

VERSION="${1:?Usage: $0 vX.Y.Z \"release notes\"}"
NOTES="${2:-}"

cd "$(dirname "$0")/.."

# Verify version matches Cargo.toml
CARGO_VERSION="v$(grep '^version = ' Cargo.toml | head -1 | cut -d'"' -f2)"
if [ "$VERSION" != "$CARGO_VERSION" ]; then
  echo "ERROR: Arg version $VERSION doesn't match Cargo.toml version $CARGO_VERSION"
  echo "Bump Cargo.toml first, commit, push, then rerun."
  exit 1
fi

# Ensure clean working tree + pushed
if [ -n "$(git status --porcelain)" ]; then
  echo "ERROR: Working tree dirty. Commit or stash first."
  exit 1
fi

# Tag if not already tagged
if ! git rev-parse "$VERSION" >/dev/null 2>&1; then
  echo "Tagging $VERSION..."
  git tag "$VERSION"
  git push origin "$VERSION"
fi

# Build Mac binary
echo "Building Mac (aarch64) binary..."
cargo build --release

ARTIFACT="tv-mcp-aarch64-apple-darwin"
cp "target/release/tv-mcp" "$ARTIFACT"

# Create release with Mac binary
echo "Creating GitHub release $VERSION..."
gh release create "$VERSION" "$ARTIFACT" \
  --title "tv-mcp ${VERSION#v}" \
  --notes "$NOTES"

rm "$ARTIFACT"

echo ""
echo "✅ Mac release done."
echo "Next: in Parallels VM, cd into tv-mcp, git pull, then:"
echo "    .\\scripts\\release-win.ps1 $VERSION"
