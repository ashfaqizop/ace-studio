//! Small helpers: wall-clock time without a date crate, JSON files, randomness, URL encoding.

use serde::{Serialize, de::DeserializeOwned};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn unix_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// (year, month, day, hour, minute, second) in UTC.
pub fn civil(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Howard Hinnant's days-from-civil, inverted
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d, (rem / 3600) as u32, ((rem % 3600) / 60) as u32, (rem % 60) as u32)
}

pub fn date() -> String {
    let (y, m, d, ..) = civil(unix_secs());
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn time() -> String {
    let (.., h, mi, s) = civil(unix_secs());
    format!("{h:02}:{mi:02}:{s:02}")
}

pub fn iso8601() -> String {
    let (y, m, d, h, mi, s) = civil(unix_secs());
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

pub fn read_json<T: DeserializeOwned + Default>(path: &str) -> T {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
            tracing::warn!("{path} is not valid JSON ({e}); starting fresh");
            T::default()
        }),
        Err(_) => T::default(),
    }
}

pub fn write_json<T: Serialize>(path: &str, value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(text) => {
            let tmp = format!("{path}.tmp");
            if std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, path)).is_err() {
                tracing::warn!("Could not save {path}");
            }
        }
        Err(e) => tracing::warn!("Could not serialise {path}: {e}"),
    }
}

pub fn append_line(path: &str, line: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}

pub fn random_u64() -> u64 {
    wasi::random::random::get_random_u64()
}

/// Percent-encodes everything outside the unreserved set (for emoji in reaction URLs).
pub fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Replaces `%name%` placeholders with plain values.
pub fn fill(template: &str, vars: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (k, v) in vars {
        if out.contains('%') {
            out = out.replace(&format!("%{k}%"), v);
        }
    }
    out
}

/// Removes ANSI escape sequences (from `to_pretty_console`).
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Truncates to at most `max` characters (not bytes).
pub fn truncate(s: &str, max: usize) -> String {
    if max == 0 || s.chars().count() <= max { s.to_string() } else { s.chars().take(max).collect() }
}
