//! Minimal date math (no chrono dependency).
//!
//! Implements Howard Hinnant's `days_from_civil` / `civil_from_days` so we can
//! convert between civil dates and Unix epoch seconds, which is all we need to
//! format the current UTC time and compute elapsed durations.

use std::time::{SystemTime, UNIX_EPOCH};

/// Days since 1970-01-01 for the given civil date (proleptic Gregorian).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400; // [0, 399]
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

/// Civil date (year, month, day) from days since 1970-01-01.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Current UTC time as `YYYY-MM-DDTHH:MM:SSZ` (Toggl `start` format).
pub fn now_rfc3339() -> String {
    let secs = now_epoch();
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Parse an ISO-8601 timestamp (`YYYY-MM-DDTHH:MM:SS[.fff][Z|±HH:MM]`) to epoch
/// seconds (UTC). Returns `None` on any malformed input.
fn parse_epoch(s: &str) -> Option<i64> {
    let (date, rest) = s.split_once('T')?;
    let mut dp = date.split('-');
    let y: i64 = dp.next()?.parse().ok()?;
    let mo: i64 = dp.next()?.parse().ok()?;
    let d: i64 = dp.next()?.parse().ok()?;

    let (time_str, offset_secs) = split_offset(rest)?;
    let mut tp = time_str.split(':');
    let h: i64 = tp.next()?.parse().ok()?;
    let mi: i64 = tp.next()?.parse().ok()?;
    // Seconds may carry a fractional part; drop it.
    let sec: i64 = tp.next().unwrap_or("0").split('.').next()?.parse().ok()?;

    let days = days_from_civil(y, mo, d);
    Some(days * 86400 + h * 3600 + mi * 60 + sec - offset_secs)
}

/// Split a time component into `(HH:MM:SS, offset_seconds_to_subtract_for_utc)`.
fn split_offset(rest: &str) -> Option<(&str, i64)> {
    if let Some(stripped) = rest.strip_suffix('Z') {
        return Some((stripped, 0));
    }
    for (i, c) in rest.char_indices().rev() {
        if c == '+' || c == '-' {
            let off = &rest[i + 1..];
            let sign = if c == '+' { 1 } else { -1 };
            let mut o = off.split(':');
            let oh: i64 = o.next()?.parse().ok()?;
            let om: i64 = o.next().unwrap_or("0").parse().ok()?;
            return Some((&rest[..i], sign * (oh * 3600 + om * 60)));
        }
    }
    // No timezone marker — assume UTC.
    Some((rest, 0))
}

/// Human-readable elapsed time since `start_iso` (e.g. `1h 05m`, `42m`, `?`).
pub fn elapsed_str(start_iso: &str) -> String {
    let Some(start) = parse_epoch(start_iso) else {
        return "?".to_string();
    };
    let secs = now_epoch() - start;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h != 0 {
        format!("{}h {:02}m", h, m)
    } else {
        format!("{}m", m)
    }
}
