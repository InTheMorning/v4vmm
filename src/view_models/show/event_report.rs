//! Timestamped Event reports with concrete subjects and outcomes (ADRs 0059/0063).

use chrono::{DateTime, Utc};

use super::{
    feed_tag_for_event, EventCommandState, EventControlIntent, EventRegistryStatus,
    EventSectionInput, EventSelectionInput, EventState, EventTargetListInput,
};

/// Response facts supplied by the command boundary, never parsed from error text.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum EventCheckResponse {
    #[default]
    Unclassified,
    NoResponse,
    Http(u16),
    SaveFailed(EventState),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EventReportOperation {
    Registration,
    Selection,
    Check,
    TargetRead,
    TargetMutation,
}

/// Only the latest recorded result per operation, retained within this app session.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct EventReport {
    entries: Vec<EventReportEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EventReportEntry {
    at: DateTime<Utc>,
    operation: EventReportOperation,
    result: EventCommandState,
    event: Option<EventSelectionInput>,
    response: EventCheckResponse,
    publisher: String,
    target: String,
    target_detail: String,
    intent: Option<EventControlIntent>,
}

impl EventSectionInput {
    pub(crate) fn record_event_report(
        &mut self,
        operation: EventReportOperation,
        at: DateTime<Utc>,
    ) {
        self.record_event_report_for(operation, at, self.selected_event.clone());
    }

    pub(crate) fn record_event_report_for(
        &mut self,
        operation: EventReportOperation,
        at: DateTime<Utc>,
        event: Option<EventSelectionInput>,
    ) {
        let result = match operation {
            EventReportOperation::Registration => &self.feedback.registration,
            EventReportOperation::Selection => &self.feedback.selection,
            EventReportOperation::Check => &self.feedback.check,
            EventReportOperation::TargetRead => &self.feedback.target_read,
            EventReportOperation::TargetMutation => &self.feedback.target_mutation,
        };
        let entry = EventReportEntry {
            at,
            operation,
            result: result.clone(),
            event,
            response: self.feedback.check_response,
            publisher: self.context.clone(),
            target: self.attach_target_name.clone(),
            target_detail: target_description(self),
            intent: self.feedback.active_action,
        };
        self.feedback
            .report
            .entries
            .retain(|entry| entry.operation != operation);
        if entry.result != EventCommandState::Idle {
            self.feedback.report.entries.push(entry);
        }
    }
}

fn time(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

fn stored_time(timestamp: i64) -> String {
    DateTime::from_timestamp(timestamp, 0).map_or_else(|| "Time unavailable".to_owned(), time)
}

pub(super) fn render(input: &EventSectionInput) -> String {
    let mut lines = vec!["Saved event details".to_owned()];
    if let Some(event) = &input.selected_event {
        lines.push(format!("App selected event {}.", event.event_id));
        if let Some(label) = &event.label {
            lines.push(format!("Event name: {label}"));
        }
        lines.extend([
            format!(
                "[{}] App stored event {}.",
                stored_time(event.created_at),
                event.event_id
            ),
            format!("App checks this event at relay {}.", event.endpoint),
            format!("App stores this event's token at {}.", event.token_path),
        ]);
        if let Some(checked) = event.last_checked_at {
            lines.push(format!(
                "[{}] App last saved event {} as {}.",
                stored_time(checked),
                event.event_id,
                event.state.display().label
            ));
        } else {
            lines.push(format!(
                "App has no saved answer about whether relay {} still has event {}.",
                event.endpoint, event.event_id
            ));
        }
        lines.push(format!("Feed tag for event {}:", event.event_id));
        lines.push(feed_tag_for_event(&event.event_id));
    } else {
        lines.push("App has no available event selected.".to_owned());
    }
    lines.push(format!("Publisher configuration: {}", input.context));
    lines.push(format!("Publisher target: {}", input.attach_target_name));
    lines.push(target_description(input));
    if let EventRegistryStatus::Failed(detail) = &input.registry.status {
        lines.push("App could not read the saved event list.".to_owned());
        lines.push(format!("Technical detail: {detail}"));
    }
    if input.feedback.report.entries.is_empty() {
        lines.push("\nApp has no recorded event actions in this session.".to_owned());
    } else {
        lines.push("\nLatest result for each action in this app session".to_owned());
        for entry in &input.feedback.report.entries {
            let stamp = time(entry.at);
            for sentence in entry.sentences() {
                lines.push(format!("[{stamp}] {sentence}"));
            }
            if let EventCommandState::Failed { detail } = &entry.result {
                lines.push(format!("[{stamp}] Technical detail: {detail}"));
            }
        }
    }
    lines.join("\n")
}

fn target_description(input: &EventSectionInput) -> String {
    let name = input.attach_target_name.trim();
    if name.is_empty() {
        return "App has no publisher target name configured.".to_owned();
    }
    match &input.targets {
        EventTargetListInput::Loaded { targets } => {
            let mut matching = targets.iter().filter(|target| target.name == name);
            match (matching.next(), matching.next()) {
                (Some(target), None) => format!("Publisher target {name} uses event {}.", target.event_id),
                (None, _) => format!("Publisher has no target named {name}."),
                (Some(_), Some(_)) => format!("Publisher listed multiple targets named {name}. App cannot confirm which event that target uses."),
            }
        }
        EventTargetListInput::Unknown => format!("App has not read publisher target {name} yet."),
        EventTargetListInput::CommandsUnavailable => {
            format!("Publisher does not provide the commands needed to read target {name}.")
        }
        EventTargetListInput::NotReachable => {
            format!("App could not reach the publisher to read target {name}.")
        }
        EventTargetListInput::Failed { .. } => format!(
            "App could not read publisher target {name}. App cannot confirm which event it uses."
        ),
    }
}

impl EventReportEntry {
    fn sentences(&self) -> Vec<String> {
        let event_id = self
            .event
            .as_ref()
            .map_or("the selected event", |event| event.event_id.as_str());
        let relay = self
            .event
            .as_ref()
            .map_or("the configured relay", |event| event.endpoint.as_str());
        match (&self.operation, &self.result) {
            (_, EventCommandState::Idle) => vec![],
            (EventReportOperation::Registration, EventCommandState::Working) =>
                vec!["App started creating a new event. App is waiting for the result.".to_owned()],
            (EventReportOperation::Registration, EventCommandState::Succeeded) => {
                let mut lines = vec![format!("App created event {event_id} at relay {relay}.")];
                if let Some(event) = &self.event {
                    lines.push(format!("App saved the token for event {event_id} at {}.", event.token_path));
                }
                lines
            }
            (EventReportOperation::Registration, EventCommandState::Failed { .. }) =>
                vec!["App could not finish creating and saving a new event. Inspect the technical detail before retrying.".to_owned()],
            (EventReportOperation::Selection, EventCommandState::Working) =>
                vec![format!("App is saving event {event_id} as your selected event.")],
            (EventReportOperation::Selection, EventCommandState::Succeeded) =>
                vec![format!("App saved event {event_id} as your selected event.")],
            (EventReportOperation::Selection, EventCommandState::Failed { .. }) =>
                vec![format!("App could not save event {event_id} as your selected event.")],
            (EventReportOperation::Check, EventCommandState::Working) =>
                vec![format!("App started checking whether relay {relay} still has event {event_id}. App is waiting for the result.")],
            (EventReportOperation::Check, EventCommandState::Succeeded) => self.checked_event(event_id, relay),
            (EventReportOperation::Check, EventCommandState::Failed { .. }) => self.failed_check(event_id, relay),
            (EventReportOperation::TargetRead, EventCommandState::Working) =>
                vec![format!("App is reading the target list from publisher {}.", self.publisher)],
            (EventReportOperation::TargetRead, EventCommandState::Succeeded) =>
                vec![format!("App read the target list from publisher {}.", self.publisher), self.target_detail.clone()],
            (EventReportOperation::TargetRead, EventCommandState::Failed { .. }) =>
                vec![format!("App could not read target {} from publisher {}. App cannot confirm which event that target uses.", self.target, self.publisher)],
            (EventReportOperation::TargetMutation, result) => self.target_action(event_id, result),
        }
    }

    fn checked_event(&self, event_id: &str, relay: &str) -> Vec<String> {
        let Some(event) = &self.event else {
            return vec![];
        };
        let answer = match event.state {
            EventState::Live => format!("Relay {relay} returned metadata for event {event_id}."),
            EventState::Dead => {
                format!("Relay {relay} reported event {event_id} missing (HTTP 404).")
            }
            EventState::Unknown | EventState::None => {
                format!("App has no confirmed relay answer for event {event_id}.")
            }
        };
        vec![
            answer,
            format!(
                "App saved event {event_id} as {}.",
                event.state.display().label
            ),
        ]
    }

    fn failed_check(&self, event_id: &str, relay: &str) -> Vec<String> {
        let answer = match self.response {
            EventCheckResponse::NoResponse => format!("App received no HTTP response from relay {relay} for event {event_id}."),
            EventCheckResponse::Http(status) if (200..300).contains(&status) =>
                format!("Relay {relay} answered HTTP {status} for event {event_id}, but the app could not read its metadata."),
            EventCheckResponse::Http(status) => format!("Relay {relay} answered HTTP {status} for event {event_id}. App could not confirm whether the relay still has this event."),
            EventCheckResponse::SaveFailed(EventState::Live) => format!("Relay {relay} returned metadata for event {event_id}. App could not finish saving that answer."),
            EventCheckResponse::SaveFailed(EventState::Dead) => format!("Relay {relay} reported event {event_id} missing (HTTP 404). App could not finish saving that answer."),
            EventCheckResponse::SaveFailed(EventState::None | EventState::Unknown) => format!("App could not save the relay answer for event {event_id}."),
            EventCheckResponse::Unclassified => format!("App could not complete the check for event {event_id} at relay {relay}."),
        };
        let mut lines = vec![answer];
        if matches!(
            self.response,
            EventCheckResponse::Http(_) | EventCheckResponse::NoResponse
        ) {
            if let Some(event) = &self.event {
                lines.push(format!(
                    "App kept the saved status of event {event_id} as {}.",
                    event.state.display().label
                ));
            }
        }
        lines
    }

    fn target_action(&self, event_id: &str, result: &EventCommandState) -> Vec<String> {
        let action = if self.intent == Some(EventControlIntent::Detach) {
            "remove"
        } else {
            "attach"
        };
        let direction = if self.intent == Some(EventControlIntent::Detach) {
            "from"
        } else {
            "to"
        };
        let request = format!(
            "{action} event {event_id} {direction} target {} on publisher {}",
            self.target, self.publisher
        );
        let sentence = match result {
            EventCommandState::Working => format!("App asked to {request} and restart Publisher. App is waiting for the result."),
            EventCommandState::Succeeded => format!("Publisher accepted the request to {request} and restart. Its current service badge reports whether it is running."),
            EventCommandState::Failed { .. } => format!("App could not finish the request to {request} and restart Publisher. The target may already have changed."),
            EventCommandState::Idle => return vec![],
        };
        vec![sentence]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::show::{EventCommandFeedback, EventRegistryInput, EventTargetInput};

    fn example() -> EventSectionInput {
        EventSectionInput {
            liveness_confirmed: true,
            registry: EventRegistryInput::default(),
            context: "Fixture publisher".to_owned(),
            request: 0,
            feedback: EventCommandFeedback::default(),
            selected_event: Some(EventSelectionInput {
                created_at: 1,
                last_checked_at: Some(2),
                label: None,
                event_id: "event-one".to_owned(),
                endpoint: "https://relay.example".to_owned(),
                token_path: "/tmp/event-one.token".to_owned(),
                state: EventState::Live,
                token_file_missing: false,
            }),
            targets: EventTargetListInput::Loaded {
                targets: vec![EventTargetInput {
                    name: "default".to_owned(),
                    event_id: "event-one".to_owned(),
                }],
            },
            attach_target_name: "default".to_owned(),
            remote_host: false,
        }
    }

    fn at(second: u32) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(&format!("2026-09-10T14:23:{second:02}Z"))
            .unwrap()
            .with_timezone(&Utc)
    }

    /// Situational ADR 0063: recorded times and subjects survive later projection and copying.
    #[test]
    fn event_report_times_subjects_and_idle_rows_are_honest() {
        let mut input = example();
        let empty = render(&input);
        assert!(!empty.contains("Not requested") && !empty.contains("Liveness"));
        input.feedback.registration = EventCommandState::Succeeded;
        input.record_event_report(EventReportOperation::Registration, at(1));
        input.feedback.selection = EventCommandState::Succeeded;
        input.record_event_report(EventReportOperation::Selection, at(2));
        input.feedback.check = EventCommandState::Working;
        input.record_event_report(EventReportOperation::Check, at(3));
        input.feedback.check = EventCommandState::Failed {
            detail: "fixture HTTP body".to_owned(),
        };
        input.feedback.check_response = EventCheckResponse::Http(503);
        input.record_event_report(EventReportOperation::Check, at(5));
        let report = render(&input);
        assert!(report.contains("[2026-09-10 14:23:01 UTC] App created event event-one at relay"));
        assert!(report.contains(
            "[2026-09-10 14:23:02 UTC] App saved event event-one as your selected event."
        ));
        assert!(report.contains("[2026-09-10 14:23:05 UTC] Relay"));
        assert!(!report.contains("14:23:03"));
        assert_eq!(input.feedback.report.entries.len(), 3);
        assert_eq!(render(&input), report);
        assert!(report.contains("<podcast:liveValue uri=\"event-one\" protocol=\"socket.io\"/>"));
        input.selected_event.as_mut().unwrap().event_id = "event-two".to_owned();
        let later = render(&input);
        assert!(later.contains("App selected event event-two."));
        assert!(later.contains("14:23:01 UTC] App created event event-one"));
        input.feedback = EventCommandFeedback::default();
        let restarted = render(&input);
        assert!(!restarted.contains("App created") && !restarted.contains("Not requested"));
        assert!(restarted.contains("no recorded event actions"));
    }

    /// Situational ADR 0059: HTTP replies, missing replies, invalid data, and save failures differ.
    #[test]
    fn event_report_check_failures_use_typed_response_facts() {
        for (response, expected, unchanged) in [
            (EventCheckResponse::Http(503), "answered HTTP 503", true),
            (
                EventCheckResponse::NoResponse,
                "received no HTTP response",
                true,
            ),
            (
                EventCheckResponse::Http(200),
                "could not read its metadata",
                true,
            ),
            (
                EventCheckResponse::SaveFailed(EventState::Live),
                "returned metadata",
                false,
            ),
            (
                EventCheckResponse::SaveFailed(EventState::Dead),
                "missing (HTTP 404)",
                false,
            ),
            (
                EventCheckResponse::Unclassified,
                "could not complete the check",
                false,
            ),
        ] {
            let mut input = example();
            input.feedback.check = EventCommandState::Failed {
                detail: "misleading HTTP 999 technical detail".to_owned(),
            };
            input.feedback.check_response = response;
            input.record_event_report(EventReportOperation::Check, at(7));
            let report = render(&input);
            assert!(report.contains(expected), "{report}");
            assert_eq!(report.contains("App kept the saved status"), unchanged);
            assert!(!report.contains("answered HTTP 999"));
            assert!(report.contains("Technical detail: misleading HTTP 999"));
            assert!(report.find(expected).unwrap() < report.find("Technical detail:").unwrap());
        }
    }

    /// Situational ADR 0059: a missing event is a successful answer, not an unreachable relay.
    #[test]
    fn event_report_success_names_the_answer_and_saved_state() {
        for (state, answer, saved) in [
            (
                EventState::Live,
                "returned metadata for event event-one",
                "as Live",
            ),
            (
                EventState::Dead,
                "reported event event-one missing (HTTP 404)",
                "as Dead",
            ),
        ] {
            let mut input = example();
            input.selected_event.as_mut().unwrap().state = state;
            input.feedback.check = EventCommandState::Succeeded;
            input.record_event_report(EventReportOperation::Check, at(8));
            let report = render(&input);
            assert!(report.contains(answer) && report.contains(saved));
            assert!(!report.contains("could not") && !report.contains("HTTP 503"));
        }
    }

    /// Situational ADR 0059: reports name the configured target and explain partial command failures.
    #[test]
    fn event_report_target_results_name_configuration_and_consequences() {
        let mut input = example();
        input.targets = EventTargetListInput::Loaded {
            targets: vec![EventTargetInput {
                name: "unused".to_owned(),
                event_id: "event-one".to_owned(),
            }],
        };
        input.feedback.target_read = EventCommandState::Succeeded;
        input.record_event_report(EventReportOperation::TargetRead, at(9));
        assert!(render(&input).contains("Publisher has no target named default."));
        input.feedback.active_action = Some(EventControlIntent::Attach);
        input.feedback.target_mutation = EventCommandState::Failed {
            detail: "restart failed".to_owned(),
        };
        input.record_event_report(EventReportOperation::TargetMutation, at(10));
        let report = render(&input);
        assert!(report.contains("attach event event-one to target default"));
        assert!(report.contains("target may already have changed"));
        input.feedback.active_action = Some(EventControlIntent::Detach);
        input.feedback.target_mutation = EventCommandState::Succeeded;
        input.record_event_report(EventReportOperation::TargetMutation, at(11));
        assert!(render(&input).contains("remove event event-one from target default"));
    }
}
