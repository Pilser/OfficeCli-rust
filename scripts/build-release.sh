#!/bin/env bash
# Build OfficeCLI release binaries for distribution
set -euo pipefail

echo "Building OfficeCLI release binaries..."

# Build for current platform
cargo build --release

BINARY="target/release/officecli"
if [ -f "${BINARY}.exe" ]; then
    BINARY="${BINARY}.exe"
fi

# Copy to distribution directory
DIST_DIR="dist"
mkdir -p "$DIST_DIR"

# Determine platform name
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
    darwin)
        case "$ARCH" in
            arm64) NAME="officecli-mac-arm64" ;;
            x86_64) NAME="officecli-mac-x64" ;;
            *) NAME="officecli-mac-$ARCH" ;;
        esac
        # macOS codesign
        codesign -s - -f "$BINARY" 2>/dev/null || true
        ;;
    linux)
        NAME="officecli-linux-$ARCH"
        ;;
    *) NAME="officecli-$OS-$ARCH" ;;
esac

cp "$BINARY" "$DIST_DIR/$NAME"
chmod +x "$DIST_DIR/$NAME"

# Generate SHA256 checksums
cd "$DIST_DIR"
sha256sum "$NAME" > SHA256SUMS || shasum -a 256 "$NAME" > SHA256SUMS

echo ""
echo "Build complete!"
echo "  Binary: $DIST_DIR/$NAME"
echo "  SHA256:"
cat SHA256SUMS

# Smoke test
echo ""
echo "Smoke test..."
"$DIST_DIR/$NAME" --version
"$DIST_DIR/$NAME" info
"$DIST_DIR/$NAME" create /tmp/smoke_test.docx
"$DIST_DIR/$NAME" view /tmp/smoke_test.docx --mode stats
rm -f /tmp/smoke_test.docx

echo ""
echo "All checks passed!"