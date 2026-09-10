//! Show surface display contract.
//!
//! ADR 0060 mounts `Show` beside the curation and settings sections. This
//! module wraps the existing Queue/Now Playing projection with page-level
//! show state while keeping renderer and playback handles out of the view
//! model layer.

#![warn(clippy::pedantic)]

mod event_report;
pub(crate) use event_report::{EventCheckResponse, EventReportOperation};

use std::collections::HashMap;
use std::time::Instant;

use crate::broadcast::{
    control::ServiceState,
    encoder::{AudioSignalState, EncoderState, ListenerCount, RecordingState},
};
use crate::runtime::{broadcast_service_watch, BroadcastReadinessSnapshot};
use crate::view_models::queue_now_playing::{
    QueueNowPlayingPageVm, QueueRowDisplay, TransportState,
};

pub(crate) const PUBLISHER_LOG_LINE_COUNT: usize = 50;

/// Service command intent owned by Show (ADR 0059).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublisherServiceOperation {
    Start,
    Stop,
    Reset,
}

/// Stream command intent owned by Show (ADR 0059).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StreamEncoderOperation {
    Connect,
    Disconnect,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum ShowCommandTarget {
    Service(PublisherServiceRole),
    Stream,
}

/// Identity of one press; obsolete completions cannot release a later press.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ShowCommandId {
    target: ShowCommandTarget,
    sequence: u64,
}

#[derive(Clone, Copy, Debug)]
enum ShowCommandIntent {
    Service(PublisherServiceRole, PublisherServiceOperation),
    Stream(StreamEncoderOperation),
}

impl ShowCommandIntent {
    fn target(self) -> ShowCommandTarget {
        match self {
            Self::Service(role, _) => ShowCommandTarget::Service(role),
            Self::Stream(_) => ShowCommandTarget::Stream,
        }
    }

    fn resolved_by(
        self,
        snapshot: &broadcast_service_watch::BroadcastServiceWatchSnapshot,
    ) -> bool {
        match self {
            Self::Service(role, operation) => snapshot
                .units
                .iter()
                .find(|unit| unit.role == role)
                .is_some_and(|unit| match unit.state {
                    ServiceState::Failed { .. }
                    | ServiceState::NotInstalled
                    | ServiceState::NotReachable => true,
                    ServiceState::Active => operation != PublisherServiceOperation::Stop,
                    ServiceState::Inactive => operation != PublisherServiceOperation::Start,
                    ServiceState::Starting | ServiceState::Stopping | ServiceState::Unknown => {
                        false
                    }
                }),
            Self::Stream(operation) => match snapshot.encoder.status.state {
                EncoderState::Connected => operation == StreamEncoderOperation::Connect,
                EncoderState::Disconnected => operation == StreamEncoderOperation::Disconnect,
                EncoderState::NotInstalled | EncoderState::NotReachable => true,
                EncoderState::Connecting | EncoderState::Unknown => false,
            },
        }
    }
}

#[derive(Debug)]
struct ShowCommandProgress {
    id: ShowCommandId,
    intent: ShowCommandIntent,
    /// None means the command still owns its controls. Only its result releases ownership.
    returned_at: Option<Instant>,
    last_read_started_at: Option<Instant>,
    fresh_samples: u8,
}

/// Retained command ownership and observation release policy for Show (ADR 0059).
#[derive(Debug, Default)]
pub(crate) struct ShowCommandState {
    next_sequence: u64,
    progress: HashMap<ShowCommandTarget, ShowCommandProgress>,
}

impl ShowCommandState {
    /// Begin one service operation, rejecting another operation on the same role.
    pub(crate) fn begin_service(
        &mut self,
        role: PublisherServiceRole,
        operation: PublisherServiceOperation,
    ) -> Option<ShowCommandId> {
        self.begin(ShowCommandIntent::Service(role, operation))
    }

    /// Begin one stream operation, rejecting another operation on the stream.
    pub(crate) fn begin_stream(
        &mut self,
        operation: StreamEncoderOperation,
    ) -> Option<ShowCommandId> {
        self.begin(ShowCommandIntent::Stream(operation))
    }

    fn begin(&mut self, intent: ShowCommandIntent) -> Option<ShowCommandId> {
        let target = intent.target();
        if self.progress.contains_key(&target) {
            return None;
        }
        self.next_sequence = self.next_sequence.checked_add(1)?;
        let id = ShowCommandId {
            target,
            sequence: self.next_sequence,
        };
        self.progress.insert(
            target,
            ShowCommandProgress {
                id,
                intent,
                returned_at: None,
                last_read_started_at: None,
                fresh_samples: 0,
            },
        );
        Some(id)
    }

    /// Apply only the matching result. Success waits for readback; failure restores observed state.
    pub(crate) fn complete(
        &mut self,
        id: ShowCommandId,
        returned_at: Instant,
        succeeded: bool,
    ) -> bool {
        let Some(progress) = self.progress.get_mut(&id.target) else {
            return false;
        };
        if progress.id != id || progress.returned_at.is_some() {
            return false;
        }
        if succeeded {
            progress.returned_at = Some(returned_at);
        } else {
            self.progress.remove(&id.target);
        }
        true
    }

    /// Reconcile a watch delivery once; projection alone must not count as another sample.
    pub(crate) fn observe(
        &mut self,
        snapshot: &broadcast_service_watch::BroadcastServiceWatchSnapshot,
    ) {
        const MAX_FRESH_SAMPLES: u8 = 3;
        self.progress.retain(|_, progress| {
            let Some(returned_at) = progress.returned_at else {
                return true;
            };
            if snapshot.read_started_at <= returned_at
                || progress
                    .last_read_started_at
                    .is_some_and(|last| snapshot.read_started_at <= last)
            {
                return true;
            }
            progress.last_read_started_at = Some(snapshot.read_started_at);
            progress.fresh_samples += 1;
            !progress.intent.resolved_by(snapshot) && progress.fresh_samples < MAX_FRESH_SAMPLES
        });
    }

    /// Invalidate the old host's commands without reusing their sequence numbers.
    pub(crate) fn clear(&mut self) {
        self.progress.clear();
    }
}

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
    /// Whether this unit's bottom log pane is open.
    pub(crate) open: bool,
    /// Action that cycles the bottom log pane.
    pub(crate) action: PublisherActionDisplay,
}

/// Display-ready service row for the Publisher section.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublisherServiceDisplay {
    pub(crate) badge: ShowItemBadge,
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

/// Identifies one log read, including successive reads of the same service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ShowLogRequestId(u64);

/// Display-ready bottom pane and request state owned by Show (ADR 0063).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShowLogPaneDisplay {
    pub(crate) open: bool,
    pub(crate) unit_name: String,
    pub(crate) text: String,
    pub(crate) line_count: usize,
    pub(crate) line_count_label: String,
    /// Preferred pane height in unscaled layout units.
    pub(crate) height: f32,
    /// Last allocated split height, excluding the transport.
    pub(crate) region_height: f32,
    available_height: f32,
    pub(crate) close: PublisherActionDisplay,
    role: Option<PublisherServiceRole>,
    event_source: Option<String>,
    pub(crate) service_detail: Option<String>,
    pub(crate) feed_tag: Option<String>,
    pub(crate) copy_feed_tag: Option<EventActionDisplay>,
    request: Option<ShowLogRequestId>,
    next_request: u64,
}

pub(crate) const SHOW_LOG_DEFAULT_HEIGHT: f32 = 200.0;
pub(crate) const SHOW_LOG_MIN_HEIGHT: f32 = 96.0;
pub(crate) const SHOW_LOG_MAX_HEIGHT: f32 = 600.0;

impl ShowLogPaneDisplay {
    /// Creates a closed pane with no outstanding read.
    #[must_use]
    pub(crate) fn closed() -> Self {
        Self {
            open: false,
            unit_name: String::new(),
            text: String::new(),
            line_count: 0,
            line_count_label: String::new(),
            height: SHOW_LOG_DEFAULT_HEIGHT,
            region_height: 0.0,
            available_height: SHOW_LOG_MAX_HEIGHT,
            close: PublisherActionDisplay {
                id: "show-log-pane-close".to_owned(),
                label: "Close",
                a11y_label: "Close logs".to_owned(),
                availability: PublisherActionAvailability::Unavailable,
            },
            role: None,
            event_source: None,
            service_detail: None,
            feed_tag: None,
            copy_feed_tag: None,
            request: None,
            next_request: 0,
        }
    }

    #[must_use]
    pub(crate) fn shows_role(&self, role: PublisherServiceRole) -> bool {
        self.open && self.role == Some(role)
    }

    fn begin_read(&mut self, role: PublisherServiceRole, unit_name: String) -> ShowLogRequestId {
        self.next_request += 1;
        let request = ShowLogRequestId(self.next_request);
        self.request = Some(request);
        self.role = Some(role);
        self.feed_tag = None;
        self.copy_feed_tag = None;
        self.service_detail = None;
        self.event_source = None;
        self.open = true;
        self.unit_name = unit_name;
        "Reading service logs…".clone_into(&mut self.text);
        self.line_count = 0;
        "Reading logs".clone_into(&mut self.line_count_label);
        self.close.availability = PublisherActionAvailability::Available;
        request
    }

    fn close(&mut self) {
        self.open = false;
        self.request = None;
        self.role = None;
        self.event_source = None;
        self.close.availability = PublisherActionAvailability::Unavailable;
    }

    /// Applies only the current read, including its failure, without reopening a pane.
    pub(crate) fn apply_result(
        &mut self,
        request: ShowLogRequestId,
        result: Result<String, String>,
    ) -> bool {
        if !self.open || self.request != Some(request) {
            return false;
        }
        self.request = None;
        match result {
            Ok(text) => {
                self.line_count = text.lines().count();
                self.line_count_label = format!("{} log lines", self.line_count);
                self.text = if text.is_empty() {
                    "No log lines returned.".to_owned()
                } else {
                    text
                };
            }
            Err(message) => {
                self.line_count = 0;
                "Log read failed".clone_into(&mut self.line_count_label);
                self.text = message;
            }
        }
        true
    }

    /// Stores a bounded preferred height; the layout also reserves the card grid's height.
    pub(crate) fn resize(&mut self, height: f32) {
        if height.is_finite() {
            let maximum = self.available_height.min(SHOW_LOG_MAX_HEIGHT);
            self.height = height.clamp(SHOW_LOG_MIN_HEIGHT.min(maximum), maximum);
        }
    }

    /// Reserves the measured grid and shared handle before bounding log height.
    pub(crate) fn update_geometry(&mut self, region_height: f32, available_height: f32) -> bool {
        if !region_height.is_finite() || !available_height.is_finite() {
            return false;
        }
        let changed = (self.region_height - region_height).abs() > f32::EPSILON
            || (self.available_height - available_height).abs() > f32::EPSILON;
        self.region_height = region_height;
        self.available_height = available_height.max(0.0);
        self.resize(self.height);
        changed
    }
}

/// Display-ready Publisher section for the `Show` screen mount.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublisherSectionDisplay {
    /// Stable section title.
    pub(crate) title: &'static str,
    /// Combined readiness of the event, attachment, and services.
    pub(crate) summary: String,
    /// Event setup precedes the producer and publisher (ADR 0059).
    pub(crate) event: Option<EventSectionDisplay>,
    /// Observed service rows, in fixed section order.
    pub(crate) services: Vec<PublisherServiceDisplay>,
}

/// Display-ready Event section for the `Show` screen mount.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventSectionDisplay {
    pub(crate) badge: ShowItemBadge,
    pub(crate) activity: Option<&'static str>,
    pub(crate) picker: EventPickerDisplay,
    pub(crate) primary: Option<EventControlDisplay>,
    pub(crate) overflow: Vec<EventControlDisplay>,
    pub(crate) logs: EventActionDisplay,
    pub(crate) diagnostics: String,
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
    /// Registration result, independent of the liveness check.
    pub(crate) registration_message: Option<String>,
    /// Liveness progress or result.
    pub(crate) check_message: Option<String>,
    /// Event setup and target action state.
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
                detail: "The app has not confirmed whether the relay still has this event.",
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
    /// Register the first event.
    pub(crate) create: EventActionDisplay,
    /// Register a replacement for a dead event.
    pub(crate) replace: EventActionDisplay,
    /// Check an unknown event, including retries.
    pub(crate) check: EventActionDisplay,
    /// Copy the listener feed tag.
    pub(crate) copy_feed_tag: EventActionDisplay,
    /// Attach action.
    pub(crate) attach: EventActionDisplay,
    /// Detach action.
    pub(crate) detach: EventActionDisplay,
}

/// Operator intent for registry commands (ADR 0059).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EventRegistryAction {
    Create,
    Replace,
    Check,
}

/// A command result retained across mounted-view reprojection.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) enum EventCommandState {
    #[default]
    Idle,
    Working,
    Succeeded,
    Failed {
        detail: String,
    },
}

/// Registration and checking have independent results (ADR 0059).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct EventCommandFeedback {
    report: event_report::EventReport,
    pub(crate) check_response: EventCheckResponse,
    pub(crate) verification_failure: Option<String>,
    pub(crate) selection: EventCommandState,
    pub(crate) target_read: EventCommandState,
    pub(crate) target_mutation: EventCommandState,
    pub(crate) active_action: Option<EventControlIntent>,
    pub(crate) registration: EventCommandState,
    pub(crate) check: EventCommandState,
}

impl EventCommandFeedback {
    pub(crate) fn working(&self) -> bool {
        matches!(self.registration, EventCommandState::Working)
            || matches!(self.check, EventCommandState::Working)
            || matches!(self.selection, EventCommandState::Working)
            || matches!(self.target_read, EventCommandState::Working)
            || matches!(self.target_mutation, EventCommandState::Working)
    }

    fn registration_message(&self) -> Option<String> {
        match &self.registration {
            EventCommandState::Idle => None,
            EventCommandState::Working => Some("Registering event…".to_owned()),
            EventCommandState::Succeeded => Some("Event registered. Token file saved.".to_owned()),
            EventCommandState::Failed { detail } => Some(format!("Registration failed: {detail}")),
        }
    }

    fn check_message(&self) -> Option<String> {
        match &self.check {
            EventCommandState::Idle => None,
            EventCommandState::Working => Some("Checking event…".to_owned()),
            EventCommandState::Succeeded => Some("Event check complete.".to_owned()),
            EventCommandState::Failed { detail } => Some(format!("Event check failed: {detail}")),
        }
    }
}

impl EventActionsDisplay {
    pub(crate) fn registry_action(&self, action: EventRegistryAction) -> &EventActionDisplay {
        match action {
            EventRegistryAction::Create => &self.create,
            EventRegistryAction::Replace => &self.replace,
            EventRegistryAction::Check => &self.check,
        }
    }
}

/// Input for projecting an Event section.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventSectionInput {
    pub(crate) liveness_confirmed: bool,
    pub(crate) registry: EventRegistryInput,
    pub(crate) context: String,
    pub(crate) request: u64,
    /// Independent registration and liveness results.
    pub(crate) feedback: EventCommandFeedback,
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
    pub(crate) created_at: i64,
    pub(crate) last_checked_at: Option<i64>,
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
    /// Stream encoder status.
    Stream,
}

impl ShowCardKind {
    const ORDER: [Self; 3] = [Self::Source, Self::LiveMetadata, Self::Stream];

    /// Returns the stable visible title for this card kind.
    #[must_use]
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::LiveMetadata => "Live Metadata",
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
    /// Bottom log pane, independent of the side-panel mode.
    pub(crate) log_pane: ShowLogPaneDisplay,
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
        Self::from_queue_and_publisher(queue, None, ShowLogPaneDisplay::closed())
    }

    /// Projects the Show page from queue display and publisher service state.
    #[must_use]
    pub(crate) fn from_queue_and_publisher(
        queue: QueueNowPlayingPageVm,
        publisher_snapshot: Option<&broadcast_service_watch::BroadcastServiceWatchSnapshot>,
        log_pane: ShowLogPaneDisplay,
    ) -> Self {
        Self::from_queue_publisher_and_readiness(queue, publisher_snapshot, log_pane, None)
    }

    /// Projects the Show page from queue, publisher state, and readiness state.
    #[must_use]
    pub(crate) fn from_queue_publisher_and_readiness(
        queue: QueueNowPlayingPageVm,
        publisher_snapshot: Option<&broadcast_service_watch::BroadcastServiceWatchSnapshot>,
        log_pane: ShowLogPaneDisplay,
        readiness_snapshot: Option<&BroadcastReadinessSnapshot>,
    ) -> Self {
        Self::from_queue_publisher_readiness_and_event(
            queue,
            publisher_snapshot,
            log_pane,
            readiness_snapshot,
            None,
        )
    }

    /// Projects the Show page from queue, publisher, readiness, and event state.
    #[must_use]
    pub(crate) fn from_queue_publisher_readiness_and_event(
        queue: QueueNowPlayingPageVm,
        publisher_snapshot: Option<&broadcast_service_watch::BroadcastServiceWatchSnapshot>,
        log_pane: ShowLogPaneDisplay,
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
        let publisher_reachable = publisher_snapshot.is_some_and(|snapshot| {
            matches!(
                source_reachability(snapshot.units.as_slice()),
                SourceReachabilityState::Reachable
            )
        });
        let event =
            event_input.map(|input| EventSectionDisplay::from_input(input, publisher_reachable));
        let publisher =
            PublisherSectionDisplay::from_snapshot(publisher_snapshot, &log_pane, event);
        let stream = publisher_snapshot.map(StreamSectionDisplay::from_snapshot);
        let cards = show_cards(source.as_ref(), publisher.as_ref(), stream.as_ref());
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
            cards,
            width_class: ShowWidthClass::default(),
            panel_mode: ShowPanelMode::default(),
            panel_open: true,
            status_message: None,
            panel_chrome: ShowPanelChromeDisplay::default(),
            log_pane,
            queue,
        }
    }

    /// Cycles the Logs action and returns an identifier only when a read is needed.
    pub(crate) fn toggle_publisher_logs(
        &mut self,
        role: PublisherServiceRole,
    ) -> Option<ShowLogRequestId> {
        if self.log_pane.shows_role(role) {
            self.close_publisher_logs();
            return None;
        }
        let service = self
            .publisher
            .as_ref()?
            .services
            .iter()
            .find(|service| service.role == role)?;
        if service.logs.action.disabled() {
            return None;
        }
        let request = self.log_pane.begin_read(role, service.unit_name.clone());
        self.log_pane.service_detail = Some(format!(
            "{}: {}\n{}",
            service.label,
            service.state.label(),
            service.state.detail()
        ));
        self.refresh_log_actions();
        Some(request)
    }

    /// Closes logs and invalidates the pending read without changing the detail panel.
    pub(crate) fn close_publisher_logs(&mut self) {
        self.log_pane.close();
        self.refresh_log_actions();
    }

    fn refresh_log_actions(&mut self) {
        if let Some(publisher) = &mut self.publisher {
            for service in &mut publisher.services {
                service.logs = service_logs(
                    service.role,
                    service.label,
                    &service.unit_name,
                    &service.state,
                    &self.log_pane,
                );
            }
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
    /// in the meantime. The retained command state determines when readback
    /// can replace it (ADR 0059).
    #[must_use]
    pub(crate) fn mark_service_working(mut self, role: PublisherServiceRole) -> Self {
        if let Some(publisher) = self.publisher.as_mut() {
            for service in &mut publisher.services {
                if service.role == role {
                    service.state = PublisherServiceStateDisplay::Working;
                    service.badge = service_badge(&service.state);
                    service.actions = service_actions(role, service.label, &service.state);
                }
            }
        }
        if let Some(publisher) = self.publisher.as_mut() {
            if role == PublisherServiceRole::Publisher {
                if let Some(event) = &mut publisher.event {
                    event.actions.attach.availability = EventActionAvailability::Unavailable;
                    event.actions.detach.availability = EventActionAvailability::Unavailable;
                    if let Some(primary) = &mut event.primary {
                        if matches!(
                            primary.intent,
                            EventControlIntent::Attach | EventControlIntent::Detach
                        ) {
                            primary.action.availability = EventActionAvailability::Unavailable;
                        }
                    }
                    for control in &mut event.overflow {
                        if control.intent == EventControlIntent::Detach {
                            control.action.availability = EventActionAvailability::Unavailable;
                        }
                    }
                }
            }
            publisher.update_summary();
        }
        self.cards = show_cards(
            self.source.as_ref(),
            self.publisher.as_ref(),
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
            stream.actions = stream_actions(true, StreamConnectionState::Working);
            stream.summary = format!("{} - {}", stream.server_label, stream.connection.label);
        }
        self.cards = show_cards(
            self.source.as_ref(),
            self.publisher.as_ref(),
            self.stream.as_ref(),
        );
        self
    }

    /// Apply retained transitions after every projection, including queue and event refreshes.
    #[must_use]
    pub(crate) fn with_command_state(mut self, commands: &ShowCommandState) -> Self {
        for target in commands.progress.keys() {
            self = match target {
                ShowCommandTarget::Service(role) => self.mark_service_working(*role),
                ShowCommandTarget::Stream => self.mark_stream_working(),
            };
        }
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
    stream: Option<&StreamSectionDisplay>,
) -> Vec<ShowCardDisplay> {
    ShowCardKind::ORDER
        .into_iter()
        .filter_map(|kind| match kind {
            ShowCardKind::Source => source.map(ShowCardDisplay::from_source),
            ShowCardKind::LiveMetadata => publisher.map(ShowCardDisplay::from_live_metadata),
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
        let (state, primary) = live_metadata_card_state(section);
        let state_label = if state == ShowCardStateKind::Ok {
            "Ready"
        } else {
            "Not ready"
        };
        Self::new(
            ShowCardKind::LiveMetadata,
            state_label,
            state,
            primary,
            section.summary.clone(),
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

fn live_metadata_card_state(section: &PublisherSectionDisplay) -> (ShowCardStateKind, String) {
    let Some(event) = &section.event else {
        return (ShowCardStateKind::Unknown, "Event: loading".to_owned());
    };
    if event.badge.kind != ShowCardStateKind::Ok {
        return (event.badge.kind, format!("Event: {}", event.badge.label));
    }
    // The earliest unready row names the remedy; failures decide the section state.
    let first_unready = section
        .services
        .iter()
        .find(|service| service.badge.kind != ShowCardStateKind::Ok);
    if let Some(service) = first_unready {
        let state = if section
            .services
            .iter()
            .any(|service| service.badge.kind == ShowCardStateKind::Failed)
        {
            ShowCardStateKind::Failed
        } else {
            ShowCardStateKind::Attention
        };
        return (state, format!("{}: {}", service.label, service.badge.label));
    }
    (
        ShowCardStateKind::Ok,
        "Event, Producer, Publisher: ready".to_owned(),
    )
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
        let hint = event_hint(input).or_else(|| event_result_hint(input));
        let mut actions = EventActionsDisplay::from_state(
            &event,
            &target,
            input.attach_target_name.trim(),
            publisher_reachable,
            &input.feedback,
        );
        if input.registry.status != EventRegistryStatus::Loaded
            || input.registry.selected_id.is_some()
            || !input.registry.events.is_empty()
        {
            actions.create.availability = EventActionAvailability::Unavailable;
        }
        if input.feedback.verification_failure.is_some() {
            actions.attach.availability = EventActionAvailability::Unavailable;
            actions.replace.availability = EventActionAvailability::Unavailable;
        }
        if let EventTargetListInput::Loaded { targets } = &input.targets {
            if let Some(target) = targets
                .iter()
                .find(|target| target.name == input.attach_target_name.trim())
            {
                if Some(target.event_id.as_str()) != event.event_id.as_deref() {
                    actions.attach.a11y_label = format!(
                        "Attach this event; replace {} on target {} and restart Publisher",
                        target.event_id, target.name
                    );
                }
            }
        }
        let summary = match &event.event_id {
            Some(event_id) => format!("{event_id} - {}", target.label),
            None => event.state.label.to_owned(),
        };

        Self {
            badge: event_badge(input, &target),
            activity: (input.feedback.check == EventCommandState::Working
                || input.feedback.target_read == EventCommandState::Working)
                .then_some("Checking"),
            picker: event_picker(input, &event),
            primary: event_primary(input, &actions),
            overflow: event_overflow(input, &actions),
            logs: event_control("show-event-logs", "Logs", true),
            diagnostics: event_diagnostics(input),
            title: "Event",
            registration_message: input.feedback.registration_message(),
            check_message: input.feedback.check_message(),
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
                .find(|target| {
                    !input.attach_target_name.trim().is_empty()
                        && target.name == input.attach_target_name.trim()
                        && target.event_id == selected_event.event_id
                })
                .map_or_else(
                    || {
                        Self::not_attached(
                            "The configured publisher target does not carry this event.",
                        )
                    },
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
        feedback: &EventCommandFeedback,
    ) -> Self {
        let has_event = event.event_id.is_some();
        let event_live_enough = matches!(event.state.state, EventState::Live);
        let commands_ready = matches!(
            target.state,
            EventTargetAttachmentState::Attached | EventTargetAttachmentState::NotAttached
        );
        let attach_available = !feedback.working()
            && has_event
            && event_live_enough
            && publisher_reachable
            && commands_ready
            && matches!(target.state, EventTargetAttachmentState::NotAttached)
            && !attach_target_name.is_empty();
        let detach_available = !feedback.working()
            && has_event
            && publisher_reachable
            && commands_ready
            && matches!(target.state, EventTargetAttachmentState::Attached);

        Self {
            create: registry_action_display(
                "event-create",
                "Create",
                "Create broadcast event",
                event.state.state == EventState::None && !feedback.working(),
            ),
            replace: registry_action_display(
                "event-replace",
                "Replace",
                "Replace dead broadcast event",
                event.state.state == EventState::Dead && !feedback.working(),
            ),
            check: registry_action_display(
                "event-check",
                if matches!(feedback.check, EventCommandState::Failed { .. }) {
                    "Retry check"
                } else {
                    "Check"
                },
                "Check whether the relay still has the selected event",
                event.state.state != EventState::None && !feedback.working(),
            ),
            copy_feed_tag: registry_action_display(
                "event-copy-feed-tag",
                "Copy feed tag",
                "Copy listener feed tag",
                has_event,
            ),
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

fn registry_action_display(
    id: &'static str,
    label: &'static str,
    a11y_label: &str,
    available: bool,
) -> EventActionDisplay {
    EventActionDisplay {
        id,
        label,
        a11y_label: a11y_label.to_owned(),
        availability: event_action_availability(available),
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
        snapshot: Option<&broadcast_service_watch::BroadcastServiceWatchSnapshot>,
        log_pane: &ShowLogPaneDisplay,
        event: Option<EventSectionDisplay>,
    ) -> Option<Self> {
        if snapshot.is_none_or(|snapshot| snapshot.units.is_empty()) && event.is_none() {
            return None;
        }
        let event = event.or_else(|| {
            Some(EventSectionDisplay::from_input(
                &EventSectionInput {
                    liveness_confirmed: true,
                    registry: EventRegistryInput {
                        status: EventRegistryStatus::Loading,
                        ..EventRegistryInput::default()
                    },
                    feedback: EventCommandFeedback::default(),
                    selected_event: None,
                    targets: EventTargetListInput::Unknown,
                    attach_target_name: String::new(),
                    context: String::new(),
                    request: 0,
                    remote_host: false,
                },
                false,
            ))
        });
        let services = [
            PublisherServiceRole::Producer,
            PublisherServiceRole::Publisher,
        ]
        .into_iter()
        .map(|role| {
            let units: Vec<_> = snapshot
                .into_iter()
                .flat_map(|snapshot| &snapshot.units)
                .filter(|unit| unit.role == role)
                .collect();
            if let [unit] = units.as_slice() {
                PublisherServiceDisplay::from_snapshot(unit, log_pane)
            } else {
                let state =
                    PublisherServiceStateDisplay::from_service_state(&ServiceState::Unknown);
                let label = service_label(role);
                PublisherServiceDisplay {
                    role,
                    id: format!("publisher-service-{}", service_id_seed(role)),
                    label,
                    unit_name: "Status unavailable".to_owned(),
                    badge: ShowItemBadge::new(ShowCardStateKind::Unknown, "Status unavailable"),
                    actions: service_actions(role, label, &state),
                    logs: service_logs(role, label, "Status unavailable", &state, log_pane),
                    state,
                }
            }
        })
        .collect();
        let mut section = Self {
            title: "Live Metadata",
            summary: String::new(),
            event,
            services,
        };
        section.update_summary();
        Some(section)
    }

    fn update_summary(&mut self) {
        if live_metadata_card_state(self).0 == ShowCardStateKind::Ok {
            "Live Metadata: ready"
        } else {
            "Live Metadata: not ready"
        }
        .clone_into(&mut self.summary);
    }
}

impl PublisherServiceDisplay {
    fn from_snapshot(
        snapshot: &broadcast_service_watch::BroadcastServiceUnitSnapshot,
        log_pane: &ShowLogPaneDisplay,
    ) -> Self {
        let state = PublisherServiceStateDisplay::from_service_state(&snapshot.state);
        let label = service_label(snapshot.role);
        let id_seed = service_id_seed(snapshot.role);
        let unit_name = snapshot.unit_name.clone();
        Self {
            badge: service_badge(&state),
            role: snapshot.role,
            id: format!("publisher-service-{id_seed}"),
            label,
            unit_name: unit_name.clone(),
            actions: service_actions(snapshot.role, label, &state),
            logs: service_logs(snapshot.role, label, &unit_name, &state, log_pane),
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
    log_pane: &ShowLogPaneDisplay,
) -> PublisherLogsDisplay {
    let id_seed = service_id_seed(role);
    PublisherLogsDisplay {
        unit_name: unit_name.to_owned(),
        line_count: PUBLISHER_LOG_LINE_COUNT,
        open: log_pane.shows_role(role),
        action: PublisherActionDisplay {
            id: format!("publisher-{id_seed}-logs"),
            label: "Logs",
            a11y_label: format!("Open {label} service logs"),
            availability: availability(
                log_pane.shows_role(role)
                    || !matches!(state, PublisherServiceStateDisplay::NotReachable),
            ),
        },
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

    fn command_snapshot(
        state: ServiceState,
        read_started_at: Instant,
    ) -> broadcast_service_watch::BroadcastServiceWatchSnapshot {
        let mut snapshot = publisher_snapshot([
            (
                PublisherServiceRole::Producer,
                "producer.service",
                ServiceState::Active,
            ),
            (PublisherServiceRole::Publisher, "publisher.service", state),
        ]);
        snapshot.read_started_at = read_started_at;
        snapshot.at = read_started_at + std::time::Duration::from_millis(100);
        snapshot.encoder.configured = true;
        snapshot.encoder.status.state = EncoderState::Disconnected;
        snapshot
    }

    fn command_page(
        snapshot: &broadcast_service_watch::BroadcastServiceWatchSnapshot,
        commands: &ShowCommandState,
    ) -> ShowPageVm {
        ShowPageVm::from_queue_and_publisher(
            QueueNowPlayingPageVm::builder().build(),
            Some(snapshot),
            ShowLogPaneDisplay::closed(),
        )
        .with_command_state(commands)
    }

    fn command_service(page: &ShowPageVm, role: PublisherServiceRole) -> &PublisherServiceDisplay {
        page.publisher
            .as_ref()
            .unwrap()
            .services
            .iter()
            .find(|service| service.role == role)
            .unwrap()
    }

    /// Situational ADR 0059: only a read begun after completion can release a requested transition.
    #[test]
    fn show_command_transition_requires_fresh_read_and_matching_direction() {
        let returned_at = Instant::now();
        let later = returned_at + std::time::Duration::from_secs(1);
        for (operation, old, expected) in [
            (
                PublisherServiceOperation::Stop,
                ServiceState::Active,
                ServiceState::Inactive,
            ),
            (
                PublisherServiceOperation::Start,
                ServiceState::Inactive,
                ServiceState::Active,
            ),
        ] {
            let mut commands = ShowCommandState::default();
            let id = commands
                .begin_service(PublisherServiceRole::Publisher, operation)
                .unwrap();
            let agreeing = command_snapshot(expected.clone(), later);
            commands.observe(&agreeing);
            let page = command_page(&agreeing, &commands);
            let service = command_service(&page, PublisherServiceRole::Publisher);
            assert_eq!(service.state, PublisherServiceStateDisplay::Working);
            assert!(service.actions.start.disabled() && service.actions.stop.disabled());
            assert!(commands
                .begin_service(PublisherServiceRole::Publisher, operation)
                .is_none());
            assert!(commands.complete(id, returned_at, true));
            // A batch begun before return but finished afterwards is not fresh, even if agreeing.
            let mut stale = command_snapshot(
                expected.clone(),
                returned_at
                    .checked_sub(std::time::Duration::from_millis(1))
                    .expect("test timestamp supports an earlier observation"),
            );
            stale.at = later;
            commands.observe(&stale);
            assert_eq!(
                command_service(
                    &command_page(&stale, &commands),
                    PublisherServiceRole::Publisher
                )
                .state,
                PublisherServiceStateDisplay::Working
            );
            let old_sample = command_snapshot(old, later);
            commands.observe(&old_sample);
            assert_eq!(
                command_service(
                    &command_page(&old_sample, &commands),
                    PublisherServiceRole::Publisher
                )
                .state,
                PublisherServiceStateDisplay::Working
            );
            let fresh =
                command_snapshot(expected.clone(), later + std::time::Duration::from_secs(1));
            commands.observe(&fresh);
            assert_eq!(
                command_service(
                    &command_page(&fresh, &commands),
                    PublisherServiceRole::Publisher
                )
                .state,
                PublisherServiceStateDisplay::from_service_state(&expected)
            );
        }
    }

    /// Situational ADR 0059: failures end transitions without clearing another role or a newer press.
    #[test]
    fn show_command_failures_and_old_results_keep_role_ownership_separate() {
        let mut commands = ShowCommandState::default();
        let now = Instant::now();
        let first = commands
            .begin_service(
                PublisherServiceRole::Publisher,
                PublisherServiceOperation::Start,
            )
            .unwrap();
        let producer = commands
            .begin_service(
                PublisherServiceRole::Producer,
                PublisherServiceOperation::Stop,
            )
            .unwrap();
        assert!(commands.complete(first, now, false));
        let failed = command_snapshot(
            ServiceState::Failed {
                reason: "exit-code".to_owned(),
            },
            now,
        );
        let page = command_page(&failed, &commands);
        assert_eq!(
            command_service(&page, PublisherServiceRole::Publisher)
                .state
                .kind(),
            PublisherServiceStateKind::Failed
        );
        assert_eq!(
            command_service(&page, PublisherServiceRole::Producer).state,
            PublisherServiceStateDisplay::Working
        );
        let second = commands
            .begin_service(
                PublisherServiceRole::Publisher,
                PublisherServiceOperation::Reset,
            )
            .unwrap();
        assert_ne!(first, second);
        assert!(!commands.complete(first, now, true));
        assert!(!commands.complete(first, now, false));
        assert_eq!(
            command_service(
                &command_page(&failed, &commands),
                PublisherServiceRole::Publisher
            )
            .state,
            PublisherServiceStateDisplay::Working
        );
        assert!(commands.complete(second, now, true));
        let fresh = command_snapshot(
            ServiceState::Inactive,
            now + std::time::Duration::from_secs(1),
        );
        commands.observe(&fresh);
        assert_eq!(
            command_service(
                &command_page(&fresh, &commands),
                PublisherServiceRole::Publisher
            )
            .state,
            PublisherServiceStateDisplay::Inactive
        );
        assert!(commands.complete(producer, now, false));
        assert!(commands.progress.is_empty());
        let old_host = commands
            .begin_stream(StreamEncoderOperation::Connect)
            .unwrap();
        commands.clear();
        let new_host = commands
            .begin_stream(StreamEncoderOperation::Connect)
            .unwrap();
        assert!(!commands.complete(old_host, now, false));
        assert!(commands.complete(new_host, now, false));
    }

    /// Situational ADR 0059: a failed start or inaccessible unit is an answer, even without agreement.
    #[test]
    fn show_command_fresh_settled_failure_releases_immediately() {
        let now = Instant::now();
        for state in [
            ServiceState::Failed {
                reason: "crashed".to_owned(),
            },
            ServiceState::NotInstalled,
            ServiceState::NotReachable,
        ] {
            let mut commands = ShowCommandState::default();
            let id = commands
                .begin_service(
                    PublisherServiceRole::Publisher,
                    PublisherServiceOperation::Start,
                )
                .unwrap();
            assert!(commands.complete(id, now, true));
            let sample = command_snapshot(state.clone(), now + std::time::Duration::from_secs(1));
            commands.observe(&sample);
            assert_eq!(
                command_service(
                    &command_page(&sample, &commands),
                    PublisherServiceRole::Publisher
                )
                .state,
                PublisherServiceStateDisplay::from_service_state(&state)
            );
        }
    }

    /// Situational ADR 0059: three distinct fresh samples bound a transition; reprojection is not a sample.
    #[test]
    fn show_command_transition_is_bounded_and_duplicate_reads_do_not_count() {
        let now = Instant::now();
        for state in [
            ServiceState::Active,
            ServiceState::Starting,
            ServiceState::Stopping,
            ServiceState::Unknown,
        ] {
            let mut commands = ShowCommandState::default();
            let id = commands
                .begin_service(
                    PublisherServiceRole::Publisher,
                    PublisherServiceOperation::Stop,
                )
                .unwrap();
            assert!(commands.complete(id, now, true));
            for n in 1..=3 {
                let sample =
                    command_snapshot(state.clone(), now + std::time::Duration::from_secs(n));
                commands.observe(&sample);
                for _ in 0..4 {
                    commands.observe(&sample);
                    let page = command_page(&sample, &commands);
                    let expected = if n < 3 {
                        PublisherServiceStateDisplay::Working
                    } else {
                        PublisherServiceStateDisplay::from_service_state(&state)
                    };
                    assert_eq!(
                        command_service(&page, PublisherServiceRole::Publisher).state,
                        expected
                    );
                }
            }
        }
    }

    /// Situational ADR 0059: stream actions stay mounted and disabled until command readback resolves.
    #[test]
    fn show_command_stream_keeps_controls_and_uses_the_service_release_policy() {
        let now = Instant::now();
        for (operation, expected) in [
            (StreamEncoderOperation::Connect, EncoderState::Connected),
            (
                StreamEncoderOperation::Disconnect,
                EncoderState::Disconnected,
            ),
        ] {
            let mut commands = ShowCommandState::default();
            let id = commands.begin_stream(operation).unwrap();
            let mut sample = command_snapshot(ServiceState::Active, now);
            sample.encoder.status.state = expected;
            commands.observe(&sample);
            let stream = command_page(&sample, &commands).stream.unwrap();
            assert_eq!(stream.connection.state, StreamConnectionState::Working);
            let actions = stream.actions.unwrap();
            assert!(actions.connect.disabled() && actions.disconnect.disabled());
            assert!(commands.begin_stream(operation).is_none());
            assert!(commands.complete(id, now, true));
            commands.observe(&sample);
            assert_eq!(
                command_page(&sample, &commands)
                    .stream
                    .unwrap()
                    .connection
                    .state,
                StreamConnectionState::Working
            );
            sample.read_started_at = now + std::time::Duration::from_secs(1);
            commands.observe(&sample);
            assert_eq!(
                command_page(&sample, &commands)
                    .stream
                    .unwrap()
                    .connection
                    .state,
                stream_connection_state(expected)
            );
            assert!(commands.begin_stream(operation).is_some());
        }
        let mut commands = ShowCommandState::default();
        let id = commands
            .begin_stream(StreamEncoderOperation::Connect)
            .unwrap();
        assert!(commands.complete(id, now, true));
        for n in 1..=3 {
            commands.observe(&command_snapshot(
                ServiceState::Active,
                now + std::time::Duration::from_secs(n),
            ));
        }
        assert!(commands.progress.is_empty());
        let id = commands
            .begin_stream(StreamEncoderOperation::Connect)
            .unwrap();
        assert!(commands.complete(id, now, false));
        assert!(commands.progress.is_empty());
    }

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
            let publisher = PublisherSectionDisplay::from_snapshot(
                Some(&snapshot),
                &ShowLogPaneDisplay::closed(),
                None,
            )
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
                    created_at: 0,
                    last_checked_at: None,
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
            let publisher = PublisherSectionDisplay::from_snapshot(
                None,
                &ShowLogPaneDisplay::closed(),
                Some(event),
            )
            .unwrap();
            cards.push(ShowCardDisplay::from_live_metadata(&publisher));
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
            ShowLogPaneDisplay::closed(),
        );

        assert!(vm.source.is_none());
        assert!(vm.publisher.is_none());
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
                created_at: 0,
                last_checked_at: None,
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
            ShowLogPaneDisplay::closed(),
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

        let detail = ShowPanelMode::Detail(ShowCardKind::LiveMetadata);
        let ShowPanelMode::Detail(kind) = detail else {
            panic!("detail mode must hold one card kind");
        };
        assert_eq!(kind, ShowCardKind::LiveMetadata);
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
        let detail = ShowPanelMode::Detail(ShowCardKind::LiveMetadata);
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
        let vm = ShowPageVm::from_queue_publisher_readiness_and_event(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            ShowLogPaneDisplay::closed(),
            None,
            Some(&ready_event_input()),
        );
        let card = vm
            .cards
            .iter()
            .find(|card| card.kind == ShowCardKind::LiveMetadata)
            .expect("live metadata card");

        assert_eq!(card.state, ShowCardStateKind::Failed);
        assert_eq!(card.state_label, "Not ready");
    }

    #[test]
    fn live_metadata_card_names_earliest_unready_row_and_section_state() {
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
        let vm = ShowPageVm::from_queue_publisher_readiness_and_event(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            ShowLogPaneDisplay::closed(),
            None,
            Some(&ready_event_input()),
        );
        let card = vm
            .cards
            .iter()
            .find(|card| card.kind == ShowCardKind::LiveMetadata)
            .expect("live metadata card");

        assert_eq!(card.primary, "Producer: Inactive");
        assert_eq!(card.secondary, "Live Metadata: not ready");
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
            ShowLogPaneDisplay::closed(),
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
        assert_eq!(publisher.summary, "Live Metadata: not ready");
        assert_eq!(publisher.services.len(), 2);
        assert_eq!(
            publisher.services[1].state,
            PublisherServiceStateDisplay::Active
        );
        assert!(publisher.services[1].actions.start.disabled());
        assert!(!publisher.services[1].actions.stop.disabled());
        assert_eq!(
            publisher.services[0].state,
            PublisherServiceStateDisplay::Inactive
        );
        assert!(!publisher.services[0].actions.start.disabled());
        assert!(publisher.services[0].actions.stop.disabled());
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
            ShowLogPaneDisplay::closed(),
        );
        let service = &vm.publisher.expect("publisher section").services[1];

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
        // Situational ADR 0063: each service state retains an explanation in Logs.
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
            ShowLogPaneDisplay::closed(),
        );
        let service = &vm.publisher.expect("publisher section").services[1];

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
            ShowLogPaneDisplay::closed(),
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
            vm.publisher.expect("publisher section").services[1].state,
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
                    no_routes_upstream: 1,
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
            ShowLogPaneDisplay::closed(),
            Some(&readiness),
        );
        let readiness = vm
            .source
            .expect("source section")
            .readiness
            .expect("readiness display");

        assert_eq!(readiness.count_label, "3 tracks not ready");
        assert_eq!(
            readiness.detail,
            "1 without payment routes, 1 need publisher routes, 1 with a missing file."
        );
        assert_eq!(readiness.state, SourceReadinessState::NeedsAttention);
        assert!(!readiness.action.disabled());
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
        let mut input = event_input(
            Some(EventSelectionInput {
                created_at: 0,
                last_checked_at: None,
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

        input.attach_target_name = "late-night".to_owned();
        let vm = ShowPageVm::from_queue_publisher_readiness_and_event(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            ShowLogPaneDisplay::closed(),
            None,
            Some(&input),
        );
        let event = vm
            .publisher
            .expect("live metadata")
            .event
            .expect("event row");

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
                created_at: 0,
                last_checked_at: None,
                label: None,
                event_id: "event-one".to_owned(),
                endpoint: "https://relay.example".to_owned(),
                token_path: "/tmp/event-one.token".to_owned(),
                state: EventState::Live,
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
            ShowLogPaneDisplay::closed(),
            None,
            Some(&input),
        );
        let event = vm
            .publisher
            .expect("live metadata")
            .event
            .expect("event row");

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
                created_at: 0,
                last_checked_at: None,
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
            ShowLogPaneDisplay::closed(),
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
            ShowLogPaneDisplay::closed(),
            None,
            Some(&live_input),
        );

        assert!(dead_vm
            .publisher
            .expect("live metadata")
            .event
            .expect("dead event section")
            .actions
            .attach
            .disabled());
        assert!(unreachable_vm
            .publisher
            .expect("live metadata")
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
                created_at: 0,
                last_checked_at: None,
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
            ShowLogPaneDisplay::closed(),
            None,
            Some(&input),
        );
        let event = vm
            .publisher
            .expect("live metadata")
            .event
            .expect("event row");

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
                created_at: 0,
                last_checked_at: None,
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
            ShowLogPaneDisplay::closed(),
            None,
            Some(&input),
        );

        assert_eq!(
            vm.publisher
                .expect("live metadata")
                .event
                .expect("event row")
                .hint
                .as_deref(),
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
            ShowLogPaneDisplay::closed(),
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
            ShowLogPaneDisplay::closed(),
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
            ShowLogPaneDisplay::closed(),
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
            ShowLogPaneDisplay::closed(),
        );
        let stream = vm.stream.expect("stream section");

        assert_eq!(stream.connection.state, StreamConnectionState::NotInstalled);
        assert_eq!(stream.recording.state, StreamRecordingState::Unknown);
        assert!(stream.actions.is_none());
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

    fn show_with_log_services() -> ShowPageVm {
        let snapshot = publisher_snapshot([
            (
                PublisherServiceRole::Producer,
                "mixxx-now-playing.service",
                ServiceState::Active,
            ),
            (
                PublisherServiceRole::Publisher,
                "musicindex-live-publisher@mixxx.service",
                ServiceState::Active,
            ),
        ]);
        ShowPageVm::from_queue_and_publisher(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            ShowLogPaneDisplay::closed(),
        )
    }

    /// Situational ADR 0063: log intent is independent of card/panel selection.
    #[test]
    fn show_logs_name_each_unit_and_survive_card_selection_and_panel_close() {
        for role in [
            PublisherServiceRole::Producer,
            PublisherServiceRole::Publisher,
        ] {
            let mut vm = show_with_log_services();
            let request = vm.toggle_publisher_logs(role).expect("read requested");
            let service = vm
                .publisher
                .as_ref()
                .unwrap()
                .services
                .iter()
                .find(|service| service.role == role)
                .unwrap();
            assert_eq!(vm.log_pane.unit_name, service.unit_name);
            assert!(service.logs.open);
            assert!(!service.logs.action.a11y_label.is_empty());
            assert!(!vm.log_pane.close.disabled());
            assert!(!vm.log_pane.close.a11y_label.is_empty());
            assert!(vm
                .log_pane
                .apply_result(request, Ok("first line\nsecond line\n".to_owned())));
            assert_eq!(vm.log_pane.line_count, 2);
            assert_eq!(vm.log_pane.line_count_label, "2 log lines");
            let pane = vm.log_pane.clone();
            vm = vm.select_card(ShowCardKind::Stream).close_panel();
            assert_eq!(vm.log_pane, pane);
            let mode = vm.panel_mode;
            let queue = vm.queue.clone();
            vm.close_publisher_logs();
            assert!(!vm.log_pane.open);
            assert!(vm.log_pane.close.disabled());
            assert_eq!(vm.panel_mode, mode);
            assert!(!vm.panel_open);
            assert_eq!(vm.queue, queue);
        }
    }

    /// Situational ADR 0063: repeated Logs presses cycle even during a pending read.
    #[test]
    fn show_logs_cycle_same_unit_and_switch_other_unit() {
        let mut vm = show_with_log_services();
        let first = vm
            .toggle_publisher_logs(PublisherServiceRole::Producer)
            .unwrap();
        assert!(vm
            .toggle_publisher_logs(PublisherServiceRole::Producer)
            .is_none());
        assert!(!vm.log_pane.open);
        assert!(!vm.log_pane.apply_result(first, Ok("late".to_owned())));
        let second = vm
            .toggle_publisher_logs(PublisherServiceRole::Producer)
            .unwrap();
        let third = vm
            .toggle_publisher_logs(PublisherServiceRole::Publisher)
            .unwrap();
        assert_ne!(first, second);
        assert_ne!(second, third);
        assert!(vm.log_pane.shows_role(PublisherServiceRole::Publisher));
        assert!(!vm
            .log_pane
            .apply_result(second, Err("late failure".to_owned())));
        assert!(vm.log_pane.apply_result(third, Ok("current".to_owned())));
        assert!(!vm
            .log_pane
            .apply_result(second, Ok("late success".to_owned())));
        assert_eq!(vm.log_pane.text, "current");
        assert!(vm
            .toggle_publisher_logs(PublisherServiceRole::Publisher)
            .is_none());
        assert!(!vm.log_pane.open);
    }

    /// Situational ADR 0063: a close/reopen of the same unit invalidates its earlier read.
    #[test]
    fn show_logs_discard_old_and_duplicate_results_after_close_and_reprojection() {
        let mut vm = show_with_log_services();
        let old = vm
            .toggle_publisher_logs(PublisherServiceRole::Producer)
            .unwrap();
        vm.close_publisher_logs();
        assert!(!vm
            .log_pane
            .apply_result(old, Err("closed failure".to_owned())));
        let current = vm
            .toggle_publisher_logs(PublisherServiceRole::Producer)
            .unwrap();
        vm = ShowPageVm::from_queue_and_publisher(vm.queue.clone(), None, vm.log_pane.clone())
            .with_panel_state(ShowPanelMode::Detail(ShowCardKind::LiveMetadata), false);
        let pane = vm.log_pane.clone();
        assert!(!vm
            .log_pane
            .apply_result(old, Ok("old unit result".to_owned())));
        assert_eq!(vm.log_pane, pane);
        assert!(vm
            .log_pane
            .apply_result(current, Err("current failure".to_owned())));
        assert_eq!(vm.log_pane.text, "current failure");
        assert_eq!(vm.log_pane.line_count_label, "Log read failed");
        assert!(!vm
            .log_pane
            .apply_result(current, Ok("duplicate".to_owned())));
        assert_eq!(
            vm.panel_mode,
            ShowPanelMode::Detail(ShowCardKind::LiveMetadata)
        );
        assert!(!vm.panel_open);
    }

    /// Situational ADR 0063: losing a host does not disable the action that closes its logs.
    #[test]
    fn show_logs_can_close_after_the_service_becomes_unreachable() {
        let mut vm = show_with_log_services();
        vm.toggle_publisher_logs(PublisherServiceRole::Publisher)
            .unwrap();
        let snapshot = publisher_snapshot([(
            PublisherServiceRole::Publisher,
            "musicindex-live-publisher@mixxx.service",
            ServiceState::NotReachable,
        )]);
        vm = ShowPageVm::from_queue_and_publisher(vm.queue, Some(&snapshot), vm.log_pane);
        assert!(!vm.publisher.as_ref().unwrap().services[0]
            .logs
            .action
            .disabled());
        assert!(vm
            .toggle_publisher_logs(PublisherServiceRole::Publisher)
            .is_none());
        assert!(!vm.log_pane.open);
        assert!(vm.publisher.as_ref().unwrap().services[1]
            .logs
            .action
            .disabled());
    }

    /// Situational ADR 0063: a successful empty journal has display-ready feedback.
    #[test]
    fn show_logs_empty_read_is_display_ready() {
        let mut vm = show_with_log_services();
        let request = vm
            .toggle_publisher_logs(PublisherServiceRole::Producer)
            .unwrap();
        assert!(vm.log_pane.apply_result(request, Ok(String::new())));
        assert_eq!(vm.log_pane.text, "No log lines returned.");
        assert_eq!(vm.log_pane.line_count, 0);
        assert_eq!(vm.log_pane.line_count_label, "0 log lines");
    }

    /// Situational ADR 0063: dragging never consumes the space reserved for the grid.
    #[test]
    fn show_log_height_reserves_cards_and_survives_close() {
        let mut vm = show_with_log_services();
        vm.toggle_publisher_logs(PublisherServiceRole::Producer)
            .unwrap();
        assert!(vm.log_pane.update_geometry(500.0, 140.0));
        vm.log_pane.resize(800.0);
        assert!((vm.log_pane.height - 140.0).abs() < f32::EPSILON);
        vm.log_pane.resize(-100.0);
        assert!((vm.log_pane.height - SHOW_LOG_MIN_HEIGHT).abs() < f32::EPSILON);
        vm.log_pane.resize(f32::NAN);
        assert!(vm.log_pane.height.is_finite());
        let height = vm.log_pane.height;
        vm.close_publisher_logs();
        vm.toggle_publisher_logs(PublisherServiceRole::Producer)
            .unwrap();
        assert!((vm.log_pane.height - height).abs() < f32::EPSILON);
        assert!(!vm.log_pane.update_geometry(500.0, 140.0));
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
            read_started_at: std::time::Instant::now(),
            at: std::time::Instant::now(),
            units,
            encoder,
        }
    }

    fn ready_event_input() -> EventSectionInput {
        event_input(
            Some(EventSelectionInput {
                created_at: 0,
                last_checked_at: None,
                label: None,
                event_id: "event-one".to_owned(),
                endpoint: "https://relay.example".to_owned(),
                token_path: "/tmp/event-one.token".to_owned(),
                state: EventState::Live,
                token_file_missing: false,
            }),
            EventTargetListInput::Loaded {
                targets: vec![EventTargetInput {
                    name: "default".to_owned(),
                    event_id: "event-one".to_owned(),
                }],
            },
        )
    }

    /// Situational ADR 0059: event liveness and attachment precede running services.
    #[test]
    fn show_event_readiness_table_covers_liveness_and_every_attachment_result() {
        let snapshot = publisher_snapshot([
            (
                PublisherServiceRole::Publisher,
                "publisher.service",
                ServiceState::Active,
            ),
            (
                PublisherServiceRole::Producer,
                "producer.service",
                ServiceState::Active,
            ),
        ]);
        for (state, expected, action) in [
            (
                EventState::None,
                ShowCardStateKind::Attention,
                Some(EventRegistryAction::Create),
            ),
            (
                EventState::Dead,
                ShowCardStateKind::Failed,
                Some(EventRegistryAction::Replace),
            ),
            (
                EventState::Unknown,
                ShowCardStateKind::Unknown,
                Some(EventRegistryAction::Check),
            ),
            (EventState::Live, ShowCardStateKind::Ok, None),
        ] {
            let mut input = ready_event_input();
            if state == EventState::None {
                input.selected_event = None;
            } else {
                input.selected_event.as_mut().unwrap().state = state;
            }
            let vm = ShowPageVm::from_queue_publisher_readiness_and_event(
                QueueNowPlayingPageVm::builder().build(),
                Some(&snapshot),
                ShowLogPaneDisplay::closed(),
                None,
                Some(&input),
            );
            assert_eq!(vm.cards.len(), 3);
            let card = vm
                .cards
                .iter()
                .find(|card| card.kind == ShowCardKind::LiveMetadata)
                .unwrap();
            assert_eq!(card.state, expected);
            let section = vm.publisher.unwrap();
            assert_eq!(
                section.services.iter().map(|s| s.label).collect::<Vec<_>>(),
                ["Producer", "Publisher"]
            );
            let event = section.event.unwrap();
            for candidate in [
                EventRegistryAction::Create,
                EventRegistryAction::Replace,
                EventRegistryAction::Check,
            ] {
                let display = event.actions.registry_action(candidate);
                assert_eq!(
                    !display.disabled(),
                    action == Some(candidate)
                        || (candidate == EventRegistryAction::Check && state != EventState::None)
                );
                assert!(!display.a11y_label.is_empty());
            }
            assert_eq!(
                event.actions.copy_feed_tag.disabled(),
                state == EventState::None
            );
            if state != EventState::None {
                assert!(event.feed_tag.unwrap().contains("event-one"));
            }
        }
        assert_event_attachment_readiness(&snapshot);
    }

    /// Situational ADR 0059: attachment query failures cannot claim confirmed absence.
    fn assert_event_attachment_readiness(
        snapshot: &broadcast_service_watch::BroadcastServiceWatchSnapshot,
    ) {
        for (targets, expected, attach) in [
            (
                EventTargetListInput::Unknown,
                ShowCardStateKind::Unknown,
                false,
            ),
            (
                EventTargetListInput::CommandsUnavailable,
                ShowCardStateKind::Unknown,
                false,
            ),
            (
                EventTargetListInput::NotReachable,
                ShowCardStateKind::Unknown,
                false,
            ),
            (
                EventTargetListInput::Failed {
                    detail: "query rejected".to_owned(),
                },
                ShowCardStateKind::Unknown,
                false,
            ),
            (
                EventTargetListInput::Loaded { targets: vec![] },
                ShowCardStateKind::Attention,
                true,
            ),
        ] {
            let mut input = ready_event_input();
            input.targets = targets;
            let event = EventSectionDisplay::from_input(&input, true);
            assert_eq!(!event.actions.attach.disabled(), attach);
            assert!(!event.target.detail.is_empty());
            let section = PublisherSectionDisplay::from_snapshot(
                Some(snapshot),
                &ShowLogPaneDisplay::closed(),
                Some(event),
            )
            .unwrap();
            assert_eq!(live_metadata_card_state(&section).0, expected);
        }
    }

    /// Situational ADR 0059: stopped, transient, absent, and unknown services cannot read Ready.
    #[test]
    fn show_event_readiness_table_covers_every_service_state() {
        for state in [
            PublisherServiceStateDisplay::Active,
            PublisherServiceStateDisplay::Inactive,
            PublisherServiceStateDisplay::Starting,
            PublisherServiceStateDisplay::Stopping,
            PublisherServiceStateDisplay::Working,
            PublisherServiceStateDisplay::Unknown,
            PublisherServiceStateDisplay::NotInstalled,
            PublisherServiceStateDisplay::NotReachable,
            PublisherServiceStateDisplay::Failed {
                reason: "exit-code".to_owned(),
            },
        ] {
            for role in [
                PublisherServiceRole::Producer,
                PublisherServiceRole::Publisher,
            ] {
                let snapshot = publisher_snapshot([
                    (
                        PublisherServiceRole::Publisher,
                        "publisher.service",
                        ServiceState::Active,
                    ),
                    (
                        PublisherServiceRole::Producer,
                        "producer.service",
                        ServiceState::Active,
                    ),
                ]);
                let mut section = PublisherSectionDisplay::from_snapshot(
                    Some(&snapshot),
                    &ShowLogPaneDisplay::closed(),
                    Some(EventSectionDisplay::from_input(&ready_event_input(), true)),
                )
                .unwrap();
                let service = section
                    .services
                    .iter_mut()
                    .find(|service| service.role == role)
                    .unwrap();
                service.state = state.clone();
                service.badge = service_badge(&state);
                section.update_summary();
                let card = ShowCardDisplay::from_live_metadata(&section);
                let expected = match state {
                    PublisherServiceStateDisplay::Active => ShowCardStateKind::Ok,
                    PublisherServiceStateDisplay::Failed { .. } => ShowCardStateKind::Failed,
                    _ => ShowCardStateKind::Attention,
                };
                assert_eq!(card.state, expected, "{role:?}: {state:?}");
                if state != PublisherServiceStateDisplay::Active {
                    assert!(card.primary.starts_with(service_label(role)));
                    assert_eq!(card.secondary, "Live Metadata: not ready");
                }
            }
        }
    }

    /// Situational ADR 0059: busy commands serialize; a failed check preserves registration success.
    #[test]
    fn show_event_unknown_check_retry_and_working_actions_are_typed() {
        let mut input = ready_event_input();
        input.selected_event.as_mut().unwrap().state = EventState::Unknown;
        input.targets = EventTargetListInput::Loaded { targets: vec![] };
        let event = EventSectionDisplay::from_input(&input, true);
        assert_eq!(event.actions.check.label, "Check");
        assert!(!event.actions.check.disabled());
        assert!(
            event.actions.create.disabled()
                && event.actions.replace.disabled()
                && event.actions.attach.disabled()
        );
        for state in [EventState::None, EventState::Dead, EventState::Unknown] {
            let mut busy = input.clone();
            if state == EventState::None {
                busy.selected_event = None;
            } else {
                busy.selected_event.as_mut().unwrap().state = state;
            }
            for feedback in [
                EventCommandFeedback {
                    verification_failure: None,
                    selection: EventCommandState::Idle,
                    target_read: EventCommandState::Idle,
                    target_mutation: EventCommandState::Idle,
                    active_action: None,
                    registration: EventCommandState::Working,
                    check: EventCommandState::Idle,
                    ..EventCommandFeedback::default()
                },
                EventCommandFeedback {
                    verification_failure: None,
                    selection: EventCommandState::Idle,
                    target_read: EventCommandState::Idle,
                    target_mutation: EventCommandState::Idle,
                    active_action: None,
                    registration: EventCommandState::Succeeded,
                    check: EventCommandState::Working,
                    ..EventCommandFeedback::default()
                },
            ] {
                busy.feedback = feedback;
                let event = EventSectionDisplay::from_input(&busy, true);
                assert!(
                    event.actions.create.disabled()
                        && event.actions.replace.disabled()
                        && event.actions.check.disabled()
                );
                assert!(event.registration_message.is_some() || event.check_message.is_some());
            }
        }
        input.feedback = EventCommandFeedback {
            verification_failure: None,
            selection: EventCommandState::Idle,
            target_read: EventCommandState::Idle,
            target_mutation: EventCommandState::Idle,
            active_action: None,
            registration: EventCommandState::Succeeded,
            check: EventCommandState::Failed {
                detail: "connection lost".to_owned(),
            },
            ..EventCommandFeedback::default()
        };
        let event = EventSectionDisplay::from_input(&input, true);
        assert_eq!(event.event.state.state, EventState::Unknown);
        assert_eq!(event.actions.check.label, "Retry check");
        assert!(!event.actions.check.disabled());
        assert!(
            event.actions.create.disabled()
                && event.actions.replace.disabled()
                && event.actions.attach.disabled()
        );
        assert!(event.registration_message.unwrap().contains("registered"));
        assert!(event.check_message.unwrap().contains("connection lost"));
    }

    fn event_input(
        selected_event: Option<EventSelectionInput>,
        targets: EventTargetListInput,
    ) -> EventSectionInput {
        EventSectionInput {
            liveness_confirmed: true,
            registry: EventRegistryInput::default(),
            context: String::new(),
            request: 0,
            feedback: EventCommandFeedback::default(),
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
    /// Situational ADR 0059: configured name AND ID decide attachment, including an empty name.
    #[test]
    fn compact_event_configured_target_and_remote_hint() {
        let mut input = ready_event_input();
        let EventTargetListInput::Loaded { targets } = &mut input.targets else {
            unreachable!()
        };
        targets[0].name = "unused".to_owned();
        let event = EventSectionDisplay::from_input(&input, true);
        assert_eq!(
            event.badge,
            ShowItemBadge::new(ShowCardStateKind::Attention, "Not attached")
        );
        assert!(event.actions.detach.disabled());
        assert!(!event.actions.attach.disabled());
        for name in ["", "   "] {
            input.attach_target_name = name.to_owned();
            let event = EventSectionDisplay::from_input(&input, true);
            assert_eq!(event.badge.label, "Target not set");
            assert!(event.actions.attach.disabled() && event.actions.detach.disabled());
        }
        input.attach_target_name = " default ".to_owned();
        let EventTargetListInput::Loaded { targets } = &mut input.targets else {
            unreachable!()
        };
        targets.push(EventTargetInput {
            name: "default".to_owned(),
            event_id: "event-one".to_owned(),
        });
        input.remote_host = true;
        input.selected_event.as_mut().unwrap().token_file_missing = true;
        let event = EventSectionDisplay::from_input(&input, true);
        assert_eq!(
            event.badge,
            ShowItemBadge::new(ShowCardStateKind::Ok, "Attached")
        );
        assert_eq!(event.target.target_name.as_deref(), Some("default"));
        assert!(event.hint.unwrap().contains("missing locally"));
        input.remote_host = false;
        assert!(event_hint(&input).is_none());
        input.remote_host = true;
        input.selected_event.as_mut().unwrap().token_file_missing = false;
        assert!(event_hint(&input).is_none());
        input.selected_event = None;
        assert!(event_hint(&input).is_none());
    }

    /// Situational ADR 0059: independently completed read failures override retained confirmation.
    #[test]
    fn compact_event_passive_checks_and_mutations_have_distinct_readiness() {
        let mut input = ready_event_input();
        input.feedback.check = EventCommandState::Working;
        input.feedback.target_read = EventCommandState::Working;
        let event = EventSectionDisplay::from_input(&input, true);
        assert_eq!(event.badge.kind, ShowCardStateKind::Ok);
        assert_eq!(event.activity, Some("Checking"));
        assert!(event.primary.unwrap().action.disabled());
        input.liveness_confirmed = false;
        assert_eq!(
            EventSectionDisplay::from_input(&input, true).badge.label,
            "Checking"
        );
        input.liveness_confirmed = true;
        input.targets = EventTargetListInput::Failed {
            detail: "private diagnostic body".to_owned(),
        };
        let event = EventSectionDisplay::from_input(&input, true);
        assert_eq!(event.badge.label, "Target read failed");
        input.feedback.check = EventCommandState::Failed {
            detail: "relay query failed".to_owned(),
        };
        input.feedback.verification_failure = Some("relay query failed".to_owned());
        input.targets = ready_event_input().targets;
        assert_eq!(
            EventSectionDisplay::from_input(&input, true).badge.label,
            "Check failed"
        );
        // Retrying preserves the failed confirmation until a new answer.
        input.feedback.check = EventCommandState::Working;
        assert_eq!(
            EventSectionDisplay::from_input(&input, true).badge.label,
            "Check failed"
        );
        input.feedback = EventCommandFeedback::default();
        input.selected_event.as_mut().unwrap().state = EventState::Unknown;
        input.feedback.check = EventCommandState::Working;
        assert_eq!(
            EventSectionDisplay::from_input(&input, true).badge.label,
            "Checking"
        );
        for (intent, expected) in [
            (EventControlIntent::Create, "Creating"),
            (EventControlIntent::Replace, "Replacing"),
            (EventControlIntent::Attach, "Attaching"),
            (EventControlIntent::Detach, "Detaching"),
        ] {
            let mut input = ready_event_input();
            input.feedback.active_action = Some(intent);
            if matches!(
                intent,
                EventControlIntent::Create | EventControlIntent::Replace
            ) {
                input.feedback.registration = EventCommandState::Working;
            } else {
                input.feedback.target_mutation = EventCommandState::Working;
            }
            let event = EventSectionDisplay::from_input(&input, true);
            assert_eq!(
                event.badge,
                ShowItemBadge::new(ShowCardStateKind::Unknown, expected)
            );
            assert!(
                event.actions.create.disabled()
                    && event.actions.replace.disabled()
                    && event.actions.check.disabled()
                    && event.actions.attach.disabled()
                    && event.actions.detach.disabled()
            );
            let expected_id = match intent {
                EventControlIntent::Create => event.actions.create.id,
                EventControlIntent::Replace => event.actions.replace.id,
                EventControlIntent::Attach => event.actions.attach.id,
                EventControlIntent::Detach => event.actions.detach.id,
                _ => unreachable!(),
            };
            assert_eq!(event.primary.as_ref().unwrap().action.id, expected_id);
            assert!(!event.logs.disabled() && !event.actions.copy_feed_tag.disabled());
        }
    }

    /// Situational ADR 0059: event badge labels distinguish unknown facts from absence.
    fn assert_compact_event_badge_table() {
        use ShowCardStateKind::{Attention, Failed, Ok, Unknown};
        let ready = ready_event_input();
        for (targets, kind, label) in [
            (EventTargetListInput::Unknown, Unknown, "Target unknown"),
            (
                EventTargetListInput::CommandsUnavailable,
                Unknown,
                "Commands unavailable",
            ),
            (EventTargetListInput::NotReachable, Unknown, "Not reachable"),
            (
                EventTargetListInput::Failed {
                    detail: "bad json".to_owned(),
                },
                Unknown,
                "Target read failed",
            ),
            (
                EventTargetListInput::Loaded { targets: vec![] },
                Attention,
                "Not attached",
            ),
            (ready.targets.clone(), Ok, "Attached"),
        ] {
            let mut input = ready.clone();
            input.targets = targets;
            assert_eq!(
                EventSectionDisplay::from_input(&input, true).badge,
                ShowItemBadge::new(kind, label)
            );
        }
        for (state, kind, label) in [
            (EventState::Dead, Failed, "Dead"),
            (EventState::Unknown, Unknown, "Unknown"),
            (EventState::None, Attention, "No event"),
        ] {
            let mut input = ready.clone();
            if state == EventState::None {
                input.selected_event = None;
            } else {
                input.selected_event.as_mut().unwrap().state = state;
            }
            assert_eq!(
                EventSectionDisplay::from_input(&input, true).badge,
                ShowItemBadge::new(kind, label)
            );
        }
        for (status, label) in [
            (EventRegistryStatus::Loading, "Loading"),
            (
                EventRegistryStatus::Failed("locked".to_owned()),
                "Events unavailable",
            ),
        ] {
            let mut input = ready.clone();
            input.registry.status = status;
            assert_eq!(
                EventSectionDisplay::from_input(&input, true).badge,
                ShowItemBadge::new(Unknown, label)
            );
        }
        let mut missing = ready.clone();
        missing.selected_event = None;
        missing.registry.selected_id = Some("gone".to_owned());
        let event = EventSectionDisplay::from_input(&missing, true);
        assert_eq!(event.badge.label, "Event unavailable");
        assert!(event.actions.create.disabled());
    }

    /// Situational ADR 0059: every label and every role combination uses the same badge facts.
    #[test]
    fn compact_event_badge_tables_and_card_equivalence() {
        use ShowCardStateKind::{Attention, Failed, Ok, Unknown};
        assert_compact_event_badge_table();
        let ready = ready_event_input();
        let states = [
            PublisherServiceStateDisplay::Active,
            PublisherServiceStateDisplay::Inactive,
            PublisherServiceStateDisplay::Starting,
            PublisherServiceStateDisplay::Stopping,
            PublisherServiceStateDisplay::Failed {
                reason: "exit-code".to_owned(),
            },
            PublisherServiceStateDisplay::NotInstalled,
            PublisherServiceStateDisplay::NotReachable,
            PublisherServiceStateDisplay::Unknown,
            PublisherServiceStateDisplay::Working,
        ];
        for (state, (kind, label)) in states.iter().zip([
            (Ok, "Active"),
            (Attention, "Inactive"),
            (Attention, "Starting"),
            (Attention, "Stopping"),
            (Failed, "Failed"),
            (Attention, "Not installed"),
            (Attention, "Not reachable"),
            (Unknown, "Unknown"),
            (Unknown, "Working"),
        ]) {
            assert_eq!(service_badge(state), ShowItemBadge::new(kind, label));
        }
        let snapshot = publisher_snapshot([
            (PublisherServiceRole::Producer, "p", ServiceState::Active),
            (PublisherServiceRole::Publisher, "q", ServiceState::Active),
        ]);
        for event_ok in [false, true] {
            for producer in &states {
                for publisher in &states {
                    let mut input = ready.clone();
                    if !event_ok {
                        input.selected_event.as_mut().unwrap().state = EventState::Dead;
                    }
                    let mut section = PublisherSectionDisplay::from_snapshot(
                        Some(&snapshot),
                        &ShowLogPaneDisplay::closed(),
                        Some(EventSectionDisplay::from_input(&input, true)),
                    )
                    .unwrap();
                    section.services[0].badge = service_badge(producer);
                    section.services[1].badge = service_badge(publisher);
                    let card_ok = live_metadata_card_state(&section).0 == Ok;
                    assert_eq!(
                        card_ok,
                        event_ok
                            && producer.kind() == PublisherServiceStateKind::Active
                            && publisher.kind() == PublisherServiceStateKind::Active
                    );
                }
            }
        }
        for units in [
            vec![],
            vec![snapshot.units[0].clone()],
            vec![snapshot.units[0].clone(), snapshot.units[0].clone()],
            vec![
                snapshot.units[0].clone(),
                snapshot.units[1].clone(),
                snapshot.units[1].clone(),
            ],
        ] {
            let mut snapshot = snapshot.clone();
            snapshot.units = units;
            let section = PublisherSectionDisplay::from_snapshot(
                Some(&snapshot),
                &ShowLogPaneDisplay::closed(),
                Some(EventSectionDisplay::from_input(&ready, true)),
            )
            .unwrap();
            assert_eq!(section.services.len(), 2);
            assert!(section
                .services
                .iter()
                .any(|service| service.badge.label == "Status unavailable"));
            assert_ne!(live_metadata_card_state(&section).0, Ok);
        }
    }

    /// Situational ADR 0063: snapshot/source switching preserves exact text and rejects stale journals.
    #[test]
    fn compact_event_logs_picker_and_diagnostics_are_identity_scoped() {
        let mut input = ready_event_input();
        input.registry.events = vec![input.selected_event.clone().unwrap()];
        input.registry.selected_id = Some("event-one".to_owned());
        let mut other = input.selected_event.clone().unwrap();
        other.event_id = "event-two".to_owned();
        other.label = Some("Same label".to_owned());
        input.registry.events.insert(0, other.clone());
        input.feedback.registration = EventCommandState::Succeeded;
        input.feedback.check = EventCommandState::Failed {
            detail: "HTTP 503 diagnostic body".to_owned(),
        };
        input.record_event_report(EventReportOperation::Registration, chrono::Utc::now());
        input.record_event_report(EventReportOperation::Check, chrono::Utc::now());
        let event = EventSectionDisplay::from_input(&input, true);
        assert!(event.picker.entries[0].detail.contains("event-two"));
        assert!(event.picker.entries[1]
            .label
            .contains("Selected · Configured"));
        assert!(!event.summary.contains("token") && !event.summary.contains("503"));
        assert!(
            event.diagnostics.contains("/tmp/event-one.token")
                && event.diagnostics.contains("HTTP 503 diagnostic body")
        );
        assert!(event
            .diagnostics
            .contains(event.feed_tag.as_deref().unwrap()));
        let mut colliding = input.registry.events.clone();
        colliding[0].event_id = "aaaaaaaaaaaaaaaaaa1-same-suffix".to_owned();
        colliding[1].event_id = "bbbbbbbbbbbbbbbbbb2-same-suffix".to_owned();
        assert_ne!(
            picker_event_id(&colliding[0].event_id, &colliding),
            picker_event_id(&colliding[1].event_id, &colliding)
        );
        assert!(
            compact_event_label(&"long label ".repeat(20))
                .chars()
                .count()
                <= 33
        );
        let mut pane = ShowLogPaneDisplay::closed();
        let stale = pane.begin_read(
            PublisherServiceRole::Producer,
            "producer.service".to_owned(),
        );
        pane.show_event(&input);
        assert!(!pane.close.disabled());
        assert!(!pane.apply_result(stale, Ok("old journal".to_owned())));
        let first_text = pane.text.clone();
        input.selected_event = Some(other);
        input.registry.selected_id = Some("event-two".to_owned());
        input.registry.revision += 1;
        pane.show_event(&input);
        assert!(pane.unit_name.contains("event-two"));
        assert_ne!(pane.text, first_text);
        assert!(pane.text.contains("App selected event event-two."));
        let current = pane.begin_read(
            PublisherServiceRole::Publisher,
            "publisher.service".to_owned(),
        );
        assert!(!pane.shows_event());
        assert!(pane.apply_result(current, Ok("line one\nline two\n".to_owned())));
        assert_eq!(pane.text, "line one\nline two\n");
    }
}

/// Renderer-free badge shared by card and item projections (ADR 0059/0063).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ShowItemBadge {
    pub(crate) kind: ShowCardStateKind,
    pub(crate) label: &'static str,
}

impl ShowItemBadge {
    const fn new(kind: ShowCardStateKind, label: &'static str) -> Self {
        Self { kind, label }
    }
}

/// Registry loading is distinct from a successfully read empty registry.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) enum EventRegistryStatus {
    Loading,
    Failed(String),
    #[default]
    Loaded,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct EventRegistryInput {
    pub(crate) status: EventRegistryStatus,
    pub(crate) events: Vec<EventSelectionInput>,
    pub(crate) selected_id: Option<String>,
    pub(crate) revision: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EventControlIntent {
    Create,
    Replace,
    Check,
    ReadTargets,
    Attach,
    Detach,
    Refresh,
    CopyFeedTag,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventControlDisplay {
    pub(crate) intent: EventControlIntent,
    pub(crate) action: EventActionDisplay,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventPickerEntry {
    pub(crate) detail: String,
    pub(crate) event_id: String,
    pub(crate) label: String,
    pub(crate) a11y_label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventPickerDisplay {
    pub(crate) label: String,
    pub(crate) a11y_label: String,
    pub(crate) availability: EventActionAvailability,
    pub(crate) entries: Vec<EventPickerEntry>,
}

fn event_control(id: &'static str, label: &'static str, available: bool) -> EventActionDisplay {
    registry_action_display(id, label, label, available)
}

fn event_badge(input: &EventSectionInput, target: &EventTargetAttachmentDisplay) -> ShowItemBadge {
    use ShowCardStateKind::{Attention, Failed, Ok, Unknown};
    let feedback = &input.feedback;
    let value = if feedback.registration == EventCommandState::Working {
        (
            Unknown,
            if feedback.active_action == Some(EventControlIntent::Replace) {
                "Replacing"
            } else {
                "Creating"
            },
        )
    } else if feedback.target_mutation == EventCommandState::Working {
        (
            Unknown,
            if feedback.active_action == Some(EventControlIntent::Detach) {
                "Detaching"
            } else {
                "Attaching"
            },
        )
    } else {
        match &input.registry.status {
            EventRegistryStatus::Loading => return ShowItemBadge::new(Unknown, "Loading"),
            EventRegistryStatus::Failed(_) => {
                return ShowItemBadge::new(Unknown, "Events unavailable")
            }
            EventRegistryStatus::Loaded => {}
        }
        let Some(event) = &input.selected_event else {
            return ShowItemBadge::new(
                Attention,
                if input.registry.selected_id.is_some() {
                    "Event unavailable"
                } else {
                    "No event"
                },
            );
        };
        if feedback.verification_failure.is_some()
            || matches!(feedback.check, EventCommandState::Failed { .. })
        {
            (Unknown, "Check failed")
        } else if feedback.check == EventCommandState::Working && !input.liveness_confirmed {
            (Unknown, "Checking")
        } else {
            match event.state {
                EventState::None => (Attention, "No event"),
                EventState::Dead => (Failed, "Dead"),
                EventState::Unknown => (
                    Unknown,
                    if feedback.check == EventCommandState::Working {
                        "Checking"
                    } else {
                        "Unknown"
                    },
                ),
                EventState::Live if input.attach_target_name.trim().is_empty() => {
                    (Attention, "Target not set")
                }
                EventState::Live => match target.state {
                    EventTargetAttachmentState::Attached => (Ok, "Attached"),
                    EventTargetAttachmentState::NotAttached => (Attention, "Not attached"),
                    EventTargetAttachmentState::Unknown => (Unknown, "Target unknown"),
                    EventTargetAttachmentState::CommandsUnavailable => {
                        (Unknown, "Commands unavailable")
                    }
                    EventTargetAttachmentState::NotReachable => (Unknown, "Not reachable"),
                    EventTargetAttachmentState::Failed => (Unknown, "Target read failed"),
                },
            }
        }
    };
    ShowItemBadge::new(value.0, value.1)
}

fn service_badge(state: &PublisherServiceStateDisplay) -> ShowItemBadge {
    let kind = match state.kind() {
        PublisherServiceStateKind::Active => ShowCardStateKind::Ok,
        PublisherServiceStateKind::Failed => ShowCardStateKind::Failed,
        PublisherServiceStateKind::Unknown | PublisherServiceStateKind::Working => {
            ShowCardStateKind::Unknown
        }
        PublisherServiceStateKind::Inactive
        | PublisherServiceStateKind::Starting
        | PublisherServiceStateKind::Stopping
        | PublisherServiceStateKind::NotInstalled
        | PublisherServiceStateKind::NotReachable => ShowCardStateKind::Attention,
    };
    ShowItemBadge::new(kind, state.label())
}

fn event_primary(
    input: &EventSectionInput,
    actions: &EventActionsDisplay,
) -> Option<EventControlDisplay> {
    use EventControlIntent::{Attach, Check, Create, Detach, ReadTargets, Refresh, Replace};
    if input.feedback.working() {
        let intent = input.feedback.active_action.unwrap_or(Check);
        let label = match intent {
            Create => "Creating",
            Replace => "Replacing",
            Attach => "Attaching",
            Detach => "Detaching",
            _ => "Checking",
        };
        let mut action = match intent {
            Create => actions.create.clone(),
            Replace => actions.replace.clone(),
            Check => actions.check.clone(),
            Attach => actions.attach.clone(),
            Detach => actions.detach.clone(),
            ReadTargets => event_control("event-read-targets", label, false),
            Refresh => event_control("event-refresh", label, false),
            EventControlIntent::CopyFeedTag => actions.copy_feed_tag.clone(),
        };
        action.label = label;
        action.a11y_label =
            format!("{label} event; controls unavailable until the request finishes");
        action.availability = EventActionAvailability::Unavailable;
        return Some(EventControlDisplay { intent, action });
    }
    let (intent, action) = match &input.registry.status {
        EventRegistryStatus::Loading => (Refresh, event_control("event-refresh", "Loading", false)),
        EventRegistryStatus::Failed(_) => (
            Refresh,
            event_control("event-refresh", "Retry events", true),
        ),
        EventRegistryStatus::Loaded => match &input.selected_event {
            None if input.registry.events.is_empty() && input.registry.selected_id.is_none() => {
                (Create, actions.create.clone())
            }
            None => return None,
            Some(_)
                if input.feedback.verification_failure.is_some()
                    || matches!(input.feedback.check, EventCommandState::Failed { .. }) =>
            {
                (Check, actions.check.clone())
            }
            Some(event) => match event.state {
                EventState::None => return None,
                EventState::Unknown => (Check, actions.check.clone()),
                EventState::Dead => (Replace, actions.replace.clone()),
                EventState::Live => match &input.targets {
                    EventTargetListInput::Loaded { .. } if actions.attach.disabled() => {
                        return None
                    }
                    EventTargetListInput::Loaded { .. } => (Attach, actions.attach.clone()),
                    _ => (
                        ReadTargets,
                        event_control("event-read-targets", "Retry config", true),
                    ),
                },
            },
        },
    };
    Some(EventControlDisplay { intent, action })
}

fn event_overflow(
    input: &EventSectionInput,
    actions: &EventActionsDisplay,
) -> Vec<EventControlDisplay> {
    if input.selected_event.is_none() {
        return Vec::new();
    }
    let mut check = actions.check.clone();
    check.label = "Check again";
    "Check whether the relay still has this event and read the publisher configuration again"
        .clone_into(&mut check.a11y_label);
    vec![
        EventControlDisplay {
            intent: EventControlIntent::CopyFeedTag,
            action: actions.copy_feed_tag.clone(),
        },
        EventControlDisplay {
            intent: EventControlIntent::Check,
            action: check,
        },
        EventControlDisplay {
            intent: EventControlIntent::Detach,
            action: actions.detach.clone(),
        },
    ]
}

fn event_picker(input: &EventSectionInput, event: &EventSelectionDisplay) -> EventPickerDisplay {
    let configured_id = match &input.targets {
        EventTargetListInput::Loaded { targets } if !input.attach_target_name.trim().is_empty() => {
            targets
                .iter()
                .find(|target| target.name == input.attach_target_name.trim())
                .map(|target| target.event_id.as_str())
        }
        _ => None,
    };
    let entries = input
        .registry
        .events
        .iter()
        .map(|entry| {
            let selected = input.registry.selected_id.as_deref() == Some(&entry.event_id);
            let configured = configured_id == Some(&entry.event_id);
            let id = picker_event_id(&entry.event_id, &input.registry.events);
            let label = format!(
                "{}{}{}",
                entry.label.as_deref().unwrap_or(&id),
                if selected { " · Selected" } else { "" },
                if configured { " · Configured" } else { "" }
            );
            let detail = format!(
                "{} · {} · {id}",
                event_date(entry.created_at),
                entry.state.display().label
            );
            EventPickerEntry {
                event_id: entry.event_id.clone(),
                a11y_label: format!("Select event {label}, {detail}, full ID {}", entry.event_id),
                label,
                detail,
            }
        })
        .collect();
    let label = if input.selected_event.is_none() && input.registry.selected_id.is_some() {
        "Choose an event".to_owned()
    } else if let Some(selected) = &input.selected_event {
        selected
            .label
            .as_deref()
            .filter(|label| !label.trim().is_empty())
            .map_or_else(
                || {
                    let date = chrono::DateTime::from_timestamp(selected.created_at, 0)
                        .map_or_else(String::new, |date| date.format("%m-%d %H:%M").to_string());
                    format!(
                        "{date} · {}",
                        picker_event_id(&selected.event_id, &input.registry.events)
                    )
                },
                compact_event_label,
            )
    } else {
        event.label.clone()
    };
    EventPickerDisplay {
        a11y_label: format!("Choose stored event, {}", event.label),
        label,
        entries,
        availability: event_action_availability(
            !input.feedback.working()
                && !input.registry.events.is_empty()
                && input.registry.status == EventRegistryStatus::Loaded,
        ),
    }
}

fn event_date(timestamp: i64) -> String {
    chrono::DateTime::from_timestamp(timestamp, 0).map_or_else(
        || timestamp.to_string(),
        |date| date.format("%Y-%m-%d %H:%M UTC").to_string(),
    )
}

fn event_diagnostics(input: &EventSectionInput) -> String {
    event_report::render(input)
}

impl ShowLogPaneDisplay {
    pub(crate) fn shows_event(&self) -> bool {
        self.open && self.event_source.is_some()
    }

    /// Source and text move together; old journal responses lose their request ownership.
    pub(crate) fn show_event(&mut self, input: &EventSectionInput) {
        self.open = true;
        self.close.availability = PublisherActionAvailability::Available;
        self.service_detail = None;
        self.role = None;
        self.request = None;
        self.event_source = Some(format!(
            "{}:{}:{}",
            input.context,
            input.registry.revision,
            input.registry.selected_id.as_deref().unwrap_or("none")
        ));
        self.unit_name = format!(
            "Event · {}",
            input
                .selected_event
                .as_ref()
                .map_or("No event", |event| event.event_id.as_str())
        );
        self.text = event_diagnostics(input);
        self.feed_tag = input
            .selected_event
            .as_ref()
            .map(|event| feed_tag_for_event(&event.event_id));
        self.copy_feed_tag = self
            .feed_tag
            .as_ref()
            .map(|_| event_control("event-log-copy-feed-tag", "Copy feed tag", true));
        self.line_count = self.text.lines().count();
        self.line_count_label = format!("{} lines", self.line_count);
    }
}

fn event_result_hint(input: &EventSectionInput) -> Option<String> {
    if matches!(input.feedback.selection, EventCommandState::Failed { .. }) {
        Some("Could not save the event choice. Open Logs.".to_owned())
    } else if matches!(
        input.feedback.registration,
        EventCommandState::Failed { .. }
    ) {
        Some("Could not create the event. Open Logs.".to_owned())
    } else if matches!(
        input.feedback.target_mutation,
        EventCommandState::Failed { .. }
    ) {
        Some("Publisher configuration action failed. Open Logs.".to_owned())
    } else if let EventTargetListInput::Loaded { targets } = &input.targets {
        targets
            .iter()
            .find(|target| {
                target.name == input.attach_target_name.trim()
                    && input
                        .selected_event
                        .as_ref()
                        .is_some_and(|event| target.event_id != event.event_id)
            })
            .map(|target| {
                format!(
                    "Publisher configured for {}. Attach replaces it.",
                    picker_event_id(&target.event_id, &input.registry.events)
                )
            })
    } else {
        None
    }
}

/// Shorten IDs only when the displayed suffix distinguishes every stored entry.
fn picker_event_id(id: &str, events: &[EventSelectionInput]) -> String {
    let chars: Vec<char> = id.chars().collect();
    if chars.len() <= 20 {
        return id.to_owned();
    }
    for count in 8..chars.len() {
        let suffix: String = chars[chars.len() - count..].iter().collect();
        if !events
            .iter()
            .any(|event| event.event_id != id && event.event_id.ends_with(&suffix))
        {
            return format!("…{suffix}");
        }
    }
    id.to_owned()
}

fn compact_event_label(label: &str) -> String {
    let mut chars = label.trim().chars();
    let prefix: String = chars.by_ref().take(32).collect();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}
