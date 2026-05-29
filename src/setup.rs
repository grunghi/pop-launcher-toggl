//! Interactive first-run setup via zenity dialogs.

use std::process::Command;

use crate::api;
use crate::config;
use crate::log;

/// Run a zenity command and return its trimmed stdout, or `None` if the user
/// cancelled (non-zero exit) or zenity is unavailable.
fn zenity(args: &[&str]) -> Option<String> {
    let output = Command::new("zenity").args(args).output();
    match output {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        Ok(_) => None,
        Err(e) => {
            log(&format!("zenity not available: {e}"));
            None
        }
    }
}

/// Launch the setup dialogs. Returns `(token, workspace_id)` on success.
pub fn run(keyword: &str) -> Option<(String, i64)> {
    let token = zenity(&[
        "--entry",
        "--title=Toggl Track Setup",
        "--text=Enter your Toggl API token\n(find it at track.toggl.com/profile):",
        "--width=400",
    ])?;

    // Try to fetch workspaces automatically.
    let workspaces = api::fetch_workspaces(&token);

    let wid = match workspaces.as_deref() {
        Some([(id, _)]) => *id, // exactly one — use it
        Some(list) if list.len() > 1 => {
            // Multiple — let the user pick.
            let mut args: Vec<String> = vec![
                "--list".into(),
                "--title=Select Workspace".into(),
                "--text=Choose your workspace:".into(),
                "--column=ID".into(),
                "--column=Workspace".into(),
                "--hide-column=1".into(),
                "--width=400".into(),
                "--height=300".into(),
            ];
            for (id, name) in list {
                args.push(id.to_string());
                args.push(name.clone());
            }
            let refs: Vec<&str> = args.iter().map(String::as_str).collect();
            zenity(&refs)?.parse::<i64>().ok()?
        }
        _ => {
            // API call failed — fall back to manual entry.
            let input = zenity(&[
                "--entry",
                "--title=Toggl Track Setup",
                "--text=Could not fetch workspaces automatically.\nEnter your Workspace ID manually\n(go to track.toggl.com, look for ?wid= or &wid= in the URL):",
                "--width=400",
            ])?;
            match input.parse::<i64>() {
                Ok(n) => n,
                Err(_) => {
                    let _ = zenity(&[
                        "--error",
                        "--text=Workspace ID must be a number.",
                        "--width=300",
                    ]);
                    return None;
                }
            }
        }
    };

    config::save(&token, wid, keyword);
    Some((token, wid))
}
