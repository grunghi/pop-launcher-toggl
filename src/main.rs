//! Pop-launcher plugin for Toggl Track time tracking (Rust port).

mod api;
mod cache;
mod config;
mod dateutil;
mod fuzzy;
mod ipc;
mod plugin;
mod setup;

use std::fs::OpenOptions;
use std::io::{BufRead, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::time::Instant;

use serde_json::Value;

use plugin::Plugin;

/// Rotate the log once it exceeds this size.
const LOG_MAX_BYTES: u64 = 2 * 1024 * 1024;

/// `$XDG_STATE_HOME/pop-launcher-toggl/toggl.log`, creating the directory.
fn log_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    let dir = base.join("pop-launcher-toggl");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("toggl.log"))
}

/// Log a timestamped diagnostic to the log file and to stderr.
///
/// stderr is inherited from the pop-launcher backend; with the `~/.local/bin/pop-launcher`
/// shim installed it lands in `backend.log`, but the file log stands on its own so the
/// plugin stays diagnosable without it.
pub fn log(msg: &str) {
    let line = format!("{} [{}] {msg}", dateutil::log_timestamp(), std::process::id());

    {
        let mut err = std::io::stderr().lock();
        let _ = writeln!(err, "toggl: {line}");
    }

    if let Some(path) = log_path() {
        // Entry descriptions are personal; keep the log owner-only.
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).mode(0o600).open(&path) {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// Rename the log aside once it grows past `LOG_MAX_BYTES`.
fn rotate_log() {
    let Some(path) = log_path() else { return };
    let too_big = std::fs::metadata(&path).map(|m| m.len() > LOG_MAX_BYTES).unwrap_or(false);
    if too_big {
        let _ = std::fs::rename(&path, path.with_extension("log.1"));
    }
}

/// Record panics before the process dies. The release profile sets `panic = "abort"`,
/// so without this a panic leaves no trace at all — the plugin just vanishes.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".to_string());
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string panic payload>".to_string());
        log(&format!("PANIC at {loc}: {msg}"));
    }));
}

/// First `n` characters of `s`, marking truncation.
fn preview(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n).collect::<String>())
    }
}

fn main() {
    // Must precede every thread spawn below, including Plugin::new's cache pre-warm.
    dateutil::init_log_offset();
    rotate_log();
    install_panic_hook();
    log("started");

    let mut plugin = Plugin::new();

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Ok(msg) = serde_json::from_str::<Value>(line) else {
            log(&format!("Bad JSON: {line}"));
            continue;
        };

        // Bracket every request: a `->` with no matching `<-` is a hung handler,
        // which is the signature we're hunting when the launcher wedges.
        let started = Instant::now();
        log(&format!("-> {}", preview(line, 160)));
        let mut exiting = false;

        match msg {
            // Unit-variant requests arrive as bare strings ("Exit", "Interrupt").
            Value::String(s) => {
                if s == "Exit" {
                    exiting = true;
                }
                // "Interrupt" and anything else: nothing async to cancel.
            }
            Value::Object(map) => {
                if let Some(q) = map.get("Search").and_then(Value::as_str) {
                    plugin.handle_search(q);
                } else if let Some(idx) = map.get("Activate").and_then(Value::as_u64) {
                    plugin.handle_activate(idx as usize);
                } else if let Some(idx) = map.get("Context").and_then(Value::as_u64) {
                    plugin.handle_context(idx as usize);
                } else if let Some(ac) = map.get("ActivateContext").and_then(Value::as_object) {
                    if let (Some(id), Some(ctx)) = (
                        ac.get("id").and_then(Value::as_u64),
                        ac.get("context").and_then(Value::as_u64),
                    ) {
                        plugin.handle_activate_context(id as usize, ctx as usize);
                    }
                } else if map.contains_key("Exit") {
                    exiting = true;
                }
                // "Interrupt" / unknown object messages: ignore.
            }
            _ => {}
        }

        log(&format!("<- done in {}ms", started.elapsed().as_millis()));
        if exiting {
            break;
        }
    }

    // Wait for any in-flight start/stop calls before exiting.
    log("stdin closed, joining in-flight actions");
    plugin.join_actions();
    log("exit");
}
