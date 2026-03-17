#!/bin/bash
set -e

PLUGIN_DIR="$HOME/.local/share/pop-launcher/plugins/toggl"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Installing Toggl Track pop-launcher plugin..."

# -- Interactive setup via zenity (if available) -----------------------------

KEYWORD="toggl"
API_TOKEN=""
WORKSPACE_ID=""

if command -v zenity &>/dev/null; then
    KEYWORD=$(zenity --entry \
        --title="Toggl Track Setup" \
        --text="Launcher keyword (triggers the plugin):" \
        --entry-text="toggl" \
        --width=400 2>/dev/null) || KEYWORD="toggl"
    [ -z "$KEYWORD" ] && KEYWORD="toggl"

    # Sanitize keyword: only allow alphanumeric, hyphens, underscores
    KEYWORD=$(echo "$KEYWORD" | tr -cd 'a-zA-Z0-9_-')
    [ -z "$KEYWORD" ] && KEYWORD="toggl"

    API_TOKEN=$(zenity --entry \
        --title="Toggl Track Setup" \
        --text="Enter your Toggl API token\n(find it at track.toggl.com/profile):" \
        --width=400 2>/dev/null) || API_TOKEN=""

    # Sanitize API token: only allow alphanumeric (Toggl tokens are hex strings)
    API_TOKEN=$(echo "$API_TOKEN" | tr -cd 'a-zA-Z0-9')

    if [ -n "$API_TOKEN" ]; then
        WORKSPACE_ID=$(zenity --entry \
            --title="Toggl Track Setup" \
            --text="Enter your Workspace ID\n(visible in the URL at track.toggl.com/reports):" \
            --width=400 2>/dev/null) || WORKSPACE_ID=""

        # Sanitize workspace ID: only allow digits
        WORKSPACE_ID=$(echo "$WORKSPACE_ID" | tr -cd '0-9')
    fi
else
    echo "zenity not found — using defaults. Edit config.toml manually after install."
fi

# -- Install files -----------------------------------------------------------

mkdir -p "$PLUGIN_DIR/icons"
cp "$SCRIPT_DIR/plugin/toggl" "$PLUGIN_DIR/toggl"
chmod +x "$PLUGIN_DIR/toggl"
cp "$SCRIPT_DIR/plugin/icons/"*.svg "$PLUGIN_DIR/icons/"

# Generate plugin.ron with the chosen keyword
cat > "$PLUGIN_DIR/plugin.ron" <<EOF
(
    name: "Toggl Track",
    description: "Control Toggl Track timers",
    bin: (
        path: "toggl",
    ),
    icon: Name("${PLUGIN_DIR}/icons/timer.svg"),
    query: (
        isolate: true,
        regex: "^${KEYWORD}.*",
        help: "${KEYWORD} ",
        no_sort: true,
        priority: High,
    ),
)
EOF

# Write config: preserve existing credentials if user didn't provide new ones
if [ ! -f "$PLUGIN_DIR/config.toml" ] || [ -n "$API_TOKEN" ]; then
    cat > "$PLUGIN_DIR/config.toml" <<EOF
# Toggl Track API configuration
# Get your API token from: https://track.toggl.com/profile
# Find your workspace ID in the URL at: https://track.toggl.com/reports

api_token = "${API_TOKEN}"
workspace_id = ${WORKSPACE_ID:-0}
keyword = "${KEYWORD}"
EOF
    chmod 600 "$PLUGIN_DIR/config.toml"
elif [ ! -s "$PLUGIN_DIR/config.toml" ]; then
    cat > "$PLUGIN_DIR/config.toml" <<EOF
# Toggl Track API configuration
# Get your API token from: https://track.toggl.com/profile
# Find your workspace ID in the URL at: https://track.toggl.com/reports

api_token = ""
workspace_id = 0
keyword = "${KEYWORD}"
EOF
    chmod 600 "$PLUGIN_DIR/config.toml"
fi

echo "Installed to $PLUGIN_DIR"
echo "Keyword: ${KEYWORD}"

# Restart pop-launcher if running
if pkill pop-launcher 2>/dev/null; then
    echo "Restarted pop-launcher"
fi

echo ""
echo "Done! Open the launcher and type '${KEYWORD}' to get started."
[ -z "$API_TOKEN" ] && echo "Click 'Setup required' to enter your API token."
