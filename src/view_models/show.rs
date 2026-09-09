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
    /// The service manager is starting the unit.
    Starting,
    /// The service manager is stopping the unit.
    Stopping,
    /// The service is failed and needs reset before start.
    Failed {
        /// Failure reason from the service manager result.
        reason: String,
    },
    /// The unit file is absent.
    NotInstalled,
    /// The host that owns the unit cannot be reached.
    NotReachable,
    /// The service manager reported a state this surface does not know.
    Unknown,
    /// This app sent a command and has no answer yet.
    ///
    /// Display only. No `ServiceState` maps to it. It holds the row between the
    /// moment an operator presses an action and the moment the next snapshot
    /// arrives, so the surface answers the press at once.
    Working,
}

impl PublisherServiceStateDisplay {
    #[cfg(test)]
    const ALL_KINDS: [PublisherServiceStateKind; 9] = [
        PublisherServiceStateKind::Active,
        PublisherServiceStateKind::Inactive,
        PublisherServiceStateKind::Starting,
        PublisherServiceStateKind::Stopping,
        PublisherServiceStateKind::Failed,
        PublisherServiceStateKind::NotInstalled,
        PublisherServiceStateKind::NotReachable,
        PublisherServiceStateKind::Unknown,
        PublisherServiceStateKind::Working,
    ];

    fn from_service_state(state: &ServiceState) -> Self {
        match state {
            ServiceState::Active => Self::Active,
            ServiceState::Inactive => Self::Inactive,
            ServiceState::Starting => Self::Starting,
            ServiceState::Stopping => Self::Stopping,
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
            Self::Starting => PublisherServiceStateKind::Starting,
            Self::Stopping => PublisherServiceStateKind::Stopping,
            Self::Failed { .. } => PublisherServiceStateKind::Failed,
            Self::NotInstalled => PublisherServiceStateKind::NotInstalled,
            Self::NotReachable => PublisherServiceStateKind::NotReachable,
            Self::Unknown => PublisherServiceStateKind::Unknown,
            Self::Working => PublisherServiceStateKind::Working,
        }
    }

    #[must_use]
    pub(crate) const fn label(&self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Inactive => "Inactive",
            Self::Starting => "Starting",
            Self::Stopping => "Stopping",
            Self::Failed { .. } => "Failed",
            Self::NotInstalled => "Not installed",
            Self::NotReachable => "Not reachable",
            Self::Unknown => "Unknown",
            Self::Working => "Working",
        }
    }

    /// Curator-facing detail for this state.
    #[must_use]
    pub(crate) fn detail(&self) -> String {
        match self {
            Self::Failed { reason } => format!("Reason: {reason}"),
            Self::NotInstalled => "Unit file not found.".to_owned(),
            Self::NotReachable => "Host cannot be reached.".to_owned(),
            Self::Unknown => {
                "The service manager reported a state this app does not know.".to_owned()
            }
            Self::Active => "Unit is running.".to_owned(),
            Self::Inactive => "Unit is stopped.".to_owned(),
            Self::Starting => "Unit is starting.".to_owned(),
            Self::Stopping => "Unit is stopping.".to_owned(),
            Self::Working => "Waiting for the service manager.".to_owned(),
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
    /// The service manager is starting the unit.
    Starting,
    /// The service manager is stopping the unit.
    Stopping,
    /// The service is failed and needs reset before start.
    Failed,
    /// The unit file is absent.
    NotInstalled,
    /// The host that owns the unit cannot be reached.
    NotReachable,
    /// The service manager reported a state this surface does not know.
    Unknown,
    /// This app sent a command and has no answer yet.
    Working,
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
    /// Returns whether the panel already shows this service's journal.
    #[must_use]
    pub(crate) fn shows_role(&self, role: PublisherServiceRole) -> bool {
        matches!(self, Self::Open { role: open, .. } if *open == role)
    }

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

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "ADR 0063 task 003 moves publisher logs into the detail panel."
        )
    )]
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

/// Display-ready Event section for the `Show` screen mount.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventSectionDisplay {
    /// Stable section title.
    pub(crate) title: &'static str,
    /// Summary label for the selected broadcast event.
    pub(crate) summary: String,
    /// Selected event label and identifiers.
    pub(crate) event: EventSelectionDisplay,
    /// Publisher target attachment state.
    pub(crate) target: EventTargetAttachmentDisplay,
    /// Complete listener feed tag, built outside the renderer.
    pub(crate) feed_tag: Option<String>,
    /// Operational hint for remote token-file setup.
    pub(crate) hint: Option<String>,
    /// Attach and detach action state.
    pub(crate) actions: EventActionsDisplay,
}

/// Display-ready selected broadcast event facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventSelectionDisplay {
    /// Curator-facing event label.
    pub(crate) label: String,
    /// Relay event identifier, when one is selected.
    pub(crate) event_id: Option<String>,
    /// Relay endpoint for the event.
    pub(crate) endpoint: Option<String>,
    /// Stored token file path.
    pub(crate) token_path: Option<String>,
    /// Stored event liveness state.
    pub(crate) state: EventStateDisplay,
}

/// Display-ready event liveness state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EventStateDisplay {
    /// Stable state.
    pub(crate) state: EventState,
    /// Curator-facing state label.
    pub(crate) label: &'static str,
    /// Curator-facing state detail.
    pub(crate) detail: &'static str,
}

/// Stable event liveness states.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EventState {
    /// No event is selected.
    None,
    /// Event liveness has not been checked.
    Unknown,
    /// The relay still has the event.
    Live,
    /// The relay reports the event as dead.
    Dead,
}

impl EventState {
    const fn display(self) -> EventStateDisplay {
        match self {
            Self::None => EventStateDisplay {
                state: self,
                label: "No event",
                detail: "No broadcast event is selected.",
            },
            Self::Unknown => EventStateDisplay {
                state: self,
                label: "Unknown",
                detail: "Event liveness has not been checked.",
            },
            Self::Live => EventStateDisplay {
                state: self,
                label: "Live",
                detail: "Relay accepts this event.",
            },
            Self::Dead => EventStateDisplay {
                state: self,
                label: "Dead",
                detail: "Relay no longer has this event.",
            },
        }
    }
}

/// Publisher target attachment display for the selected event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventTargetAttachmentDisplay {
    /// Stable attachment state.
    pub(crate) state: EventTargetAttachmentState,
    /// Target name or a non-empty state label.
    pub(crate) label: String,
    /// Attached target name, only when known.
    pub(crate) target_name: Option<String>,
    /// Curator-facing detail.
    pub(crate) detail: String,
}

/// Stable publisher target attachment state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EventTargetAttachmentState {
    /// The selected event is attached to a target.
    Attached,
    /// The selected event is not attached to any listed target.
    NotAttached,
    /// Target command output is not available yet.
    Unknown,
    /// The publisher is too old to expose target commands.
    CommandsUnavailable,
    /// The publisher host cannot be reached.
    NotReachable,
    /// Target list failed without a narrower state.
    Failed,
}

/// Typed availability for event target actions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EventActionAvailability {
    /// The action can be run.
    Available,
    /// The action is visible but unavailable in this state.
    Unavailable,
}

impl EventActionAvailability {
    #[must_use]
    pub(crate) const fn disabled(self) -> bool {
        matches!(self, Self::Unavailable)
    }
}

/// Display-ready event target action state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventActionDisplay {
    /// Stable element identifier.
    pub(crate) id: &'static str,
    /// Visible action label.
    pub(crate) label: &'static str,
    /// Accessibility label for the action.
    pub(crate) a11y_label: String,
    /// Typed action availability.
    pub(crate) availability: EventActionAvailability,
}

impl EventActionDisplay {
    #[must_use]
    pub(crate) const fn disabled(&self) -> bool {
        self.availability.disabled()
    }
}

/// Display-ready event target actions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventActionsDisplay {
    /// Attach action.
    pub(crate) attach: EventActionDisplay,
    /// Detach action.
    pub(crate) detach: EventActionDisplay,
}

/// Input for projecting an Event section.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventSectionInput {
    /// Selected event, if the registry has one.
    pub(crate) selected_event: Option<EventSelectionInput>,
    /// Latest target-list read for the selected publisher host.
    pub(crate) targets: EventTargetListInput,
    /// Target name chosen by the app surface for an attach command.
    pub(crate) attach_target_name: String,
    /// Whether the selected publisher host uses a remote transport.
    pub(crate) remote_host: bool,
}

/// Input for projecting a selected broadcast event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventSelectionInput {
    /// Optional operator label.
    pub(crate) label: Option<String>,
    /// Relay event identifier.
    pub(crate) event_id: String,
    /// Relay endpoint.
    pub(crate) endpoint: String,
    /// Stored token file path.
    pub(crate) token_path: String,
    /// Stored liveness state.
    pub(crate) state: EventState,
    /// Whether the local token path is missing.
    pub(crate) token_file_missing: bool,
}

/// Input target list state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum EventTargetListInput {
    /// No target command has completed yet.
    Unknown,
    /// Target list read from the publisher.
    Loaded {
        /// Listed targets.
        targets: Vec<EventTargetInput>,
    },
    /// The selected publisher is too old for target commands.
    CommandsUnavailable,
    /// The selected host did not answer.
    NotReachable,
    /// Target listing failed without a narrower state.
    Failed {
        /// Failure detail.
        detail: String,
    },
}

/// Input target row from the publisher target list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventTargetInput {
    /// Configured target name.
    pub(crate) name: String,
    /// Event identifier currently attached to the target.
    pub(crate) event_id: String,
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
    /// This app sent a connect or a disconnect and has no answer yet.
    Working,
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
            Self::Working => StreamConnectionDisplay {
                state: self,
                label: "Working",
                detail: "Waiting for the encoder.",
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

/// Stable card identities for the Show dashboard.
///
/// The order of this enum is the ADR 0059 section order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShowCardKind {
    /// Source host and library readiness.
    Source,
    /// Live metadata producer and publisher services.
    LiveMetadata,
    /// Relay event and publisher target attachment.
    Event,
    /// Stream encoder status.
    Stream,
}

impl ShowCardKind {
    const ORDER: [Self; 4] = [Self::Source, Self::LiveMetadata, Self::Event, Self::Stream];

    /// Returns the stable visible title for this card kind.
    #[must_use]
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::LiveMetadata => "Live Metadata",
            Self::Event => "Event",
            Self::Stream => "Stream",
        }
    }
}

/// Summary state for a Show dashboard card.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShowCardStateKind {
    /// The section summary is healthy.
    Ok,
    /// The section needs operator attention.
    Attention,
    /// The section reports a failed state.
    Failed,
    /// The section exists but its subject is absent.
    Absent,
    /// The section state is not known.
    Unknown,
}

/// Display-ready summary for one Show dashboard card.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ShowCardDisplay {
    /// Stable card identity.
    pub(crate) kind: ShowCardKind,
    /// Visible card title.
    pub(crate) title: &'static str,
    /// Visible state badge label.
    pub(crate) state_label: String,
    /// Typed state for semantic card styling.
    pub(crate) state: ShowCardStateKind,
    /// Primary summary line. Always present and non-empty.
    pub(crate) primary: String,
    /// Secondary summary line. Always present, empty when unused.
    pub(crate) secondary: String,
    /// Accessibility label for the whole card.
    pub(crate) a11y_label: String,
}

/// Side-panel mode for the Show screen.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ShowPanelMode {
    /// The panel shows the cuelist.
    #[default]
    Cuelist,
    /// The panel shows detail for one dashboard card.
    Detail(ShowCardKind),
}

impl ShowPanelMode {
    /// Returns the visible title for the current panel mode.
    #[must_use]
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Cuelist => "Cuelist",
            Self::Detail(kind) => kind.title(),
        }
    }
}

/// Typed availability for Show panel controls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShowPanelActionAvailability {
    /// The control can be run.
    Available,
    /// The control is visible but unavailable for the current mode.
    Unavailable,
}

impl ShowPanelActionAvailability {
    #[must_use]
    pub(crate) const fn disabled(self) -> bool {
        matches!(self, Self::Unavailable)
    }
}

/// Display-ready state for a Show panel chrome action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ShowPanelActionDisplay {
    /// Stable element identifier.
    pub(crate) id: &'static str,
    /// Visible action label.
    pub(crate) label: &'static str,
    /// Accessibility label for the action.
    pub(crate) a11y_label: &'static str,
    /// Typed action availability.
    pub(crate) availability: ShowPanelActionAvailability,
}

impl ShowPanelActionDisplay {
    #[must_use]
    pub(crate) const fn disabled(self) -> bool {
        self.availability.disabled()
    }
}

/// Display-ready chrome actions for the Show side panel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ShowPanelChromeDisplay {
    /// Action that opens the panel.
    pub(crate) open_panel: ShowPanelActionDisplay,
    /// Action that closes the panel.
    pub(crate) close_panel: ShowPanelActionDisplay,
    /// Action that returns detail mode to the cuelist.
    pub(crate) show_cuelist: ShowPanelActionDisplay,
}

impl ShowPanelChromeDisplay {
    const fn for_mode(mode: ShowPanelMode) -> Self {
        Self {
            open_panel: ShowPanelActionDisplay {
                id: "show-panel-open",
                label: "Open",
                a11y_label: "Open show side panel",
                availability: ShowPanelActionAvailability::Available,
            },
            close_panel: ShowPanelActionDisplay {
                id: "show-panel-close",
                label: "Close",
                a11y_label: "Close show side panel",
                availability: ShowPanelActionAvailability::Available,
            },
            show_cuelist: ShowPanelActionDisplay {
                id: "show-panel-cuelist",
                label: "Cuelist",
                a11y_label: "Return show side panel to cuelist",
                availability: if matches!(mode, ShowPanelMode::Detail(_)) {
                    ShowPanelActionAvailability::Available
                } else {
                    ShowPanelActionAvailability::Unavailable
                },
            },
        }
    }
}

impl Default for ShowPanelChromeDisplay {
    fn default() -> Self {
        Self::for_mode(ShowPanelMode::default())
    }
}

/// Width class for the Show dashboard card grid.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ShowWidthClass {
    /// One dashboard column.
    Compact,
    /// Two dashboard columns.
    Medium,
    /// Three dashboard columns.
    #[default]
    Wide,
}

const SHOW_GRID_MEDIUM_MIN: f32 = 712.0;
const SHOW_GRID_WIDE_MIN: f32 = 1_056.0;

impl ShowWidthClass {
    /// Returns the dashboard width class for a window width.
    #[must_use]
    pub(crate) fn for_window_width(window_width: f32) -> Self {
        if window_width < SHOW_GRID_MEDIUM_MIN {
            Self::Compact
        } else if window_width < SHOW_GRID_WIDE_MIN {
            Self::Medium
        } else {
            Self::Wide
        }
    }

    /// Returns the dashboard grid column count.
    #[must_use]
    pub(crate) const fn columns(self) -> u16 {
        match self {
            Self::Compact => 1,
            Self::Medium => 2,
            Self::Wide => 3,
        }
    }
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
    /// Optional Event section; absent sections render nothing.
    pub(crate) event: Option<EventSectionDisplay>,
    /// Optional Stream section; absent sections render nothing.
    pub(crate) stream: Option<StreamSectionDisplay>,
    /// Dashboard card summaries in `ShowCardKind` order.
    pub(crate) cards: Vec<ShowCardDisplay>,
    /// Dashboard width class.
    pub(crate) width_class: ShowWidthClass,
    /// Current side-panel mode.
    pub(crate) panel_mode: ShowPanelMode,
    /// Whether the side panel is open.
    pub(crate) panel_open: bool,
    /// Side-panel chrome action display state.
    pub(crate) panel_chrome: ShowPanelChromeDisplay,
    /// Queue and transport display projected by the existing queue VM.
    pub(crate) queue: QueueNowPlayingPageVm,
    /// Last command message, shown on this screen.
    ///
    /// Every broadcast command wrote its error to a field that only `Settings`
    /// rendered, so a failed action on `Show` looked like no action at all.
    pub(crate) status_message: Option<String>,
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
        Self::from_queue_publisher_readiness_and_event(
            queue,
            publisher_snapshot,
            log_panel,
            readiness_snapshot,
            None,
        )
    }

    /// Projects the Show page from queue, publisher, readiness, and event state.
    #[must_use]
    pub(crate) fn from_queue_publisher_readiness_and_event(
        queue: QueueNowPlayingPageVm,
        publisher_snapshot: Option<&broadcast_service_watch::BroadcastServiceWatchSnapshot>,
        log_panel: PublisherLogPanelState,
        readiness_snapshot: Option<&BroadcastReadinessSnapshot>,
        event_input: Option<&EventSectionInput>,
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
        let publisher_reachable = publisher_snapshot.is_some_and(|snapshot| {
            matches!(
                source_reachability(snapshot.units.as_slice()),
                SourceReachabilityState::Reachable
            )
        });
        let event =
            event_input.map(|input| EventSectionDisplay::from_input(input, publisher_reachable));
        let stream = publisher_snapshot.map(StreamSectionDisplay::from_snapshot);
        let cards = show_cards(
            source.as_ref(),
            publisher.as_ref(),
            event.as_ref(),
            stream.as_ref(),
        );
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
            event,
            stream,
            cards,
            width_class: ShowWidthClass::default(),
            panel_mode: ShowPanelMode::default(),
            panel_open: true,
            status_message: None,
            panel_chrome: ShowPanelChromeDisplay::default(),
            queue,
        }
    }

    /// Applies a window-width projection to the Show dashboard.
    #[must_use]
    pub(crate) fn with_window_width(mut self, window_width: f32) -> Self {
        self.width_class = ShowWidthClass::for_window_width(window_width);
        self
    }

    /// Preserves panel mode and open state across a fresh Show projection.
    #[must_use]
    pub(crate) fn with_panel_state(mut self, panel_mode: ShowPanelMode, panel_open: bool) -> Self {
        self.set_panel_mode(panel_mode);
        self.panel_open = panel_open;
        self
    }

    /// Opens the panel in detail mode for a selected dashboard card.
    #[must_use]
    /// Selects a card, or closes the panel when that card is already open.
    ///
    /// A second select on the open card closes the panel, so the card behaves
    /// like the panel close control.
    pub(crate) fn select_card(self, kind: ShowCardKind) -> Self {
        if self.panel_open && self.panel_mode == ShowPanelMode::Detail(kind) {
            let mut closed = self;
            closed.panel_open = false;
            return closed;
        }
        self.show_card_detail(kind)
    }

    /// Marks one service as working, and disables its actions.
    ///
    /// An operator presses an action and the answer takes seconds: the command
    /// blocks, and then the watch actor reads the new state. This holds the row
    /// in the meantime, so the press has an immediate effect. The next snapshot
    /// replaces it.
    #[must_use]
    pub(crate) fn mark_service_working(mut self, role: PublisherServiceRole) -> Self {
        if let Some(publisher) = self.publisher.as_mut() {
            for service in &mut publisher.services {
                if service.role == role {
                    service.state = PublisherServiceStateDisplay::Working;
                    service.actions = service_actions(role, service.label, &service.state);
                }
            }
        }
        self.cards = show_cards(
            self.source.as_ref(),
            self.publisher.as_ref(),
            self.event.as_ref(),
            self.stream.as_ref(),
        );
        self
    }

    /// Marks the stream encoder as working, and disables its actions.
    ///
    /// The connect command blocks and the watch actor then reads the new state,
    /// so without this a press has no visible effect for seconds.
    #[must_use]
    pub(crate) fn mark_stream_working(mut self) -> Self {
        if let Some(stream) = self.stream.as_mut() {
            stream.connection = StreamConnectionState::Working.display();
            stream.actions = None;
        }
        self.cards = show_cards(
            self.source.as_ref(),
            self.publisher.as_ref(),
            self.event.as_ref(),
            self.stream.as_ref(),
        );
        self
    }

    /// Sets the message this screen shows, or clears it when the text is empty.
    #[must_use]
    pub(crate) fn with_status_message(mut self, message: &str) -> Self {
        let message = message.trim();
        self.status_message = (!message.is_empty()).then(|| message.to_owned());
        self
    }

    /// Opens one card's detail. Never toggles.
    ///
    /// An action that must land on a card, such as opening the log, calls this.
    /// `select_card` toggles, so it would close the panel the action needs.
    #[must_use]
    pub(crate) fn show_card_detail(mut self, kind: ShowCardKind) -> Self {
        self.set_panel_mode(ShowPanelMode::Detail(kind));
        self.panel_open = true;
        self
    }

    /// Returns the panel to cuelist mode and leaves it open.
    #[must_use]
    pub(crate) fn show_cuelist_panel(mut self) -> Self {
        self.set_panel_mode(ShowPanelMode::Cuelist);
        self.panel_open = true;
        self
    }

    /// Closes the side panel without changing its current mode.
    #[must_use]
    pub(crate) fn close_panel(mut self) -> Self {
        self.panel_open = false;
        self
    }

    /// Opens the side panel without changing its current mode.
    #[must_use]
    pub(crate) fn open_panel(mut self) -> Self {
        self.panel_open = true;
        self
    }

    /// Returns whether the page represents active show playback.
    #[must_use]
    pub(crate) const fn is_active(&self) -> bool {
        self.empty_state.is_none()
    }

    fn set_panel_mode(&mut self, panel_mode: ShowPanelMode) {
        self.panel_mode = panel_mode;
        self.panel_chrome = ShowPanelChromeDisplay::for_mode(panel_mode);
    }
}

fn show_cards(
    source: Option<&SourceSectionDisplay>,
    publisher: Option<&PublisherSectionDisplay>,
    event: Option<&EventSectionDisplay>,
    stream: Option<&StreamSectionDisplay>,
) -> Vec<ShowCardDisplay> {
    ShowCardKind::ORDER
        .into_iter()
        .filter_map(|kind| match kind {
            ShowCardKind::Source => source.map(ShowCardDisplay::from_source),
            ShowCardKind::LiveMetadata => publisher.map(ShowCardDisplay::from_live_metadata),
            ShowCardKind::Event => event.map(ShowCardDisplay::from_event),
            ShowCardKind::Stream => stream.map(ShowCardDisplay::from_stream),
        })
        .collect()
}

impl ShowCardDisplay {
    fn from_source(section: &SourceSectionDisplay) -> Self {
        Self::new(
            ShowCardKind::Source,
            section.reachability.label,
            source_card_state(section.reachability.state),
            non_empty_summary_line(&section.host_name, "Unknown host"),
            section
                .readiness
                .as_ref()
                .map_or_else(String::new, |readiness| readiness.count_label.clone()),
        )
    }

    fn from_live_metadata(section: &PublisherSectionDisplay) -> Self {
        let (state, state_label) = live_metadata_card_state(section.services.as_slice());
        let primary = section
            .services
            .first()
            .map_or_else(String::new, live_metadata_service_line);
        let secondary = section
            .services
            .get(1)
            .map_or_else(String::new, live_metadata_service_line);

        Self::new(
            ShowCardKind::LiveMetadata,
            state_label,
            state,
            non_empty_summary_line(&primary, "No services observed"),
            secondary,
        )
    }

    fn from_event(section: &EventSectionDisplay) -> Self {
        let secondary = if section.event.event_id.is_some() {
            format!(
                "Target: {}",
                non_empty_summary_line(&section.target.label, "unknown")
            )
        } else {
            String::new()
        };

        Self::new(
            ShowCardKind::Event,
            section.event.state.label,
            event_card_state(section.event.state.state),
            non_empty_summary_line(&section.event.label, "No event selected"),
            secondary,
        )
    }

    fn from_stream(section: &StreamSectionDisplay) -> Self {
        Self::new(
            ShowCardKind::Stream,
            section.connection.label,
            stream_card_state(section.connection.state),
            non_empty_summary_line(section.connection.label, "Unknown connection"),
            section.listeners.label.clone(),
        )
    }

    fn new(
        kind: ShowCardKind,
        state_label: impl Into<String>,
        state: ShowCardStateKind,
        primary: String,
        secondary: String,
    ) -> Self {
        let title = kind.title();
        let state_label = state_label.into();
        let a11y_label = show_card_a11y_label(title, &state_label, &primary, &secondary);
        Self {
            kind,
            title,
            state_label,
            state,
            primary,
            secondary,
            a11y_label,
        }
    }
}

fn non_empty_summary_line(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_owned()
    } else {
        value.to_owned()
    }
}

fn show_card_a11y_label(title: &str, state_label: &str, primary: &str, secondary: &str) -> String {
    if secondary.trim().is_empty() {
        format!("{title}, {state_label}, {primary}")
    } else {
        format!("{title}, {state_label}, {primary}, {secondary}")
    }
}

const fn source_card_state(state: SourceReachabilityState) -> ShowCardStateKind {
    match state {
        SourceReachabilityState::Reachable => ShowCardStateKind::Ok,
        SourceReachabilityState::NotReachable => ShowCardStateKind::Failed,
        SourceReachabilityState::Unknown => ShowCardStateKind::Unknown,
    }
}

fn live_metadata_card_state(
    services: &[PublisherServiceDisplay],
) -> (ShowCardStateKind, &'static str) {
    if services
        .iter()
        .any(|service| matches!(service.state.kind(), PublisherServiceStateKind::Failed))
    {
        return (ShowCardStateKind::Failed, "Failed");
    }

    if services.iter().any(|service| {
        matches!(
            service.state.kind(),
            PublisherServiceStateKind::NotInstalled | PublisherServiceStateKind::NotReachable
        )
    }) {
        return (ShowCardStateKind::Attention, "Needs attention");
    }

    if services
        .iter()
        .any(|service| matches!(service.state.kind(), PublisherServiceStateKind::Unknown))
    {
        return (ShowCardStateKind::Unknown, "Unknown");
    }

    if services
        .iter()
        .all(|service| matches!(service.state.kind(), PublisherServiceStateKind::Active))
    {
        return (ShowCardStateKind::Ok, "Active");
    }

    (ShowCardStateKind::Attention, "Needs attention")
}

fn live_metadata_service_line(service: &PublisherServiceDisplay) -> String {
    format!("{}: {}", service.label, service.state.label())
}

const fn event_card_state(state: EventState) -> ShowCardStateKind {
    match state {
        EventState::None => ShowCardStateKind::Absent,
        EventState::Unknown => ShowCardStateKind::Unknown,
        EventState::Live => ShowCardStateKind::Ok,
        EventState::Dead => ShowCardStateKind::Failed,
    }
}

const fn stream_card_state(state: StreamConnectionState) -> ShowCardStateKind {
    match state {
        StreamConnectionState::Connected => ShowCardStateKind::Ok,
        StreamConnectionState::Connecting
        | StreamConnectionState::Disconnected
        | StreamConnectionState::NotInstalled
        | StreamConnectionState::NotReachable => ShowCardStateKind::Attention,
        StreamConnectionState::Unknown | StreamConnectionState::Working => {
            ShowCardStateKind::Unknown
        }
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

        let total = report.summary.ready
            + report.summary.no_route_tag
            + report.summary.no_routes_upstream
            + report.summary.file_missing
            + report.summary.not_downloaded;
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
            detail: readiness_detail_label(
                report.summary.no_route_tag,
                report.summary.no_routes_upstream,
                report.summary.file_missing,
                report.summary.not_downloaded,
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

impl EventSectionDisplay {
    fn from_input(input: &EventSectionInput, publisher_reachable: bool) -> Self {
        let event = EventSelectionDisplay::from_input(input.selected_event.as_ref());
        let target = EventTargetAttachmentDisplay::from_input(input);
        let feed_tag = input
            .selected_event
            .as_ref()
            .map(|event| feed_tag_for_event(&event.event_id));
        let hint = event_hint(input);
        let actions = EventActionsDisplay::from_state(
            &event,
            &target,
            input.attach_target_name.trim(),
            publisher_reachable,
        );
        let summary = match &event.event_id {
            Some(event_id) => format!("{event_id} - {}", target.label),
            None => event.state.label.to_owned(),
        };

        Self {
            title: "Event",
            summary,
            event,
            target,
            feed_tag,
            hint,
            actions,
        }
    }
}

impl EventSelectionDisplay {
    fn from_input(input: Option<&EventSelectionInput>) -> Self {
        let Some(input) = input else {
            return Self {
                label: "No event selected".to_owned(),
                event_id: None,
                endpoint: None,
                token_path: None,
                state: EventState::None.display(),
            };
        };

        Self {
            label: input
                .label
                .as_deref()
                .map(str::trim)
                .filter(|label| !label.is_empty())
                .unwrap_or(&input.event_id)
                .to_owned(),
            event_id: Some(input.event_id.clone()),
            endpoint: Some(input.endpoint.clone()),
            token_path: Some(input.token_path.clone()),
            state: input.state.display(),
        }
    }
}

impl EventTargetAttachmentDisplay {
    fn from_input(input: &EventSectionInput) -> Self {
        let Some(selected_event) = input.selected_event.as_ref() else {
            return Self::not_attached("No broadcast event selected.");
        };

        match &input.targets {
            EventTargetListInput::Loaded { targets } => targets
                .iter()
                .find(|target| target.event_id == selected_event.event_id)
                .map_or_else(
                    || Self::not_attached("No publisher target carries this event."),
                    |target| Self::attached(&target.name),
                ),
            EventTargetListInput::Unknown => Self {
                state: EventTargetAttachmentState::Unknown,
                label: "Target unknown".to_owned(),
                target_name: None,
                detail: "Target list has not been read.".to_owned(),
            },
            EventTargetListInput::CommandsUnavailable => Self {
                state: EventTargetAttachmentState::CommandsUnavailable,
                label: "Target commands unavailable".to_owned(),
                target_name: None,
                detail: "Publisher target commands are not installed.".to_owned(),
            },
            EventTargetListInput::NotReachable => Self {
                state: EventTargetAttachmentState::NotReachable,
                label: "Publisher not reachable".to_owned(),
                target_name: None,
                detail: "Target list cannot be read from the host.".to_owned(),
            },
            EventTargetListInput::Failed { detail } => Self {
                state: EventTargetAttachmentState::Failed,
                label: "Target list failed".to_owned(),
                target_name: None,
                detail: detail.clone(),
            },
        }
    }

    fn attached(target_name: &str) -> Self {
        let target_name = target_name.trim();
        if target_name.is_empty() {
            return Self::not_attached("Publisher returned an empty target name.");
        }
        Self {
            state: EventTargetAttachmentState::Attached,
            label: target_name.to_owned(),
            target_name: Some(target_name.to_owned()),
            detail: "This target carries the selected event.".to_owned(),
        }
    }

    fn not_attached(detail: &str) -> Self {
        Self {
            state: EventTargetAttachmentState::NotAttached,
            label: "not attached".to_owned(),
            target_name: None,
            detail: detail.to_owned(),
        }
    }
}

impl EventActionsDisplay {
    fn from_state(
        event: &EventSelectionDisplay,
        target: &EventTargetAttachmentDisplay,
        attach_target_name: &str,
        publisher_reachable: bool,
    ) -> Self {
        let has_event = event.event_id.is_some();
        let event_live_enough = !matches!(event.state.state, EventState::Dead | EventState::None);
        let commands_ready = matches!(
            target.state,
            EventTargetAttachmentState::Attached | EventTargetAttachmentState::NotAttached
        );
        let attach_available = has_event
            && event_live_enough
            && publisher_reachable
            && commands_ready
            && matches!(target.state, EventTargetAttachmentState::NotAttached)
            && !attach_target_name.is_empty();
        let detach_available = has_event
            && publisher_reachable
            && commands_ready
            && matches!(target.state, EventTargetAttachmentState::Attached);

        Self {
            attach: EventActionDisplay {
                id: "event-target-attach",
                label: "Attach",
                a11y_label: if attach_target_name.is_empty() {
                    "Attach selected event to publisher target".to_owned()
                } else {
                    format!("Attach selected event to publisher target {attach_target_name}")
                },
                availability: event_action_availability(attach_available),
            },
            detach: EventActionDisplay {
                id: "event-target-detach",
                label: "Detach",
                a11y_label: target.target_name.as_ref().map_or_else(
                    || "Detach selected event from publisher target".to_owned(),
                    |target_name| {
                        format!("Detach selected event from publisher target {target_name}")
                    },
                ),
                availability: event_action_availability(detach_available),
            },
        }
    }
}

fn event_hint(input: &EventSectionInput) -> Option<String> {
    let event = input.selected_event.as_ref()?;
    (input.remote_host && event.token_file_missing)
        .then(|| "Token file is missing locally; copy it to the publisher host.".to_owned())
}

fn feed_tag_for_event(event_id: &str) -> String {
    format!(
        "<podcast:liveValue uri=\"{}\" protocol=\"socket.io\"/>",
        escape_xml_attribute(event_id)
    )
}

fn escape_xml_attribute(value: &str) -> String {
    value.chars().fold(String::new(), |mut escaped, character| {
        match character {
            '&' => escaped.push_str("&amp;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            _ => escaped.push(character),
        }
        escaped
    })
}

const fn event_action_availability(available: bool) -> EventActionAvailability {
    if available {
        EventActionAvailability::Available
    } else {
        EventActionAvailability::Unavailable
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
            title: "Live Metadata",
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

/// Names each reason a track is not ready, and drops a reason with no tracks.
///
/// A library row with no download is not a missing file. The operator fixes the
/// two with different actions, so the two never share a phrase.
fn readiness_detail_label(
    no_route_tag: usize,
    no_routes_upstream: usize,
    file_missing: usize,
    not_downloaded: usize,
) -> String {
    let mut parts = Vec::new();
    if no_route_tag > 0 {
        parts.push(format!("{no_route_tag} without payment routes"));
    }
    if no_routes_upstream > 0 {
        parts.push(format!("{no_routes_upstream} need publisher routes"));
    }
    if file_missing > 0 {
        parts.push(format!("{file_missing} with a missing file"));
    }
    if not_downloaded > 0 {
        parts.push(format!("{not_downloaded} not downloaded"));
    }
    if parts.is_empty() {
        return "Every scanned track carries payment routes.".to_owned();
    }
    format!("{}.", parts.join(", "))
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
        assert!(vm.event.is_none());
        assert!(vm.stream.is_none());
        assert!(vm.cards.is_empty());
        assert_eq!(vm.width_class, ShowWidthClass::default());
        assert_eq!(vm.panel_mode, ShowPanelMode::default());
        assert!(vm.panel_open);
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
    fn card_summaries_have_uniform_lines_for_each_section_state() {
        let mut cards = Vec::new();

        for state in [
            SourceReachabilityState::Reachable,
            SourceReachabilityState::NotReachable,
            SourceReachabilityState::Unknown,
        ] {
            cards.push(ShowCardDisplay::from_source(&SourceSectionDisplay {
                title: "Source",
                summary: "Local".to_owned(),
                host_name: "Local".to_owned(),
                reachability: state.display(),
                readiness: None,
            }));
        }

        cards.push(ShowCardDisplay::from_source(&SourceSectionDisplay {
            title: "Source",
            summary: "Local".to_owned(),
            host_name: "Local".to_owned(),
            reachability: SourceReachabilityState::Reachable.display(),
            readiness: Some(SourceReadinessDisplay::checking()),
        }));

        for state in service_states() {
            let snapshot = publisher_snapshot([(
                PublisherServiceRole::Publisher,
                "musicindex-live-publisher@mixxx.service",
                state,
            )]);
            let publisher =
                PublisherSectionDisplay::from_snapshot(&snapshot, PublisherLogPanelState::closed())
                    .expect("publisher section");
            cards.push(ShowCardDisplay::from_live_metadata(&publisher));
        }

        for state in [
            EventState::None,
            EventState::Unknown,
            EventState::Live,
            EventState::Dead,
        ] {
            let selected_event =
                (!matches!(state, EventState::None)).then(|| EventSelectionInput {
                    label: None,
                    event_id: "event-one".to_owned(),
                    endpoint: "https://relay.example".to_owned(),
                    token_path: "/tmp/event-one.token".to_owned(),
                    state,
                    token_file_missing: false,
                });
            let input = event_input(
                selected_event,
                EventTargetListInput::Loaded {
                    targets: Vec::new(),
                },
            );
            let event = EventSectionDisplay::from_input(&input, true);
            cards.push(ShowCardDisplay::from_event(&event));
        }

        for state in [
            EncoderState::Connected,
            EncoderState::Connecting,
            EncoderState::Disconnected,
            EncoderState::NotInstalled,
            EncoderState::NotReachable,
            EncoderState::Unknown,
        ] {
            let stream = StreamSectionDisplay::from_snapshot(&snapshot_with_encoder(
                Vec::new(),
                broadcast_service_watch::BroadcastEncoderSnapshot {
                    server_name: "Main".to_owned(),
                    configured: true,
                    status: EncoderStatus {
                        state,
                        recording: RecordingState::Unknown,
                        signal: AudioSignalState::Unknown,
                        listeners: ListenerCount::Unknown,
                        song: None,
                        stream_seconds: None,
                    },
                },
            ));
            cards.push(ShowCardDisplay::from_stream(&stream));
        }

        for card in cards {
            assert_uniform_card(card);
        }
    }

    #[test]
    fn absent_sections_leave_no_card_cell() {
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

        assert!(vm.source.is_none());
        assert!(vm.publisher.is_none());
        assert!(vm.event.is_none());
        assert_eq!(
            vm.cards.iter().map(|card| card.kind).collect::<Vec<_>>(),
            vec![ShowCardKind::Stream]
        );
    }

    #[test]
    fn cards_follow_show_card_kind_order() {
        let snapshot = publisher_snapshot([
            (
                PublisherServiceRole::Publisher,
                "musicindex-live-publisher@mixxx.service",
                ServiceState::Active,
            ),
            (
                PublisherServiceRole::Producer,
                "mixxx-now-playing.service",
                ServiceState::Active,
            ),
        ]);
        let input = event_input(
            Some(EventSelectionInput {
                label: Some("Late Night".to_owned()),
                event_id: "event-one".to_owned(),
                endpoint: "https://relay.example".to_owned(),
                token_path: "/tmp/event-one.token".to_owned(),
                state: EventState::Live,
                token_file_missing: false,
            }),
            EventTargetListInput::Loaded {
                targets: Vec::new(),
            },
        );

        let vm = ShowPageVm::from_queue_publisher_readiness_and_event(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
            None,
            Some(&input),
        );

        assert_eq!(
            vm.cards.iter().map(|card| card.kind).collect::<Vec<_>>(),
            ShowCardKind::ORDER.to_vec()
        );
    }

    #[test]
    fn show_width_classes_return_column_counts() {
        assert_eq!(ShowWidthClass::Compact.columns(), 1);
        assert_eq!(ShowWidthClass::Medium.columns(), 2);
        assert_eq!(ShowWidthClass::Wide.columns(), 3);
    }

    #[test]
    fn window_width_selects_show_width_class() {
        assert_eq!(
            ShowWidthClass::for_window_width(711.0),
            ShowWidthClass::Compact
        );
        assert_eq!(
            ShowWidthClass::for_window_width(712.0),
            ShowWidthClass::Medium
        );
        assert_eq!(
            ShowWidthClass::for_window_width(1_055.0),
            ShowWidthClass::Medium
        );
        assert_eq!(
            ShowWidthClass::for_window_width(1_056.0),
            ShowWidthClass::Wide
        );
        assert_eq!(
            ShowPageVm::idle().with_window_width(711.0).width_class,
            ShowWidthClass::Compact
        );
    }

    #[test]
    fn show_panel_mode_defaults_to_cuelist_and_detail_holds_one_card() {
        assert_eq!(ShowPanelMode::default(), ShowPanelMode::Cuelist);

        let detail = ShowPanelMode::Detail(ShowCardKind::Event);
        let ShowPanelMode::Detail(kind) = detail else {
            panic!("detail mode must hold one card kind");
        };
        assert_eq!(kind, ShowCardKind::Event);
    }

    #[test]
    fn selecting_show_card_sets_detail_panel_mode() {
        let vm = ShowPageVm::idle().select_card(ShowCardKind::Stream);

        assert_eq!(vm.panel_mode, ShowPanelMode::Detail(ShowCardKind::Stream));
        assert!(vm.panel_open);
        assert!(!vm.panel_chrome.show_cuelist.disabled());
    }

    #[test]
    fn closing_show_detail_returns_to_cuelist_and_leaves_panel_open() {
        let vm = ShowPageVm::idle()
            .select_card(ShowCardKind::LiveMetadata)
            .show_cuelist_panel();

        assert_eq!(vm.panel_mode, ShowPanelMode::Cuelist);
        assert!(vm.panel_open);
        assert!(vm.panel_chrome.show_cuelist.disabled());
    }

    #[test]
    fn selecting_show_card_while_panel_closed_reopens_panel() {
        let vm = ShowPageVm::idle()
            .close_panel()
            .select_card(ShowCardKind::Source);

        assert_eq!(vm.panel_mode, ShowPanelMode::Detail(ShowCardKind::Source));
        assert!(vm.panel_open);
    }

    #[test]
    fn show_panel_mode_is_exclusive_and_preserved_across_projection() {
        let detail = ShowPanelMode::Detail(ShowCardKind::Event);
        assert!(!matches!(detail, ShowPanelMode::Cuelist));

        let vm = ShowPageVm::from_queue(QueueNowPlayingPageVm::builder().build())
            .with_panel_state(detail, false);

        assert_eq!(vm.panel_mode, detail);
        assert!(!vm.panel_open);
        assert!(!vm.panel_chrome.show_cuelist.disabled());
    }

    #[test]
    fn live_metadata_card_failed_when_any_service_failed() {
        let snapshot = publisher_snapshot([
            (
                PublisherServiceRole::Publisher,
                "musicindex-live-publisher@mixxx.service",
                ServiceState::Active,
            ),
            (
                PublisherServiceRole::Producer,
                "mixxx-now-playing.service",
                ServiceState::Failed {
                    reason: "exit-code".to_owned(),
                },
            ),
        ]);
        let vm = ShowPageVm::from_queue_and_publisher(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
        );
        let card = vm
            .cards
            .iter()
            .find(|card| card.kind == ShowCardKind::LiveMetadata)
            .expect("live metadata card");

        assert_eq!(card.state, ShowCardStateKind::Failed);
        assert_eq!(card.state_label, "Failed");
    }

    #[test]
    fn live_metadata_card_uses_two_service_lines() {
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
        let card = vm
            .cards
            .iter()
            .find(|card| card.kind == ShowCardKind::LiveMetadata)
            .expect("live metadata card");

        assert_eq!(card.primary, "Publisher: Active");
        assert_eq!(card.secondary, "Producer: Inactive");
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
        assert_eq!(publisher.title, "Live Metadata");
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
        assert_eq!(service.state.detail(), "Reason: exit-code");
        assert!(service.actions.start.disabled());
        assert!(!service.actions.reset.disabled());
    }

    #[test]
    fn every_publisher_service_state_returns_a_detail_line() {
        // An optional detail line changed the height of the publisher strip every
        // time a unit started, failed, or restarted, and moved the log panel with
        // it. Every state must return a line so the row height stays fixed.
        let states = [
            PublisherServiceStateDisplay::Active,
            PublisherServiceStateDisplay::Inactive,
            PublisherServiceStateDisplay::Failed {
                reason: "start-limit-hit".to_owned(),
            },
            PublisherServiceStateDisplay::NotInstalled,
            PublisherServiceStateDisplay::NotReachable,
            PublisherServiceStateDisplay::Unknown,
        ];

        for state in states {
            assert!(
                !state.detail().trim().is_empty(),
                "state {state:?} returns an empty detail line"
            );
        }
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
        assert_eq!(service.state.detail(), "Unit file not found.");
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
                    not_downloaded: 0,
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
        assert_eq!(
            readiness.detail,
            "1 without payment routes, 1 with a missing file."
        );
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
    fn publisher_service_state_exposes_nine_variants_without_transport_error_payload() {
        assert_eq!(PublisherServiceStateDisplay::ALL_KINDS.len(), 9);
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
    fn event_section_projects_attached_target_and_feed_tag() {
        let snapshot = publisher_snapshot([(
            PublisherServiceRole::Publisher,
            "musicindex-live-publisher@mixxx.service",
            ServiceState::Active,
        )]);
        let input = event_input(
            Some(EventSelectionInput {
                label: Some("Late Night".to_owned()),
                event_id: "event&one".to_owned(),
                endpoint: "https://relay.example".to_owned(),
                token_path: "/tmp/event-one.token".to_owned(),
                state: EventState::Live,
                token_file_missing: false,
            }),
            EventTargetListInput::Loaded {
                targets: vec![EventTargetInput {
                    name: "late-night".to_owned(),
                    event_id: "event&one".to_owned(),
                }],
            },
        );

        let vm = ShowPageVm::from_queue_publisher_readiness_and_event(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
            None,
            Some(&input),
        );
        let event = vm.event.expect("event section");

        assert_eq!(event.title, "Event");
        assert_eq!(event.event.label, "Late Night");
        assert_eq!(event.event.state.state, EventState::Live);
        assert_eq!(event.target.state, EventTargetAttachmentState::Attached);
        assert_eq!(event.target.label, "late-night");
        assert_eq!(event.target.target_name.as_deref(), Some("late-night"));
        assert_eq!(
            event.feed_tag.as_deref(),
            Some("<podcast:liveValue uri=\"event&amp;one\" protocol=\"socket.io\"/>")
        );
        assert!(event.actions.attach.disabled());
        assert!(!event.actions.detach.disabled());
    }

    #[test]
    fn event_section_projects_not_attached_without_empty_target_label() {
        let snapshot = publisher_snapshot([(
            PublisherServiceRole::Publisher,
            "musicindex-live-publisher@mixxx.service",
            ServiceState::Active,
        )]);
        let input = event_input(
            Some(EventSelectionInput {
                label: None,
                event_id: "event-one".to_owned(),
                endpoint: "https://relay.example".to_owned(),
                token_path: "/tmp/event-one.token".to_owned(),
                state: EventState::Unknown,
                token_file_missing: false,
            }),
            EventTargetListInput::Loaded {
                targets: vec![EventTargetInput {
                    name: "default".to_owned(),
                    event_id: "event-two".to_owned(),
                }],
            },
        );

        let vm = ShowPageVm::from_queue_publisher_readiness_and_event(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
            None,
            Some(&input),
        );
        let event = vm.event.expect("event section");

        assert_eq!(event.target.state, EventTargetAttachmentState::NotAttached);
        assert_eq!(event.target.label, "not attached");
        assert!(!event.target.label.is_empty());
        assert_eq!(event.target.target_name, None);
        assert!(!event.actions.attach.disabled());
        assert!(event.actions.detach.disabled());
    }

    #[test]
    fn event_section_disables_attach_for_dead_event_or_unreachable_publisher() {
        let reachable = publisher_snapshot([(
            PublisherServiceRole::Publisher,
            "musicindex-live-publisher@mixxx.service",
            ServiceState::Active,
        )]);
        let unreachable = publisher_snapshot_on_host(
            "Studio",
            [(
                PublisherServiceRole::Publisher,
                "musicindex-live-publisher@mixxx.service",
                ServiceState::NotReachable,
            )],
        );
        let input = event_input(
            Some(EventSelectionInput {
                label: None,
                event_id: "event-one".to_owned(),
                endpoint: "https://relay.example".to_owned(),
                token_path: "/tmp/event-one.token".to_owned(),
                state: EventState::Dead,
                token_file_missing: false,
            }),
            EventTargetListInput::Loaded {
                targets: Vec::new(),
            },
        );

        let dead_vm = ShowPageVm::from_queue_publisher_readiness_and_event(
            QueueNowPlayingPageVm::builder().build(),
            Some(&reachable),
            PublisherLogPanelState::closed(),
            None,
            Some(&input),
        );
        let live_input = EventSectionInput {
            selected_event: Some(EventSelectionInput {
                state: EventState::Live,
                ..input.selected_event.clone().expect("event")
            }),
            ..input.clone()
        };
        let unreachable_vm = ShowPageVm::from_queue_publisher_readiness_and_event(
            QueueNowPlayingPageVm::builder().build(),
            Some(&unreachable),
            PublisherLogPanelState::closed(),
            None,
            Some(&live_input),
        );

        assert!(dead_vm
            .event
            .expect("dead event section")
            .actions
            .attach
            .disabled());
        assert!(unreachable_vm
            .event
            .expect("unreachable event section")
            .actions
            .attach
            .disabled());
    }

    #[test]
    fn event_section_reports_unavailable_target_commands() {
        let snapshot = publisher_snapshot([(
            PublisherServiceRole::Publisher,
            "musicindex-live-publisher@mixxx.service",
            ServiceState::Active,
        )]);
        let input = event_input(
            Some(EventSelectionInput {
                label: None,
                event_id: "event-one".to_owned(),
                endpoint: "https://relay.example".to_owned(),
                token_path: "/tmp/event-one.token".to_owned(),
                state: EventState::Live,
                token_file_missing: false,
            }),
            EventTargetListInput::CommandsUnavailable,
        );

        let vm = ShowPageVm::from_queue_publisher_readiness_and_event(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
            None,
            Some(&input),
        );
        let event = vm.event.expect("event section");

        assert_eq!(
            event.target.state,
            EventTargetAttachmentState::CommandsUnavailable
        );
        assert_eq!(event.target.label, "Target commands unavailable");
        assert!(event.actions.attach.disabled());
        assert!(event.actions.detach.disabled());
    }

    #[test]
    fn event_section_hints_when_remote_token_file_is_missing() {
        let snapshot = publisher_snapshot_on_host(
            "Studio",
            [(
                PublisherServiceRole::Publisher,
                "musicindex-live-publisher@mixxx.service",
                ServiceState::Active,
            )],
        );
        let mut input = event_input(
            Some(EventSelectionInput {
                label: None,
                event_id: "event-one".to_owned(),
                endpoint: "https://relay.example".to_owned(),
                token_path: "/tmp/event-one.token".to_owned(),
                state: EventState::Live,
                token_file_missing: true,
            }),
            EventTargetListInput::Loaded {
                targets: Vec::new(),
            },
        );
        input.remote_host = true;

        let vm = ShowPageVm::from_queue_publisher_readiness_and_event(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            PublisherLogPanelState::closed(),
            None,
            Some(&input),
        );

        assert_eq!(
            vm.event.expect("event section").hint.as_deref(),
            Some("Token file is missing locally; copy it to the publisher host.")
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

    #[test]
    fn log_panel_reports_the_role_it_shows() {
        // The `Logs` action cycles, so it asks the panel what it already shows.
        let closed = PublisherLogPanelState::closed();
        assert!(!closed.shows_role(PublisherServiceRole::Publisher));

        let open = PublisherLogPanelState::Open {
            role: PublisherServiceRole::Publisher,
            unit_name: "musicindex-live-publisher@mixxx.service".to_owned(),
            line_count: 50,
            text: "one line\n".to_owned(),
        };
        assert!(open.shows_role(PublisherServiceRole::Publisher));
        assert!(
            !open.shows_role(PublisherServiceRole::Producer),
            "the other service switches the panel, it does not close it"
        );
    }

    #[test]
    fn show_page_carries_a_command_message() {
        // Every broadcast command wrote its error to `settings_status`, which
        // only `Settings` rendered. A failed Connect on `Show` looked like a
        // press that did nothing at all.
        let vm = ShowPageVm::idle();
        assert_eq!(vm.status_message, None);

        let vm = vm.with_status_message("Stream command error: butt refused");
        assert_eq!(
            vm.status_message.as_deref(),
            Some("Stream command error: butt refused")
        );

        let vm = vm.with_status_message("   ");
        assert_eq!(vm.status_message, None, "blank text clears the message");
    }

    #[test]
    fn opening_logs_does_not_close_the_panel() {
        // `select_card` toggles, so the log action must not use it. Clicking
        // `Logs` on the open card closed the panel instead of showing the log.
        let vm = ShowPageVm::idle().select_card(ShowCardKind::LiveMetadata);
        assert!(vm.panel_open);

        let vm = vm.show_card_detail(ShowCardKind::LiveMetadata);

        assert!(
            vm.panel_open,
            "an explicit detail open never closes the panel"
        );
        assert_eq!(
            vm.panel_mode,
            ShowPanelMode::Detail(ShowCardKind::LiveMetadata)
        );
    }

    #[test]
    fn selecting_the_open_card_again_closes_the_panel() {
        let vm = ShowPageVm::idle().select_card(ShowCardKind::Source);
        assert!(vm.panel_open);
        assert_eq!(vm.panel_mode, ShowPanelMode::Detail(ShowCardKind::Source));

        let vm = vm.select_card(ShowCardKind::Source);
        assert!(!vm.panel_open, "a second select closes the panel");

        let vm = vm.select_card(ShowCardKind::Source);
        assert!(vm.panel_open, "a third select opens it again");
    }

    #[test]
    fn selecting_a_different_card_keeps_the_panel_open() {
        let vm = ShowPageVm::idle()
            .select_card(ShowCardKind::Source)
            .select_card(ShowCardKind::Stream);
        assert!(vm.panel_open);
        assert_eq!(vm.panel_mode, ShowPanelMode::Detail(ShowCardKind::Stream));
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

    fn event_input(
        selected_event: Option<EventSelectionInput>,
        targets: EventTargetListInput,
    ) -> EventSectionInput {
        EventSectionInput {
            selected_event,
            targets,
            attach_target_name: "default".to_owned(),
            remote_host: false,
        }
    }

    fn service_states() -> Vec<ServiceState> {
        vec![
            ServiceState::Active,
            ServiceState::Inactive,
            ServiceState::Starting,
            ServiceState::Stopping,
            ServiceState::Failed {
                reason: "exit-code".to_owned(),
            },
            ServiceState::NotInstalled,
            ServiceState::NotReachable,
            ServiceState::Unknown,
        ]
    }

    fn assert_uniform_card(card: ShowCardDisplay) {
        assert!(
            !card.title.trim().is_empty(),
            "card {card:?} has an empty title"
        );
        assert!(
            !card.state_label.trim().is_empty(),
            "card {card:?} has an empty state label"
        );
        assert!(
            !card.primary.trim().is_empty(),
            "card {card:?} has an empty primary line"
        );
        assert!(
            card.secondary.is_empty() || !card.secondary.trim().is_empty(),
            "card {card:?} has a whitespace-only secondary line"
        );
        assert!(
            !card.a11y_label.trim().is_empty(),
            "card {card:?} has an empty accessibility label"
        );
    }
}
