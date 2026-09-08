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
use crate::{broadcast::control::ServiceState, runtime::broadcast_service_watch};

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
    /// Optional Publisher section; absent sections render nothing.
    pub(crate) publisher: Option<PublisherSectionDisplay>,
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
        let state_label = transport_state_label(queue.transport.play_pause_state);
        let now_playing = queue
            .rows
            .iter()
            .find(|row| row.now_playing)
            .map(ShowNowPlayingDisplay::from_queue_row);
        let active = queue.transport.play_pause_state.is_active();
        let publisher = match publisher_snapshot {
            Some(snapshot) => PublisherSectionDisplay::from_snapshot(snapshot, log_panel),
            None => None,
        };
        Self {
            title: "Show",
            state_label,
            now_playing,
            empty_state: (!active).then_some(ShowEmptyStateDisplay {
                id: "show-empty-state",
                title: "No active show",
                subtitle: "Show playback is idle.",
            }),
            publisher,
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
        assert!(vm.publisher.is_none());
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

    fn publisher_snapshot<const N: usize>(
        units: [(PublisherServiceRole, &str, ServiceState); N],
    ) -> broadcast_service_watch::BroadcastServiceWatchSnapshot {
        broadcast_service_watch::BroadcastServiceWatchSnapshot {
            at: std::time::Instant::now(),
            units: units
                .into_iter()
                .map(|(role, unit_name, state)| {
                    broadcast_service_watch::BroadcastServiceUnitSnapshot {
                        role,
                        unit_name: unit_name.to_owned(),
                        state,
                    }
                })
                .collect(),
        }
    }
}
