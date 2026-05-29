#!/bin/bash
set -e

REPO="grunghi/pop-launcher-toggl"
BRANCH="main"

echo "Installing Toggl Track pop-launcher plugin..."

for tool in git cargo; do
    if ! command -v "$tool" &>/dev/null; then
        echo "Error: '$tool' is required to build the plugin from source." >&2
        if [ "$tool" = "cargo" ]; then
            echo "Install the Rust toolchain from https://rustup.rs and re-run." >&2
        fi
        exit 1
    fi
done

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Cloning $REPO ($BRANCH)..."
git clone --depth 1 --branch "$BRANCH" "https://github.com/$REPO.git" "$TMP_DIR/src"

# Delegate to the from-source installer (builds + interactive setup).
bash "$TMP_DIR/src/install.sh"
