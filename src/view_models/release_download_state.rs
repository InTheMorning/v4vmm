//! The download-state icon of a release row (ADR 0083 Decision 6, amended
//! 2026-10-22; ADR 0084 task 033).
//!
//! The Releases list, a singles group row, a group disclosure row, the
//! feed page, the name page, the search list and the saved-search page
//! all show the download state of a release through this one type. Each
//! of those view models builds this type from its own facts, and
//! `src/ui/composites/release_download_state.rs` is the one composite
//! that maps the typed state to a named icon and a color token. This
//! module names no renderer type.

#![warn(clippy::pedantic)]

/// The download state of a release row (ADR 0083 Decision 6, amended
/// 2026-10-22).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReleaseDownloadState {
    /// No track of the release is downloaded, or the release has no
    /// track.
    NotDownloaded,
    /// A download job of the release waits in the queue.
    Queued,
    /// A download job of the release runs.
    Downloading,
    /// Some, but not every, track of the release is downloaded.
    Partial { downloaded: usize, total: usize },
    /// Each track of the release is downloaded.
    Downloaded,
}

impl ReleaseDownloadState {
    /// The state of a release with `downloaded` of `total` tracks in the
    /// Library, and no download job queued or running.
    #[must_use]
    pub(crate) const fn of_tracks(downloaded: usize, total: usize) -> Self {
        if downloaded == 0 || total == 0 {
            Self::NotDownloaded
        } else if downloaded >= total {
            Self::Downloaded
        } else {
            Self::Partial { downloaded, total }
        }
    }

    /// The word that names the state. It is the tooltip and the start of
    /// the accessibility label of the icon, so color is never the only
    /// signal.
    #[must_use]
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Self::NotDownloaded => "Not downloaded",
            Self::Queued => "Queued",
            Self::Downloading => "Downloading",
            Self::Partial { .. } => "Partly downloaded",
            Self::Downloaded => "Downloaded",
        }
    }

    /// The short count beside the icon, for example "3 of 9". `None`
    /// when the state carries no count.
    #[must_use]
    pub(crate) fn count_text(self) -> Option<String> {
        match self {
            Self::Partial { downloaded, total } => Some(format!("{downloaded} of {total}")),
            Self::NotDownloaded | Self::Queued | Self::Downloading | Self::Downloaded => None,
        }
    }
}

/// The download-state icon of a release row, ready to view: the typed
/// state, the count text, the tooltip and the accessibility label (ADR
/// 0083 Decision 6, amended 2026-10-22). The renderer only maps
/// [`Self::state`] to a named icon and a color token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseDownloadStateVm {
    pub(crate) state: ReleaseDownloadState,
    pub(crate) count_text: Option<String>,
    pub(crate) tooltip: &'static str,
    pub(crate) a11y_label: String,
}

impl ReleaseDownloadStateVm {
    /// The state of a release with `downloaded` of `total` tracks in the
    /// Library, and no download job queued or running.
    #[must_use]
    pub(crate) fn of_tracks(downloaded: usize, total: usize) -> Self {
        Self::of_state(ReleaseDownloadState::of_tracks(downloaded, total))
    }

    /// A download job of the release waits in the queue.
    #[must_use]
    pub(crate) fn queued() -> Self {
        Self::of_state(ReleaseDownloadState::Queued)
    }

    /// A download job of the release runs.
    #[must_use]
    pub(crate) fn downloading() -> Self {
        Self::of_state(ReleaseDownloadState::Downloading)
    }

    /// The binary state of a release row with no per-track count and no
    /// live queue fact, for example the feed page and the name page
    /// (ADR 0083 Decision 6, amended 2026-10-22, Flag: these pages hold
    /// only a release-level `in_library` fact, so they show
    /// [`ReleaseDownloadState::Downloaded`] or
    /// [`ReleaseDownloadState::NotDownloaded`], never a partial count nor
    /// a queued or downloading phase).
    #[must_use]
    pub(crate) fn in_library(in_library: bool) -> Self {
        if in_library {
            Self::of_state(ReleaseDownloadState::Downloaded)
        } else {
            Self::of_state(ReleaseDownloadState::NotDownloaded)
        }
    }

    fn of_state(state: ReleaseDownloadState) -> Self {
        let word = state.word();
        let count_text = state.count_text();
        let a11y_label = match &count_text {
            Some(count) => format!("{word}, {count}"),
            None => word.to_owned(),
        };
        Self {
            state,
            count_text,
            tooltip: word,
            a11y_label,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn of_tracks_gives_not_downloaded_for_zero_of_zero_or_zero_downloaded() {
        assert_eq!(
            ReleaseDownloadState::of_tracks(0, 0),
            ReleaseDownloadState::NotDownloaded
        );
        assert_eq!(
            ReleaseDownloadState::of_tracks(0, 7),
            ReleaseDownloadState::NotDownloaded
        );
    }

    #[test]
    fn of_tracks_gives_downloaded_when_each_track_is_downloaded() {
        assert_eq!(
            ReleaseDownloadState::of_tracks(7, 7),
            ReleaseDownloadState::Downloaded
        );
    }

    #[test]
    fn of_tracks_gives_partial_with_its_count_otherwise() {
        assert_eq!(
            ReleaseDownloadState::of_tracks(3, 7),
            ReleaseDownloadState::Partial {
                downloaded: 3,
                total: 7
            }
        );
    }

    #[test]
    fn only_partial_carries_a_count_text() {
        assert_eq!(
            ReleaseDownloadState::Partial {
                downloaded: 3,
                total: 7
            }
            .count_text(),
            Some("3 of 7".to_owned())
        );
        for state in [
            ReleaseDownloadState::NotDownloaded,
            ReleaseDownloadState::Queued,
            ReleaseDownloadState::Downloading,
            ReleaseDownloadState::Downloaded,
        ] {
            assert_eq!(state.count_text(), None, "{state:?}");
        }
    }

    #[test]
    fn vm_of_tracks_carries_the_word_and_the_count_in_its_a11y_label() {
        let vm = ReleaseDownloadStateVm::of_tracks(3, 7);
        assert_eq!(vm.tooltip, "Partly downloaded");
        assert_eq!(vm.count_text.as_deref(), Some("3 of 7"));
        assert_eq!(vm.a11y_label, "Partly downloaded, 3 of 7");
    }

    #[test]
    fn vm_queued_and_downloading_carry_no_count() {
        let queued = ReleaseDownloadStateVm::queued();
        assert_eq!(queued.state, ReleaseDownloadState::Queued);
        assert_eq!(queued.tooltip, "Queued");
        assert_eq!(queued.count_text, None);
        assert_eq!(queued.a11y_label, "Queued");

        let downloading = ReleaseDownloadStateVm::downloading();
        assert_eq!(downloading.state, ReleaseDownloadState::Downloading);
        assert_eq!(downloading.tooltip, "Downloading");
    }

    #[test]
    fn vm_in_library_gives_the_binary_state_with_no_count() {
        let in_library = ReleaseDownloadStateVm::in_library(true);
        assert_eq!(in_library.state, ReleaseDownloadState::Downloaded);
        assert_eq!(in_library.count_text, None);

        let not_in_library = ReleaseDownloadStateVm::in_library(false);
        assert_eq!(not_in_library.state, ReleaseDownloadState::NotDownloaded);
        assert_eq!(not_in_library.count_text, None);
    }
}
