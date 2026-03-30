#!/bin/bash
set -e

REPO="grunghi/pop-launcher-toggl"
BRANCH="main"
BASE_URL="https://raw.githubusercontent.com/$REPO/$BRANCH/plugin"
PLUGIN_DIR="$HOME/.local/share/pop-launcher/plugins/toggl"

if [ ! -d "$PLUGIN_DIR" ]; then
    echo "Plugin not installed. Run install.sh or install-remote.sh first."
    exit 1
fi

echo "Updating Toggl Track pop-launcher plugin..."

echo "Downloading plugin script..."
curl -sSfL "$BASE_URL/toggl" -o "$PLUGIN_DIR/toggl"
chmod +x "$PLUGIN_DIR/toggl"

echo "Downloading icons..."
mkdir -p "$PLUGIN_DIR/icons"
for icon in timer play stop plus warning settings; do
    curl -sSfL "$BASE_URL/icons/${icon}.svg" -o "$PLUGIN_DIR/icons/${icon}.svg"
done

# Restart pop-launcher if running
if pkill pop-launcher 2>/dev/null; then
    echo "Restarted pop-launcher"
fi

echo "Done! Plugin updated (config preserved)."
