#!/bin/bash
set -e

PLUGIN_DIR="$HOME/.local/share/pop-launcher/plugins/toggl"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Installing Toggl Track pop-launcher plugin..."

# -- Require the Rust toolchain ----------------------------------------------

if ! command -v cargo &>/dev/null; then
    echo "Error: 'cargo' not found. The plugin is built from Rust source." >&2
    echo "Install the Rust toolchain from https://rustup.rs and re-run." >&2
    exit 1
fi

# -- Build the binary --------------------------------------------------------

echo "Building (cargo build --release)..."
( cd "$SCRIPT_DIR" && cargo build --release )
BINARY="$SCRIPT_DIR/target/release/toggl"

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
        # Try to fetch workspaces from the API (parsed without external deps)
        WS_JSON=$(curl -sSf -u "${API_TOKEN}:api_token" \
            -H "Content-Type: application/json" \
            "https://api.track.toggl.com/api/v9/workspaces" 2>/dev/null) || WS_JSON=""

        # Count workspaces by the number of top-level "id": fields
        WS_IDS=$(echo "$WS_JSON" | grep -oE '"id":[[:space:]]*[0-9]+' | grep -oE '[0-9]+')
        WS_COUNT=$(echo "$WS_IDS" | grep -c '[0-9]' || true)

        if [ "$WS_COUNT" -eq 1 ]; then
            # Single workspace — use it directly
            WORKSPACE_ID=$(echo "$WS_IDS" | head -1)
            echo "Using workspace $WORKSPACE_ID"
        else
            # Zero, many, or unparseable — ask for the ID directly
            WORKSPACE_ID=$(zenity --entry \
                --title="Toggl Track Setup" \
                --text="Enter your Workspace ID\n(go to track.toggl.com, look for ?wid= or &wid= in the URL):" \
                --width=400 2>/dev/null) || WORKSPACE_ID=""
            WORKSPACE_ID=$(echo "$WORKSPACE_ID" | tr -cd '0-9')
        fi
    fi
else
    echo "zenity not found — using defaults. Edit config.toml manually after install."
fi

# -- Install files -----------------------------------------------------------

mkdir -p "$PLUGIN_DIR/icons"
cp "$BINARY" "$PLUGIN_DIR/toggl"
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
# Find your workspace ID: go to track.toggl.com, look for ?wid= or &wid= in the URL

api_token = "${API_TOKEN}"
workspace_id = ${WORKSPACE_ID:-0}
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
