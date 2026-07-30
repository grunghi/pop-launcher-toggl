//! Timestamp helpers, backed by the `time` crate.

use std::sync::OnceLock;

use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};

static LOG_OFFSET: OnceLock<UtcOffset> = OnceLock::new();

/// Capture the machine's UTC offset while the process is still single-threaded.
///
/// `time` refuses to determine the local offset once other threads exist (it would be
/// unsound), so this must be called first thing in `main`, before any thread is spawned.
/// Falls back to UTC if the offset can't be determined — the formatted timestamp always
/// carries its offset, so it stays unambiguous either way.
pub fn init_log_offset() {
    let _ = LOG_OFFSET.set(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC));
}

/// Current UTC time as `YYYY-MM-DDTHH:MM:SSZ` (Toggl `start` format).
pub fn now_rfc3339() -> String {
    let fmt = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");
    OffsetDateTime::now_utc().format(&fmt).unwrap_or_default()
}

/// Current time as `YYYY-MM-DD HH:MM:SS.mmm+HH:MM`, for log line prefixes.
///
/// Rendered in the offset captured by [`init_log_offset`] so log lines line up with
/// `journalctl` without mental arithmetic; the trailing offset keeps it unambiguous.
pub fn log_timestamp() -> String {
    let fmt = format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]\
         [offset_hour sign:mandatory]:[offset_minute]"
    );
    let offset = LOG_OFFSET.get().copied().unwrap_or(UtcOffset::UTC);
    OffsetDateTime::now_utc()
        .to_offset(offset)
        .format(&fmt)
        .unwrap_or_default()
}

/// Human-readable elapsed time since `start_iso` (e.g. `1h 05m`, `42m`, `?`).
pub fn elapsed_str(start_iso: &str) -> String {
    let Ok(start) = OffsetDateTime::parse(start_iso, &Rfc3339) else {
        return "?".to_string();
    };
    let secs = (OffsetDateTime::now_utc() - start).whole_seconds();
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h != 0 {
        format!("{h}h {m:02}m")
    } else {
        format!("{m}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    fn ago(d: Duration) -> String {
        (OffsetDateTime::now_utc() - d).format(&Rfc3339).unwrap()
    }

    #[test]
    fn now_rfc3339_shape() {
        let s = now_rfc3339();
        assert_eq!(s.len(), 20, "got {s}");
        assert!(s.ends_with('Z'), "got {s}");
        // Round-trips through the parser.
        assert!(OffsetDateTime::parse(&s, &Rfc3339).is_ok(), "got {s}");
    }

    #[test]
    fn elapsed_minutes_and_hours() {
        assert_eq!(elapsed_str(&ago(Duration::seconds(0))), "0m");
        assert_eq!(elapsed_str(&ago(Duration::minutes(42))), "42m");
        assert_eq!(
            elapsed_str(&ago(Duration::minutes(125))), // 2h05m
            "2h 05m"
        );
    }

    #[test]
    fn elapsed_handles_offset_and_z_equivalently() {
        // Same instant expressed with a +02:00 offset and as UTC.
        let with_offset = "2020-01-01T02:00:00+02:00";
        let as_utc = "2020-01-01T00:00:00Z";
        assert_eq!(elapsed_str(with_offset), elapsed_str(as_utc));
    }

    #[test]
    fn elapsed_bad_input() {
        assert_eq!(elapsed_str("not a date"), "?");
        assert_eq!(elapsed_str(""), "?");
    }
}
