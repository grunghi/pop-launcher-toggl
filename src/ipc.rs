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

/// A right-click context menu. pop-launcher's `ContextOption` is `{ id, name }`;
/// the option's `id` is what comes back in the `ActivateContext.context` field,
/// so we assign ids by position.
pub fn context(id: usize, options: Vec<String>) {
    let options: Vec<Value> = options
        .into_iter()
        .enumerate()
        .map(|(i, name)| json!({ "id": i, "name": name }))
        .collect();
    send(json!({ "Context": { "id": id, "options": options } }));
}
