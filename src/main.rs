//! Pop-launcher plugin for Toggl Track time tracking (Rust port).

mod api;
mod cache;
mod config;
mod dateutil;
mod fuzzy;
mod ipc;
mod plugin;
mod setup;

use std::io::{BufRead, Write};

use serde_json::Value;

use plugin::Plugin;

/// Log a diagnostic message to stderr (ignoring a closed pipe).
pub fn log(msg: &str) {
    let mut err = std::io::stderr().lock();
    let _ = writeln!(err, "{msg}");
}

fn main() {
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

        match msg {
            // Unit-variant requests arrive as bare strings ("Exit", "Interrupt").
            Value::String(s) => {
                if s == "Exit" {
                    break;
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
                    break;
                }
                // "Interrupt" / unknown object messages: ignore.
            }
            _ => {}
        }
    }

    // Wait for any in-flight start/stop calls before exiting.
    plugin.join_actions();
}
