#!/usr/bin/env sh
set -e

REPO="adamschlesinger/uepm"
INSTALL_DIR="$HOME/.uepm/bin"

OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64) ARTIFACT="uepm-linux-x86_64" ;;
      *) echo "Unsupported architecture: $ARCH" && exit 1 ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      arm64)  ARTIFACT="uepm-macos-arm64" ;;
      x86_64) ARTIFACT="uepm-macos-x86_64" ;;
      *) echo "Unsupported architecture: $ARCH" && exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS"
    exit 1
    ;;
esac

if [ -n "${UEPM_VERSION:-}" ]; then
  LATEST="$UEPM_VERSION"
else
  LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
fi

echo "Installing uepm $LATEST..."

URL="https://github.com/$REPO/releases/download/$LATEST/$ARTIFACT"
mkdir -p "$INSTALL_DIR"
curl -fsSL "$URL" -o "$INSTALL_DIR/uepm"
chmod +x "$INSTALL_DIR/uepm"

add_to_path() {
  PROFILE="$1"
  if [ -f "$PROFILE" ] && ! grep -q 'uepm/bin' "$PROFILE"; then
    echo '' >> "$PROFILE"
    echo '# UEPM' >> "$PROFILE"
    echo 'export PATH="$HOME/.uepm/bin:$PATH"' >> "$PROFILE"
    echo "Added ~/.uepm/bin to PATH in $PROFILE"
  fi
}

add_to_path "$HOME/.bashrc"
add_to_path "$HOME/.zshrc"

echo ""
echo "✓ uepm installed to $INSTALL_DIR/uepm"
echo "  Restart your shell or run: export PATH=\"\$HOME/.uepm/bin:\$PATH\""
echo ""
echo "  Get started: uepm init"
