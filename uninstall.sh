#!/bin/bash
set -e

PLUGIN_DIR="$HOME/.local/share/pop-launcher/plugins/toggl"

echo "Uninstalling Toggl Track pop-launcher plugin..."

if [ -d "$PLUGIN_DIR" ]; then
    rm -rf "$PLUGIN_DIR"
    echo "Removed $PLUGIN_DIR"
else
    echo "Plugin not found at $PLUGIN_DIR"
fi

if pkill pop-launcher 2>/dev/null; then
    echo "Restarted pop-launcher"
fi

echo "Done!"
