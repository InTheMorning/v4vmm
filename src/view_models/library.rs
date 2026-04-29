//! Library screen view-models.
//!
//! Pure projections of [`db::TrackRow`] + library-screen-owned state
//! (`MbTrackStatus`) into the strings the library inspector and album
//! detail rows render. Same layer rules as [`super`]: no GPUI imports,
//! no service mutation; constructed fresh each render.
//!
//! The album detail track row was the first call site to migrate, so
//! its projection ([`LibraryTrackRowVm`]) lives here. Future entries
//! (artist node summary, playlist row, `MusicBrainz` panel header) will
//! join as `library.rs` is whittled down.

#![warn(clippy::pedantic)]

use crate::db::TrackRow;
use crate::library::MbTrackStatus;

/// Display-ready projection of a [`TrackRow`] in the library album
/// detail listing.
///
/// Borrow-only — constructed fresh each render and dropped before the
/// element tree is painted.
pub(crate) struct LibraryTrackRowVm<'a> {
    track: &'a TrackRow,
    mb: Option<&'a MbTrackStatus>,
}

/// Semantic colour bucket for the `MusicBrainz` status hint. The screen
/// maps each variant to a token at render time, keeping the VM free of
/// GPUI types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MbStatusKind {
    Success,
    Warning,
    Danger,
    Muted,
}

impl<'a> LibraryTrackRowVm<'a> {
    #[must_use]
    pub(crate) fn new(track: &'a TrackRow, mb: Option<&'a MbTrackStatus>) -> Self {
        Self { track, mb }
    }

    /// Display title — the row's `track_title`, or `"[untitled]"` if
    /// absent. Matches the legacy `library::render_library_track_row`
    /// fallback exactly.
    #[must_use]
    pub(crate) fn title(&self) -> String {
        self.track
            .track_title
            .as_deref()
            .unwrap_or("[untitled]")
            .to_string()
    }

    /// Leading `"{n}. "` segment, empty when there is no track number.
    #[must_use]
    pub(crate) fn number_prefix(&self) -> String {
        self.track
            .track_number
            .map(|n| format!("{n}. "))
            .unwrap_or_default()
    }

    /// Trailing `"  (M:SS)"` segment, empty when there is no
    /// duration.
    #[must_use]
    pub(crate) fn duration_suffix(&self) -> String {
        self.track
            .duration_seconds
            .map(|s| format!("  ({}:{:02})", s / 60, s % 60))
            .unwrap_or_default()
    }

    /// Concatenated single-line label: `"{n}. {title}  (M:SS)"`.
    #[must_use]
    pub(crate) fn full_label(&self) -> String {
        format!(
            "{}{}{}",
            self.number_prefix(),
            self.title(),
            self.duration_suffix()
        )
    }

    /// Human-readable `MusicBrainz` status hint, or `None` when no
    /// lookup has been started for this track.
    #[must_use]
    pub(crate) fn mb_status_text(&self) -> Option<&'static str> {
        match self.mb? {
            MbTrackStatus::Pending => Some("MB: pending"),
            MbTrackStatus::Processing => Some("MB: looking up..."),
            MbTrackStatus::Done(0) => Some("MB: no missing fields"),
            MbTrackStatus::Done(_) => Some("MB: done"),
            MbTrackStatus::Skipped(_) => Some("MB: skipped"),
        }
    }

    /// Semantic colour bucket for the status hint (or `None` when
    /// there is no hint).
    #[must_use]
    pub(crate) fn mb_status_kind(&self) -> Option<MbStatusKind> {
        match self.mb? {
            MbTrackStatus::Done(n) if *n > 0 => Some(MbStatusKind::Success),
            MbTrackStatus::Skipped(_) => Some(MbStatusKind::Danger),
            MbTrackStatus::Processing => Some(MbStatusKind::Warning),
            _ => Some(MbStatusKind::Muted),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> TrackRow {
        TrackRow {
            id: 0,
            feed_id: 0,
            feed_guid: None,
            item_guid: String::new(),
            track_title: None,
            artist_name: None,
            album_title: None,
            album_artist_name: None,
            track_number: None,
            disc_number: None,
            duration_seconds: None,
            enclosure_url: None,
            enclosure_type: None,
            track_image_href: None,
            is_in_library: false,
            feed_title: None,
            album_image_href: None,
            local_path: None,
            transcript_url: None,
        }
    }

    #[test]
    fn title_falls_back_to_untitled_marker() {
        assert_eq!(LibraryTrackRowVm::new(&row(), None).title(), "[untitled]");
        let mut r = row();
        r.track_title = Some("Hello".into());
        assert_eq!(LibraryTrackRowVm::new(&r, None).title(), "Hello");
    }

    #[test]
    fn number_prefix_renders_only_when_present() {
        assert_eq!(LibraryTrackRowVm::new(&row(), None).number_prefix(), "");
        let mut r = row();
        r.track_number = Some(7);
        assert_eq!(LibraryTrackRowVm::new(&r, None).number_prefix(), "7. ");
    }

    #[test]
    fn duration_suffix_pads_seconds_below_ten() {
        let mut r = row();
        r.duration_seconds = Some(65);
        assert_eq!(
            LibraryTrackRowVm::new(&r, None).duration_suffix(),
            "  (1:05)"
        );
    }

    #[test]
    fn duration_suffix_is_empty_when_absent() {
        assert_eq!(LibraryTrackRowVm::new(&row(), None).duration_suffix(), "");
    }

    #[test]
    fn full_label_concatenates_segments() {
        let mut r = row();
        r.track_number = Some(3);
        r.track_title = Some("Track Three".into());
        r.duration_seconds = Some(245);
        assert_eq!(
            LibraryTrackRowVm::new(&r, None).full_label(),
            "3. Track Three  (4:05)"
        );
    }

    #[test]
    fn mb_status_text_distinguishes_done_zero_and_done_nonzero() {
        let r = row();
        let pending = MbTrackStatus::Pending;
        let processing = MbTrackStatus::Processing;
        let done_zero = MbTrackStatus::Done(0);
        let done_some = MbTrackStatus::Done(2);
        let skipped = MbTrackStatus::Skipped("bad".into());

        assert_eq!(LibraryTrackRowVm::new(&r, None).mb_status_text(), None);
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&pending)).mb_status_text(),
            Some("MB: pending")
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&processing)).mb_status_text(),
            Some("MB: looking up...")
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&done_zero)).mb_status_text(),
            Some("MB: no missing fields")
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&done_some)).mb_status_text(),
            Some("MB: done")
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&skipped)).mb_status_text(),
            Some("MB: skipped")
        );
    }

    #[test]
    fn mb_status_kind_routes_done_zero_to_muted_not_success() {
        let r = row();
        let done_zero = MbTrackStatus::Done(0);
        let done_some = MbTrackStatus::Done(3);
        let processing = MbTrackStatus::Processing;
        let skipped = MbTrackStatus::Skipped("nope".into());
        let pending = MbTrackStatus::Pending;

        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&done_zero)).mb_status_kind(),
            Some(MbStatusKind::Muted)
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&done_some)).mb_status_kind(),
            Some(MbStatusKind::Success)
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&processing)).mb_status_kind(),
            Some(MbStatusKind::Warning)
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&skipped)).mb_status_kind(),
            Some(MbStatusKind::Danger)
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&pending)).mb_status_kind(),
            Some(MbStatusKind::Muted)
        );
        assert_eq!(LibraryTrackRowVm::new(&r, None).mb_status_kind(), None);
    }
}
