#!/bin/bash
set -e

REPO="grunghi/pop-launcher-toggl"
BRANCH="main"
PLUGIN_DIR="$HOME/.local/share/pop-launcher/plugins/toggl"

if [ ! -d "$PLUGIN_DIR" ]; then
    echo "Plugin not installed. Run install.sh or install-remote.sh first."
    exit 1
fi

for tool in git cargo; do
    if ! command -v "$tool" &>/dev/null; then
        echo "Error: '$tool' is required to build the plugin from source." >&2
        if [ "$tool" = "cargo" ]; then
            echo "Install the Rust toolchain from https://rustup.rs and re-run." >&2
        fi
        exit 1
    fi
done

echo "Updating Toggl Track pop-launcher plugin..."

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Cloning $REPO ($BRANCH)..."
git clone --depth 1 --branch "$BRANCH" "https://github.com/$REPO.git" "$TMP_DIR/src"

echo "Building (cargo build --release)..."
( cd "$TMP_DIR/src" && cargo build --release )

# Replace the binary and icons; leave config.toml and plugin.ron untouched.
cp "$TMP_DIR/src/target/release/toggl" "$PLUGIN_DIR/toggl"
chmod +x "$PLUGIN_DIR/toggl"
mkdir -p "$PLUGIN_DIR/icons"
cp "$TMP_DIR/src/plugin/icons/"*.svg "$PLUGIN_DIR/icons/"

# Restart pop-launcher if running
if pkill pop-launcher 2>/dev/null; then
    echo "Restarted pop-launcher"
fi

echo "Done! Plugin updated (config preserved)."
