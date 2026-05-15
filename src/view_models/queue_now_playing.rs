//! Queue and now-playing frame display contracts.
//!
//! ADR 0046 Phase 4 moves detailed playback controls out of the global toolbar
//! and into the Queue workspace frame. This module keeps those display
//! contracts GPUI-free so the shell can bind them to primitives without owning
//! playback state.

#![warn(clippy::pedantic)]

use crate::view_models::format::fmt_total_runtime_clock;
use crate::view_models::workspace::FrameChromeButtonDisplay;

/// Transport state for the queue frame.
///
/// The state intentionally stores only command availability facts. Playback
/// engines and session handles remain in the application layer that dispatches
/// commands.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum TransportState {
    /// No playable queue item is active.
    #[default]
    Stopped,
    /// Playback is currently advancing.
    Playing,
    /// Playback is active but paused.
    Paused,
}

impl TransportState {
    /// Returns whether transport commands should be enabled.
    #[must_use]
    pub(crate) const fn is_active(self) -> bool {
        !matches!(self, Self::Stopped)
    }

    /// Returns the visible play/pause command label.
    #[must_use]
    pub(crate) const fn play_pause_label(self) -> &'static str {
        match self {
            Self::Playing => "Pause",
            Self::Paused | Self::Stopped => "Play",
        }
    }

    /// Returns the play/pause command accessibility label.
    #[must_use]
    pub(crate) const fn play_pause_a11y_label(self) -> &'static str {
        match self {
            Self::Playing => "Pause playback",
            Self::Paused => "Resume playback",
            Self::Stopped => "Play",
        }
    }
}

/// Plain queue-track input projected by the application layer.
///
/// This input prevents DB rows from becoming part of the queue-frame VM public
/// surface. Callers extract only the facts the frame needs and the VM owns the
/// resulting display labels.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct QueueTrackInput {
    /// Stable local track identifier.
    pub(crate) id: i64,
    /// Primary track title.
    pub(crate) title: Option<String>,
    /// Secondary artist label.
    pub(crate) artist: Option<String>,
    /// Track duration in seconds.
    pub(crate) duration_seconds: Option<i64>,
    /// Whether this row represents the active playback item.
    pub(crate) now_playing: bool,
}

/// Display-ready queue row.
///
/// The shell renders rows without checking playback state or formatting
/// duration values. Unavailable or now-playing state is already named here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QueueRowDisplay {
    /// Stable row element identifier.
    pub(crate) id: String,
    /// Primary row title.
    pub(crate) title: String,
    /// Optional secondary artist label.
    pub(crate) artist: Option<String>,
    /// Optional clock-style duration label.
    pub(crate) duration_label: Option<String>,
    /// Whether this row is the active now-playing item.
    pub(crate) now_playing: bool,
    /// Accessibility label summarizing the row.
    pub(crate) a11y_label: String,
}

impl QueueRowDisplay {
    /// Projects a queue-track input into display-ready row data.
    #[must_use]
    pub(crate) fn from_track(input: QueueTrackInput) -> Self {
        let title = input
            .title
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "Untitled track".to_string());
        let duration_label = input.duration_seconds.and_then(fmt_total_runtime_clock);
        let a11y_label = queue_row_a11y_label(
            &title,
            input.artist.as_deref(),
            duration_label.as_deref(),
            input.now_playing,
        );

        Self {
            id: format!("queue-row-{}", input.id),
            title,
            artist: input.artist,
            duration_label,
            now_playing: input.now_playing,
            a11y_label,
        }
    }
}

/// Display-ready transport command state.
///
/// The queue shell maps these display facts to icon buttons. Command handlers
/// are supplied separately by the application shell so this VM stays pure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TransportDisplay {
    /// Stable play/pause control identifier.
    pub(crate) play_pause_id: String,
    /// Visible play/pause label.
    pub(crate) play_pause_label: &'static str,
    /// Accessibility label for the play/pause command.
    pub(crate) play_pause_a11y_label: &'static str,
    /// Current transport state.
    pub(crate) play_pause_state: TransportState,
    /// Previous-track command display.
    pub(crate) skip_previous: FrameChromeButtonDisplay,
    /// Next-track command display.
    pub(crate) skip_next: FrameChromeButtonDisplay,
    /// Whether transport commands should render unavailable.
    pub(crate) disabled: bool,
}

impl TransportDisplay {
    /// Creates a transport display with independent skip availability.
    #[must_use]
    pub(crate) fn from_state_with_skip_availability(
        state: TransportState,
        can_skip_previous: bool,
        can_skip_next: bool,
    ) -> Self {
        let disabled = !state.is_active();
        Self {
            play_pause_id: "queue-transport-playpause".to_string(),
            play_pause_label: state.play_pause_label(),
            play_pause_a11y_label: state.play_pause_a11y_label(),
            play_pause_state: state,
            skip_previous: FrameChromeButtonDisplay::new(
                "queue-transport-previous",
                "Previous track",
                disabled || !can_skip_previous,
            ),
            skip_next: FrameChromeButtonDisplay::new(
                "queue-transport-next",
                "Next track",
                disabled || !can_skip_next,
            ),
            disabled,
        }
    }
}

/// Display-ready liveValue output option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LiveValueDeviceOption {
    /// Stable option identifier.
    pub(crate) id: String,
    /// Visible output label.
    pub(crate) label: String,
    /// Accessibility label for the output option.
    pub(crate) a11y_label: String,
    /// Whether this option is informational instead of selectable.
    pub(crate) disabled: bool,
}

impl LiveValueDeviceOption {
    /// Creates a liveValue output option.
    #[must_use]
    pub(crate) fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        a11y_label: impl Into<String>,
        disabled: bool,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            a11y_label: a11y_label.into(),
            disabled,
        }
    }
}

/// Display-ready liveValue output picker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LiveValueDeviceDisplay {
    /// Stable picker identifier.
    pub(crate) picker_id: String,
    /// Ordered output options.
    pub(crate) options: Vec<LiveValueDeviceOption>,
    /// Currently selected output identifier.
    pub(crate) selected_id: Option<String>,
    /// Accessibility label for the picker.
    pub(crate) a11y_label: &'static str,
    /// Whether the picker should render as unavailable.
    pub(crate) disabled: bool,
}

impl LiveValueDeviceDisplay {
    /// Creates a liveValue output display.
    #[must_use]
    pub(crate) fn new(
        options: Vec<LiveValueDeviceOption>,
        selected_id: Option<String>,
        disabled: bool,
    ) -> Self {
        Self {
            picker_id: "queue-livevalue-output".to_string(),
            options,
            selected_id,
            a11y_label: "Choose liveValue output",
            disabled,
        }
    }

    /// Returns an informational display when output routing is unavailable.
    #[must_use]
    pub(crate) fn unavailable() -> Self {
        Self::new(
            vec![LiveValueDeviceOption::new(
                "livevalue-output-unavailable",
                "No liveValue output",
                "No liveValue output is available",
                true,
            )],
            None,
            true,
        )
    }

    /// Returns the selected output label for the picker trigger.
    #[must_use]
    pub(crate) fn selected_label(&self) -> &str {
        self.selected_id
            .as_deref()
            .and_then(|selected| {
                self.options
                    .iter()
                    .find(|option| option.id == selected)
                    .map(|option| option.label.as_str())
            })
            .unwrap_or("Output")
    }
}

/// Display-ready output volume state.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VolumeDisplay {
    /// Stable slider identifier.
    pub(crate) slider_id: String,
    /// Normalized volume level from `0.0` through `1.0`.
    pub(crate) level: f32,
    /// Accessibility label for the slider.
    pub(crate) a11y_label: &'static str,
    /// Whether the slider should render unavailable.
    pub(crate) disabled: bool,
}

impl VolumeDisplay {
    /// Creates a volume display with clamped level.
    #[must_use]
    pub(crate) fn new(level: f32, disabled: bool) -> Self {
        Self {
            slider_id: "queue-output-volume".to_string(),
            level: level.clamp(0.0, 1.0),
            a11y_label: "Output volume",
            disabled,
        }
    }
}

/// Display-ready Queue/Now Playing page.
///
/// The page groups the queue rows with transport and output controls. It
/// contains no GPUI values and no playback engine handles.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct QueueNowPlayingPageVm {
    /// Ordered queue rows.
    pub(crate) rows: Vec<QueueRowDisplay>,
    /// Transport control display state.
    pub(crate) transport: TransportDisplay,
    /// liveValue output picker display state.
    pub(crate) live_value: LiveValueDeviceDisplay,
    /// Output volume slider display state.
    pub(crate) volume: VolumeDisplay,
    /// Empty-state label for the queue list.
    pub(crate) empty_label: &'static str,
}

impl QueueNowPlayingPageVm {
    /// Creates a queue page builder.
    pub(crate) fn builder() -> QueueNowPlayingPageVmBuilder {
        QueueNowPlayingPageVmBuilder::default()
    }
}

/// Builder for [`QueueNowPlayingPageVm`].
#[derive(Clone, Debug)]
#[must_use]
pub(crate) struct QueueNowPlayingPageVmBuilder {
    tracks: Vec<QueueTrackInput>,
    transport_state: TransportState,
    can_skip_previous: bool,
    can_skip_next: bool,
    live_value: LiveValueDeviceDisplay,
    volume: VolumeDisplay,
}

impl Default for QueueNowPlayingPageVmBuilder {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            transport_state: TransportState::Stopped,
            can_skip_previous: false,
            can_skip_next: false,
            live_value: LiveValueDeviceDisplay::unavailable(),
            volume: VolumeDisplay::new(1.0, true),
        }
    }
}

impl QueueNowPlayingPageVmBuilder {
    /// Supplies ordered queue-track inputs.
    pub(crate) fn tracks(mut self, tracks: impl IntoIterator<Item = QueueTrackInput>) -> Self {
        self.tracks = tracks.into_iter().collect();
        self
    }

    /// Supplies current transport state.
    pub(crate) const fn transport_state(mut self, state: TransportState) -> Self {
        self.transport_state = state;
        self
    }

    /// Supplies skip command availability.
    pub(crate) const fn skip_availability(
        mut self,
        can_skip_previous: bool,
        can_skip_next: bool,
    ) -> Self {
        self.can_skip_previous = can_skip_previous;
        self.can_skip_next = can_skip_next;
        self
    }

    /// Supplies liveValue output display state.
    pub(crate) fn live_value(mut self, display: LiveValueDeviceDisplay) -> Self {
        self.live_value = display;
        self
    }

    /// Supplies output volume display state.
    pub(crate) fn volume(mut self, display: VolumeDisplay) -> Self {
        self.volume = display;
        self
    }

    /// Projects the builder input into a display-ready page.
    #[must_use]
    pub(crate) fn build(self) -> QueueNowPlayingPageVm {
        QueueNowPlayingPageVm {
            rows: self
                .tracks
                .into_iter()
                .map(QueueRowDisplay::from_track)
                .collect(),
            transport: TransportDisplay::from_state_with_skip_availability(
                self.transport_state,
                self.can_skip_previous,
                self.can_skip_next,
            ),
            live_value: self.live_value,
            volume: self.volume,
            empty_label: "Queue is empty",
        }
    }
}

fn queue_row_a11y_label(
    title: &str,
    artist: Option<&str>,
    duration: Option<&str>,
    now_playing: bool,
) -> String {
    let mut parts = Vec::new();
    if now_playing {
        parts.push("Now playing".to_string());
    }
    parts.push(title.to_string());
    if let Some(artist) = artist {
        if !artist.trim().is_empty() {
            parts.push(format!("by {artist}"));
        }
    }
    if let Some(duration) = duration {
        parts.push(duration.to_string());
    }
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: i64, title: &str, now_playing: bool) -> QueueTrackInput {
        QueueTrackInput {
            id,
            title: Some(title.to_string()),
            artist: Some("Artist".to_string()),
            duration_seconds: Some(245),
            now_playing,
        }
    }

    #[test]
    fn empty_queue_disables_transport_and_output_controls() {
        let vm = QueueNowPlayingPageVm::builder().build();

        assert!(vm.rows.is_empty());
        assert!(vm.transport.disabled);
        assert!(vm.transport.skip_previous.disabled);
        assert!(vm.transport.skip_next.disabled);
        assert!(vm.live_value.disabled);
        assert!(vm.volume.disabled);
        assert_eq!(vm.empty_label, "Queue is empty");
    }

    #[test]
    fn single_track_playing_marks_now_playing_row() {
        let vm = QueueNowPlayingPageVm::builder()
            .tracks([track(7, "Opening", true)])
            .transport_state(TransportState::Playing)
            .build();

        assert_eq!(vm.rows.len(), 1);
        assert_eq!(vm.rows[0].id, "queue-row-7");
        assert_eq!(vm.rows[0].title, "Opening");
        assert_eq!(vm.rows[0].duration_label.as_deref(), Some("4:05"));
        assert!(vm.rows[0].now_playing);
        assert_eq!(
            vm.rows[0].a11y_label,
            "Now playing, Opening, by Artist, 4:05"
        );
        assert_eq!(vm.transport.play_pause_state, TransportState::Playing);
        assert_eq!(vm.transport.play_pause_label, "Pause");
        assert!(!vm.transport.disabled);
        assert!(vm.transport.skip_previous.disabled);
        assert!(vm.transport.skip_next.disabled);
    }

    #[test]
    fn multi_track_paused_keeps_queue_enabled() {
        let vm = QueueNowPlayingPageVm::builder()
            .tracks([track(1, "First", true), track(2, "Second", false)])
            .transport_state(TransportState::Paused)
            .skip_availability(false, true)
            .build();

        assert_eq!(vm.rows.len(), 2);
        assert_eq!(vm.transport.play_pause_state, TransportState::Paused);
        assert_eq!(vm.transport.play_pause_a11y_label, "Resume playback");
        assert!(!vm.transport.disabled);
        assert!(vm.transport.skip_previous.disabled);
        assert!(!vm.transport.skip_next.disabled);
        assert!(vm.rows[0].now_playing);
        assert!(!vm.rows[1].now_playing);
    }

    #[test]
    fn no_device_picker_selection_uses_output_label() {
        let picker = LiveValueDeviceDisplay::unavailable();

        assert_eq!(picker.selected_label(), "Output");
        assert!(picker.selected_id.is_none());
        assert_eq!(picker.options.len(), 1);
        assert!(picker.options[0].disabled);
    }

    #[test]
    fn volume_level_is_clamped() {
        assert_eq!(VolumeDisplay::new(1.5, false).level, 1.0);
        assert_eq!(VolumeDisplay::new(-0.5, false).level, 0.0);
    }
}
