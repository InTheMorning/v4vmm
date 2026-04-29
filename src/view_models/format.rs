//! Pure formatting helpers shared by view-models.
//!
//! These were previously defined inside `search.rs`. They are pure
//! string formatters with no GPUI dependency, so they belong in the
//! domain-adjacent layer where view-models can use them without
//! pulling in the entire screen module. `search.rs` now re-exports
//! these names for backward compatibility with the rest of the UI.

#![warn(clippy::pedantic)]

/// Format a duration in seconds as `"H h M min"` or `"M min"` when
/// less than an hour. Matches the legacy `search::fmt_runtime`
/// contract exactly.
#[must_use]
pub fn fmt_runtime(total_secs: i32) -> String {
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    if hours > 0 {
        format!("{hours} h {minutes} min")
    } else {
        format!("{minutes} min")
    }
}

/// Format a Unix epoch timestamp as `"Mon D, YYYY"` (e.g. `"Apr 5,
/// 2024"`). Returns `None` if the timestamp is out of range.
#[must_use]
pub fn fmt_date(ts: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.format("%b %-d, %Y").to_string())
}

/// Format a total-runtime in seconds as a clock-style label:
///
/// - `"M:SS"` when the total is under one hour.
/// - `"Hh Mm"` once it crosses 60 minutes (seconds are dropped, the
///   trailing `"m"` is the legacy abbreviation used by the playlist /
///   album panels).
/// - `None` when the total is `<= 0`, which the screen renders as "no
///   duration row".
///
/// This pins the formatter the playlist and album detail panels both
/// used to inline. The two paths diverged once already (one rendered
/// `"M:SS"`, the other `"Mh Sm"`); centralising here keeps them
/// honest.
#[must_use]
pub fn fmt_total_runtime_clock(total_secs: i64) -> Option<String> {
    if total_secs <= 0 {
        return None;
    }
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    Some(if mins >= 60 {
        format!("{}h {}m", mins / 60, mins % 60)
    } else {
        format!("{mins}:{secs:02}")
    })
}

/// Push `(key, value)` into `rows` only when `value` is `Some` and not
/// empty. Lifted from the deleted `ui_common::optional_row`.
pub fn optional_row(rows: &mut Vec<(String, String)>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        if !value.is_empty() {
            rows.push((key.into(), value));
        }
    }
}

/// English plural suffix: empty for `1`, otherwise `"s"`. Lifted from
/// the deleted `ui_common::plural`.
#[must_use]
pub fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_runtime_under_one_hour() {
        assert_eq!(fmt_runtime(0), "0 min");
        assert_eq!(fmt_runtime(59), "0 min");
        assert_eq!(fmt_runtime(60), "1 min");
        assert_eq!(fmt_runtime(3599), "59 min");
    }

    #[test]
    fn fmt_runtime_one_hour_or_more() {
        assert_eq!(fmt_runtime(3600), "1 h 0 min");
        assert_eq!(fmt_runtime(3660), "1 h 1 min");
        assert_eq!(fmt_runtime(7325), "2 h 2 min");
    }

    #[test]
    fn fmt_date_renders_ymd() {
        // 2024-04-05T00:00:00 UTC
        assert_eq!(fmt_date(1_712_275_200).as_deref(), Some("Apr 5, 2024"));
    }

    #[test]
    fn fmt_total_runtime_clock_returns_none_for_zero_or_negative() {
        assert_eq!(fmt_total_runtime_clock(0), None);
        assert_eq!(fmt_total_runtime_clock(-1), None);
    }

    #[test]
    fn fmt_total_runtime_clock_pads_seconds_below_ten() {
        assert_eq!(fmt_total_runtime_clock(65).as_deref(), Some("1:05"));
        assert_eq!(fmt_total_runtime_clock(305).as_deref(), Some("5:05"));
    }

    #[test]
    fn fmt_total_runtime_clock_renders_minutes_below_an_hour() {
        assert_eq!(fmt_total_runtime_clock(60).as_deref(), Some("1:00"));
        assert_eq!(fmt_total_runtime_clock(3599).as_deref(), Some("59:59"));
    }

    #[test]
    fn fmt_total_runtime_clock_switches_to_hours_at_sixty_minutes() {
        assert_eq!(fmt_total_runtime_clock(3600).as_deref(), Some("1h 0m"));
        assert_eq!(fmt_total_runtime_clock(4980).as_deref(), Some("1h 23m"));
        assert_eq!(fmt_total_runtime_clock(7320).as_deref(), Some("2h 2m"));
    }

    #[test]
    fn plural_returns_empty_for_one() {
        assert_eq!(plural(0), "s");
        assert_eq!(plural(1), "");
        assert_eq!(plural(2), "s");
        assert_eq!(plural(99), "s");
    }

    #[test]
    fn optional_row_skips_none_and_empty() {
        let mut rows: Vec<(String, String)> = Vec::new();
        optional_row(&mut rows, "A", None);
        optional_row(&mut rows, "B", Some(String::new()));
        optional_row(&mut rows, "C", Some("ok".into()));
        assert_eq!(rows, vec![("C".to_string(), "ok".to_string())]);
    }
}
