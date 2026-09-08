//! Show surface display contract.
//!
//! ADR 0060 mounts `Show` beside the curation and settings sections. This
//! module wraps the existing Queue/Now Playing projection with page-level
//! show state while keeping renderer and playback handles out of the view
//! model layer.

#![warn(clippy::pedantic)]

use crate::view_models::queue_now_playing::{
    QueueNowPlayingPageVm, QueueRowDisplay, TransportState,
};

/// Display-ready empty state for an idle show surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ShowEmptyStateDisplay {
    /// Stable element identifier for the empty state.
    pub(crate) id: &'static str,
    /// Primary empty-state label.
    pub(crate) title: &'static str,
    /// Secondary empty-state label.
    pub(crate) subtitle: &'static str,
}

/// Display-ready now-playing summary for the show header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ShowNowPlayingDisplay {
    /// Primary listener-facing track title.
    pub(crate) title: String,
    /// Optional listener-facing artist label.
    pub(crate) artist: Option<String>,
    /// Optional duration label projected by the queue view model.
    pub(crate) duration_label: Option<String>,
    /// Accessibility label summarizing the now-playing track.
    pub(crate) a11y_label: String,
}

/// Display-ready state for the `Show` screen mount.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShowPageVm {
    /// Stable page title.
    pub(crate) title: &'static str,
    /// Current show playback state label.
    pub(crate) state_label: &'static str,
    /// Now-playing summary, present only while show playback is active.
    pub(crate) now_playing: Option<ShowNowPlayingDisplay>,
    /// Empty state shown when there is no active show playback.
    pub(crate) empty_state: Option<ShowEmptyStateDisplay>,
    /// Queue and transport display projected by the existing queue VM.
    pub(crate) queue: QueueNowPlayingPageVm,
}

impl ShowPageVm {
    /// Creates an idle Show page with no active playback.
    #[must_use]
    pub(crate) fn idle() -> Self {
        Self::from_queue(QueueNowPlayingPageVm::builder().build())
    }

    /// Projects the Show page from the existing queue display contract.
    #[must_use]
    pub(crate) fn from_queue(queue: QueueNowPlayingPageVm) -> Self {
        let state_label = transport_state_label(queue.transport.play_pause_state);
        let now_playing = queue
            .rows
            .iter()
            .find(|row| row.now_playing)
            .map(ShowNowPlayingDisplay::from_queue_row);
        let active = queue.transport.play_pause_state.is_active();
        Self {
            title: "Show",
            state_label,
            now_playing,
            empty_state: (!active).then_some(ShowEmptyStateDisplay {
                id: "show-empty-state",
                title: "No active show",
                subtitle: "Show playback is idle.",
            }),
            queue,
        }
    }

    /// Returns whether the page represents active show playback.
    #[must_use]
    pub(crate) const fn is_active(&self) -> bool {
        self.empty_state.is_none()
    }
}

impl ShowNowPlayingDisplay {
    fn from_queue_row(row: &QueueRowDisplay) -> Self {
        Self {
            title: row.title.clone(),
            artist: row.artist.clone(),
            duration_label: row.duration_label.clone(),
            a11y_label: row.a11y_label.clone(),
        }
    }
}

const fn transport_state_label(state: TransportState) -> &'static str {
    match state {
        TransportState::Stopped => "Idle",
        TransportState::Playing => "Playing",
        TransportState::Paused => "Paused",
    }
}

#[cfg(test)]
mod tests {
    use crate::view_models::queue_now_playing::{QueueTrackInput, TransportState};

    use super::*;

    fn track(id: i64, title: &str, now_playing: bool) -> QueueTrackInput {
        QueueTrackInput {
            id,
            title: Some(title.to_string()),
            artist: Some("Artist".to_string()),
            duration_seconds: Some(125),
            now_playing,
        }
    }

    #[test]
    fn stopped_queue_projects_idle_empty_state() {
        let vm = ShowPageVm::idle();

        assert!(!vm.is_active());
        assert_eq!(vm.title, "Show");
        assert_eq!(vm.state_label, "Idle");
        assert_eq!(
            vm.empty_state,
            Some(ShowEmptyStateDisplay {
                id: "show-empty-state",
                title: "No active show",
                subtitle: "Show playback is idle.",
            })
        );
        assert!(vm.now_playing.is_none());
        assert!(vm.queue.rows.is_empty());
    }

    #[test]
    fn playing_queue_projects_now_playing_summary() {
        let vm = ShowPageVm::from_queue(
            QueueNowPlayingPageVm::builder()
                .tracks([track(7, "Opening", true), track(8, "Next", false)])
                .transport_state(TransportState::Playing)
                .build(),
        );

        assert!(vm.is_active());
        assert_eq!(vm.state_label, "Playing");
        assert!(vm.empty_state.is_none());
        assert_eq!(
            vm.now_playing,
            Some(ShowNowPlayingDisplay {
                title: "Opening".to_string(),
                artist: Some("Artist".to_string()),
                duration_label: Some("2:05".to_string()),
                a11y_label: "Now playing, Opening, by Artist, 2:05".to_string(),
            })
        );
        assert_eq!(vm.queue.rows.len(), 2);
    }

    #[test]
    fn paused_queue_remains_active() {
        let vm = ShowPageVm::from_queue(
            QueueNowPlayingPageVm::builder()
                .tracks([track(7, "Opening", true)])
                .transport_state(TransportState::Paused)
                .build(),
        );

        assert!(vm.is_active());
        assert_eq!(vm.state_label, "Paused");
        assert!(vm.empty_state.is_none());
    }
}
