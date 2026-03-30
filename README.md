# Toggl Track for Pop Launcher

Control [Toggl Track](https://toggl.com/track/) timers from your COSMIC / Pop!_OS launcher. See your running timer, stop it, restart recent entries, or start new ones - no browser needed.

## Install

```bash
curl -sSL https://raw.githubusercontent.com/grunghi/pop-launcher-toggl/main/install-remote.sh | bash
```

The installer will prompt you for your API token and workspace ID. Then open the launcher and type `toggl`.

<details>
<summary>Other install methods</summary>

### From source

```bash
git clone https://github.com/grunghi/pop-launcher-toggl.git
cd pop-launcher-toggl
./install.sh
```

### Manual configuration

Edit `~/.local/share/pop-launcher/plugins/toggl/config.toml`:

```toml
api_token = "your_api_token_here"
workspace_id = 1234567
keyword = "toggl"
```

- **API token**: [track.toggl.com/profile](https://track.toggl.com/profile) (scroll to "API Token")
- **Workspace ID**: the number in your Toggl URL at [track.toggl.com/reports](https://track.toggl.com/reports)

</details>

## Usage

Type your keyword (default `toggl`) to see your running timer and recent entries. Click the running timer to stop it, click a recent entry to restart it, or select "New timer" to create one with a description and project.

Right-click any entry for extra options (open Toggl in browser, stop timer).

## Update

```bash
curl -sSL https://raw.githubusercontent.com/grunghi/pop-launcher-toggl/main/update.sh | bash
```

Your configuration (API token, workspace, keyword) is preserved.

## Uninstall

```bash
rm -rf ~/.local/share/pop-launcher/plugins/toggl && pkill pop-launcher
```

## Requirements

- Python 3.6+
- pop-launcher (comes with Pop!_OS / COSMIC)
- zenity (optional, for setup dialogs)

## License

MIT
