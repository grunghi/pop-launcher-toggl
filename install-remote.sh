#!/bin/bash
set -e

REPO="grunghi/pop-launcher-toggl"
BRANCH="main"
BASE_URL="https://raw.githubusercontent.com/$REPO/$BRANCH/plugin"
PLUGIN_DIR="$HOME/.local/share/pop-launcher/plugins/toggl"

echo "Installing Toggl Track pop-launcher plugin..."

# -- Download plugin files ---------------------------------------------------

mkdir -p "$PLUGIN_DIR/icons"

echo "Downloading plugin files..."
curl -sSfL "$BASE_URL/toggl" -o "$PLUGIN_DIR/toggl"
chmod +x "$PLUGIN_DIR/toggl"

echo "Downloading icons..."
for icon in timer play stop plus warning settings; do
    curl -sSfL "$BASE_URL/icons/${icon}.svg" -o "$PLUGIN_DIR/icons/${icon}.svg"
done

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
        # Try to fetch workspaces from API
        WS_JSON=$(curl -sSf -u "${API_TOKEN}:api_token" \
            -H "Content-Type: application/json" \
            "https://api.track.toggl.com/api/v9/workspaces" 2>/dev/null) || WS_JSON=""

        WS_COUNT=0
        if [ -n "$WS_JSON" ] && command -v python3 &>/dev/null; then
            WS_COUNT=$(python3 -c "import json,sys; data=json.loads(sys.stdin.read()); print(len(data))" <<< "$WS_JSON" 2>/dev/null) || WS_COUNT=0
        fi

        if [ "$WS_COUNT" -eq 1 ] 2>/dev/null; then
            # Single workspace — use it directly
            WORKSPACE_ID=$(python3 -c "import json,sys; data=json.loads(sys.stdin.read()); print(data[0]['id'])" <<< "$WS_JSON")
            WS_NAME=$(python3 -c "import json,sys; data=json.loads(sys.stdin.read()); print(data[0].get('name',''))" <<< "$WS_JSON")
            echo "Using workspace: $WS_NAME ($WORKSPACE_ID)"
        elif [ "$WS_COUNT" -gt 1 ] 2>/dev/null; then
            # Multiple workspaces — let user pick
            ZENITY_ITEMS=$(python3 -c "
import json, sys
data = json.loads(sys.stdin.read())
for w in data:
    print(w['id'])
    print(w.get('name', 'Workspace ' + str(w['id'])))
" <<< "$WS_JSON")
            WORKSPACE_ID=$(zenity --list \
                --title="Select Workspace" \
                --text="Choose your workspace:" \
                --column=ID --column=Workspace \
                --hide-column=1 \
                --width=400 --height=300 \
                $ZENITY_ITEMS 2>/dev/null) || WORKSPACE_ID=""
            WORKSPACE_ID=$(echo "$WORKSPACE_ID" | tr -cd '0-9')
        else
            # API call failed — fall back to manual entry
            WORKSPACE_ID=$(zenity --entry \
                --title="Toggl Track Setup" \
                --text="Could not fetch workspaces automatically.\nEnter your Workspace ID manually\n(go to track.toggl.com, look for ?wid= or &wid= in the URL):" \
                --width=400 2>/dev/null) || WORKSPACE_ID=""
            WORKSPACE_ID=$(echo "$WORKSPACE_ID" | tr -cd '0-9')
        fi
    fi
else
    echo "zenity not found — using defaults."
    echo "Edit $PLUGIN_DIR/config.toml manually after install."
fi

# -- Write config and plugin.ron ---------------------------------------------

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

# -- Restart pop-launcher ----------------------------------------------------

if pkill pop-launcher 2>/dev/null; then
    echo "Restarted pop-launcher"
fi

echo ""
echo "Done! Open the launcher and type '${KEYWORD}' to get started."
[ -z "$API_TOKEN" ] && echo "Type '${KEYWORD}' and click 'Setup required' to enter your API token."
