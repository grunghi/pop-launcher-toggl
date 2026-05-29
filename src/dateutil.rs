//! Timestamp helpers, backed by the `time` crate.

use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::OffsetDateTime;

/// Current UTC time as `YYYY-MM-DDTHH:MM:SSZ` (Toggl `start` format).
pub fn now_rfc3339() -> String {
    let fmt = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");
    OffsetDateTime::now_utc().format(&fmt).unwrap_or_default()
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
