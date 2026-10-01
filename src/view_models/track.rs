//! Track header view-model.
//!
//! `fmt_dur` is the shared duration formatter every track-row and
//! track-detail view-model uses. `TrackHeaderVm` is the display
//! contract the shared track header composite renders. Same rules as
//! [`super`]: no GPUI imports, no service mutation.

#![warn(clippy::pedantic)]

#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct TrackHeaderVm {
    pub title: String,
    pub artist: String,
}

/// Format a duration in seconds as `"M:SS"`. Matches the legacy
/// `search::fmt_dur` contract exactly.
#[must_use]
pub fn fmt_dur(secs: i32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_dur_pads_seconds_below_ten() {
        assert_eq!(fmt_dur(65), "1:05");
        assert_eq!(fmt_dur(0), "0:00");
        assert_eq!(fmt_dur(3725), "62:05");
    }
}
