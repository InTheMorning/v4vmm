//! Live status strip display contract.
//!
//! ADR 0060 shows this compact glance surface above Music and Settings only
//! while a show is active. The projection is GPUI-free and derives from the
//! same `ShowPageVm` snapshot that renders the Show screen.

#![warn(clippy::pedantic)]

use crate::view_models::show::{ShowNowPlayingDisplay, ShowPageVm};

/// Aggregate live health state for the status strip.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum LiveStatusHealthState {
    /// No broadcast health source is available yet.
    #[default]
    Unknown,
    /// The known live path is healthy.
    Healthy,
    /// One or more live-path checks need operator attention.
    Degraded,
}

impl LiveStatusHealthState {
    const fn from_broadcast_state(active: bool, degraded: bool) -> Self {
        if !active {
            Self::Unknown
        } else if degraded {
            Self::Degraded
        } else {
            Self::Healthy
        }
    }

    const fn display(self) -> LiveStatusHealthDisplay {
        match self {
            Self::Unknown => LiveStatusHealthDisplay {
                state: self,
                label: "Health unknown",
                icon_role: LiveStatusHealthIconRole::Info,
            },
            Self::Healthy => LiveStatusHealthDisplay {
                state: self,
                label: "On air",
                icon_role: LiveStatusHealthIconRole::Success,
            },
            Self::Degraded => LiveStatusHealthDisplay {
                state: self,
                label: "Needs attention",
                icon_role: LiveStatusHealthIconRole::Warning,
            },
        }
    }
}

/// Icon role paired with the health label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LiveStatusHealthIconRole {
    Info,
    Success,
    Warning,
}

/// Display-ready aggregate health facts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LiveStatusHealthDisplay {
    pub(crate) state: LiveStatusHealthState,
    pub(crate) label: &'static str,
    pub(crate) icon_role: LiveStatusHealthIconRole,
}

/// Listener-facing now-playing facts for the live status strip.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LiveStatusNowPlayingDisplay {
    pub(crate) line: String,
    pub(crate) title: String,
    pub(crate) artist: Option<String>,
    pub(crate) state_label: &'static str,
    pub(crate) a11y_label: String,
}

impl LiveStatusNowPlayingDisplay {
    fn from_show(now_playing: &ShowNowPlayingDisplay, state_label: &'static str) -> Self {
        let line = now_playing.artist.as_ref().map_or_else(
            || now_playing.title.clone(),
            |artist| format!("{artist} - {}", now_playing.title),
        );
        Self {
            line,
            title: now_playing.title.clone(),
            artist: now_playing.artist.clone(),
            state_label,
            a11y_label: now_playing.a11y_label.clone(),
        }
    }
}

/// Recording state shown while a show recording is active.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LiveStatusRecordingDisplay {
    pub(crate) label: &'static str,
    pub(crate) elapsed_label: String,
    pub(crate) summary_label: String,
}

/// Display-ready command for opening the Show section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LiveStatusOpenShowActionDisplay {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
}

/// Source facts used by the shared live-status projector.
#[derive(Clone, Debug)]
pub(crate) struct LiveStatusProjection<'a> {
    show: &'a ShowPageVm,
    broadcast_active: bool,
    broadcast_degraded: bool,
    recording_elapsed_label: Option<String>,
}

impl<'a> LiveStatusProjection<'a> {
    /// Starts a live-status projection from the Show page snapshot.
    #[must_use]
    pub(crate) const fn from_show_page(show: &'a ShowPageVm) -> Self {
        Self {
            show,
            broadcast_active: false,
            broadcast_degraded: false,
            recording_elapsed_label: None,
        }
    }

    #[cfg(test)]
    const fn broadcast_active(mut self, active: bool) -> Self {
        self.broadcast_active = active;
        self
    }

    #[cfg(test)]
    const fn broadcast_degraded(mut self, degraded: bool) -> Self {
        self.broadcast_degraded = degraded;
        self
    }

    #[cfg(test)]
    fn recording_elapsed_label(mut self, elapsed_label: impl Into<String>) -> Self {
        self.recording_elapsed_label = Some(elapsed_label.into());
        self
    }
}

/// Display-ready live status strip.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LiveStatusDisplay {
    pub(crate) id: &'static str,
    pub(crate) active: bool,
    pub(crate) heading_label: &'static str,
    pub(crate) summary_label: String,
    pub(crate) now_playing: Option<LiveStatusNowPlayingDisplay>,
    pub(crate) health: LiveStatusHealthDisplay,
    pub(crate) recording: Option<LiveStatusRecordingDisplay>,
    pub(crate) open_show: LiveStatusOpenShowActionDisplay,
}

impl LiveStatusDisplay {
    /// Projects the live strip from the same Show page snapshot used by Show.
    #[must_use]
    pub(crate) fn from_show_page(show: &ShowPageVm) -> Self {
        Self::from_projection(LiveStatusProjection::from_show_page(show))
    }

    /// Projects display state from show playback plus future broadcast facts.
    #[must_use]
    pub(crate) fn from_projection(projection: LiveStatusProjection<'_>) -> Self {
        let playback_active = projection.show.is_active();
        let recording_active = projection.recording_elapsed_label.is_some();
        let active = playback_active || projection.broadcast_active || recording_active;
        let health = LiveStatusHealthState::from_broadcast_state(
            projection.broadcast_active,
            projection.broadcast_degraded,
        )
        .display();
        let now_playing = projection.show.now_playing.as_ref().map(|now_playing| {
            LiveStatusNowPlayingDisplay::from_show(now_playing, projection.show.state_label)
        });
        let summary_label = now_playing.as_ref().map_or_else(
            || {
                if active {
                    "Show active".to_string()
                } else {
                    "No active show".to_string()
                }
            },
            |now_playing| now_playing.line.clone(),
        );
        Self {
            id: "live-status-strip",
            active,
            heading_label: "Live status",
            summary_label,
            now_playing,
            health,
            recording: projection.recording_elapsed_label.map(|elapsed_label| {
                let label = "Recording";
                let summary_label = format!("{label} {elapsed_label}");
                LiveStatusRecordingDisplay {
                    label,
                    elapsed_label,
                    summary_label,
                }
            }),
            open_show: LiveStatusOpenShowActionDisplay {
                id: "live-status-open-show",
                label: "Open Show",
                a11y_label: "Open Show",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::view_models::queue_now_playing::{
        QueueNowPlayingPageVm, QueueTrackInput, TransportState,
    };

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
    fn live_status_no_show_is_absent() {
        let show = ShowPageVm::idle();
        let display = LiveStatusDisplay::from_show_page(&show);

        assert!(!display.active);
        assert_eq!(display.summary_label, "No active show");
        assert!(display.now_playing.is_none());
        assert_eq!(display.health.state, LiveStatusHealthState::Unknown);
        assert_eq!(display.recording, None);
        assert_eq!(display.open_show.label, "Open Show");
    }

    #[test]
    fn live_status_playing_show_projects_now_playing_line() {
        let show = ShowPageVm::from_queue(
            QueueNowPlayingPageVm::builder()
                .tracks([track(7, "Opening", true), track(8, "Next", false)])
                .transport_state(TransportState::Playing)
                .build(),
        );
        let display = LiveStatusDisplay::from_show_page(&show);

        assert!(display.active);
        assert_eq!(display.summary_label, "Artist - Opening");
        assert_eq!(
            display.now_playing,
            Some(LiveStatusNowPlayingDisplay {
                line: "Artist - Opening".to_string(),
                title: "Opening".to_string(),
                artist: Some("Artist".to_string()),
                state_label: "Playing",
                a11y_label: "Now playing, Opening, by Artist, 2:05".to_string(),
            })
        );
        assert_eq!(display.health.label, "Health unknown");
        assert_eq!(display.health.icon_role, LiveStatusHealthIconRole::Info);
    }

    #[test]
    fn live_status_recording_projects_elapsed_label() {
        let show = ShowPageVm::idle();
        let display = LiveStatusDisplay::from_projection(
            LiveStatusProjection::from_show_page(&show)
                .broadcast_active(true)
                .recording_elapsed_label("00:42"),
        );

        assert!(display.active);
        assert_eq!(display.summary_label, "Show active");
        assert_eq!(display.health.state, LiveStatusHealthState::Healthy);
        assert_eq!(
            display.recording,
            Some(LiveStatusRecordingDisplay {
                label: "Recording",
                elapsed_label: "00:42".to_string(),
                summary_label: "Recording 00:42".to_string(),
            })
        );
    }

    #[test]
    fn live_status_degraded_health_pairs_text_with_warning_icon_role() {
        let show = ShowPageVm::idle();
        let display = LiveStatusDisplay::from_projection(
            LiveStatusProjection::from_show_page(&show)
                .broadcast_active(true)
                .broadcast_degraded(true),
        );

        assert!(display.active);
        assert_eq!(display.health.state, LiveStatusHealthState::Degraded);
        assert_eq!(display.health.label, "Needs attention");
        assert_eq!(display.health.icon_role, LiveStatusHealthIconRole::Warning);
    }
}
