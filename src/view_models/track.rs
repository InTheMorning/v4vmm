//! Track row / inspector view-model.
//!
//! Pure projection of an [`api::Track`] into the strings the
//! discover-row, search-row, and track-inspector views need. Same
//! rules as [`super`]: no GPUI imports, no service mutation.
//!
//! The view-model owns the legacy fallback chains that used to live
//! as free helpers in `search.rs` (`track_title`, `track_play_url`,
//! `fmt_dur`) so that every screen rendering a track gets the same
//! display contract by construction.

#![warn(clippy::pedantic)]

use crate::api::{SourceEnclosure, Track};

/// Display-ready projection of an [`api::Track`].
///
/// Borrow-only — constructed fresh each render and dropped before the
/// element tree is painted.
pub struct TrackVm<'a> {
    track: &'a Track,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct TrackPlayAudioDisplay {
    pub url: Option<String>,
    pub tooltip: String,
    pub disabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct TrackRowControlsDisplay {
    pub row_id: String,
    pub play_button_id: String,
    pub playlist_popover_id: String,
    pub playlist_trigger_label: &'static str,
}

impl<'a> TrackVm<'a> {
    #[must_use]
    pub fn new(track: &'a Track) -> Self {
        Self { track }
    }

    /// The underlying track. Useful when the screen still needs raw
    /// fields the VM hasn't projected yet.
    #[must_use]
    pub fn track(&self) -> &'a Track {
        self.track
    }

    /// Stable identifier. Empty string when the track has no GUID.
    #[must_use]
    pub fn guid(&self) -> String {
        self.track.track_guid.clone().unwrap_or_default()
    }

    /// Display title, with the legacy fallback chain
    /// `title -> name -> guid -> "Untitled"`.
    #[must_use]
    pub fn title(&self) -> String {
        self.track
            .title
            .clone()
            .or_else(|| self.track.name.clone())
            .or_else(|| self.track.track_guid.clone())
            .unwrap_or_else(|| "Untitled".into())
    }

    /// Display title with an optional inspector-provided override.
    /// Empty overrides preserve the legacy fallback to [`Self::title`].
    #[must_use]
    pub fn display_title(&self, override_title: Option<&str>) -> String {
        override_title.map_or_else(
            || self.title(),
            |title| {
                if title.is_empty() {
                    self.title()
                } else {
                    title.to_string()
                }
            },
        )
    }

    /// Header artist, with the shared fallback chain
    /// `track_artist -> release_artist -> "Unknown"`.
    #[must_use]
    pub fn artist(&self) -> String {
        self.track
            .track_artist
            .clone()
            .or_else(|| self.track.release_artist.clone())
            .unwrap_or_else(|| "Unknown".into())
    }

    /// Track number formatted for the discover-row leading column —
    /// the number itself, or `"·"` when absent.
    #[must_use]
    pub fn track_number_label(&self) -> String {
        self.track
            .track_number
            .map_or_else(|| "·".into(), |n| n.to_string())
    }

    /// Duration as `"M:SS"`, or `None` when the track has no
    /// duration.
    #[must_use]
    pub fn duration_display(&self) -> Option<String> {
        self.track.duration_secs.map(fmt_dur)
    }

    /// Best playable URL for the track. Tries, in order:
    ///
    /// 1. The track's direct enclosure URL (when non-empty).
    /// 2. The first source enclosure marked `is_primary = true`.
    /// 3. The first source enclosure with any non-empty URL.
    #[must_use]
    pub fn play_url(&self) -> Option<String> {
        nonempty_url(self.track.enclosure_url.as_deref())
            .map(str::to_string)
            .or_else(|| {
                self.track
                    .source_enclosures
                    .as_deref()
                    .and_then(primary_source_enclosure_url)
            })
            .or_else(|| {
                self.track
                    .source_enclosures
                    .as_deref()
                    .and_then(first_source_enclosure_url)
            })
    }

    pub fn play_audio_display(&self) -> TrackPlayAudioDisplay {
        let url = self.play_url();
        let tooltip = url.clone().unwrap_or_else(|| "No audio URL".into());
        let disabled = url.is_none();
        TrackPlayAudioDisplay {
            url,
            tooltip,
            disabled,
        }
    }

    pub fn row_controls_display(&self) -> TrackRowControlsDisplay {
        let guid = self.guid();
        TrackRowControlsDisplay {
            row_id: format!("track-row:{guid}"),
            play_button_id: format!("track-row-play:{guid}"),
            playlist_popover_id: format!("add-pl:{guid}"),
            playlist_trigger_label: "+ Playlist",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct TrackHeaderVm {
    pub title: String,
    pub artist: String,
}

impl TrackHeaderVm {
    pub fn new(track: &Track, override_title: Option<&str>) -> Self {
        let vm = TrackVm::new(track);
        Self {
            title: vm.display_title(override_title),
            artist: vm.artist(),
        }
    }
}

/// Format a duration in seconds as `"M:SS"`. Matches the legacy
/// `search::fmt_dur` contract exactly.
#[must_use]
pub fn fmt_dur(secs: i32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

fn primary_source_enclosure_url(enclosures: &[SourceEnclosure]) -> Option<String> {
    enclosures
        .iter()
        .filter(|enclosure| enclosure.is_primary.unwrap_or(false))
        .find_map(|enclosure| nonempty_url(enclosure.url.as_deref()).map(str::to_string))
}

fn first_source_enclosure_url(enclosures: &[SourceEnclosure]) -> Option<String> {
    enclosures
        .iter()
        .find_map(|enclosure| nonempty_url(enclosure.url.as_deref()).map(str::to_string))
}

fn nonempty_url(url: Option<&str>) -> Option<&str> {
    url.map(str::trim).filter(|url| !url.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track() -> Track {
        Track::default()
    }

    fn enc(url: &str, primary: bool) -> SourceEnclosure {
        SourceEnclosure {
            url: Some(url.into()),
            is_primary: Some(primary),
            ..SourceEnclosure::default()
        }
    }

    #[test]
    fn fmt_dur_pads_seconds_below_ten() {
        assert_eq!(fmt_dur(65), "1:05");
        assert_eq!(fmt_dur(0), "0:00");
        assert_eq!(fmt_dur(3725), "62:05");
    }

    #[test]
    fn title_prefers_title_then_name_then_guid_then_default() {
        let mut t = track();
        t.title = Some("T".into());
        t.name = Some("N".into());
        t.track_guid = Some("G".into());
        assert_eq!(TrackVm::new(&t).title(), "T");

        t.title = None;
        assert_eq!(TrackVm::new(&t).title(), "N");

        t.name = None;
        assert_eq!(TrackVm::new(&t).title(), "G");

        t.track_guid = None;
        assert_eq!(TrackVm::new(&t).title(), "Untitled");
    }

    #[test]
    fn display_title_uses_nonempty_override_else_title_fallback() {
        let mut t = track();
        t.title = Some("Track title".into());

        assert_eq!(
            TrackVm::new(&t).display_title(Some("Inspector title")),
            "Inspector title"
        );
        assert_eq!(TrackVm::new(&t).display_title(Some("")), "Track title");
        assert_eq!(TrackVm::new(&t).display_title(None), "Track title");
    }

    #[test]
    fn artist_prefers_track_then_release_then_unknown() {
        let mut t = track();
        t.track_artist = Some("Track Artist".into());
        t.release_artist = Some("Release Artist".into());
        assert_eq!(TrackVm::new(&t).artist(), "Track Artist");

        t.track_artist = None;
        assert_eq!(TrackVm::new(&t).artist(), "Release Artist");

        t.release_artist = None;
        assert_eq!(TrackVm::new(&t).artist(), "Unknown");
    }

    #[test]
    fn track_header_vm_projects_display_contract() {
        let mut t = track();
        t.title = Some("Track title".into());
        t.track_artist = Some("Artist".into());

        assert_eq!(
            TrackHeaderVm::new(&t, Some("Header title")),
            TrackHeaderVm {
                title: "Header title".into(),
                artist: "Artist".into(),
            }
        );
    }

    #[test]
    fn guid_defaults_to_empty_string() {
        assert_eq!(TrackVm::new(&track()).guid(), "");
        let mut t = track();
        t.track_guid = Some("abc".into());
        assert_eq!(TrackVm::new(&t).guid(), "abc");
    }

    #[test]
    fn track_number_label_is_dot_when_absent() {
        let t = track();
        assert_eq!(TrackVm::new(&t).track_number_label(), "·");
    }

    #[test]
    fn track_number_label_is_number_when_present() {
        let mut t = track();
        t.track_number = Some(7);
        assert_eq!(TrackVm::new(&t).track_number_label(), "7");
    }

    #[test]
    fn duration_display_is_none_when_absent() {
        assert!(TrackVm::new(&track()).duration_display().is_none());
    }

    #[test]
    fn duration_display_formats_seconds() {
        let mut t = track();
        t.duration_secs = Some(125);
        assert_eq!(TrackVm::new(&t).duration_display().as_deref(), Some("2:05"));
    }

    #[test]
    fn play_url_prefers_direct_enclosure_when_nonempty() {
        let mut t = track();
        t.enclosure_url = Some("https://e/x.mp3".into());
        t.source_enclosures = Some(vec![enc("https://s/y.mp3", true)]);
        assert_eq!(
            TrackVm::new(&t).play_url().as_deref(),
            Some("https://e/x.mp3")
        );
    }

    #[test]
    fn play_audio_display_uses_url_tooltip_else_missing_audio_fallback() {
        let mut playable = track();
        playable.enclosure_url = Some(" https://e/x.mp3 ".into());
        assert_eq!(
            TrackVm::new(&playable).play_audio_display(),
            TrackPlayAudioDisplay {
                url: Some("https://e/x.mp3".into()),
                tooltip: "https://e/x.mp3".into(),
                disabled: false,
            }
        );

        assert_eq!(
            TrackVm::new(&track()).play_audio_display(),
            TrackPlayAudioDisplay {
                url: None,
                tooltip: "No audio URL".into(),
                disabled: true,
            }
        );
    }

    #[test]
    fn row_controls_display_projects_track_row_ids_and_playlist_label() {
        let mut t = track();
        t.track_guid = Some("guid-1".into());

        assert_eq!(
            TrackVm::new(&t).row_controls_display(),
            TrackRowControlsDisplay {
                row_id: "track-row:guid-1".into(),
                play_button_id: "track-row-play:guid-1".into(),
                playlist_popover_id: "add-pl:guid-1".into(),
                playlist_trigger_label: "+ Playlist",
            }
        );
    }

    #[test]
    fn row_controls_display_preserves_empty_guid_ids() {
        assert_eq!(
            TrackVm::new(&track()).row_controls_display(),
            TrackRowControlsDisplay {
                row_id: "track-row:".into(),
                play_button_id: "track-row-play:".into(),
                playlist_popover_id: "add-pl:".into(),
                playlist_trigger_label: "+ Playlist",
            }
        );
    }

    #[test]
    fn play_url_skips_blank_enclosure_for_primary_source() {
        let mut t = track();
        t.enclosure_url = Some("   ".into());
        t.source_enclosures = Some(vec![
            enc("https://s/non-primary.mp3", false),
            enc("https://s/primary.mp3", true),
        ]);
        assert_eq!(
            TrackVm::new(&t).play_url().as_deref(),
            Some("https://s/primary.mp3")
        );
    }

    #[test]
    fn play_url_falls_back_to_first_source_when_no_primary() {
        let mut t = track();
        t.enclosure_url = None;
        t.source_enclosures = Some(vec![
            enc("https://s/a.mp3", false),
            enc("https://s/b.mp3", false),
        ]);
        assert_eq!(
            TrackVm::new(&t).play_url().as_deref(),
            Some("https://s/a.mp3")
        );
    }

    #[test]
    fn play_url_is_none_when_no_sources() {
        assert!(TrackVm::new(&track()).play_url().is_none());
    }
}
