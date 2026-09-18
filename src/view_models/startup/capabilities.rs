//! Background-tool reports and retries (ADR 0066), with a full tool page (ADR 0074).

use std::fmt::Write as _;
use std::time::SystemTime;

use super::StartupAvailability;
pub(crate) use crate::application::capability::configuration_dependency;
pub(crate) use crate::application::capability::CapabilityAction;
use crate::application::capability::{
    CapabilityFailure, CapabilityObservation, CapabilitySnapshot, Dependency,
};
use crate::application::capability_recovery::{RecoveryAction, RecoveryIntent, RecoveryIntents};

#[derive(Clone, Debug)]
pub struct CapabilityActionDisplay {
    pub action: CapabilityAction,
    pub label: &'static str,
    pub a11y_label: String,
    pub availability: StartupAvailability,
}

pub struct CapabilityRowDisplay {
    pub label: String,
    pub help: Option<String>,
    pub actions: Vec<CapabilityActionDisplay>,
}

#[derive(Clone, Debug)]
pub struct CapabilityReportVm {
    pub(crate) pending: RecoveryIntents,
    pub(crate) queued: std::collections::VecDeque<Dependency>,
    pub(crate) check_message: Option<String>,
    pub(crate) repair_blocked: Vec<Dependency>,
    pub observations: CapabilitySnapshot,
    pub worker_available: bool,
    generation: u64,
    running: Option<(Dependency, u64)>,
    last_check: Option<(Dependency, u64, SystemTime)>,
}

impl CapabilityReportVm {
    /// Display-ready fixture for shared-report geometry tests (ADR 0066).
    #[cfg(test)]
    pub(crate) fn setup_failures_fixture() -> Self {
        let observations = [
            Dependency::MusicIndex,
            Dependency::Playback,
            Dependency::Converter,
        ]
        .map(|dependency| {
            (
                dependency,
                CapabilityObservation::new(dependency, Some(CapabilityFailure::Preparation)),
            )
        })
        .into_iter()
        .collect();
        Self::new(observations, true)
    }

    pub(crate) fn running_dependency(&self) -> Option<Dependency> {
        self.running.map(|(dependency, _)| dependency)
    }

    pub(crate) fn is_working(&self) -> bool {
        self.running.is_some()
    }

    pub const TITLE: &'static str = "Background tools";

    const TOOLS: [Dependency; 9] = [
        Dependency::BackgroundRuntime,
        Dependency::ThumbnailMaintenance,
        Dependency::MusicIndex,
        Dependency::Playback,
        Dependency::Publisher,
        Dependency::Producer,
        Dependency::Encoder,
        Dependency::Converter,
        Dependency::Presentation,
    ];

    #[must_use]
    pub fn notice_summary(&self) -> String {
        let issues = self.issues().count();
        let actions = self.pending.entries().len();
        let mut parts = Vec::new();
        if issues > 0 {
            parts.push(format!(
                "{issues} setup {}",
                if issues == 1 { "issue" } else { "issues" }
            ));
        }
        if actions > 0 {
            parts.push(format!(
                "{actions} retained {}",
                if actions == 1 { "action" } else { "actions" }
            ));
        }
        parts.join("; ")
    }

    #[must_use]
    pub fn new(observations: CapabilitySnapshot, worker_available: bool) -> Self {
        Self {
            pending: RecoveryIntents::default(),
            queued: std::collections::VecDeque::new(),
            check_message: None,
            repair_blocked: Vec::new(),
            observations,
            worker_available,
            generation: 0,
            running: None,
            last_check: None,
        }
    }

    pub fn issues(&self) -> impl Iterator<Item = &CapabilityObservation> {
        self.observations
            .values()
            .filter(|observation| observation.failure.is_some())
    }

    #[must_use]
    pub fn rows(&self, expanded: bool) -> Vec<CapabilityRowDisplay> {
        let mut rows: Vec<_> = self
            .issues()
            .map(|issue| {
                if expanded {
                    return self.tool_row(issue.dependency);
                }
                let dependency = canonical_dependency(issue.dependency);
                let mut intents = vec![CapabilityAction::Configure(issue.dependency)];
                if dependency != Dependency::LibraryPaths {
                    intents.push(CapabilityAction::CheckAgain(dependency));
                }
                CapabilityRowDisplay {
                    label: observation_text(issue),
                    help: Some(edit_help(issue.dependency).into()),
                    actions: intents
                        .into_iter()
                        .map(|intent| self.action(intent))
                        .collect(),
                }
            })
            .collect();
        rows.extend(self.pending.entries().iter().map(|entry| {
            CapabilityRowDisplay {
                label: recovery_summary(entry),
                help: if entry.completed() {
                    None
                } else if expanded {
                    Some(format!("{} {} {}", edit_help(entry.dependency), check_help(entry.dependency), run_help(&entry.action)))
                } else if entry.checked.is_some() {
                    Some("This button opens the original action's controls in Settings. It does not run the action.".into())
                } else {
                    Some(format!("{} It does not run the original action.", edit_help(entry.dependency)))
                },
                actions: if entry.completed() {
                    vec![CapabilityAction::Dismiss(entry.id)]
                } else if expanded {
                    vec![
                        CapabilityAction::Repair(entry.id),
                        CapabilityAction::Verify(entry.id),
                        CapabilityAction::Retry(entry.id),
                        CapabilityAction::Dismiss(entry.id),
                    ]
                } else if entry.checked.is_some() {
                    vec![CapabilityAction::Review(entry.id)]
                } else {
                    vec![CapabilityAction::Repair(entry.id)]
                }
                .into_iter()
                .map(|action| self.action(action))
                .collect(),
            }
        }));
        if expanded {
            rows.extend(
                Self::TOOLS
                    .into_iter()
                    .filter(|dependency| {
                        !self
                            .issues()
                            .any(|issue| canonical_dependency(issue.dependency) == *dependency)
                    })
                    .map(|dependency| self.tool_row(dependency)),
            );
        }
        rows
    }

    fn tool_row(&self, source: Dependency) -> CapabilityRowDisplay {
        let dependency = canonical_dependency(source);
        let mut actions = Vec::new();
        let mut help = String::new();
        if correction_field(source).is_some() || dependency == Dependency::LibraryPaths {
            actions.push(self.action(CapabilityAction::Configure(source)));
            help.push_str(edit_help(source));
            help.push(' ');
        }
        if dependency != Dependency::LibraryPaths {
            actions.push(self.action(CapabilityAction::CheckAgain(dependency)));
        }
        help.push_str(check_help(dependency));
        CapabilityRowDisplay {
            label: dependency_title(source).into(),
            help: Some(help),
            actions,
        }
    }

    #[must_use]
    pub fn action(&self, action: CapabilityAction) -> CapabilityActionDisplay {
        let (label, a11y_label, availability) = match action {
            CapabilityAction::OpenReport => (
                "View tools in Settings",
                "Open Background tools in Settings without checking or retrying an action".into(),
                StartupAvailability::Available,
            ),
            CapabilityAction::Configure(dependency) => (
                edit_label(dependency),
                edit_help(dependency).into(),
                StartupAvailability::Available,
            ),
            CapabilityAction::Repair(id)
            | CapabilityAction::Review(id)
            | CapabilityAction::Verify(id)
            | CapabilityAction::Retry(id)
            | CapabilityAction::Dismiss(id) => {
                let entry = self.pending.entries().iter().find(|entry| entry.id == id);
                let label = match action {
                    CapabilityAction::Repair(_) => {
                        entry.map_or("Open Settings", |entry| edit_label(entry.dependency))
                    }
                    CapabilityAction::Review(_) => entry
                        .map_or("View action in Settings", |entry| {
                            review_label(&entry.action)
                        }),
                    CapabilityAction::Verify(_) => {
                        entry.map_or("Check setup", |entry| check_label(entry.dependency))
                    }
                    CapabilityAction::Retry(_) => {
                        entry.map_or("Run original action", |entry| run_label(&entry.action))
                    }
                    _ => "Dismiss",
                };
                let allowed = entry.is_some_and(|entry| {
                    !entry.running
                        && (!entry.completed() || matches!(action, CapabilityAction::Dismiss(_)))
                        && match action {
                            CapabilityAction::Verify(_) => {
                                self.worker_available
                                    && self.running.is_none()
                                    && !self.repair_blocked.contains(&entry.dependency)
                            }
                            CapabilityAction::Retry(_) => {
                                entry.checked.is_some() && self.running.is_none()
                            }
                            _ => true,
                        }
                });
                (
                    label,
                    format!(
                        "{label}: {}",
                        entry.map_or_else(|| "Unavailable original action".into(), recovery_title)
                    ),
                    if allowed {
                        StartupAvailability::Available
                    } else {
                        StartupAvailability::Unavailable
                    },
                )
            }
            CapabilityAction::CopyReport => (
                "Copy report",
                "Copy the complete background tools report".into(),
                StartupAvailability::Available,
            ),
            CapabilityAction::CheckAgain(dependency) => {
                let availability = if self.running.is_some_and(|(active, _)| active == dependency) {
                    StartupAvailability::Working
                } else if !self.repair_blocked.contains(&dependency)
                    && dependency != Dependency::LibraryPaths
                    && self.running.is_none()
                    && self.worker_available
                {
                    StartupAvailability::Available
                } else {
                    StartupAvailability::Unavailable
                };
                (
                    if availability == StartupAvailability::Working {
                        "Checking…"
                    } else {
                        check_label(dependency)
                    },
                    check_help(dependency).into(),
                    availability,
                )
            }
        };
        CapabilityActionDisplay {
            action,
            label,
            a11y_label,
            availability,
        }
    }

    pub fn begin(&mut self, dependency: Dependency) -> Option<u64> {
        if self
            .action(CapabilityAction::CheckAgain(dependency))
            .availability
            != StartupAvailability::Available
        {
            return None;
        }
        self.pending.checking(dependency);
        self.generation += 1;
        self.running = Some((dependency, self.generation));
        Some(self.generation)
    }

    /// Call before installing a result; a stale completion never installs resources.
    pub fn complete(&mut self, dependency: Dependency, generation: u64) -> bool {
        if self.running != Some((dependency, generation)) {
            return false;
        }
        self.running = None;
        true
    }

    pub fn record_completion(&mut self, dependency: Dependency, generation: u64, at: SystemTime) {
        self.last_check = Some((dependency, generation, at));
    }

    #[must_use]
    pub fn feedback(&self) -> Option<String> {
        let (dependency, generation, at) = self.last_check?;
        let at: chrono::DateTime<chrono::Utc> = at.into();
        Some(format!(
            "App finished check {generation} for {} at {}.",
            dependency_title(dependency),
            at.format("%Y-%m-%d %H:%M:%S UTC")
        ))
    }

    #[must_use]
    pub fn report(&self) -> String {
        let mut text = format!("{}\n", Self::TITLE);
        if let Some(feedback) = self.feedback() {
            let _ = writeln!(text, "{feedback}");
        }
        if let Some(message) = &self.check_message {
            let _ = writeln!(text, "{message}");
        }
        for entry in self.pending.entries() {
            let _ = writeln!(text, "\n{}", recovery_summary(entry));
        }
        for observation in self.observations.values() {
            let at: chrono::DateTime<chrono::Utc> = observation.observed_at.into();
            let _ = writeln!(
                text,
                "\n[{}] {}",
                at.format("%Y-%m-%d %H:%M:%S UTC"),
                observation_text(observation)
            );
            if observation.failure.is_some() {
                let _ = writeln!(
                    text,
                    "{}\nNext: {}",
                    consequence(observation.dependency),
                    recovery(observation)
                );
            }
            if let Some(path) = &observation.resource {
                let _ = writeln!(text, "Location: {}", path.display());
            }
        }
        if !self.worker_available {
            text.push_str("\nApp cannot run checks because its independent maintenance worker is unavailable. Copy this report, free system resources and relaunch the app.\n");
        }
        text
    }
}

#[must_use]
pub fn dependency_title(dependency: Dependency) -> &'static str {
    match dependency {
        Dependency::BackgroundRuntime => "Background runtime",
        Dependency::ThumbnailMaintenance => "Thumbnail maintenance",
        Dependency::MusicIndex => "MusicIndex",
        Dependency::Playback => "Built-in playback",
        Dependency::Publisher => "Publisher host",
        Dependency::Producer => "Drop-file publication",
        Dependency::Encoder => "Stream encoder",
        Dependency::Converter => "Audio converter",
        Dependency::Presentation => "Presentation settings",
        Dependency::LibraryPaths => "Local file bindings",
        Dependency::Configuration(field) => field,
    }
}

#[must_use]
pub fn consequence(dependency: Dependency) -> &'static str {
    match dependency {
        Dependency::BackgroundRuntime => "App cannot run background library loads, online searches, playback commands or broadcast checks. Navigation, local search, existing playlist browsing and these repair tools remain available.",
        Dependency::ThumbnailMaintenance => "App keeps its image cache usable, but cannot finish removing old thumbnail files.",
        Dependency::MusicIndex => "App cannot send MusicIndex requests. Local library work and known RSS URLs remain available.",
        Dependency::Playback => "App cannot use built-in playback. Library work and independent external broadcast controls remain available.",
        Dependency::Publisher => "App cannot use this publisher host. Independent audio playback, drop-file publication and encoder controls remain available.",
        Dependency::Producer => "App cannot publish built-in playback metadata to this drop file. Audio playback and external publisher controls remain available.",
        Dependency::Encoder => "App cannot use this stream encoder. Publisher controls and library work remain available.",
        Dependency::Converter => "App cannot use this converter setting. Other audio formats remain usable; WAV results report whether conversion succeeded or the WAV was retained.",
        Dependency::Presentation => "App uses its documented presentation defaults. App leaves the configuration unchanged and pauses ordinary configuration saves.",
        Dependency::LibraryPaths => "App retained completed path changes and remaining bindings. Only validated relative paths can drive file operations; unvalidated bindings remain stored.",
        Dependency::Configuration("broadcast") => "App cannot use the malformed broadcast table for publisher controls, drop-file publication or encoder controls. Music and built-in playback remain available.",
        Dependency::Configuration("playback.mpv_path") => "App cannot use this player path setting. Explicit or default Null playback does not depend on that path. Ordinary configuration saves remain paused.",
        Dependency::Configuration(field) => consequence(configuration_dependency(field)),
    }
}

#[must_use]
pub fn observation_text(observation: &CapabilityObservation) -> String {
    let (subject, kind) = match observation.failure {
        Some(CapabilityFailure::RuntimeStart(kind)) => ("App could not start its background runtime", Some(kind)),
        Some(CapabilityFailure::CacheWorker(kind)) => ("App could not start its thumbnail cleanup worker", Some(kind)),
        Some(CapabilityFailure::CachePrune(kind)) => ("App could not finish scanning or removing old thumbnail files", Some(kind)),
        Some(CapabilityFailure::MaintenanceUnavailable) => ("App could not run this check on its independent maintenance worker", None),
        Some(CapabilityFailure::Configuration(issue)) => return format!("{issue}. App did not change this setting. Ordinary configuration saves remain paused."),
        Some(CapabilityFailure::Preparation) => return format!("App could not prepare {} from its configured resource.", dependency_title(observation.dependency)),
        Some(CapabilityFailure::PathRepair) => ("App could not finish repairing local file paths. App rechecked music storage and SQLite and kept the library open", None),
        None => return match observation.dependency {
            Dependency::BackgroundRuntime => "App started its background runtime. This does not confirm that any external service is reachable.".into(),
            Dependency::ThumbnailMaintenance => "App completed its thumbnail cleanup scan.".into(),
            Dependency::Publisher => "App read the configured publisher service state and refreshed its setup. App sent no service command.".into(),
            Dependency::Encoder => "App read the configured encoder status and refreshed its setup. App sent no connect or disconnect command.".into(),
            _ => format!("App verified {} configuration and local setup. No external service response was checked.", dependency_title(observation.dependency)),
        },
    };
    match kind {
        Some(kind) => format!("{subject}. Operating system error: {kind}."),
        None => format!("{subject}."),
    }
}

fn recovery(observation: &CapabilityObservation) -> &'static str {
    match observation.failure {
        Some(CapabilityFailure::CachePrune(_)) => "Check thumbnail-cache permissions and available storage, then choose Check again.",
        Some(CapabilityFailure::Configuration(_)) => "Use Edit to open the named setting in Settings. Save the correction. Use Check to test the saved setup.",
        Some(CapabilityFailure::Preparation) => "Check the configured path, permissions and tool settings. Use Edit to open Settings. Use Check to test the saved setup. Other tools stay available.",
        Some(CapabilityFailure::PathRepair) => "Inspect the music folder and preserved bindings before explicitly running local path repair. App has not rolled back completed statements.",
        _ => "Free system resources if necessary, then choose Check again. App will make a fresh attempt.",
    }
}

pub(crate) fn canonical_dependency(dependency: Dependency) -> Dependency {
    match dependency {
        Dependency::Configuration(field) => configuration_dependency(field),
        dependency => dependency,
    }
}

pub(crate) fn correction_field(
    dependency: Dependency,
) -> Option<crate::config::correction::CorrectionField> {
    use crate::config::correction::CorrectionField;
    Some(CorrectionField(match dependency {
        Dependency::Configuration(field) => field,
        Dependency::MusicIndex => "musicindex_endpoint",
        Dependency::Playback => "playback.driver",
        Dependency::Publisher => "broadcast.hosts",
        Dependency::Producer => "broadcast.drop_directory",
        Dependency::Encoder => "broadcast.encoder",
        Dependency::Converter => "flac_path",
        Dependency::Presentation => "ui_scale",
        _ => return None,
    }))
}

pub(crate) fn recovery_title(entry: &RecoveryIntent) -> String {
    match &entry.action {
        RecoveryAction::Conversion {
            title, track_id, ..
        } => format!("Conversion: {title} (track {track_id})"),
        RecoveryAction::IndexSearch { query } => format!(
            "Index search: {}",
            crate::diagnostics::redact_endpoint_details(query)
        ),
        RecoveryAction::Playback {
            operation,
            track_id,
            ..
        } => format!(
            "Playback {operation:?}, original track {}",
            track_id.map_or_else(|| "unavailable".into(), |id| id.to_string())
        ),
        RecoveryAction::Publisher {
            role,
            operation,
            host,
            event_id,
        } => format!(
            "{operation:?} {role:?} on {}, original event {}",
            recovery_host(host.as_ref()),
            event_id.as_deref().unwrap_or("none")
        ),
        RecoveryAction::Event {
            operation,
            event_id,
            host,
            target,
            ..
        } => format!(
            "{operation:?} original event {}, publisher {}, target {}",
            event_id.as_deref().unwrap_or("new event"),
            recovery_host(host.as_ref()),
            target.as_deref().map_or_else(
                || "not applicable".into(),
                crate::diagnostics::redact_endpoint_details
            )
        ),
        RecoveryAction::Encoder { operation, target } => format!(
            "{operation:?} stream encoder, original server {}",
            target
                .as_ref()
                .and_then(|target| target.default_server_name.as_deref())
                .map_or_else(
                    || "encoder's selected server".into(),
                    crate::diagnostics::redact_endpoint_details
                )
        ),
    }
}

fn recovery_host(host: Option<&crate::config::BroadcastHostConfig>) -> String {
    host.map_or_else(
        || "unavailable original host".into(),
        |host| {
            crate::diagnostics::redact_endpoint_details(&format!(
                "{} ({})",
                host.name, host.instance_name
            ))
        },
    )
}

fn recovery_summary(entry: &RecoveryIntent) -> String {
    let at: chrono::DateTime<chrono::Utc> = entry.recorded_at.into();
    let state;
    let result = if let Some(result) = &entry.result {
        result.message()
    } else if entry.running {
        "App is running the original action."
    } else if entry.checked.is_some() {
        state = format!(
            "Setup check passed. {} is available in Settings.",
            run_label(&entry.action)
        );
        &state
    } else {
        "App retained the original action. Its setup needs a successful check before it can run."
    };
    format!(
        "[{}] {}. {result}",
        at.format("%Y-%m-%d %H:%M:%S UTC"),
        recovery_title(entry)
    )
}

pub(crate) fn edit_label(dependency: Dependency) -> &'static str {
    match canonical_dependency(dependency) {
        Dependency::MusicIndex => "Edit endpoint",
        Dependency::Playback => "Edit player settings",
        Dependency::Publisher => "Edit publisher settings",
        Dependency::Producer => "Edit publication settings",
        Dependency::Encoder => "Edit encoder settings",
        Dependency::Converter => "Edit converter setting",
        Dependency::Presentation => "Edit display settings",
        _ => "Open report in Settings",
    }
}

fn edit_help(dependency: Dependency) -> &'static str {
    match canonical_dependency(dependency) {
        Dependency::MusicIndex => "Edit endpoint opens musicindex_endpoint in Settings.",
        Dependency::Playback => "Edit player settings opens the player configuration in Settings.",
        Dependency::Publisher => {
            "Edit publisher settings opens the host configuration in Settings."
        }
        Dependency::Producer => {
            "Edit publication settings opens the drop-file configuration in Settings."
        }
        Dependency::Encoder => "Edit encoder settings opens the encoder configuration in Settings.",
        Dependency::Converter => "Edit converter setting opens flac_path in Settings.",
        Dependency::Presentation => {
            "Edit display settings opens the configuration editor in Settings."
        }
        _ => "Open report in Settings shows the problem and its available checks.",
    }
}

pub(crate) fn check_label(dependency: Dependency) -> &'static str {
    match canonical_dependency(dependency) {
        Dependency::MusicIndex => "Check endpoint",
        Dependency::Playback => "Check player",
        Dependency::Publisher => "Check publisher",
        Dependency::Producer => "Check publication folder",
        Dependency::Encoder => "Check encoder",
        Dependency::Converter => "Check converter setting",
        Dependency::Presentation => "Check display settings",
        _ => "Check again",
    }
}

fn check_help(dependency: Dependency) -> &'static str {
    match canonical_dependency(dependency) {
        Dependency::MusicIndex => {
            "Check endpoint validates the saved URL without sending a search."
        }
        Dependency::Playback => {
            "Check player prepares the saved player setup without playing a track."
        }
        Dependency::Publisher => {
            "Check publisher reads service state without starting or stopping a service."
        }
        Dependency::Producer => {
            "Check publication folder tests directory access without publishing metadata."
        }
        Dependency::Encoder => {
            "Check encoder reads encoder status without connecting or disconnecting the stream."
        }
        Dependency::Converter => {
            "Check converter setting freshly tests the saved executables without converting a file."
        }
        Dependency::Presentation => {
            "Check display settings validates and applies the saved display configuration."
        }
        Dependency::BackgroundRuntime => {
            "Check again starts the app's background runtime without repeating a previous action."
        }
        Dependency::ThumbnailMaintenance => "Check again retries the thumbnail cleanup scan.",
        _ => "The report explains which resource needs a check.",
    }
}

fn review_label(action: &RecoveryAction) -> &'static str {
    match action {
        RecoveryAction::Conversion { .. } => "View conversion actions",
        RecoveryAction::IndexSearch { .. } => "View search actions",
        RecoveryAction::Playback { .. } => "View playback actions",
        RecoveryAction::Publisher { .. } => "View service actions",
        RecoveryAction::Event { .. } => "View event actions",
        RecoveryAction::Encoder { .. } => "View stream actions",
    }
}

fn run_label(action: &RecoveryAction) -> &'static str {
    use crate::application::capability_recovery::{
        PlaybackOperation, PublisherServiceOperation, StreamEncoderOperation,
    };
    use crate::runtime::BroadcastServiceRole;
    use crate::view_models::show::EventControlIntent;
    match action {
        RecoveryAction::Conversion {
            redownload: true, ..
        } => "Redownload original track",
        RecoveryAction::Conversion {
            redownload: false, ..
        } => "Retry original conversion",
        RecoveryAction::IndexSearch { .. } => "Run search again",
        RecoveryAction::Playback { operation, .. } => match operation {
            PlaybackOperation::Playlist { .. } => "Play original track",
            PlaybackOperation::Pause => "Pause original track",
            PlaybackOperation::Resume => "Resume original track",
            PlaybackOperation::Next => "Play next track",
            PlaybackOperation::Previous => "Play previous track",
        },
        RecoveryAction::Publisher {
            operation, role, ..
        } => match (operation, role) {
            (PublisherServiceOperation::Start, BroadcastServiceRole::Publisher) => {
                "Start publisher"
            }
            (PublisherServiceOperation::Stop, BroadcastServiceRole::Publisher) => "Stop publisher",
            (PublisherServiceOperation::Reset, BroadcastServiceRole::Publisher) => {
                "Clear publisher failure"
            }
            (PublisherServiceOperation::Start, BroadcastServiceRole::Producer) => "Start producer",
            (PublisherServiceOperation::Stop, BroadcastServiceRole::Producer) => "Stop producer",
            (PublisherServiceOperation::Reset, BroadcastServiceRole::Producer) => {
                "Clear producer failure"
            }
        },
        RecoveryAction::Event { operation, .. } => match operation {
            EventControlIntent::Create => "Create event",
            EventControlIntent::Replace => "Replace event",
            EventControlIntent::Attach => "Attach event",
            EventControlIntent::Detach => "Detach event",
            EventControlIntent::Check => "Check event",
            EventControlIntent::ReadTargets => "Read event targets",
            EventControlIntent::Refresh => "Refresh event",
            EventControlIntent::CopyFeedTag => "Copy feed tag",
        },
        RecoveryAction::Encoder { operation, .. } => match operation {
            StreamEncoderOperation::Connect => "Connect stream",
            StreamEncoderOperation::Disconnect => "Disconnect stream",
        },
    }
}

fn run_help(action: &RecoveryAction) -> String {
    let effect = match action {
        RecoveryAction::Conversion { redownload: true, .. } => "downloads the same original enclosure again because retained input could not be reused; it preserves the original edits and library entry",
        RecoveryAction::Conversion { redownload: false, .. } => "validates and reuses the original track's input, preserves its edits, and updates its existing library entry",
        RecoveryAction::IndexSearch { .. } => "sends this original query",
        RecoveryAction::Playback { .. } => "runs the original playback command",
        RecoveryAction::Publisher { .. } => "sends the original service command",
        RecoveryAction::Event { .. } => "runs the original event command",
        RecoveryAction::Encoder { .. } => "sends the original stream command",
    };
    format!(
        "{} {effect} after a successful setup check.",
        run_label(action)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::time::{Duration, SystemTime};

    /// Situational ADR 0074: the full page must not reuse the failure-only notice.
    #[test]
    fn adr_0074_background_tools_remain_visible_without_failures() {
        let mut vm = CapabilityReportVm::new(Default::default(), true);
        let before = vm.report();
        let rows = vm.rows(true);
        let labels: Vec<_> = rows.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Background runtime",
                "Thumbnail maintenance",
                "MusicIndex",
                "Built-in playback",
                "Publisher host",
                "Drop-file publication",
                "Stream encoder",
                "Audio converter",
                "Presentation settings",
            ]
        );
        for row in &rows {
            assert!(!row.help.as_ref().unwrap().is_empty());
            assert!(row
                .actions
                .iter()
                .any(|action| matches!(action.action, CapabilityAction::CheckAgain(_))));
            assert!(row.actions.iter().all(|action| action.availability
                == StartupAvailability::Available
                && !action.a11y_label.is_empty()));
        }
        assert!(vm.rows(false).is_empty());
        assert!(vm.observations.is_empty());
        assert_eq!(vm.report(), before);
        assert!(vm.running_dependency().is_none());

        // A recorded success must not remove a tool or imply checks of other tools.
        vm.observations.insert(
            Dependency::BackgroundRuntime,
            CapabilityObservation {
                dependency: Dependency::BackgroundRuntime,
                observed_at: SystemTime::UNIX_EPOCH + Duration::from_secs(10),
                failure: None,
                resource: None,
            },
        );
        let before = vm.observations.clone();
        assert_eq!(
            vm.rows(true)
                .iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            labels
        );
        assert!(vm.rows(false).is_empty());
        assert_eq!(vm.observations, before);
        let report = vm.report();
        assert!(report.contains("1970-01-01 00:00:10 UTC"));
        assert!(report.contains("does not confirm that any external service is reachable"));
        assert!(!report.contains("App verified MusicIndex"));
    }

    /// Situational ADR 0074: failures keep their routes and command availability.
    #[test]
    fn adr_0074_background_tools_keep_failure_routes_and_check_gates() {
        let source = Dependency::Configuration("musicindex_endpoint");
        let mut vm = CapabilityReportVm::new(
            [(
                source,
                CapabilityObservation::new(source, Some(CapabilityFailure::Preparation)),
            )]
            .into_iter()
            .collect(),
            true,
        );
        let rows = vm.rows(true);
        assert_eq!(rows[0].label, "musicindex_endpoint");
        assert_eq!(
            rows[0].actions[0].action,
            CapabilityAction::Configure(source)
        );
        assert_eq!(
            rows.iter()
                .flat_map(|row| &row.actions)
                .filter(
                    |action| action.action == CapabilityAction::CheckAgain(Dependency::MusicIndex)
                )
                .count(),
            1
        );
        assert_eq!(vm.rows(false).len(), 1);

        let generation = vm.begin(Dependency::MusicIndex).unwrap();
        for action in vm.rows(true).iter().flat_map(|row| &row.actions) {
            match action.action {
                CapabilityAction::CheckAgain(Dependency::MusicIndex) => {
                    assert_eq!(action.availability, StartupAvailability::Working);
                }
                CapabilityAction::CheckAgain(_) => {
                    assert_eq!(action.availability, StartupAvailability::Unavailable);
                }
                _ => {}
            }
        }
        assert!(vm.complete(Dependency::MusicIndex, generation));
        vm.observations.get_mut(&source).unwrap().failure = None;
        vm.worker_available = false;
        let rows = vm.rows(true);
        assert!(rows.iter().any(|row| row.label == "MusicIndex"));
        assert!(vm.rows(false).is_empty());
        for action in rows.iter().flat_map(|row| &row.actions) {
            assert_eq!(
                action.availability,
                if matches!(action.action, CapabilityAction::CheckAgain(_)) {
                    StartupAvailability::Unavailable
                } else {
                    StartupAvailability::Available
                }
            );
        }
    }

    #[test]
    fn adr_0066_conversion_controls_preserve_subject_and_explain_explicit_redownload() {
        use crate::application::conversion_recovery::{ConversionReport, ConversionState};
        let mut vm = CapabilityReportVm::new(Default::default(), true);
        let mut report = ConversionReport {
            id: 7,
            track_id: 41,
            title: "Original track".into(),
            state: ConversionState::WavRetained,
            message: "App kept usable WAV input in the library.".into(),
            recorded_at: SystemTime::now(),
        };
        vm.pending.sync_conversions(&[report.clone()], 1);
        let id = vm.pending.entries()[0].id;
        assert!(vm.rows(true)[0].label.contains("Original track (track 41)"));
        assert_eq!(
            vm.action(CapabilityAction::Retry(id)).label,
            "Retry original conversion"
        );
        assert_eq!(
            vm.action(CapabilityAction::Retry(id)).availability,
            StartupAvailability::Unavailable
        );
        let snapshot = crate::config::ConfigSnapshot::from_bytes(
            std::path::Path::new("config.toml"),
            Vec::new(),
        )
        .unwrap();
        vm.pending.checked(Dependency::Converter, &snapshot);
        vm.pending.sync_conversions(&[report.clone()], 1);
        assert_eq!(
            vm.action(CapabilityAction::Retry(id)).availability,
            StartupAvailability::Available
        );
        vm.pending.saved();
        assert_eq!(
            vm.action(CapabilityAction::Retry(id)).availability,
            StartupAvailability::Unavailable
        );
        report.state = ConversionState::RedownloadRequired;
        report.message = "App could not find retained input /music/original.wav.".into();
        report.recorded_at += Duration::from_secs(1);
        vm.pending.sync_conversions(&[report], 1);
        assert_eq!(vm.pending.entries().len(), 1);
        assert_eq!(
            vm.action(CapabilityAction::Retry(id)).label,
            "Redownload original track"
        );
        assert!(vm.rows(true)[0]
            .help
            .as_ref()
            .unwrap()
            .contains("same original enclosure"));
        assert!(vm.report().contains("/music/original.wav"));
    }

    #[test]
    fn adr_0066_search_controls_explain_edit_check_and_execution() {
        let mut vm = CapabilityReportVm::new(Default::default(), true);
        let id = vm.pending.retain(
            RecoveryAction::IndexSearch {
                query: "original query".into(),
            },
            Dependency::MusicIndex,
            1,
        );
        assert_eq!(
            vm.action(CapabilityAction::Repair(id)).label,
            "Edit endpoint"
        );
        assert_eq!(
            vm.action(CapabilityAction::Verify(id)).label,
            "Check endpoint"
        );
        assert_eq!(
            vm.action(CapabilityAction::Retry(id)).label,
            "Run search again"
        );
        assert!(vm.rows(false)[0]
            .help
            .as_ref()
            .unwrap()
            .contains("opens musicindex_endpoint in Settings"));
        let help = vm.rows(true)[0].help.clone().unwrap();
        assert!(help.contains("without sending a search"));
        assert!(help.contains("Run search again sends this original query"));
        let snapshot = crate::config::ConfigSnapshot::from_bytes(
            std::path::Path::new("config.toml"),
            Vec::new(),
        )
        .unwrap();
        vm.pending.checked(Dependency::MusicIndex, &snapshot);
        let ready = vm.rows(false);
        assert_eq!(ready[0].actions[0].action, CapabilityAction::Review(id));
        assert_eq!(ready[0].actions[0].label, "View search actions");
        assert!(ready[0]
            .help
            .as_ref()
            .unwrap()
            .contains("does not run the action"));
        assert_eq!(
            vm.action(CapabilityAction::Retry(id)).availability,
            StartupAvailability::Available
        );
        vm.pending
            .succeed(id, "App completed the original search.".into());
        for expanded in [false, true] {
            let rows = vm.rows(expanded);
            assert_eq!(rows[0].actions.len(), 1);
            assert_eq!(rows[0].actions[0].action, CapabilityAction::Dismiss(id));
            assert!(rows[0].label.contains("completed the original search"));
            assert!(!rows[0].label.contains("needs a successful check"));
        }
        for action in [
            CapabilityAction::Repair(id),
            CapabilityAction::Review(id),
            CapabilityAction::Verify(id),
            CapabilityAction::Retry(id),
        ] {
            assert_eq!(
                vm.action(action).availability,
                StartupAvailability::Unavailable
            );
        }
    }

    #[test]
    fn adr_0066_service_controls_distinguish_observation_from_mutation() {
        let mut vm = CapabilityReportVm::new(Default::default(), true);
        let id = vm.pending.retain(
            RecoveryAction::Publisher {
                role: crate::runtime::BroadcastServiceRole::Publisher,
                operation:
                    crate::application::capability_recovery::PublisherServiceOperation::Start,
                host: Some(crate::config::BroadcastHostConfig::default_local()),
                event_id: None,
            },
            Dependency::Publisher,
            1,
        );
        assert_eq!(
            vm.action(CapabilityAction::Repair(id)).label,
            "Edit publisher settings"
        );
        assert_eq!(
            vm.action(CapabilityAction::Verify(id)).label,
            "Check publisher"
        );
        assert_eq!(
            vm.action(CapabilityAction::Retry(id)).label,
            "Start publisher"
        );
        let rows = vm.rows(true);
        let help = rows[0].help.as_ref().unwrap();
        assert!(help.contains("without starting or stopping a service"));
        assert!(help.contains("Start publisher sends the original service command"));
        vm.pending
            .finish(id, "App could not start the original publisher.".into());
        assert_eq!(vm.rows(true)[0].actions.len(), 4);
        assert_eq!(
            vm.action(CapabilityAction::Repair(id)).availability,
            StartupAvailability::Available
        );
        assert_eq!(
            vm.action(CapabilityAction::Retry(id)).availability,
            StartupAvailability::Unavailable
        );
    }

    #[test]
    fn adr_0066_repair_routes_keep_subjects_actions_and_failed_checks_separate() {
        let mut vm = CapabilityReportVm::new(Default::default(), true);
        let first = vm.pending.retain(
            RecoveryAction::IndexSearch {
                query: "original search".into(),
            },
            Dependency::MusicIndex,
            7,
        );
        let second = vm.pending.retain(
            RecoveryAction::Playback {
                operation: crate::application::capability_recovery::PlaybackOperation::Pause,
                track_id: Some(44),
                queue: vec![44],
            },
            Dependency::Playback,
            7,
        );
        for id in [first, second] {
            assert_eq!(
                vm.action(CapabilityAction::Repair(id)).availability,
                StartupAvailability::Available
            );
            assert_eq!(
                vm.action(CapabilityAction::Verify(id)).availability,
                StartupAvailability::Available
            );
            assert_eq!(
                vm.action(CapabilityAction::Retry(id)).availability,
                StartupAvailability::Unavailable
            );
        }
        let snapshot = crate::config::ConfigSnapshot::from_bytes(
            std::path::Path::new("config.toml"),
            Vec::new(),
        )
        .unwrap();
        vm.pending.checked(Dependency::MusicIndex, &snapshot);
        assert_eq!(
            vm.action(CapabilityAction::Retry(first)).availability,
            StartupAvailability::Available
        );
        let generation = vm.begin(Dependency::MusicIndex).unwrap();
        assert!(vm.complete(Dependency::MusicIndex, generation));
        // A failed check supplies no new verified revision.
        assert_eq!(
            vm.action(CapabilityAction::Retry(first)).availability,
            StartupAvailability::Unavailable
        );
        assert_eq!(
            vm.rows(true)
                .iter()
                .filter(|row| row
                    .actions
                    .iter()
                    .any(|action| matches!(action.action, CapabilityAction::Verify(_))))
                .count(),
            2
        );
        assert!(vm.report().contains("original search"));
        assert!(vm.report().contains("original track 44"));
        for dependency in [
            Dependency::MusicIndex,
            Dependency::Playback,
            Dependency::Publisher,
            Dependency::Producer,
            Dependency::Encoder,
            Dependency::Presentation,
            Dependency::Converter,
        ] {
            assert!(correction_field(dependency).is_some());
            assert_eq!(
                vm.action(CapabilityAction::Configure(dependency))
                    .availability,
                StartupAvailability::Available
            );
        }
        assert!(correction_field(Dependency::BackgroundRuntime).is_none());
    }

    fn vm() -> CapabilityReportVm {
        let entries = [
            (
                Dependency::BackgroundRuntime,
                CapabilityFailure::RuntimeStart(io::ErrorKind::Other),
            ),
            (
                Dependency::ThumbnailMaintenance,
                CapabilityFailure::CacheWorker(io::ErrorKind::PermissionDenied),
            ),
        ]
        .map(|(dependency, failure)| {
            (
                dependency,
                CapabilityObservation {
                    dependency,
                    failure: Some(failure),
                    observed_at: SystemTime::UNIX_EPOCH + Duration::from_secs(10),
                    resource: None,
                },
            )
        });
        CapabilityReportVm::new(entries.into_iter().collect(), true)
    }

    #[test]
    fn adr_0066_notice_summary_counts_issues_and_retained_actions_without_running_them() {
        let mut vm = vm();
        assert_eq!(vm.notice_summary(), "2 setup issues");
        vm.pending.retain(
            RecoveryAction::IndexSearch {
                query: "original query".into(),
            },
            Dependency::MusicIndex,
            1,
        );
        assert_eq!(vm.notice_summary(), "2 setup issues; 1 retained action");
        let display = vm.action(CapabilityAction::OpenReport);
        assert_eq!(display.availability, StartupAvailability::Available);
        assert_eq!(display.label, "View tools in Settings");
        assert!(display.a11y_label.contains("without checking or retrying"));
        assert_eq!(vm.pending.entries().len(), 1);
        assert!(vm.running_dependency().is_none());
    }

    #[test]
    fn adr_0066_independent_issues_keep_typed_remedies_and_recorded_times() {
        let mut vm = vm();
        assert_eq!(vm.issues().count(), 2);
        for row in vm.rows(false) {
            assert!(!row.label.is_empty());
            assert_eq!(row.actions.len(), 2);
            assert!(row.actions.iter().all(|action| action.availability
                == StartupAvailability::Available
                && !action.a11y_label.is_empty()));
        }
        assert!(vm.rows(true).iter().all(|row| row
            .actions
            .iter()
            .any(|action| matches!(action.action, CapabilityAction::CheckAgain(_)))));
        let report = vm.report();
        assert!(report.contains("1970-01-01 00:00:10 UTC"));
        assert!(report.contains("App could not start its background runtime"));
        for issue in vm.issues() {
            assert_eq!(
                vm.action(CapabilityAction::CheckAgain(issue.dependency))
                    .availability,
                StartupAvailability::Available
            );
            assert_eq!(
                vm.action(CapabilityAction::Configure(issue.dependency))
                    .availability,
                StartupAvailability::Available
            );
        }
        vm.observations
            .get_mut(&Dependency::BackgroundRuntime)
            .unwrap()
            .failure = None;
        assert_eq!(vm.issues().count(), 1);
        assert!(vm
            .report()
            .contains("does not confirm that any external service is reachable"));
        vm.worker_available = false;
        assert_eq!(
            vm.action(CapabilityAction::CopyReport).availability,
            StartupAvailability::Available
        );
        assert_eq!(
            vm.action(CapabilityAction::Configure(
                Dependency::ThumbnailMaintenance
            ))
            .availability,
            StartupAvailability::Available
        );
    }

    #[test]
    fn adr_0066_retry_generation_admits_one_install_and_rejects_stale_results() {
        let mut vm = vm();
        let dependency = Dependency::BackgroundRuntime;
        let generation = vm.begin(dependency).unwrap();
        assert!(vm.begin(dependency).is_none());
        assert!(vm.begin(Dependency::ThumbnailMaintenance).is_none());
        assert!(!vm.complete(dependency, generation + 1));
        assert!(!vm.complete(Dependency::ThumbnailMaintenance, generation));
        let mut installs = 0;
        for _ in 0..2 {
            if vm.complete(dependency, generation) {
                installs += 1;
            }
        }
        assert_eq!(installs, 1);
        assert!(vm
            .issues()
            .any(|issue| issue.dependency == Dependency::ThumbnailMaintenance));
    }

    #[test]
    fn adr_0066_repeated_tool_failures_keep_distinct_recorded_feedback() {
        let mut vm = vm();
        let dependency = Dependency::BackgroundRuntime;
        for expected in 1..=2 {
            let generation = vm.begin(dependency).unwrap();
            assert!(vm.complete(dependency, generation));
            vm.record_completion(dependency, generation, SystemTime::UNIX_EPOCH);
            assert!(vm
                .feedback()
                .unwrap()
                .contains(&format!("check {expected}")));
            assert!(vm.report().contains("1970-01-01 00:00:00 UTC"));
        }
    }

    #[test]
    fn adr_0069_report_navigation_preserves_issues_times_and_retry_result() {
        use crate::view_models::settings::{SettingsAction, SettingsGroup, SettingsVm};

        let mut report = vm();
        let generation = report.begin(Dependency::BackgroundRuntime).unwrap();
        assert!(report.complete(Dependency::BackgroundRuntime, generation));
        report.record_completion(
            Dependency::BackgroundRuntime,
            generation,
            SystemTime::UNIX_EPOCH,
        );
        let before = report.report();
        let mut settings = SettingsVm::default();
        for group in SettingsGroup::ALL {
            settings.dispatch(SettingsAction::SelectGroup(group));
            settings.dispatch(SettingsAction::OpenReport);
            assert_eq!(settings.selected(), SettingsGroup::Diagnostics);
            assert_eq!(report.issues().count(), 2);
            assert_eq!(report.report(), before);
            assert_eq!(
                report.action(CapabilityAction::CopyReport).availability,
                StartupAvailability::Available
            );
        }
    }
}
