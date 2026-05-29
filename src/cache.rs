//! Shared cache with stale-while-revalidate semantics.
//!
//! Mirrors the Python plugin: a process-global cache, a background refresh
//! worker, and a one-shot "ready" signal so a cold start can wait briefly for
//! the first successful fetch before rendering.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::api::{self, ApiError, TimeEntry};

pub const CACHE_TTL: Duration = Duration::from_secs(120);

#[derive(Default)]
pub struct Cache {
    pub current: Option<TimeEntry>,
    pub entries: Vec<TimeEntry>,
    pub projects: HashMap<i64, String>,
    /// Time of the last write; `None` means cold / invalidated.
    pub ts: Option<Instant>,
}

fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Cache::default()))
}

fn ready() -> &'static (Mutex<bool>, Condvar) {
    static READY: OnceLock<(Mutex<bool>, Condvar)> = OnceLock::new();
    READY.get_or_init(|| (Mutex::new(false), Condvar::new()))
}

static RATE_LIMITED: AtomicBool = AtomicBool::new(false);
static BG_REFRESHING: AtomicBool = AtomicBool::new(false);

pub fn is_rate_limited() -> bool {
    RATE_LIMITED.load(Ordering::SeqCst)
}

/// Age of the cached data, or `None` if the cache is cold/invalidated.
pub fn age() -> Option<Duration> {
    cache().lock().unwrap().ts.map(|t| t.elapsed())
}

/// Force the next refresh to hit the API.
pub fn invalidate() {
    cache().lock().unwrap().ts = None;
}

/// Snapshot the cached data as `(current, entries, projects)`.
pub fn snapshot() -> (Option<TimeEntry>, Vec<TimeEntry>, HashMap<i64, String>) {
    let c = cache().lock().unwrap();
    (c.current.clone(), c.entries.clone(), c.projects.clone())
}

/// Fetch everything from the API (in parallel) and update the cache.
fn do_refresh(token: &str, wid: i64) {
    let (cur, recent, projects) = {
        let t1 = token.to_string();
        let t2 = token.to_string();
        let t3 = token.to_string();
        let h_cur = thread::spawn(move || api::get_current(&t1));
        let h_rec = thread::spawn(move || api::get_recent(&t2));
        let h_prj = thread::spawn(move || api::get_projects(&t3, wid));
        (
            h_cur.join().unwrap_or(Err(ApiError::Other)),
            h_rec.join().unwrap_or_default(),
            h_prj.join().unwrap_or(Err(ApiError::Other)),
        )
    };

    // A rate-limited current timer aborts the refresh (matches Python).
    let current = match cur {
        Err(ApiError::RateLimit) => {
            RATE_LIMITED.store(true, Ordering::SeqCst);
            cache().lock().unwrap().ts = Some(Instant::now());
            return;
        }
        Err(ApiError::Other) => None,
        Ok(c) => c,
    };

    let projects = match projects {
        Err(ApiError::RateLimit) => {
            RATE_LIMITED.store(true, Ordering::SeqCst);
            cache().lock().unwrap().ts = Some(Instant::now());
            return;
        }
        Err(ApiError::Other) => HashMap::new(),
        Ok(p) => p,
    };

    RATE_LIMITED.store(false, Ordering::SeqCst);

    // Deduplicate by (description, project_id), keep the most recent, cap at 20.
    let current_id = current.as_ref().map(|c| c.id);
    let mut seen: std::collections::HashSet<(String, Option<i64>)> = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for e in recent {
        if Some(e.id) == current_id {
            continue;
        }
        let key = (e.description.clone().unwrap_or_default(), e.project_id);
        if seen.insert(key) {
            deduped.push(e);
        }
        if deduped.len() >= 20 {
            break;
        }
    }

    let mut c = cache().lock().unwrap();
    c.current = current;
    c.entries = deduped;
    c.projects = projects;
    c.ts = Some(Instant::now());
}

/// Kick off a background refresh if one isn't already running.
pub fn start_bg_refresh(token: String, wid: i64) {
    if BG_REFRESHING.swap(true, Ordering::SeqCst) {
        return; // already refreshing
    }
    thread::spawn(move || {
        do_refresh(&token, wid);
        BG_REFRESHING.store(false, Ordering::SeqCst);
        let (lock, cv) = ready();
        *lock.lock().unwrap() = true;
        cv.notify_all();
    });
}

/// Block until the first refresh signals ready, or `timeout` elapses.
pub fn wait_ready(timeout: Duration) {
    let (lock, cv) = ready();
    let guard = lock.lock().unwrap();
    if *guard {
        return;
    }
    let _ = cv.wait_timeout_while(guard, timeout, |ready| !*ready);
}

/// Refresh strategy: fresh→use cache, cold→wait briefly, stale→serve + refresh.
pub fn refresh(token: &str, wid: i64, force: bool) {
    match age() {
        Some(a) if !force && a < CACHE_TTL => {} // fresh — nothing to do
        None => {
            // Cold start — wait briefly for the pre-warm to deliver data.
            start_bg_refresh(token.to_string(), wid);
            wait_ready(Duration::from_secs(2));
        }
        _ => {
            // Stale (or forced) — serve what we have, refresh in the background.
            start_bg_refresh(token.to_string(), wid);
        }
    }
}
