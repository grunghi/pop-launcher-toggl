//! pop-launcher IPC: JSON messages over stdout, mirroring the wire format the
//! Python plugin used (kept byte-compatible with the installed launcher).

use std::io::Write;

use serde_json::{json, Value};

/// Serialize a response and write it as a single line. Exit quietly if the
/// launcher has closed our stdout.
fn send(value: Value) {
    let mut out = std::io::stdout().lock();
    let line = value.to_string();
    if out
        .write_all(line.as_bytes())
        .and_then(|_| out.write_all(b"\n"))
        .and_then(|_| out.flush())
        .is_err()
    {
        std::process::exit(0);
    }
}

pub fn append(id: usize, name: &str, description: &str, icon: &str) {
    send(json!({
        "Append": {
            "id": id,
            "name": name,
            "description": description,
            "keywords": Value::Null,
            "icon": { "Name": icon },
            "exec": Value::Null,
            "window": Value::Null,
        }
    }));
}

pub fn clear() {
    send(json!({ "Clear": Value::Null }));
}

pub fn finished() {
    send(json!({ "Finished": Value::Null }));
}

pub fn fill(text: &str) {
    send(json!({ "Fill": text }));
}

pub fn close() {
    send(json!({ "Close": Value::Null }));
}

/// A right-click context menu option (name + description, as the Python sent).
pub fn context(id: usize, options: Vec<(&str, String)>) {
    let options: Vec<Value> = options
        .into_iter()
        .map(|(name, description)| json!({ "name": name, "description": description }))
        .collect();
    send(json!({ "Context": { "id": id, "options": options } }));
}
