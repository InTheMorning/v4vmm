//! Show surface display contract.
//!
//! ADR 0060 mounts `Show` beside the curation and settings sections. This
//! module wraps the existing Queue/Now Playing projection with page-level
//! show state while keeping renderer and playback handles out of the view
//! model layer.

#![warn(clippy::pedantic)]

use crate::broadcast::{
    control::ServiceState,
    encoder::{AudioSignalState, EncoderState, ListenerCount, RecordingState},
};
use crate::runtime::{broadcast_service_watch, BroadcastReadinessSnapshot};
use crate::view_models::queue_now_playing::{
    QueueNowPlayingPageVm, QueueRowDisplay, TransportState,
};

pub(crate) const PUBLISHER_LOG_LINE_COUNT: usize = 50;

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

/// Display-ready Source section for the `Show` screen mount.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceSectionDisplay {
    /// Stable section title.
    pub(crate) title: &'static str,
    /// Selected host summary.
    pub(crate) summary: String,
    /// Curator-facing host name.
    pub(crate) host_name: String,
    /// Host reachability display.
    pub(crate) reachability: SourceReachabilityDisplay,
    /// Local library payment-route readiness display.
    pub(crate) readiness: Option<SourceReadinessDisplay>,
}

/// Display-ready host reachability state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SourceReachabilityDisplay {
    /// Stable reachability kind.
    pub(crate) state: SourceReachabilityState,
    /// Curator-facing label.
    pub(crate) label: &'static str,
    /// Curator-facing detail.
    pub(crate) detail: &'static str,
}

/// Stable host reachability state for the Source section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceReachabilityState {
    /// The selected host accepted at least one broadcast service read.
    Reachable,
    /// The selected host did not answer the SSH transport.
    NotReachable,
    /// No service read has established host reachability yet.
    Unknown,
}

/// Display-ready local library readiness state for the Source section.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceReadinessDisplay {
    /// Stable row identifier.
    pub(crate) id: &'static str,
    /// Curator-facing count label.
    pub(crate) count_label: String,
    /// Curator-facing detail.
    pub(crate) detail: String,
    /// Stable readiness state.
    pub(crate) state: SourceReadinessState,
    /// Action that opens the Music readiness list.
    pub(crate) action: SourceReadinessActionDisplay,
}

/// Stable local library readiness state for the Source section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceReadinessState {
    /// Readiness scan is pending.
    Checking,
    /// No local library tracks are present.
    Empty,
    /// Every scanned local library track is ready.
    Ready,
    /// Some scanned tracks need routes or files.
    NeedsAttention,
    /// The readiness scan failed.
    Failed,
}

/// Typed availability for the Source readiness action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceReadinessActionAvailability {
    /// The action can be run.
    Available,
    /// The action is visible but unavailable in this state.
    Unavailable,
}

impl SourceReadinessActionAvailability {
    #[must_use]
    pub(crate) const fn disabled(self) -> bool {
        matches!(self, Self::Unavailable)
    }
}

/// Display-ready action state for opening readiness problems in Music.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceReadinessActionDisplay {
    /// Stable element identifier.
    pub(crate) id: &'static str,
    /// Visible action label.
    pub(crate) label: &'static str,
    /// Accessibility label for the action.
    pub(crate) a11y_label: String,
    /// Typed action availability.
    pub(crate) availability: SourceReadinessActionAvailability,
}

impl SourceReadinessActionDisplay {
    #[must_use]
    pub(crate) const fn disabled(&self) -> bool {
        self.availability.disabled()
    }
}

impl SourceReachabilityState {
    const fn display(self) -> SourceReachabilityDisplay {
        match self {
            Self::Reachable => SourceReachabilityDisplay {
                state: self,
                label: "Reachable",
                detail: "Host accepts broadcast control.",
            },
            Self::NotReachable => SourceReachabilityDisplay {
                state: self,
                label: "Not reachable",
                detail: "Host cannot be reached.",
            },
            Self::Unknown => SourceReachabilityDisplay {
                state: self,
                label: "Unknown",
                detail: "Host reachability has not been checked.",
            },
        }
    }
}

/// Role of one publisher-side service in the show surface.
pub(crate) type PublisherServiceRole = broadcast_service_watch::BroadcastServiceRole;

/// Display state for one publisher-side service.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PublisherServiceStateDisplay {
    /// The service is installed and running.
    Active,
    /// The service is installed and stopped.
    Inactive,
    /// The service is failed and needs reset before start.
    Failed {
        /// Failure reason from the service manager result.
        reason: String,
    },
    /// The unit file is absent.
    NotInstalled,
    /// The host that owns the unit cannot be reached.
    NotReachable,
    /// The service state is not classified by this surface.
    Unknown,
}

impl PublisherServiceStateDisplay {
    #[cfg(test)]
    const ALL_KINDS: [PublisherServiceStateKind; 6] = [
        PublisherServiceStateKind::Active,
        PublisherServiceStateKind::Inactive,
        PublisherServiceStateKind::Failed,
        PublisherServiceStateKind::NotInstalled,
        PublisherServiceStateKind::NotReachable,
        PublisherServiceStateKind::Unknown,
    ];

    fn from_service_state(state: &ServiceState) -> Self {
        match state {
            ServiceState::Active => Self::Active,
            ServiceState::Inactive => Self::Inactive,
            ServiceState::Failed { reason } => Self::Failed {
                reason: reason.clone(),
            },
            ServiceState::NotInstalled => Self::NotInstalled,
            ServiceState::NotReachable => Self::NotReachable,
            ServiceState::Unknown => Self::Unknown,
        }
    }

    #[must_use]
    pub(crate) const fn kind(&self) -> PublisherServiceStateKind {
        match self {
            Self::Active => PublisherServiceStateKind::Active,
            Self::Inactive => PublisherServiceStateKind::Inactive,
            Self::Failed { .. } => PublisherServiceStateKind::Failed,
            Self::NotInstalled => PublisherServiceStateKind::NotInstalled,
            Self::NotReachable => PublisherServiceStateKind::NotReachable,
            Self::Unknown => PublisherServiceStateKind::Unknown,
        }
    }

    #[must_use]
    pub(crate) const fn label(&self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Inactive => "Inactive",
            Self::Failed { .. } => "Failed",
            Self::NotInstalled => "Not installed",
            Self::NotReachable => "Not reachable",
            Self::Unknown => "Unknown",
        }
    }

    #[must_use]
    pub(crate) fn detail(&self) -> Option<String> {
        match self {
            Self::Failed { reason } => Some(format!("Reason: {reason}")),
            Self::NotInstalled => Some("Unit file not found.".to_owned()),
            Self::NotReachable => Some("Host cannot be reached.".to_owned()),
            Self::Unknown => Some("State is not classified.".to_owned()),
            Self::Active | Self::Inactive => None,
        }
    }
}

/// Stable kind set for publisher service states.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublisherServiceStateKind {
    /// The service is installed and running.
    Active,
    /// The service is installed and stopped.
    Inactive,
    /// The service is failed and needs reset before start.
    Failed,
    /// The unit file is absent.
    NotInstalled,
    /// The host that owns the unit cannot be reached.
    NotReachable,
    /// The service state is not classified by this surface.
    Unknown,
}

/// Typed availability for publisher service actions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublisherActionAvailability {
    /// The action can be run.
    Available,
    /// The action is visible but unavailable in this state.
    Unavailable,
}

impl PublisherActionAvailability {
    #[must_use]
    pub(crate) const fn disabled(self) -> bool {
        matches!(self, Self::Unavailable)
    }
}

/// Display-ready action state for publisher service controls.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublisherActionDisplay {
    /// Stable element identifier.
    pub(crate) id: String,
    /// Visible action label.
    pub(crate) label: &'static str,
    /// Accessibility label for the action.
    pub(crate) a11y_label: String,
    /// Typed action availability.
    pub(crate) availability: PublisherActionAvailability,
}

impl PublisherActionDisplay {
    #[must_use]
    pub(crate) fn disabled(&self) -> bool {
        self.availability.disabled()
    }
}

/// Display-ready publisher service actions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublisherServiceActionsDisplay {
    /// Start action.
    pub(crate) start: PublisherActionDisplay,
    /// Stop action.
    pub(crate) stop: PublisherActionDisplay,
    /// Reset failed unit action.
    pub(crate) reset: PublisherActionDisplay,
}

/// Display-ready log action for one service.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublisherLogsDisplay {
    /// Complete service unit name.
    pub(crate) unit_name: String,
    /// Number of journal lines requested.
    pub(crate) line_count: usize,
    /// Whether this unit's log panel is open.
    pub(crate) open: bool,
    /// Action that opens the log panel.
    pub(crate) action: PublisherActionDisplay,
}

/// Display-ready service row for the Publisher section.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublisherServiceDisplay {
    /// Role this unit plays in the broadcast chain.
    pub(crate) role: PublisherServiceRole,
    /// Stable row identifier.
    pub(crate) id: String,
    /// Curator-facing service label.
    pub(crate) label: &'static str,
    /// Complete service unit name.
    pub(crate) unit_name: String,
    /// Service state display.
    pub(crate) state: PublisherServiceStateDisplay,
    /// Start, stop, and reset actions.
    pub(crate) actions: PublisherServiceActionsDisplay,
    /// Log action display.
    pub(crate) logs: PublisherLogsDisplay,
}

/// Log panel state carried by the Show view model.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) enum PublisherLogPanelState {
    /// The log panel is closed.
    #[default]
    Closed,
    /// The log panel is open for one service unit.
    Open {
        /// Service role whose journal text is shown.
        role: PublisherServiceRole,
        /// Complete service unit name.
        unit_name: String,
        /// Number of journal lines requested.
        line_count: usize,
        /// Journal text rendered by the shell as plain text.
        text: String,
    },
}

impl PublisherLogPanelState {
    /// Create a closed log-panel state.
    #[must_use]
    pub(crate) const fn closed() -> Self {
        Self::Closed
    }

    /// Create an open log-panel state.
    #[must_use]
    pub(crate) fn open(
        role: PublisherServiceRole,
        unit_name: impl Into<String>,
        line_count: usize,
        text: impl Into<String>,
    ) -> Self {
        Self::Open {
            role,
            unit_name: unit_name.into(),
            line_count,
            text: text.into(),
        }
    }

    #[must_use]
    pub(crate) const fn is_open(&self) -> bool {
        matches!(self, Self::Open { .. })
    }

    fn is_open_for(&self, role: PublisherServiceRole) -> bool {
        matches!(self, Self::Open { role: open_role, .. } if *open_role == role)
    }
}

/// Display-ready Publisher section for the `Show` screen mount.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublisherSectionDisplay {
    /// Stable section title.
    pub(crate) title: &'static str,
    /// Summary label for observed services.
    pub(crate) summary: String,
    /// Observed service rows, in fixed section order.
    pub(crate) services: Vec<PublisherServiceDisplay>,
    /// Current log-panel state.
    pub(crate) log_panel: PublisherLogPanelState,
    /// Close action for the log panel.
    pub(crate) close_logs: PublisherActionDisplay,
}

/// Display-ready Stream section for the `Show` screen mount.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StreamSectionDisplay {
    /// Stable section title.
    pub(crate) title: &'static str,
    /// Summary label for the encoder.
    pub(crate) summary: String,
    /// Configured server label.
    pub(crate) server_label: String,
    /// Stream connection state.
    pub(crate) connection: StreamConnectionDisplay,
    /// Audio-signal state.
    pub(crate) signal: StreamSignalDisplay,
    /// Recording state.
    pub(crate) recording: StreamRecordingDisplay,
    /// Listener-count display.
    pub(crate) listeners: StreamListenersDisplay,
    /// Optional encoder song title cross-check.
    pub(crate) encoder_song: Option<String>,
    /// Optional stream elapsed timer.
    pub(crate) stream_elapsed_label: Option<String>,
    /// Connect and disconnect actions, absent when no command should be shown.
    pub(crate) actions: Option<StreamActionsDisplay>,
}

/// Display-ready stream connection state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StreamConnectionDisplay {
    /// Stable connection state.
    pub(crate) state: StreamConnectionState,
    /// Curator-facing state label.
    pub(crate) label: &'static str,
    /// Curator-facing state detail.
    pub(crate) detail: &'static str,
    /// Icon role paired with the state label.
    pub(crate) icon_role: StreamStateIconRole,
}

/// Stable stream connection states.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StreamConnectionState {
    /// The encoder is connected to a stream server.
    Connected,
    /// The encoder is connecting to a stream server.
    Connecting,
    /// The encoder is running and disconnected.
    Disconnected,
    /// No encoder binary or configured target is available.
    NotInstalled,
    /// The addressed encoder instance did not answer.
    NotReachable,
    /// The connection state was not classified.
    Unknown,
}

impl StreamConnectionState {
    const fn display(self) -> StreamConnectionDisplay {
        match self {
            Self::Connected => StreamConnectionDisplay {
                state: self,
                label: "Connected",
                detail: "Encoder is feeding the stream.",
                icon_role: StreamStateIconRole::Success,
            },
            Self::Connecting => StreamConnectionDisplay {
                state: self,
                label: "Connecting",
                detail: "Encoder is connecting.",
                icon_role: StreamStateIconRole::Warning,
            },
            Self::Disconnected => StreamConnectionDisplay {
                state: self,
                label: "Disconnected",
                detail: "Encoder is not feeding the stream.",
                icon_role: StreamStateIconRole::Info,
            },
            Self::NotInstalled => StreamConnectionDisplay {
                state: self,
                label: "Not installed",
                detail: "Encoder control is not available.",
                icon_role: StreamStateIconRole::Warning,
            },
            Self::NotReachable => StreamConnectionDisplay {
                state: self,
                label: "Not reachable",
                detail: "Encoder control did not answer.",
                icon_role: StreamStateIconRole::Warning,
            },
            Self::Unknown => StreamConnectionDisplay {
                state: self,
                label: "Unknown",
                detail: "Encoder status is not classified.",
                icon_role: StreamStateIconRole::Info,
            },
        }
    }
}

/// Display-ready stream audio-signal state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StreamSignalDisplay {
    /// Stable signal state.
    pub(crate) state: StreamSignalState,
    /// Curator-facing state label.
    pub(crate) label: &'static str,
    /// Curator-facing state detail.
    pub(crate) detail: &'static str,
    /// Icon role paired with the state label.
    pub(crate) icon_role: StreamStateIconRole,
}

/// Stable stream audio-signal states.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StreamSignalState {
    /// Audio signal is present.
    Present,
    /// Audio signal is absent.
    Absent,
    /// Audio-signal state was not classified.
    Unknown,
}

impl StreamSignalState {
    const fn display(self) -> StreamSignalDisplay {
        match self {
            Self::Present => StreamSignalDisplay {
                state: self,
                label: "Audio present",
                detail: "Input signal is reaching the encoder.",
                icon_role: StreamStateIconRole::Success,
            },
            Self::Absent => StreamSignalDisplay {
                state: self,
                label: "No audio signal",
                detail: "Connected stream would carry silence.",
                icon_role: StreamStateIconRole::Warning,
            },
            Self::Unknown => StreamSignalDisplay {
                state: self,
                label: "Signal unknown",
                detail: "Encoder did not report input signal.",
                icon_role: StreamStateIconRole::Info,
            },
        }
    }
}

/// Display-ready stream recording state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StreamRecordingDisplay {
    /// Stable recording state.
    pub(crate) state: StreamRecordingState,
    /// Curator-facing state label.
    pub(crate) label: &'static str,
    /// Curator-facing state detail.
    pub(crate) detail: &'static str,
    /// Optional elapsed recording timer.
    pub(crate) elapsed_label: Option<String>,
    /// Optional recording file path.
    pub(crate) path: Option<String>,
    /// Icon role paired with the state label.
    pub(crate) icon_role: StreamStateIconRole,
}

/// Stable stream recording states.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StreamRecordingState {
    /// The encoder is recording.
    Recording,
    /// The encoder is not recording.
    Stopped,
    /// Recording state was not classified.
    Unknown,
}

/// Display-ready stream listener count.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StreamListenersDisplay {
    /// Curator-facing label.
    pub(crate) label: String,
    /// Curator-facing detail.
    pub(crate) detail: &'static str,
}

/// Stream state icon roles used by shells.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StreamStateIconRole {
    /// Informational state.
    Info,
    /// Healthy state.
    Success,
    /// State that needs attention.
    Warning,
}

/// Typed availability for stream actions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StreamActionAvailability {
    /// The action can be run.
    Available,
    /// The action is visible but unavailable in this state.
    Unavailable,
}

impl StreamActionAvailability {
    #[must_use]
    pub(crate) const fn disabled(self) -> bool {
        matches!(self, Self::Unavailable)
    }
}

/// Display-ready action state for stream controls.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StreamActionDisplay {
    /// Stable element identifier.
    pub(crate) id: &'static str,
    /// Visible action label.
    pub(crate) label: &'static str,
    /// Accessibility label for the action.
    pub(crate) a11y_label: String,
    /// Typed action availability.
    pub(crate) availability: StreamActionAvailability,
}

impl StreamActionDisplay {
    #[must_use]
    pub(crate) const fn disabled(&self) -> bool {
        self.availability.disabled()
    }
}

/// Display-ready stream actions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StreamActionsDisplay {
    /// Connect action.
    pub(crate) connect: StreamActionDisplay,
    /// Disconnect action.
    pub(crate) disconnect: StreamActionDisplay,
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
    /// Optional Source section; absent sections render nothing.
    pub(crate) source: Option<SourceSectionDisplay>,
    /// Optional Publisher section; absent sections render nothing.
    pub(crate) publisher: Option<PublisherSectionDisplay>,
    /// Optional Stream section; absent sections render nothing.
    pub(crate) stream: Option<StreamSectionDisplay>,
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
        Self::from_queue_and_publisher(queue, None, PublisherLogPanelState::closed())
    }

    /// Projects the Show page from queue display and publisher service state.
    #[must_use]
    pub(crate) fn from_queue_and_publisher(
        queue: QueueNowPlayingPageVm,
        publisher_snapshot: Option<&broadcast_service_watch::BroadcastServiceWatchSnapshot>,
        log_panel: PublisherLogPanelState,
    ) -> Self {
        Self::from_queue_publisher_and_readiness(queue, publisher_snapshot, log_panel, None)
    }

    /// Projects the Show page from queue, publisher state, and readiness state.
    #[must_use]
    pub(crate) fn from_queue_publisher_and_readiness(
        queue: QueueNowPlayingPageVm,
        publisher_snapshot: Option<&broadcast_service_watch::BroadcastServiceWatchSnapshot>,
        log_panel: PublisherLogPanelState,
        readiness_snapshot: Option<&BroadcastReadinessSnapshot>,
    ) -> Self {
        let state_label = transport_state_label(queue.transport.play_pause_state);
        let now_playing = queue
            .rows
            .iter()
            .find(|row| row.now_playing)
            .map(ShowNowPlayingDisplay::from_queue_row);
        let active = queue.transport.play_pause_state.is_active();
        let source = publisher_snapshot
            .and_then(|snapshot| SourceSectionDisplay::from_snapshot(snapshot, readiness_snapshot));
        let publisher = match publisher_snapshot {
            Some(snapshot) => PublisherSectionDisplay::from_snapshot(snapshot, log_panel),
            None => None,
        };
        let stream = publisher_snapshot.map(StreamSectionDisplay::from_snapshot);
        Self {
            title: "Show",
            state_label,
            now_playing,
            empty_state: (!active).then_some(ShowEmptyStateDisplay {
                id: "show-empty-state",
                title: "No active show",
                subtitle: "Show playback is idle.",
            }),
            source,
            publisher,
            stream,
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

impl SourceSectionDisplay {
    fn from_snapshot(
        snapshot: &broadcast_service_watch::BroadcastServiceWatchSnapshot,
        readiness_snapshot: Option<&BroadcastReadinessSnapshot>,
    ) -> Option<Self> {
        let host_name = snapshot.units.first()?.host_name.clone();
        let reachability = source_reachability(snapshot.units.as_slice()).display();
        let summary = format!("{host_name} - {}", reachability.label);
        Some(Self {
            title: "Source",
            summary,
            host_name,
            reachability,
            readiness: readiness_snapshot.map(SourceReadinessDisplay::from_snapshot),
        })
    }
}

impl SourceReadinessDisplay {
    const ID: &'static str = "source-readiness-row";
    const ACTION_ID: &'static str = "source-readiness-open";

    fn from_snapshot(snapshot: &BroadcastReadinessSnapshot) -> Self {
        if let Some(error) = snapshot.error.as_ref() {
            return Self {
                id: Self::ID,
                count_label: "Readiness unavailable".to_owned(),
                detail: error.clone(),
                state: SourceReadinessState::Failed,
                action: Self::action(SourceReadinessActionAvailability::Unavailable),
            };
        }
        let Some(report) = snapshot.report.as_ref() else {
            return Self::checking();
        };

        let total =
            report.summary.ready + report.summary.no_route_tag + report.summary.file_missing;
        if total == 0 {
            return Self {
                id: Self::ID,
                count_label: "No local tracks".to_owned(),
                detail: "Library has no downloaded tracks to check.".to_owned(),
                state: SourceReadinessState::Empty,
                action: Self::action(SourceReadinessActionAvailability::Unavailable),
            };
        }

        let problem_count = report.problem_count();
        if problem_count == 0 {
            return Self {
                id: Self::ID,
                count_label: format!("{} ready", track_count_label(report.summary.ready)),
                detail: "Every scanned track carries payment routes.".to_owned(),
                state: SourceReadinessState::Ready,
                action: Self::action(SourceReadinessActionAvailability::Unavailable),
            };
        }

        Self {
            id: Self::ID,
            count_label: format!("{} not ready", track_count_label(problem_count)),
            detail: format!(
                "{} missing routes, {} missing files.",
                report.summary.no_route_tag, report.summary.file_missing
            ),
            state: SourceReadinessState::NeedsAttention,
            action: Self::action(SourceReadinessActionAvailability::Available),
        }
    }

    fn checking() -> Self {
        Self {
            id: Self::ID,
            count_label: "Checking readiness".to_owned(),
            detail: "Reading local library payment-route tags.".to_owned(),
            state: SourceReadinessState::Checking,
            action: Self::action(SourceReadinessActionAvailability::Unavailable),
        }
    }

    fn action(availability: SourceReadinessActionAvailability) -> SourceReadinessActionDisplay {
        SourceReadinessActionDisplay {
            id: Self::ACTION_ID,
            label: "Open",
            a11y_label: "Open broadcast readiness issues in Music".to_owned(),
            availability,
        }
    }
}

impl PublisherSectionDisplay {
    fn from_snapshot(
        snapshot: &broadcast_service_watch::BroadcastServiceWatchSnapshot,
        log_panel: PublisherLogPanelState,
    ) -> Option<Self> {
        if snapshot.units.is_empty() {
            return None;
        }
        let services: Vec<_> = snapshot
            .units
            .iter()
            .map(|unit| PublisherServiceDisplay::from_snapshot(unit, &log_panel))
            .collect();
        let summary = service_summary(&services);

        Some(Self {
            title: "Publisher",
            summary,
            services,
            log_panel,
            close_logs: PublisherActionDisplay {
                id: "publisher-close-logs".to_owned(),
                label: "Close",
                a11y_label: "Close publisher logs".to_owned(),
                availability: PublisherActionAvailability::Available,
            },
        })
    }
}

impl PublisherServiceDisplay {
    fn from_snapshot(
        snapshot: &broadcast_service_watch::BroadcastServiceUnitSnapshot,
        log_panel: &PublisherLogPanelState,
    ) -> Self {
        let state = PublisherServiceStateDisplay::from_service_state(&snapshot.state);
        let label = service_label(snapshot.role);
        let id_seed = service_id_seed(snapshot.role);
        let unit_name = snapshot.unit_name.clone();
        Self {
            role: snapshot.role,
            id: format!("publisher-service-{id_seed}"),
            label,
            unit_name: unit_name.clone(),
            actions: service_actions(snapshot.role, label, &state),
            logs: service_logs(snapshot.role, label, &unit_name, &state, log_panel),
            state,
        }
    }
}

impl StreamSectionDisplay {
    fn from_snapshot(snapshot: &broadcast_service_watch::BroadcastServiceWatchSnapshot) -> Self {
        let encoder = &snapshot.encoder;
        let connection = stream_connection_state(encoder.status.state).display();
        let signal = stream_signal_state(encoder.status.signal).display();
        let recording = stream_recording_display(&encoder.status.recording);
        let summary = format!("{} - {}", encoder.server_name, connection.label);
        Self {
            title: "Stream",
            summary,
            server_label: encoder.server_name.clone(),
            connection,
            signal,
            recording,
            listeners: stream_listeners_display(encoder.status.listeners),
            encoder_song: encoder.status.song.clone(),
            stream_elapsed_label: encoder.status.stream_seconds.map(elapsed_label),
            actions: stream_actions(encoder.configured, connection.state),
        }
    }
}

fn stream_connection_state(state: EncoderState) -> StreamConnectionState {
    match state {
        EncoderState::Connected => StreamConnectionState::Connected,
        EncoderState::Connecting => StreamConnectionState::Connecting,
        EncoderState::Disconnected => StreamConnectionState::Disconnected,
        EncoderState::NotInstalled => StreamConnectionState::NotInstalled,
        EncoderState::NotReachable => StreamConnectionState::NotReachable,
        EncoderState::Unknown => StreamConnectionState::Unknown,
    }
}

fn stream_signal_state(state: AudioSignalState) -> StreamSignalState {
    match state {
        AudioSignalState::Present => StreamSignalState::Present,
        AudioSignalState::Absent => StreamSignalState::Absent,
        AudioSignalState::Unknown => StreamSignalState::Unknown,
    }
}

fn stream_recording_display(recording: &RecordingState) -> StreamRecordingDisplay {
    match recording {
        RecordingState::Recording { seconds, path } => StreamRecordingDisplay {
            state: StreamRecordingState::Recording,
            label: "Recording",
            detail: "Encoder is writing an episode file.",
            elapsed_label: seconds.map(elapsed_label),
            path: path.clone(),
            icon_role: StreamStateIconRole::Success,
        },
        RecordingState::Stopped { seconds, path } => StreamRecordingDisplay {
            state: StreamRecordingState::Stopped,
            label: "Stopped",
            detail: "Encoder recording is stopped.",
            elapsed_label: seconds.map(elapsed_label),
            path: path.clone(),
            icon_role: StreamStateIconRole::Info,
        },
        RecordingState::Unknown => StreamRecordingDisplay {
            state: StreamRecordingState::Unknown,
            label: "Recording unknown",
            detail: "Encoder did not report recording state.",
            elapsed_label: None,
            path: None,
            icon_role: StreamStateIconRole::Info,
        },
    }
}

fn stream_listeners_display(listeners: ListenerCount) -> StreamListenersDisplay {
    match listeners {
        ListenerCount::Known(count) => StreamListenersDisplay {
            label: format!("{count} listeners"),
            detail: "Reported by the stream server.",
        },
        ListenerCount::Unknown => StreamListenersDisplay {
            label: "Listeners unknown".to_owned(),
            detail: "Encoder did not report a useful count.",
        },
    }
}

fn track_count_label(count: usize) -> String {
    if count == 1 {
        "1 track".to_owned()
    } else {
        format!("{count} tracks")
    }
}

fn stream_actions(
    configured: bool,
    connection: StreamConnectionState,
) -> Option<StreamActionsDisplay> {
    if !configured || matches!(connection, StreamConnectionState::NotInstalled) {
        return None;
    }

    Some(StreamActionsDisplay {
        connect: StreamActionDisplay {
            id: "stream-connect",
            label: "Connect",
            a11y_label: "Connect stream encoder".to_owned(),
            availability: stream_action_availability(matches!(
                connection,
                StreamConnectionState::Disconnected
            )),
        },
        disconnect: StreamActionDisplay {
            id: "stream-disconnect",
            label: "Disconnect",
            a11y_label: "Disconnect stream encoder".to_owned(),
            availability: stream_action_availability(matches!(
                connection,
                StreamConnectionState::Connected
            )),
        },
    })
}

const fn stream_action_availability(available: bool) -> StreamActionAvailability {
    if available {
        StreamActionAvailability::Available
    } else {
        StreamActionAvailability::Unavailable
    }
}

fn elapsed_label(seconds: u64) -> String {
    let hours = seconds / 3_600;
    let minutes = (seconds % 3_600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

fn service_actions(
    role: PublisherServiceRole,
    label: &'static str,
    state: &PublisherServiceStateDisplay,
) -> PublisherServiceActionsDisplay {
    let id_seed = service_id_seed(role);
    PublisherServiceActionsDisplay {
        start: PublisherActionDisplay {
            id: format!("publisher-{id_seed}-start"),
            label: "Start",
            a11y_label: format!("Start {label} service"),
            availability: availability(matches!(state, PublisherServiceStateDisplay::Inactive)),
        },
        stop: PublisherActionDisplay {
            id: format!("publisher-{id_seed}-stop"),
            label: "Stop",
            a11y_label: format!("Stop {label} service"),
            availability: availability(matches!(state, PublisherServiceStateDisplay::Active)),
        },
        reset: PublisherActionDisplay {
            id: format!("publisher-{id_seed}-reset"),
            label: "Reset",
            a11y_label: format!("Reset failed {label} service"),
            availability: availability(matches!(
                state,
                PublisherServiceStateDisplay::Failed { .. }
            )),
        },
    }
}

fn service_logs(
    role: PublisherServiceRole,
    label: &'static str,
    unit_name: &str,
    state: &PublisherServiceStateDisplay,
    log_panel: &PublisherLogPanelState,
) -> PublisherLogsDisplay {
    let id_seed = service_id_seed(role);
    PublisherLogsDisplay {
        unit_name: unit_name.to_owned(),
        line_count: PUBLISHER_LOG_LINE_COUNT,
        open: log_panel.is_open_for(role),
        action: PublisherActionDisplay {
            id: format!("publisher-{id_seed}-logs"),
            label: "Logs",
            a11y_label: format!("Open {label} service logs"),
            availability: availability(!matches!(
                state,
                PublisherServiceStateDisplay::NotReachable
            )),
        },
    }
}

fn service_summary(services: &[PublisherServiceDisplay]) -> String {
    let failed_count = services
        .iter()
        .filter(|service| {
            matches!(
                service.state,
                PublisherServiceStateDisplay::Failed { .. }
                    | PublisherServiceStateDisplay::NotInstalled
                    | PublisherServiceStateDisplay::NotReachable
            )
        })
        .count();
    if failed_count == 0 {
        format!("{} services observed", services.len())
    } else {
        format!("{failed_count} services need attention")
    }
}

fn source_reachability(
    units: &[broadcast_service_watch::BroadcastServiceUnitSnapshot],
) -> SourceReachabilityState {
    if units
        .iter()
        .any(|unit| matches!(unit.state, ServiceState::NotReachable))
    {
        return SourceReachabilityState::NotReachable;
    }
    if units
        .iter()
        .all(|unit| matches!(unit.state, ServiceState::Unknown))
    {
        return SourceReachabilityState::Unknown;
    }
    SourceReachabilityState::Reachable
}

const fn availability(available: bool) -> PublisherActionAvailability {
    if available {
        PublisherActionAvailability::Available
    } else {
        PublisherActionAvailability::Unavailable
    }
}

const fn service_label(role: PublisherServiceRole) -> &'static str {
    match role {
        PublisherServiceRole::Publisher => "Publisher",
        PublisherServiceRole::Producer => "Producer",
    }
}

const fn service_id_seed(role: PublisherServiceRole) -> &'static str {
    match role {
        PublisherServiceRole::Publisher => "publisher",
        PublisherServiceRole::Producer => "producer",
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
    use crate::application::queries::broadcast::{
        BroadcastReadinessReport, BroadcastReadinessState, BroadcastReadinessSummary,
        BroadcastReadinessTrack,
    };
    use crate::broadcast::encoder::EncoderStatus;
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

    fn readiness_track(id: i64, state: BroadcastReadinessState) -> BroadcastReadinessTrack {
        BroadcastReadinessTrack {
            track_id: id,
            title: "Track".to_owned(),
            artist: Some("Artist".to_owned()),
            album: Some("Album".to_owned()),
            path: Some("/tmp/track.mp3".to_owned()),
            state,
            reason: "Embedded MusicIndex Value Routes tag is missing.".to_owned(),
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
        assert!(vm.source.is_none());
        assert!(vm.publisher.is_none());
        assert!(vm.stream.is_none());
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

    #[test]
    fn publisher_section_projects_active_and_inactive_services() {
        let snapshot = publisher_snapshot([
            (
                PublisherServiceRole::Publisher,
                "musicindex-live-publisher@mixxx.service",
                ServiceState::Active,
            ),
            (
                PublisherServiceRole::Producer,
                "mixxx-now-playing.service",
                ServiceState::Inactive,
            ),
        ]);
        let vm = ShowPageVm::from_queue_and_publisher(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
        );
        let publisher = vm.publisher.expect("publisher section");

        assert_eq!(
            vm.source,
            Some(SourceSectionDisplay {
                title: "Source",
                summary: "Local - Reachable".to_owned(),
                host_name: "Local".to_owned(),
                reachability: SourceReachabilityDisplay {
                    state: SourceReachabilityState::Reachable,
                    label: "Reachable",
                    detail: "Host accepts broadcast control.",
                },
                readiness: None,
            })
        );
        assert_eq!(publisher.title, "Publisher");
        assert_eq!(publisher.summary, "2 services observed");
        assert_eq!(publisher.services.len(), 2);
        assert_eq!(
            publisher.services[0].state,
            PublisherServiceStateDisplay::Active
        );
        assert!(publisher.services[0].actions.start.disabled());
        assert!(!publisher.services[0].actions.stop.disabled());
        assert_eq!(
            publisher.services[1].state,
            PublisherServiceStateDisplay::Inactive
        );
        assert!(!publisher.services[1].actions.start.disabled());
        assert!(publisher.services[1].actions.stop.disabled());
        assert_eq!(
            vm.stream.expect("stream section").connection.state,
            StreamConnectionState::NotInstalled
        );
    }

    #[test]
    fn publisher_failed_state_carries_reason_and_offers_reset() {
        let snapshot = publisher_snapshot([(
            PublisherServiceRole::Publisher,
            "musicindex-live-publisher@mixxx.service",
            ServiceState::Failed {
                reason: "exit-code".to_owned(),
            },
        )]);
        let vm = ShowPageVm::from_queue_and_publisher(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
        );
        let service = &vm.publisher.expect("publisher section").services[0];

        assert_eq!(
            service.state,
            PublisherServiceStateDisplay::Failed {
                reason: "exit-code".to_owned(),
            }
        );
        assert_eq!(service.state.detail().as_deref(), Some("Reason: exit-code"));
        assert!(service.actions.start.disabled());
        assert!(!service.actions.reset.disabled());
    }

    #[test]
    fn publisher_not_installed_state_disables_service_commands() {
        let snapshot = publisher_snapshot([(
            PublisherServiceRole::Publisher,
            "musicindex-live-publisher@mixxx.service",
            ServiceState::NotInstalled,
        )]);
        let vm = ShowPageVm::from_queue_and_publisher(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
        );
        let service = &vm.publisher.expect("publisher section").services[0];

        assert_eq!(service.state, PublisherServiceStateDisplay::NotInstalled);
        assert_eq!(
            service.state.detail().as_deref(),
            Some("Unit file not found.")
        );
        assert!(service.actions.start.disabled());
        assert!(service.actions.stop.disabled());
        assert!(service.actions.reset.disabled());
    }

    #[test]
    fn source_section_projects_not_reachable_host_state() {
        let snapshot = publisher_snapshot_on_host(
            "Studio",
            [(
                PublisherServiceRole::Publisher,
                "musicindex-live-publisher@mixxx.service",
                ServiceState::NotReachable,
            )],
        );
        let vm = ShowPageVm::from_queue_and_publisher(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
        );

        assert_eq!(
            vm.source,
            Some(SourceSectionDisplay {
                title: "Source",
                summary: "Studio - Not reachable".to_owned(),
                host_name: "Studio".to_owned(),
                reachability: SourceReachabilityDisplay {
                    state: SourceReachabilityState::NotReachable,
                    label: "Not reachable",
                    detail: "Host cannot be reached.",
                },
                readiness: None,
            })
        );
        assert_eq!(
            vm.publisher.expect("publisher section").services[0].state,
            PublisherServiceStateDisplay::NotReachable
        );
    }

    #[test]
    fn source_section_projects_cached_broadcast_readiness() {
        let snapshot = publisher_snapshot([(
            PublisherServiceRole::Publisher,
            "musicindex-live-publisher@mixxx.service",
            ServiceState::Active,
        )]);
        let readiness = BroadcastReadinessSnapshot {
            report: Some(BroadcastReadinessReport {
                summary: BroadcastReadinessSummary {
                    ready: 3,
                    no_route_tag: 1,
                    file_missing: 1,
                },
                tracks: vec![readiness_track(7, BroadcastReadinessState::NoRouteTag)],
            }),
            error: None,
        };

        let vm = ShowPageVm::from_queue_publisher_and_readiness(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
            Some(&readiness),
        );
        let readiness = vm
            .source
            .expect("source section")
            .readiness
            .expect("readiness display");

        assert_eq!(readiness.count_label, "2 tracks not ready");
        assert_eq!(readiness.detail, "1 missing routes, 1 missing files.");
        assert_eq!(readiness.state, SourceReadinessState::NeedsAttention);
        assert!(!readiness.action.disabled());
    }

    #[test]
    fn publisher_log_panel_open_state_carries_journal_text() {
        let snapshot = publisher_snapshot([(
            PublisherServiceRole::Publisher,
            "musicindex-live-publisher@mixxx.service",
            ServiceState::Active,
        )]);
        let vm = ShowPageVm::from_queue_and_publisher(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::open(
                PublisherServiceRole::Publisher,
                "musicindex-live-publisher@mixxx.service",
                50,
                "line one\nline two\n",
            ),
        );
        let publisher = vm.publisher.expect("publisher section");

        assert!(publisher.log_panel.is_open());
        assert!(publisher.services[0].logs.open);
        assert_eq!(
            publisher.log_panel,
            PublisherLogPanelState::Open {
                role: PublisherServiceRole::Publisher,
                unit_name: "musicindex-live-publisher@mixxx.service".to_owned(),
                line_count: 50,
                text: "line one\nline two\n".to_owned(),
            }
        );
    }

    #[test]
    fn publisher_service_state_exposes_six_variants_without_transport_error_payload() {
        assert_eq!(PublisherServiceStateDisplay::ALL_KINDS.len(), 6);
        assert_eq!(
            PublisherServiceStateDisplay::from_service_state(&ServiceState::NotReachable),
            PublisherServiceStateDisplay::NotReachable
        );
        assert_eq!(
            PublisherServiceStateDisplay::from_service_state(&ServiceState::Unknown),
            PublisherServiceStateDisplay::Unknown
        );
    }

    #[test]
    fn stream_section_projects_connected_signal_recording_and_actions() {
        let snapshot = snapshot_with_encoder(
            Vec::new(),
            broadcast_service_watch::BroadcastEncoderSnapshot {
                server_name: "Main".to_owned(),
                configured: true,
                status: EncoderStatus {
                    state: EncoderState::Connected,
                    recording: RecordingState::Stopped {
                        seconds: Some(0),
                        path: None,
                    },
                    signal: AudioSignalState::Present,
                    listeners: ListenerCount::Known(12),
                    song: Some("Artist - Title".to_owned()),
                    stream_seconds: Some(3_725),
                },
            },
        );
        let vm = ShowPageVm::from_queue_and_publisher(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
        );
        let stream = vm.stream.expect("stream section");
        let actions = stream.actions.expect("stream actions");

        assert!(vm.source.is_none());
        assert!(vm.publisher.is_none());
        assert_eq!(stream.title, "Stream");
        assert_eq!(stream.summary, "Main - Connected");
        assert_eq!(stream.connection.state, StreamConnectionState::Connected);
        assert_eq!(stream.signal.state, StreamSignalState::Present);
        assert_eq!(stream.signal.icon_role, StreamStateIconRole::Success);
        assert_eq!(stream.recording.state, StreamRecordingState::Stopped);
        assert_eq!(stream.listeners.label, "12 listeners");
        assert_eq!(stream.encoder_song.as_deref(), Some("Artist - Title"));
        assert_eq!(stream.stream_elapsed_label.as_deref(), Some("1:02:05"));
        assert!(actions.connect.disabled());
        assert!(!actions.disconnect.disabled());
    }

    #[test]
    fn stream_section_distinguishes_connected_without_audio_signal() {
        let with_signal = StreamSignalState::Present.display();
        let without_signal = StreamSignalState::Absent.display();

        assert_ne!(with_signal.label, without_signal.label);
        assert_ne!(with_signal.icon_role, without_signal.icon_role);

        let snapshot = snapshot_with_encoder(
            Vec::new(),
            broadcast_service_watch::BroadcastEncoderSnapshot {
                server_name: "Main".to_owned(),
                configured: true,
                status: EncoderStatus {
                    state: EncoderState::Connected,
                    recording: RecordingState::Stopped {
                        seconds: Some(0),
                        path: None,
                    },
                    signal: AudioSignalState::Absent,
                    listeners: ListenerCount::Unknown,
                    song: None,
                    stream_seconds: None,
                },
            },
        );
        let vm = ShowPageVm::from_queue_and_publisher(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
        );
        let stream = vm.stream.expect("stream section");

        assert_eq!(stream.connection.state, StreamConnectionState::Connected);
        assert_eq!(stream.signal.state, StreamSignalState::Absent);
        assert_eq!(stream.signal.label, "No audio signal");
        assert_eq!(stream.signal.icon_role, StreamStateIconRole::Warning);
    }

    #[test]
    fn stream_section_projects_connecting_and_recording_file_path() {
        let snapshot = snapshot_with_encoder(
            Vec::new(),
            broadcast_service_watch::BroadcastEncoderSnapshot {
                server_name: "Main".to_owned(),
                configured: true,
                status: EncoderStatus {
                    state: EncoderState::Connecting,
                    recording: RecordingState::Recording {
                        seconds: Some(42),
                        path: Some("/recordings/show.mp3".to_owned()),
                    },
                    signal: AudioSignalState::Unknown,
                    listeners: ListenerCount::Unknown,
                    song: None,
                    stream_seconds: None,
                },
            },
        );
        let vm = ShowPageVm::from_queue_and_publisher(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
        );
        let stream = vm.stream.expect("stream section");
        let actions = stream.actions.expect("stream actions");

        assert_eq!(stream.connection.state, StreamConnectionState::Connecting);
        assert_eq!(stream.recording.state, StreamRecordingState::Recording);
        assert_eq!(stream.recording.elapsed_label.as_deref(), Some("0:42"));
        assert_eq!(
            stream.recording.path.as_deref(),
            Some("/recordings/show.mp3")
        );
        assert!(actions.connect.disabled());
        assert!(actions.disconnect.disabled());
    }

    #[test]
    fn stream_not_installed_is_empty_state_without_actions() {
        let snapshot = snapshot_with_encoder(
            Vec::new(),
            broadcast_service_watch::BroadcastEncoderSnapshot {
                server_name: "Not configured".to_owned(),
                configured: false,
                status: EncoderStatus::not_installed(),
            },
        );
        let vm = ShowPageVm::from_queue_and_publisher(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
        );
        let stream = vm.stream.expect("stream section");

        assert_eq!(stream.connection.state, StreamConnectionState::NotInstalled);
        assert_eq!(stream.recording.state, StreamRecordingState::Unknown);
        assert!(stream.actions.is_none());
    }

    fn publisher_snapshot<const N: usize>(
        units: [(PublisherServiceRole, &str, ServiceState); N],
    ) -> broadcast_service_watch::BroadcastServiceWatchSnapshot {
        publisher_snapshot_on_host("Local", units)
    }

    fn publisher_snapshot_on_host<const N: usize>(
        host_name: &str,
        units: [(PublisherServiceRole, &str, ServiceState); N],
    ) -> broadcast_service_watch::BroadcastServiceWatchSnapshot {
        snapshot_with_encoder(
            units
                .into_iter()
                .map(|(role, unit_name, state)| {
                    broadcast_service_watch::BroadcastServiceUnitSnapshot {
                        role,
                        host_name: host_name.to_owned(),
                        unit_name: unit_name.to_owned(),
                        state,
                    }
                })
                .collect(),
            broadcast_service_watch::BroadcastEncoderSnapshot {
                server_name: "Not configured".to_owned(),
                configured: false,
                status: EncoderStatus::not_installed(),
            },
        )
    }

    fn snapshot_with_encoder(
        units: Vec<broadcast_service_watch::BroadcastServiceUnitSnapshot>,
        encoder: broadcast_service_watch::BroadcastEncoderSnapshot,
    ) -> broadcast_service_watch::BroadcastServiceWatchSnapshot {
        broadcast_service_watch::BroadcastServiceWatchSnapshot {
            at: std::time::Instant::now(),
            units,
            encoder,
        }
    }
}
