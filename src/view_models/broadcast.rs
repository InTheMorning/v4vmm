#![warn(clippy::pedantic)]
#![expect(
    dead_code,
    reason = "ADR 0059 task 005 defines the BroadcastPageVm before shell wiring consumes it"
)]

//! Broadcast frame display contracts for ADR 0059.
//!
//! This module owns the GPUI-free projection for the Broadcast frame. It
//! carries display-ready section facts, typed action availability, and the
//! ready-to-paste RSS `podcast:liveValue` tag. Screens and shells bind these
//! facts to controls without reaching into registry services, database rows, or
//! runtime handles.

use crate::runtime::BroadcastObservationOutcome;
use crate::view_models::workspace::FrameChromeButtonDisplay;

/// Current state of a selected broadcast source.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum SourceState {
    /// The source has not reported state yet.
    #[default]
    Unknown,
    /// The source is reachable and not playing.
    Idle,
    /// The source is playing a track.
    Playing,
    /// The source has an active track but playback is paused.
    Paused,
    /// The source cannot be reached from this host.
    NotReachable,
}

/// Current state of one publisher-side unit.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ServiceState {
    /// The service state has not been checked yet.
    #[default]
    Unknown,
    /// The unit is active.
    Active,
    /// The unit is installed and stopped.
    Inactive,
    /// The unit is installed and failed.
    Failed,
    /// The unit is not installed on the target host.
    NotInstalled,
    /// The target host or service manager is not reachable.
    NotReachable,
}

/// Current state of the selected relay event.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum EventState {
    /// No event is selected.
    #[default]
    None,
    /// An event is selected but listener truth is unknown.
    Unknown,
    /// The relay has listener-visible metadata for the event.
    Live,
    /// The relay reports the event as dead.
    Dead,
}

/// Typed command availability for broadcast actions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ActionAvailability {
    /// The command can be dispatched.
    Available,
    /// The command is visible but cannot be dispatched.
    Unavailable,
}

impl ActionAvailability {
    const fn disabled(self) -> bool {
        matches!(self, Self::Unavailable)
    }
}

/// Display contract for one broadcast action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ActionDisplay {
    /// Visible command label.
    pub(crate) label: &'static str,
    /// Typed command availability.
    pub(crate) availability: ActionAvailability,
    /// Shared frame button facts for id, accessibility, and disabled state.
    pub(crate) chrome: FrameChromeButtonDisplay,
}

impl ActionDisplay {
    /// Creates a broadcast action display.
    #[must_use]
    pub(crate) fn new(
        id: impl Into<String>,
        label: &'static str,
        a11y_label: &'static str,
        availability: ActionAvailability,
    ) -> Self {
        Self {
            label,
            availability,
            chrome: FrameChromeButtonDisplay::new(id, a11y_label, availability.disabled()),
        }
    }
}

/// Display-ready empty state for a broadcast section.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SectionEmptyStateDisplay {
    /// Primary empty-state label.
    pub(crate) title: &'static str,
    /// Secondary empty-state detail.
    pub(crate) message: &'static str,
}

impl SectionEmptyStateDisplay {
    const fn new(title: &'static str, message: &'static str) -> Self {
        Self { title, message }
    }
}

/// Plain source facts supplied by the application layer.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SourceSectionInput {
    /// Operator-visible source name.
    pub(crate) source_name: Option<String>,
    /// Operator-visible source kind.
    pub(crate) source_kind_label: Option<String>,
    /// Operator-visible host label.
    pub(crate) host_label: Option<String>,
    /// Current source state.
    pub(crate) state: SourceState,
    /// Optional title of the current track.
    pub(crate) current_track_title: Option<String>,
    /// Optional artist label of the current track.
    pub(crate) current_track_artist: Option<String>,
    /// Reserved readiness label populated by ADR 0059 task 012.
    pub(crate) readiness_label: Option<String>,
}

/// Display-ready Source section state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceSectionDisplay {
    /// Operator-visible source name.
    pub(crate) source_name: String,
    /// Operator-visible source kind.
    pub(crate) source_kind_label: String,
    /// Operator-visible host label.
    pub(crate) host_label: String,
    /// Current source state.
    pub(crate) state: SourceState,
    /// Optional title of the current track.
    pub(crate) current_track_title: Option<String>,
    /// Optional artist label of the current track.
    pub(crate) current_track_artist: Option<String>,
    /// Reserved readiness label populated by ADR 0059 task 012.
    pub(crate) readiness_label: Option<String>,
    /// Empty-state display for non-playing or unreachable source states.
    pub(crate) empty_state: Option<SectionEmptyStateDisplay>,
}

impl SourceSectionDisplay {
    fn from_input(input: SourceSectionInput) -> Self {
        let empty_state = match input.state {
            SourceState::Unknown => Some(SectionEmptyStateDisplay::new(
                "No source selected",
                "Choose a source before starting broadcast observation.",
            )),
            SourceState::Idle => Some(SectionEmptyStateDisplay::new(
                "Source idle",
                "The source is reachable and not playing.",
            )),
            SourceState::NotReachable => Some(SectionEmptyStateDisplay::new(
                "Source not reachable",
                "The selected source cannot be reached.",
            )),
            SourceState::Playing | SourceState::Paused => None,
        };

        Self {
            source_name: display_text(input.source_name, "No source selected"),
            source_kind_label: display_text(input.source_kind_label, "Unknown source"),
            host_label: display_text(input.host_label, "Local host"),
            state: input.state,
            current_track_title: nonempty_owned(input.current_track_title),
            current_track_artist: nonempty_owned(input.current_track_artist),
            readiness_label: nonempty_owned(input.readiness_label),
            empty_state,
        }
    }
}

/// Plain publisher facts supplied by the application layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublisherSectionInput {
    /// Publisher unit label.
    pub(crate) publisher_unit_name: Option<String>,
    /// Producer unit label.
    pub(crate) producer_unit_name: Option<String>,
    /// Current publisher unit state.
    pub(crate) publisher_state: ServiceState,
    /// Current producer unit state.
    pub(crate) producer_state: ServiceState,
    /// Optional failure reason.
    pub(crate) failure_reason_label: Option<String>,
}

impl Default for PublisherSectionInput {
    fn default() -> Self {
        Self {
            publisher_unit_name: None,
            producer_unit_name: None,
            publisher_state: ServiceState::Unknown,
            producer_state: ServiceState::Unknown,
            failure_reason_label: None,
        }
    }
}

/// Display-ready Publisher section state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublisherSectionDisplay {
    /// Publisher unit label.
    pub(crate) publisher_unit_name: String,
    /// Producer unit label.
    pub(crate) producer_unit_name: String,
    /// Current publisher unit state.
    pub(crate) publisher_state: ServiceState,
    /// Current producer unit state.
    pub(crate) producer_state: ServiceState,
    /// Start command display.
    pub(crate) start: ActionDisplay,
    /// Stop command display.
    pub(crate) stop: ActionDisplay,
    /// Reset command display.
    pub(crate) reset: ActionDisplay,
    /// Logs command display.
    pub(crate) logs: ActionDisplay,
    /// Optional failure reason.
    pub(crate) failure_reason_label: Option<String>,
    /// Empty-state display for missing or unreachable publisher states.
    pub(crate) empty_state: Option<SectionEmptyStateDisplay>,
}

impl PublisherSectionDisplay {
    fn from_input(input: PublisherSectionInput) -> Self {
        let states = [input.publisher_state, input.producer_state];
        let failed = states.contains(&ServiceState::Failed);
        let active = states.contains(&ServiceState::Active);
        let unavailable = states.iter().any(|state| {
            matches!(
                state,
                ServiceState::Unknown | ServiceState::NotInstalled | ServiceState::NotReachable
            )
        });
        let empty_state = publisher_empty_state(states);

        Self {
            publisher_unit_name: display_text(input.publisher_unit_name, "Publisher unit"),
            producer_unit_name: display_text(input.producer_unit_name, "Producer unit"),
            publisher_state: input.publisher_state,
            producer_state: input.producer_state,
            start: ActionDisplay::new(
                "broadcast-publisher-start",
                "Start",
                "Start broadcast publisher services",
                availability(!failed && !active && !unavailable),
            ),
            stop: ActionDisplay::new(
                "broadcast-publisher-stop",
                "Stop",
                "Stop broadcast publisher services",
                availability(active && !unavailable),
            ),
            reset: ActionDisplay::new(
                "broadcast-publisher-reset",
                "Reset",
                "Reset failed broadcast publisher services",
                availability(failed && !unavailable),
            ),
            logs: ActionDisplay::new(
                "broadcast-publisher-logs",
                "Logs",
                "Show broadcast publisher logs",
                availability(!states.iter().any(|state| {
                    matches!(
                        state,
                        ServiceState::NotInstalled | ServiceState::NotReachable
                    )
                })),
            ),
            failure_reason_label: nonempty_owned(input.failure_reason_label),
            empty_state,
        }
    }
}

/// Plain selected event facts supplied by the application layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventSectionInput {
    /// Optional operator label.
    pub(crate) event_label: Option<String>,
    /// Relay event identifier.
    pub(crate) event_identifier: String,
    /// Relay endpoint used for this event.
    pub(crate) endpoint: String,
    /// Path to the broadcaster token file.
    pub(crate) token_path: String,
    /// Stored event state before observation is applied.
    pub(crate) state: EventState,
}

/// Display-ready Event section state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventSectionDisplay {
    /// Optional operator label.
    pub(crate) event_label: Option<String>,
    /// Relay event identifier.
    pub(crate) event_identifier: Option<String>,
    /// Relay endpoint used for this event.
    pub(crate) endpoint: Option<String>,
    /// Path to the broadcaster token file.
    pub(crate) token_path: Option<String>,
    /// Current event state.
    pub(crate) state: EventState,
    /// Create command display.
    pub(crate) create: ActionDisplay,
    /// Resume command display.
    pub(crate) resume: ActionDisplay,
    /// Forget command display.
    pub(crate) forget: ActionDisplay,
    /// Copy-feed-tag command display.
    pub(crate) copy_feed_tag: ActionDisplay,
    /// Listener-truth summary line.
    pub(crate) listener_truth_line: String,
    /// Ready-to-paste RSS `podcast:liveValue` tag.
    pub(crate) feed_tag: Option<String>,
    /// Empty-state display for missing or dead events.
    pub(crate) empty_state: Option<SectionEmptyStateDisplay>,
}

impl EventSectionDisplay {
    fn from_input(
        event: Option<&EventSectionInput>,
        observation: &BroadcastObservationOutcome,
    ) -> Self {
        let state = event_state(event, observation);
        let has_event = event.is_some();
        let feed_tag = event
            .as_ref()
            .map(|event| feed_tag_for_event(&event.event_identifier));
        let listener_truth_line = listener_truth_line(has_event, observation);
        let empty_state = event_empty_state(state, observation);

        Self {
            event_label: event.and_then(|event| {
                nonempty_owned(event.event_label.clone())
                    .or_else(|| Some(event.event_identifier.clone()))
            }),
            event_identifier: event
                .and_then(|event| nonempty_owned(Some(event.event_identifier.clone()))),
            endpoint: event.and_then(|event| nonempty_owned(Some(event.endpoint.clone()))),
            token_path: event.and_then(|event| nonempty_owned(Some(event.token_path.clone()))),
            state,
            create: ActionDisplay::new(
                "broadcast-event-create",
                "Create",
                "Create broadcast event",
                ActionAvailability::Available,
            ),
            resume: ActionDisplay::new(
                "broadcast-event-resume",
                "Resume",
                "Resume selected broadcast event",
                availability(has_event && state != EventState::Dead),
            ),
            forget: ActionDisplay::new(
                "broadcast-event-forget",
                "Forget",
                "Forget selected broadcast event",
                availability(has_event),
            ),
            copy_feed_tag: ActionDisplay::new(
                "broadcast-event-copy-feed-tag",
                "Copy",
                "Copy broadcast live value RSS tag",
                availability(has_event),
            ),
            listener_truth_line,
            feed_tag,
            empty_state,
        }
    }
}

/// Plain page facts supplied by the application layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BroadcastPageInput {
    /// Source section input.
    pub(crate) source: SourceSectionInput,
    /// Publisher section input.
    pub(crate) publisher: PublisherSectionInput,
    /// Selected event input.
    pub(crate) event: Option<EventSectionInput>,
    /// Latest relay observation outcome.
    pub(crate) observation: BroadcastObservationOutcome,
}

impl Default for BroadcastPageInput {
    fn default() -> Self {
        Self {
            source: SourceSectionInput::default(),
            publisher: PublisherSectionInput::default(),
            event: None,
            observation: BroadcastObservationOutcome::NoEvent,
        }
    }
}

/// Display-ready Broadcast page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BroadcastPageVm {
    /// Display-ready Source section.
    pub(crate) source: SourceSectionDisplay,
    /// Display-ready Publisher section.
    pub(crate) publisher: PublisherSectionDisplay,
    /// Display-ready Event section.
    pub(crate) event: EventSectionDisplay,
    /// Page-level empty label.
    pub(crate) empty_label: &'static str,
}

impl BroadcastPageVm {
    /// Creates a Broadcast page builder.
    pub(crate) fn builder() -> BroadcastPageVmBuilder {
        BroadcastPageVmBuilder::default()
    }

    /// Projects plain inputs into a display-ready Broadcast page.
    #[must_use]
    pub(crate) fn project(input: BroadcastPageInput) -> Self {
        Self {
            source: SourceSectionDisplay::from_input(input.source),
            publisher: PublisherSectionDisplay::from_input(input.publisher),
            event: EventSectionDisplay::from_input(input.event.as_ref(), &input.observation),
            empty_label: "No broadcast event selected",
        }
    }
}

/// Builder for [`BroadcastPageVm`].
#[derive(Clone, Debug, Default)]
#[must_use]
pub(crate) struct BroadcastPageVmBuilder {
    input: BroadcastPageInput,
}

impl BroadcastPageVmBuilder {
    /// Supplies source section facts.
    pub(crate) fn source(mut self, source: SourceSectionInput) -> Self {
        self.input.source = source;
        self
    }

    /// Supplies publisher section facts.
    pub(crate) fn publisher(mut self, publisher: PublisherSectionInput) -> Self {
        self.input.publisher = publisher;
        self
    }

    /// Supplies selected event facts.
    pub(crate) fn event(mut self, event: EventSectionInput) -> Self {
        self.input.event = Some(event);
        self
    }

    /// Supplies relay observation outcome.
    pub(crate) fn observation(mut self, observation: BroadcastObservationOutcome) -> Self {
        self.input.observation = observation;
        self
    }

    /// Projects the builder input into a display-ready page.
    #[must_use]
    pub(crate) fn build(self) -> BroadcastPageVm {
        BroadcastPageVm::project(self.input)
    }
}

fn event_state(
    event: Option<&EventSectionInput>,
    observation: &BroadcastObservationOutcome,
) -> EventState {
    let Some(event) = event else {
        return EventState::None;
    };

    match observation {
        BroadcastObservationOutcome::Live { .. } => EventState::Live,
        BroadcastObservationOutcome::Dead => EventState::Dead,
        BroadcastObservationOutcome::NoEvent
        | BroadcastObservationOutcome::Empty
        | BroadcastObservationOutcome::Error(_) => event.state,
    }
}

fn listener_truth_line(has_event: bool, observation: &BroadcastObservationOutcome) -> String {
    if !has_event {
        return "No event selected".to_string();
    }

    match observation {
        BroadcastObservationOutcome::NoEvent => "Relay not checked".to_string(),
        BroadcastObservationOutcome::Live {
            seq,
            updated_at,
            title,
            destination_count,
        } => {
            let title = title
                .as_deref()
                .and_then(nonempty_str)
                .unwrap_or("Untitled payload");
            format!(
                "Relay live: {title}, {}, seq {seq}, updated {updated_at}",
                destination_count_label(*destination_count)
            )
        }
        BroadcastObservationOutcome::Empty => "Relay event has no payload".to_string(),
        BroadcastObservationOutcome::Dead => "Relay event is dead".to_string(),
        BroadcastObservationOutcome::Error(error) => format!("Relay check failed: {error}"),
    }
}

fn publisher_empty_state(states: [ServiceState; 2]) -> Option<SectionEmptyStateDisplay> {
    if states.contains(&ServiceState::NotInstalled) {
        return Some(SectionEmptyStateDisplay::new(
            "Publisher not installed",
            "Install the publisher tools before controlling broadcast services.",
        ));
    }
    if states.contains(&ServiceState::NotReachable) {
        return Some(SectionEmptyStateDisplay::new(
            "Publisher not reachable",
            "The selected host or service manager cannot be reached.",
        ));
    }
    if states.iter().all(|state| *state == ServiceState::Unknown) {
        return Some(SectionEmptyStateDisplay::new(
            "Publisher status unknown",
            "Publisher service state has not been checked.",
        ));
    }
    None
}

fn event_empty_state(
    state: EventState,
    observation: &BroadcastObservationOutcome,
) -> Option<SectionEmptyStateDisplay> {
    match (state, observation) {
        (EventState::None, _) => Some(SectionEmptyStateDisplay::new(
            "No broadcast event",
            "Create or select an event before listener-truth checks.",
        )),
        (EventState::Dead, _) => Some(SectionEmptyStateDisplay::new(
            "Broadcast event dead",
            "Create a new event before listeners tune in again.",
        )),
        (_, BroadcastObservationOutcome::Empty) => Some(SectionEmptyStateDisplay::new(
            "No relay payload",
            "The relay event exists but has no listener-visible payload.",
        )),
        _ => None,
    }
}

fn availability(available: bool) -> ActionAvailability {
    if available {
        ActionAvailability::Available
    } else {
        ActionAvailability::Unavailable
    }
}

fn destination_count_label(count: usize) -> String {
    match count {
        1 => "1 destination".to_string(),
        count => format!("{count} destinations"),
    }
}

fn display_text(value: Option<String>, fallback: &str) -> String {
    nonempty_owned(value).unwrap_or_else(|| fallback.to_string())
}

fn nonempty_owned(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn nonempty_str(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn feed_tag_for_event(event_identifier: &str) -> String {
    format!(
        r#"<podcast:liveValue uri="{}" protocol="socket.io"/>"#,
        escape_xml_attribute(event_identifier)
    )
}

fn escape_xml_attribute(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            ch => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(state: EventState) -> EventSectionInput {
        EventSectionInput {
            event_label: Some("Sunday set".to_string()),
            event_identifier: "EVENT_ID".to_string(),
            endpoint: "https://relay.example.test".to_string(),
            token_path: "/tmp/event.token".to_string(),
            state,
        }
    }

    fn active_publisher() -> PublisherSectionInput {
        PublisherSectionInput {
            publisher_unit_name: Some("musicindex-live-publisher.service".to_string()),
            producer_unit_name: Some("producer.service".to_string()),
            publisher_state: ServiceState::Active,
            producer_state: ServiceState::Active,
            failure_reason_label: None,
        }
    }

    #[test]
    fn no_event_projects_event_empty_state_and_unavailable_event_actions() {
        let vm = BroadcastPageVm::builder().build();

        assert_eq!(vm.event.state, EventState::None);
        assert_eq!(vm.event.listener_truth_line, "No event selected");
        assert!(vm.event.feed_tag.is_none());
        assert_eq!(
            vm.event.empty_state,
            Some(SectionEmptyStateDisplay::new(
                "No broadcast event",
                "Create or select an event before listener-truth checks.",
            ))
        );
        assert_eq!(vm.event.create.availability, ActionAvailability::Available);
        assert_eq!(
            vm.event.resume.availability,
            ActionAvailability::Unavailable
        );
        assert_eq!(
            vm.event.forget.availability,
            ActionAvailability::Unavailable
        );
        assert!(vm.event.copy_feed_tag.chrome.disabled);
    }

    #[test]
    fn live_event_projects_listener_truth_and_available_resume() {
        let vm = BroadcastPageVm::builder()
            .event(event(EventState::Unknown))
            .observation(BroadcastObservationOutcome::Live {
                seq: 7,
                updated_at: "2026-09-07T00:00:00Z".to_string(),
                title: Some("Now Playing".to_string()),
                destination_count: 2,
            })
            .build();

        assert_eq!(vm.event.state, EventState::Live);
        assert_eq!(
            vm.event.listener_truth_line,
            "Relay live: Now Playing, 2 destinations, seq 7, updated 2026-09-07T00:00:00Z"
        );
        assert_eq!(
            vm.event.feed_tag.as_deref(),
            Some(r#"<podcast:liveValue uri="EVENT_ID" protocol="socket.io"/>"#)
        );
        assert_eq!(vm.event.resume.availability, ActionAvailability::Available);
        assert!(!vm.event.copy_feed_tag.chrome.disabled);
    }

    #[test]
    fn dead_event_disables_resume_and_keeps_forget_available() {
        let vm = BroadcastPageVm::builder()
            .event(event(EventState::Unknown))
            .observation(BroadcastObservationOutcome::Dead)
            .build();

        assert_eq!(vm.event.state, EventState::Dead);
        assert_eq!(
            vm.event.resume.availability,
            ActionAvailability::Unavailable
        );
        assert_eq!(vm.event.forget.availability, ActionAvailability::Available);
        assert!(!vm.event.forget.chrome.disabled);
        assert_eq!(vm.event.listener_truth_line, "Relay event is dead");
    }

    #[test]
    fn failed_unit_disables_start_and_enables_reset() {
        let publisher = PublisherSectionInput {
            publisher_state: ServiceState::Failed,
            producer_state: ServiceState::Inactive,
            failure_reason_label: Some("restart limit hit".to_string()),
            ..PublisherSectionInput::default()
        };

        let vm = BroadcastPageVm::builder().publisher(publisher).build();

        assert_eq!(
            vm.publisher.start.availability,
            ActionAvailability::Unavailable
        );
        assert_eq!(
            vm.publisher.reset.availability,
            ActionAvailability::Available
        );
        assert_eq!(
            vm.publisher.failure_reason_label.as_deref(),
            Some("restart limit hit")
        );
        assert!(vm.publisher.start.chrome.disabled);
        assert!(!vm.publisher.reset.chrome.disabled);
    }

    #[test]
    fn active_publisher_disables_start_and_enables_stop() {
        let vm = BroadcastPageVm::builder()
            .publisher(active_publisher())
            .build();

        assert_eq!(
            vm.publisher.start.availability,
            ActionAvailability::Unavailable
        );
        assert_eq!(
            vm.publisher.stop.availability,
            ActionAvailability::Available
        );
        assert!(vm.publisher.start.chrome.disabled);
        assert!(!vm.publisher.stop.chrome.disabled);
    }

    #[test]
    fn inactive_publisher_enables_start_and_disables_reset() {
        let publisher = PublisherSectionInput {
            publisher_state: ServiceState::Inactive,
            producer_state: ServiceState::Inactive,
            ..PublisherSectionInput::default()
        };

        let vm = BroadcastPageVm::builder().publisher(publisher).build();

        assert_eq!(
            vm.publisher.start.availability,
            ActionAvailability::Available
        );
        assert_eq!(
            vm.publisher.reset.availability,
            ActionAvailability::Unavailable
        );
    }

    #[test]
    fn publisher_not_installed_projects_empty_state_and_unavailable_actions() {
        let publisher = PublisherSectionInput {
            publisher_state: ServiceState::NotInstalled,
            producer_state: ServiceState::Unknown,
            ..PublisherSectionInput::default()
        };

        let vm = BroadcastPageVm::builder().publisher(publisher).build();

        assert_eq!(
            vm.publisher.empty_state,
            Some(SectionEmptyStateDisplay::new(
                "Publisher not installed",
                "Install the publisher tools before controlling broadcast services.",
            ))
        );
        assert_eq!(
            vm.publisher.start.availability,
            ActionAvailability::Unavailable
        );
        assert!(vm.publisher.logs.chrome.disabled);
    }

    #[test]
    fn source_paused_keeps_current_track_visible() {
        let source = SourceSectionInput {
            source_name: Some("Studio deck".to_string()),
            source_kind_label: Some("Deck".to_string()),
            host_label: Some("Booth".to_string()),
            state: SourceState::Paused,
            current_track_title: Some("Quiet Track".to_string()),
            current_track_artist: Some("Artist".to_string()),
            readiness_label: None,
        };

        let vm = BroadcastPageVm::builder().source(source).build();

        assert_eq!(vm.source.state, SourceState::Paused);
        assert_eq!(vm.source.source_kind_label, "Deck");
        assert_eq!(
            vm.source.current_track_title.as_deref(),
            Some("Quiet Track")
        );
        assert_eq!(vm.source.current_track_artist.as_deref(), Some("Artist"));
        assert!(vm.source.empty_state.is_none());
    }

    #[test]
    fn source_not_reachable_projects_explicit_empty_state() {
        let source = SourceSectionInput {
            source_name: Some("Remote source".to_string()),
            state: SourceState::NotReachable,
            ..SourceSectionInput::default()
        };

        let vm = BroadcastPageVm::builder().source(source).build();

        assert_eq!(vm.source.state, SourceState::NotReachable);
        assert_eq!(
            vm.source.empty_state,
            Some(SectionEmptyStateDisplay::new(
                "Source not reachable",
                "The selected source cannot be reached.",
            ))
        );
    }

    #[test]
    fn feed_tag_is_complete_escaped_and_copy_unavailable_without_event() {
        let no_event = BroadcastPageVm::builder().build();
        assert!(no_event.event.feed_tag.is_none());
        assert_eq!(
            no_event.event.copy_feed_tag.availability,
            ActionAvailability::Unavailable
        );

        let selected = BroadcastPageVm::builder()
            .event(EventSectionInput {
                event_identifier: "EVENT_&\"'<>".to_string(),
                ..event(EventState::Unknown)
            })
            .build();

        assert_eq!(
            selected.event.feed_tag.as_deref(),
            Some(
                r#"<podcast:liveValue uri="EVENT_&amp;&quot;&apos;&lt;&gt;" protocol="socket.io"/>"#
            )
        );
        assert_eq!(
            selected.event.copy_feed_tag.availability,
            ActionAvailability::Available
        );
    }
}
