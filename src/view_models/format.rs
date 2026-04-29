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
