//! Plugin configuration and filesystem paths.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const DEFAULT_KEYWORD: &str = "toggl";

/// Directory the binary lives in — also holds `config.toml` and `icons/`.
fn plugin_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."))
    })
}

pub fn config_path() -> PathBuf {
    plugin_dir().join("config.toml")
}

/// Absolute path to a bundled SVG icon (without the `.svg` it's referenced by name).
pub fn icon(name: &str) -> String {
    plugin_dir()
        .join("icons")
        .join(format!("{name}.svg"))
        .to_string_lossy()
        .into_owned()
}

#[derive(Clone, Debug)]
pub struct Config {
    pub token: Option<String>,
    pub wid: Option<i64>,
    pub keyword: String,
}

/// Parse `config.toml` manually (matches the Python plugin's lenient parsing).
pub fn load() -> Config {
    let mut cfg = Config {
        token: None,
        wid: None,
        keyword: DEFAULT_KEYWORD.to_string(),
    };

    let Ok(text) = std::fs::read_to_string(config_path()) else {
        return cfg;
    };

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        // Strip inline comments, then surrounding whitespace and quotes.
        let val = val
            .split('#')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches(|c| c == '"' || c == '\'');
        match key {
            "api_token" => {
                if !val.is_empty() {
                    cfg.token = Some(val.to_string());
                }
            }
            "workspace_id" => {
                cfg.wid = val.parse::<i64>().ok().filter(|&n| n != 0);
            }
            "keyword" => {
                if !val.is_empty() {
                    cfg.keyword = val.to_string();
                }
            }
            _ => {}
        }
    }
    cfg
}

/// Write `config.toml` with restricted (0600) permissions.
pub fn save(token: &str, wid: i64, keyword: &str) {
    let contents = format!(
        "# Toggl Track API configuration\n\
         # Get your API token from: https://track.toggl.com/profile\n\n\
         api_token = \"{token}\"\n\
         workspace_id = {wid}\n\
         keyword = \"{keyword}\"\n"
    );
    let path = config_path();
    if std::fs::write(&path, contents).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
    }
}
