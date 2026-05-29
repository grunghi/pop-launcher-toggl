//! Toggl Track API v9 client.

use std::collections::HashMap;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::dateutil;
use crate::log;

pub const TOGGL_URL: &str = "https://track.toggl.com";
const API_BASE: &str = "https://api.track.toggl.com/api/v9";
const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, Deserialize)]
pub struct TimeEntry {
    pub id: i64,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub project_id: Option<i64>,
    #[serde(default)]
    pub workspace_id: Option<i64>,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub billable: bool,
}

impl TimeEntry {
    pub fn description_or_default(&self) -> &str {
        match &self.description {
            Some(d) if !d.is_empty() => d,
            _ => "(no description)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiError {
    RateLimit,
    Other,
}

fn auth_header(token: &str) -> String {
    let cred = STANDARD.encode(format!("{token}:api_token"));
    format!("Basic {cred}")
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(TIMEOUT)
        .timeout_read(TIMEOUT)
        .timeout_write(TIMEOUT)
        .build()
}

/// Perform an API request. Returns the parsed JSON body (or `None` if empty).
fn request(method: &str, path: &str, token: &str, body: Option<Value>) -> Result<Option<Value>, ApiError> {
    let url = format!("{API_BASE}{path}");
    let req = agent()
        .request(method, &url)
        .set("Authorization", &auth_header(token))
        .set("Content-Type", "application/json");

    let result = match body {
        Some(b) => req.send_json(b),
        None => req.call(),
    };

    match result {
        Ok(resp) => {
            let text = resp.into_string().unwrap_or_default();
            if text.is_empty() {
                Ok(None)
            } else {
                Ok(serde_json::from_str(&text).ok())
            }
        }
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            log(&format!("API {method} {path} -> {code}: {body}"));
            if code == 402 || code == 429 {
                Err(ApiError::RateLimit)
            } else {
                Err(ApiError::Other)
            }
        }
        Err(e) => {
            log(&format!("API error: {e}"));
            Err(ApiError::Other)
        }
    }
}

pub fn get_current(token: &str) -> Result<Option<TimeEntry>, ApiError> {
    match request("GET", "/me/time_entries/current", token, None) {
        Ok(Some(Value::Null)) | Ok(None) => Ok(None),
        Ok(Some(v)) => Ok(serde_json::from_value(v).ok()),
        Err(ApiError::RateLimit) => Err(ApiError::RateLimit),
        Err(ApiError::Other) => Ok(None),
    }
}

pub fn get_recent(token: &str) -> Vec<TimeEntry> {
    match request("GET", "/me/time_entries", token, None) {
        Ok(Some(Value::Array(items))) => items
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect(),
        _ => Vec::new(),
    }
}

pub fn get_projects(token: &str, wid: i64) -> Result<HashMap<i64, String>, ApiError> {
    match request("GET", "/me/projects", token, None) {
        Ok(Some(Value::Array(items))) => {
            let mut map = HashMap::new();
            for item in items {
                let pid = item.get("id").and_then(Value::as_i64);
                let pwid = item.get("workspace_id").and_then(Value::as_i64);
                if let (Some(pid), Some(pwid)) = (pid, pwid) {
                    if pwid == wid {
                        let name = item
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        map.insert(pid, name);
                    }
                }
            }
            Ok(map)
        }
        Ok(_) => Ok(HashMap::new()),
        Err(ApiError::RateLimit) => Err(ApiError::RateLimit),
        Err(ApiError::Other) => Ok(HashMap::new()),
    }
}

pub fn start_timer(
    token: &str,
    wid: i64,
    description: &str,
    project_id: Option<i64>,
    tags: Option<&Vec<String>>,
    billable: bool,
) {
    let mut body = json!({
        "description": description,
        "start": dateutil::now_rfc3339(),
        "duration": -1,
        "workspace_id": wid,
        "billable": billable,
        "created_with": "pop-launcher-toggl",
    });
    if let Some(pid) = project_id {
        body["project_id"] = json!(pid);
    }
    if let Some(tags) = tags {
        body["tags"] = json!(tags);
    }
    let _ = request("POST", &format!("/workspaces/{wid}/time_entries"), token, Some(body));
}

pub fn stop_timer(token: &str, wid: i64, entry_id: i64) {
    let _ = request(
        "PATCH",
        &format!("/workspaces/{wid}/time_entries/{entry_id}/stop"),
        token,
        None,
    );
}

/// Fetch the user's workspaces as `(id, name)` pairs (used during setup).
pub fn fetch_workspaces(token: &str) -> Option<Vec<(i64, String)>> {
    match request("GET", "/workspaces", token, None) {
        Ok(Some(Value::Array(items))) => Some(
            items
                .into_iter()
                .filter_map(|w| {
                    let id = w.get("id").and_then(Value::as_i64)?;
                    let name = w
                        .get("name")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("Workspace {id}"));
                    Some((id, name))
                })
                .collect(),
        ),
        _ => None,
    }
}
