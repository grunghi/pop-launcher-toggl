//! Plugin state and request handlers.

use std::collections::HashMap;
use std::process::Command;
use std::thread::{self, JoinHandle};

use crate::api::{self, TimeEntry, TOGGL_URL};
use crate::cache;
use crate::config::{self, icon};
use crate::fuzzy::fuzzy_score;
use crate::ipc;
use crate::log;
use crate::setup;

// Icon names (resolved to absolute SVG paths via config::icon).
const ICON_PLAY: &str = "play";
const ICON_STOP: &str = "stop";
const ICON_PLUS: &str = "plus";
const ICON_WARNING: &str = "warning";
const ICON_SETTINGS: &str = "settings";

/// What activating a given result row should do.
#[derive(Clone)]
enum Action {
    Setup,
    Noop,
    Stop(TimeEntry),
    New,
    SelectProject(Option<i64>),
    CancelNew,
    Start(TimeEntry),
}

pub struct Plugin {
    token: Option<String>,
    wid: Option<i64>,
    keyword: String,
    current: Option<TimeEntry>,
    entries: Vec<TimeEntry>,
    projects: HashMap<i64, String>,
    results: Vec<Action>,
    filter_text_value: String,
    pending_new_desc: Option<String>,
    actions: Vec<JoinHandle<()>>,
}

impl Plugin {
    pub fn new() -> Self {
        let cfg = config::load();
        let plugin = Plugin {
            token: cfg.token,
            wid: cfg.wid,
            keyword: cfg.keyword,
            current: None,
            entries: Vec::new(),
            projects: HashMap::new(),
            results: Vec::new(),
            filter_text_value: String::new(),
            pending_new_desc: None,
            actions: Vec::new(),
        };
        // Pre-warm the cache so data is ready before the user types.
        if let (Some(token), Some(wid)) = (plugin.token.clone(), plugin.wid) {
            cache::start_bg_refresh(token, wid);
        }
        plugin
    }

    fn configured(&self) -> bool {
        self.token.is_some() && self.wid.is_some()
    }

    /// Wait for the activate threads we spawned so timer actions complete before
    /// the process exits (the Python plugin relied on non-daemon threads here).
    pub fn join_actions(&mut self) {
        for h in self.actions.drain(..) {
            let _ = h.join();
        }
    }

    fn fire<F: FnOnce() + Send + 'static>(&mut self, f: F) {
        self.actions.push(thread::spawn(f));
    }

    fn load_from_cache(&mut self) {
        let (current, entries, projects) = cache::snapshot();
        self.current = current;
        self.entries = entries;
        self.projects = projects;
    }

    fn refresh(&mut self) {
        if let (Some(token), Some(wid)) = (self.token.as_deref(), self.wid) {
            cache::refresh(token, wid, false);
        }
        self.load_from_cache();
    }

    fn project_name(&self, entry: &TimeEntry) -> Option<String> {
        entry
            .project_id
            .and_then(|pid| self.projects.get(&pid).cloned())
    }

    fn filter_text(&self, query: &str) -> String {
        let q = query.to_lowercase();
        let kw = self.keyword.to_lowercase();
        let rest = if q.starts_with(&kw) {
            &query[kw.len()..]
        } else {
            query
        };
        rest.trim().to_string()
    }

    /// Best fuzzy score of the filter against the entry's description or project.
    fn score_entry(&self, entry: &TimeEntry, filter: &str) -> Option<i32> {
        let desc = entry.description_or_default().to_string();
        let proj = self.project_name(entry).unwrap_or_default();
        [desc, proj]
            .iter()
            .filter_map(|text| fuzzy_score(filter, text))
            .max()
    }

    // -- Search ---------------------------------------------------------------

    pub fn handle_search(&mut self, query: &str) {
        if !self.configured() {
            ipc::clear();
            self.results = vec![Action::Setup];
            ipc::append(
                0,
                "Toggl Track: Setup required",
                "Click to enter your API token & workspace ID",
                &icon(ICON_SETTINGS),
            );
            ipc::finished();
            return;
        }

        self.refresh();
        self.results.clear();
        ipc::clear();

        if cache::is_rate_limited() {
            self.results = vec![Action::Noop];
            ipc::append(
                0,
                "API rate limit reached",
                "Try again in a few minutes",
                &icon(ICON_WARNING),
            );
            ipc::finished();
            return;
        }

        let filter = self.filter_text(query);
        self.filter_text_value = filter.clone();

        if self.pending_new_desc.is_some() {
            self.show_project_selection(&filter);
            return;
        }

        let mut idx = 0usize;

        // Running timer — click to stop.
        if let Some(current) = self.current.clone() {
            let show = filter.is_empty() || self.score_entry(&current, &filter).is_some();
            if show {
                let desc = current.description_or_default();
                let elapsed = crate::dateutil::elapsed_str(current.start.as_deref().unwrap_or(""));
                let mut detail = format!("Elapsed: {elapsed}");
                if let Some(proj) = self.project_name(&current) {
                    detail = format!("{proj} \u{2022} {detail}");
                }
                ipc::append(
                    idx,
                    &format!("Running: {desc}"),
                    &format!("{detail} \u{2022} Click to stop"),
                    &icon(ICON_STOP),
                );
                self.results.push(Action::Stop(current));
                idx += 1;
            }
        }

        // New timer — always shown; when filtering, the typed text is the description.
        if filter.is_empty() {
            ipc::append(
                idx,
                "New timer",
                "Start a new timer with project selection",
                &icon(ICON_PLUS),
            );
        } else {
            ipc::append(
                idx,
                &format!("New timer: {filter}"),
                "Start timer with project selection",
                &icon(ICON_PLUS),
            );
        }
        self.results.push(Action::New);
        idx += 1;

        // Recent entries — fuzzy filter and rank when there's a filter.
        let max_shown = if filter.is_empty() { 5 } else { 8 };
        let ranked: Vec<TimeEntry> = if filter.is_empty() {
            self.entries.clone()
        } else {
            let mut scored: Vec<(i32, TimeEntry)> = self
                .entries
                .iter()
                .filter_map(|e| self.score_entry(e, &filter).map(|s| (s, e.clone())))
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            scored.into_iter().map(|(_, e)| e).collect()
        };

        for entry in ranked.into_iter().take(max_shown) {
            let desc = entry.description_or_default().to_string();
            let detail = self.project_name(&entry).unwrap_or_else(|| "No project".to_string());
            ipc::append(idx, &desc, &detail, &icon(ICON_PLAY));
            self.results.push(Action::Start(entry));
            idx += 1;
        }

        ipc::finished();
    }

    fn show_project_selection(&mut self, filter: &str) {
        self.results.clear();
        ipc::clear();
        let mut idx = 0usize;

        // Header — clicking cancels.
        let title = match &self.pending_new_desc {
            Some(d) if !d.is_empty() => format!("Creating: {d}"),
            _ => "New timer (no description)".to_string(),
        };
        ipc::append(
            idx,
            &title,
            "Select a project below (click here to cancel)",
            &icon(ICON_PLUS),
        );
        self.results.push(Action::CancelNew);
        idx += 1;

        // No-project option.
        ipc::append(idx, "No project", "Start timer without a project", &icon(ICON_PLAY));
        self.results.push(Action::SelectProject(None));
        idx += 1;

        // Project list — fuzzy-filtered and ranked if typing, else alphabetical.
        let project_list: Vec<(i64, String)> = if filter.is_empty() {
            let mut list: Vec<(i64, String)> =
                self.projects.iter().map(|(id, n)| (*id, n.clone())).collect();
            list.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
            list
        } else {
            let mut scored: Vec<(i32, i64, String)> = self
                .projects
                .iter()
                .filter_map(|(id, name)| fuzzy_score(filter, name).map(|s| (s, *id, name.clone())))
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            scored.into_iter().map(|(_, id, name)| (id, name)).collect()
        };

        for (pid, pname) in project_list {
            ipc::append(idx, &pname, "Select project", &icon(ICON_PLAY));
            self.results.push(Action::SelectProject(Some(pid)));
            idx += 1;
        }

        ipc::finished();
    }

    // -- Activate -------------------------------------------------------------

    pub fn handle_activate(&mut self, id: usize) {
        let Some(action) = self.results.get(id).cloned() else {
            return;
        };
        let token = self.token.clone();
        let wid = self.wid;

        match action {
            Action::Stop(entry) => {
                ipc::fill(&format!("Stopped: {}", entry.description_or_default()));
                ipc::close();
                cache::invalidate();
                if let (Some(token), Some(wid)) = (token, wid) {
                    self.fire(move || stop_entry(&token, wid, &entry));
                }
            }

            Action::New => {
                self.pending_new_desc = Some(self.filter_text_value.clone());
                ipc::fill(&format!("{} ", self.keyword));
            }

            Action::SelectProject(pid) => {
                let desc = self.pending_new_desc.take().unwrap_or_default();
                ipc::fill(&format!("Started: {desc}"));
                ipc::close();
                cache::invalidate();
                let current = self.current.clone();
                if let (Some(token), Some(wid)) = (token, wid) {
                    self.fire(move || {
                        if let Some(c) = &current {
                            stop_entry(&token, wid, c);
                        }
                        api::start_timer(&token, wid, &desc, pid, None, false);
                    });
                }
            }

            Action::CancelNew => {
                self.pending_new_desc = None;
                ipc::fill(&format!("{} ", self.keyword));
            }

            Action::Start(entry) => {
                let desc = entry.description_or_default().to_string();
                ipc::fill(&format!("Started: {desc}"));
                ipc::close();
                cache::invalidate();
                let current = self.current.clone();
                if let (Some(token), Some(wid)) = (token, wid) {
                    self.fire(move || {
                        if let Some(c) = &current {
                            stop_entry(&token, wid, c);
                        }
                        api::start_timer(
                            &token,
                            wid,
                            &desc,
                            entry.project_id,
                            entry.tags.as_ref(),
                            entry.billable,
                        );
                    });
                }
            }

            Action::Setup => match setup::run(&self.keyword) {
                Some((t, w)) => {
                    self.token = Some(t);
                    self.wid = Some(w);
                    ipc::fill(&format!("{} ", self.keyword));
                }
                None => ipc::close(),
            },

            Action::Noop => {}
        }
    }

    // -- Context menu ---------------------------------------------------------

    pub fn handle_context(&mut self, id: usize) {
        let mut options: Vec<(&str, String)> =
            vec![("Open Toggl in browser", "Open track.toggl.com".to_string())];
        if let Some(current) = &self.current {
            options.push(("Stop timer", format!("Stop: {}", current.description_or_default())));
        }
        ipc::context(id, options);
    }

    pub fn handle_activate_context(&mut self, _id: usize, context: usize) {
        match context {
            0 => {
                if let Err(e) = Command::new("xdg-open")
                    .arg(TOGGL_URL)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    log(&format!("xdg-open failed: {e}"));
                }
                ipc::close();
            }
            1 => {
                if let Some(current) = self.current.clone() {
                    ipc::fill(&format!("Stopped: {}", current.description_or_default()));
                    ipc::close();
                    cache::invalidate();
                    if let (Some(token), Some(wid)) = (self.token.clone(), self.wid) {
                        self.fire(move || stop_entry(&token, wid, &current));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Stop a specific entry, using its own workspace if set, else the default.
fn stop_entry(token: &str, default_wid: i64, entry: &TimeEntry) {
    let wid = entry.workspace_id.unwrap_or(default_wid);
    api::stop_timer(token, wid, entry.id);
}
