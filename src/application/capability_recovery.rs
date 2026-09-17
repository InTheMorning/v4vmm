//! Retained action identity and explicit retry admission (ADR 0066, invariant 9).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::config::{BroadcastEncoderConfig, BroadcastHostConfig, ConfigSnapshot};
use crate::runtime::BroadcastServiceRole;
use crate::{db, playback};

use super::capability::{Dependency, ExecutionUnavailable, FeatureAvailability};
use super::session_lifecycle::SessionLifecycle;
use super::{ApplicationCommand, CommandContext, CommandError, CommandResult};

pub(crate) mod setup;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublisherServiceOperation {
    Start,
    Stop,
    Reset,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StreamEncoderOperation {
    Connect,
    Disconnect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlaybackOperation {
    Playlist { playlist_id: i64, position: i64 },
    Pause,
    Resume,
    Next,
    Previous,
}

/// Inputs identify the original operation, independently of the mounted view.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum RecoveryAction {
    IndexSearch {
        query: String,
    },
    Playback {
        operation: PlaybackOperation,
        track_id: Option<i64>,
        queue: Vec<i64>,
    },
    Publisher {
        role: BroadcastServiceRole,
        operation: PublisherServiceOperation,
        host: Option<BroadcastHostConfig>,
        event_id: Option<String>,
    },
    Event {
        operation: crate::view_models::show::EventControlIntent,
        event_id: Option<String>,
        selection_revision: i64,
        host: Option<BroadcastHostConfig>,
        target: Option<String>,
    },
    Encoder {
        operation: StreamEncoderOperation,
        target: Option<BroadcastEncoderConfig>,
    },
}

impl std::fmt::Debug for RecoveryAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Queries and configured hosts may contain private input.
        f.debug_struct("RecoveryAction")
            .field("dependency", &self.dependency())
            .finish_non_exhaustive()
    }
}

impl RecoveryAction {
    pub(crate) fn dependency(&self) -> Dependency {
        match self {
            Self::IndexSearch { .. } => Dependency::MusicIndex,
            Self::Playback { .. } => Dependency::Playback,
            Self::Publisher { .. } => Dependency::Publisher,
            Self::Encoder { .. } => Dependency::Encoder,
            Self::Event { operation, .. } => match operation {
                crate::view_models::show::EventControlIntent::Create
                | crate::view_models::show::EventControlIntent::Replace
                | crate::view_models::show::EventControlIntent::Check => Dependency::MusicIndex,
                _ => Dependency::Publisher,
            },
        }
    }

    pub(crate) fn event_target_name(&self) -> Result<String, RetryRejection> {
        match self {
            Self::Event {
                target: Some(target),
                ..
            } => Ok(target.clone()),
            _ => Err(RetryRejection::PublisherChanged),
        }
    }

    pub(crate) fn validate_event_target(
        &self,
        event_id: &str,
        target_name: &str,
        transport: &crate::broadcast::transport::Transport,
        instance_name: &str,
    ) -> Result<(), RetryRejection> {
        match self {
            Self::Event {
                event_id: Some(original_event),
                target: Some(original_target),
                host: Some(host),
                ..
            } if original_event == event_id
                && original_target == target_name
                && &host.transport == transport
                && host.instance_name == instance_name =>
            {
                Ok(())
            }
            _ => Err(RetryRejection::PublisherChanged),
        }
    }

    pub(crate) fn validate_subject(
        &self,
        conn: &Connection,
        snapshot: &ConfigSnapshot,
    ) -> Result<(), RetryRejection> {
        match self {
            Self::IndexSearch { query } => {
                if query.trim().is_empty() {
                    return Err(RetryRejection::SubjectChanged);
                }
            }
            Self::Playback {
                operation,
                track_id,
                queue,
            } => {
                let track_id = track_id.ok_or(RetryRejection::SubjectChanged)?;
                if db::track_row_by_id(conn, track_id)
                    .map_err(|_| RetryRejection::SubjectChanged)?
                    .is_none()
                {
                    return Err(RetryRejection::SubjectChanged);
                }
                match operation {
                    PlaybackOperation::Playlist {
                        playlist_id,
                        position,
                    } => {
                        let selection =
                            crate::playlist_service::select_track_at(conn, *playlist_id, *position)
                                .map_err(|_| RetryRejection::SubjectChanged)?;
                        if selection.track_id != track_id {
                            return Err(RetryRejection::SubjectChanged);
                        }
                    }
                    _ => {
                        let current = db::playback_session(conn, playback::DEFAULT_SESSION_ID)
                            .map_err(|_| RetryRejection::SubjectChanged)?;
                        let session = current.ok_or(RetryRejection::SubjectChanged)?;
                        if session.local_track_id != track_id || session.state == "stopped" {
                            return Err(RetryRejection::SubjectChanged);
                        }
                        let current_queue = match session.playlist_id {
                            Some(id) => db::playlist_tracks(conn, id)
                                .map_err(|_| RetryRejection::SubjectChanged)?
                                .into_iter()
                                .map(|track| track.id)
                                .collect(),
                            None => vec![session.local_track_id],
                        };
                        if *queue != current_queue {
                            return Err(RetryRejection::SubjectChanged);
                        }
                    }
                }
            }
            Self::Publisher { host, event_id, .. } => {
                let current = snapshot.broadcast();
                if host
                    .as_ref()
                    .is_none_or(|host| current.selected_host().ok() != Some(host))
                {
                    return Err(RetryRejection::PublisherChanged);
                }
                let selection = db::broadcast_event_selection(conn)
                    .map_err(|_| RetryRejection::SubjectChanged)?;
                if &selection.event_id != event_id {
                    return Err(RetryRejection::SubjectChanged);
                }
                if let Some(id) = event_id {
                    if db::broadcast_event_by_event_id(conn, id)
                        .map_err(|_| RetryRejection::SubjectChanged)?
                        .is_none()
                    {
                        return Err(RetryRejection::SubjectChanged);
                    }
                }
            }
            Self::Event {
                operation,
                event_id,
                selection_revision,
                host,
                target,
            } => {
                use crate::view_models::show::EventControlIntent;
                let selection = db::broadcast_event_selection(conn)
                    .map_err(|_| RetryRejection::SubjectChanged)?;
                if &selection.event_id != event_id || selection.revision != *selection_revision {
                    return Err(RetryRejection::SubjectChanged);
                }
                if let Some(id) = event_id {
                    if db::broadcast_event_by_event_id(conn, id)
                        .map_err(|_| RetryRejection::SubjectChanged)?
                        .is_none()
                    {
                        return Err(RetryRejection::SubjectChanged);
                    }
                }
                if matches!(
                    operation,
                    EventControlIntent::Attach | EventControlIntent::Detach
                ) {
                    if host
                        .as_ref()
                        .is_none_or(|host| snapshot.broadcast().selected_host().ok() != Some(host))
                        || target.is_none()
                    {
                        return Err(RetryRejection::PublisherChanged);
                    }
                    if *operation == EventControlIntent::Attach
                        && snapshot.drop_file_target.as_ref().ok() != target.as_ref()
                    {
                        return Err(RetryRejection::PublisherChanged);
                    }
                }
            }
            Self::Encoder { target, .. } => {
                if target.is_none() || snapshot.encoder.as_ref().ok() != Some(target) {
                    return Err(RetryRejection::PublisherChanged);
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RetryRejection {
    CheckRequired,
    Completed,
    SessionChanged,
    ConfigurationChanged,
    SubjectChanged,
    PublisherChanged,
    Unavailable(ExecutionUnavailable),
}

impl std::fmt::Display for RetryRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Completed => "App already completed this action. Start a new action to run it again.",
            Self::CheckRequired => "App did not retry the action. Check the repaired tool again first.",
            Self::SessionChanged => "App did not retry the action because its original app session ended. Start the action again in this session.",
            Self::ConfigurationChanged => "App did not retry the action because its configuration changed after verification. Check the tool again.",
            Self::SubjectChanged => "App did not retry the action because its original track, playlist position or event changed or was removed. Select the intended subject and start a new action.",
            Self::PublisherChanged => "App did not retry the action because its original publisher or encoder target is unavailable or changed. Check the intended target and start a new action.",
            Self::Unavailable(reason) => return write!(f, "{reason}"),
        })
    }
}

/// A scoped revision ignores unrelated workspace autosaves and retains no diagnostic values.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CapabilityRevision {
    core: (
        crate::config::ConfigField<PathBuf>,
        crate::config::ConfigField<PathBuf>,
    ),
    value: String,
}

impl std::fmt::Debug for CapabilityRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CapabilityRevision(..)")
    }
}

impl CapabilityRevision {
    pub(crate) fn of(snapshot: &ConfigSnapshot, dependency: Dependency) -> Self {
        let value = match dependency {
            Dependency::MusicIndex => format!("{:?}", snapshot.musicindex_endpoint),
            Dependency::Playback => format!("{:?}", snapshot.playback()),
            Dependency::Publisher => format!(
                "{:?}",
                (
                    snapshot.broadcast_hosts.clone(),
                    snapshot.selected_host.clone(),
                    snapshot.drop_file_target.clone()
                )
            ),
            Dependency::Producer => format!(
                "{:?}",
                (
                    snapshot.drop_directory.clone(),
                    snapshot.drop_file_target.clone()
                )
            ),
            Dependency::Encoder => format!("{:?}", snapshot.encoder),
            Dependency::Converter => format!("{:?}", snapshot.flac_path),
            Dependency::Presentation => {
                format!(
                    "{:?}",
                    (
                        snapshot.ui_scale,
                        snapshot.theme_profile,
                        &snapshot.workspace_layout,
                        snapshot.content_pane_width,
                        snapshot.content_list_view_mode
                    )
                )
            }
            _ => String::new(),
        };
        Self {
            core: (snapshot.music_dir.clone(), snapshot.db_path.clone()),
            value,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum RecoveryResult {
    Completed(String),
    Failed(String),
}

impl RecoveryResult {
    pub(crate) fn message(&self) -> &str {
        match self {
            Self::Completed(message) | Self::Failed(message) => message,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RecoveryIntent {
    pub(crate) id: u64,
    pub(crate) action: RecoveryAction,
    pub(crate) dependency: Dependency,
    pub(crate) session: u64,
    pub(crate) config_generation: u64,
    checked_generation: Option<u64>,
    pub(crate) recorded_at: std::time::SystemTime,
    pub(crate) result: Option<RecoveryResult>,
    pub(crate) checked: Option<CapabilityRevision>,
    pub(crate) running: bool,
}

impl RecoveryIntent {
    pub(crate) fn completed(&self) -> bool {
        matches!(self.result, Some(RecoveryResult::Completed(_)))
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RecoveryIntents {
    entries: Vec<RecoveryIntent>,
    sequence: u64,
    config_generation: u64,
}

impl RecoveryIntents {
    pub(crate) fn entries(&self) -> &[RecoveryIntent] {
        &self.entries
    }

    pub(crate) fn retain(
        &mut self,
        action: RecoveryAction,
        dependency: Dependency,
        session: u64,
    ) -> u64 {
        // Repeated attempts for the same immutable subject refresh its check requirement.
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.action == action && entry.session == session && !entry.running)
        {
            entry.dependency = dependency;
            entry.checked = None;
            entry.result = None;
            entry.recorded_at = std::time::SystemTime::now();
            return entry.id;
        }
        self.sequence += 1;
        self.entries.push(RecoveryIntent {
            id: self.sequence,
            action,
            dependency,
            session,
            config_generation: self.config_generation,
            checked_generation: None,
            recorded_at: std::time::SystemTime::now(),
            result: None,
            checked: None,
            running: false,
        });
        self.sequence
    }

    pub(crate) fn saved(&mut self) {
        self.config_generation += 1;
        for entry in &mut self.entries {
            entry.checked = None;
        }
    }

    pub(crate) fn checking(&mut self, dependency: Dependency) {
        for entry in &mut self.entries {
            if entry.completed() {
                continue;
            }
            if entry.dependency == dependency || entry.action.dependency() == dependency {
                entry.checked = None;
            }
        }
    }

    pub(crate) fn checked(&mut self, dependency: Dependency, snapshot: &ConfigSnapshot) {
        for entry in &mut self.entries {
            if entry.completed() {
                continue;
            }
            if entry.dependency == dependency || entry.action.dependency() == dependency {
                if dependency == Dependency::BackgroundRuntime {
                    entry.dependency = entry.action.dependency();
                    entry.checked = None;
                    continue;
                }
                entry.checked = Some(CapabilityRevision::of(snapshot, entry.action.dependency()));
                entry.checked_generation = Some(self.config_generation);
                entry.result = None;
                entry.recorded_at = std::time::SystemTime::now();
            }
        }
    }

    pub(crate) fn begin_retry(
        &mut self,
        id: u64,
        session: &SessionLifecycle,
        features: FeatureAvailability,
    ) -> Result<RecoveryIntent, RetryRejection> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or(RetryRejection::SubjectChanged)?;
        if entry.completed() {
            return Err(RetryRejection::Completed);
        }
        if entry.running
            || entry.checked.is_none()
            || entry.checked_generation != Some(self.config_generation)
            || entry.config_generation > self.config_generation
        {
            return Err(RetryRejection::CheckRequired);
        }
        if !session.accepts(entry.session) {
            return Err(RetryRejection::SessionChanged);
        }
        features
            .require(entry.action.dependency())
            .map_err(RetryRejection::Unavailable)?;
        entry.running = true;
        Ok(entry.clone())
    }

    pub(crate) fn finish(&mut self, id: u64, message: String) {
        self.record_result(id, RecoveryResult::Failed(message));
    }

    pub(crate) fn succeed(&mut self, id: u64, message: String) {
        self.record_result(id, RecoveryResult::Completed(message));
    }

    fn record_result(&mut self, id: u64, result: RecoveryResult) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
            if entry.completed() {
                return;
            }
            entry.running = false;
            entry.checked = None;
            entry.result = Some(result);
            entry.recorded_at = std::time::SystemTime::now();
        }
    }

    pub(crate) fn dismiss(&mut self, id: u64) {
        self.entries.retain(|entry| entry.id != id || entry.running);
    }
}

/// Reuses the original command after checking consent at the execution boundary.
pub(crate) struct RetryCommand<C> {
    pub(crate) command: C,
    pub(crate) intent: RecoveryIntent,
    pub(crate) path: PathBuf,
    pub(crate) conn: Arc<Mutex<Connection>>,
    pub(crate) session: SessionLifecycle,
}

pub(crate) enum OriginalOrRetry<C> {
    Original(C),
    Retry(Box<RetryCommand<C>>),
}

impl<C: ApplicationCommand> ApplicationCommand for OriginalOrRetry<C> {
    type Output = C::Output;
    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        match self {
            Self::Original(command) => command.execute(context),
            Self::Retry(command) => command.execute(context),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RetrySubject {
    pub(crate) action: RecoveryAction,
    pub(crate) snapshot: ConfigSnapshot,
}

impl<C: ApplicationCommand> ApplicationCommand for RetryCommand<C> {
    type Output = C::Output;
    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let _configuration = crate::config::ConfigWriteLease::acquire(&self.path)
            .map_err(|_| CommandError::Other(RetryRejection::ConfigurationChanged.to_string()))?;
        let snapshot = self
            .validate()
            .map_err(|error| CommandError::Other(error.to_string()))?;
        let context = context.clone().with_retry_subject(RetrySubject {
            action: self.intent.action,
            snapshot,
        });
        self.command.execute(&context)
    }
}

impl<C> RetryCommand<C> {
    fn validate(&self) -> Result<ConfigSnapshot, RetryRejection> {
        if !self.session.accepts(self.intent.session) {
            return Err(RetryRejection::SessionChanged);
        }
        let snapshot = ConfigSnapshot::read_existing(Path::new(&self.path))
            .map_err(|_| RetryRejection::ConfigurationChanged)?;
        if self.intent.checked.as_ref()
            != Some(&CapabilityRevision::of(
                &snapshot,
                self.intent.action.dependency(),
            ))
        {
            return Err(RetryRejection::ConfigurationChanged);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| RetryRejection::SubjectChanged)?;
        self.intent.action.validate_subject(&conn, &snapshot)?;
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::super::CommandOutcome;
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn fixture() -> (tempfile::TempDir, PathBuf, Arc<Mutex<Connection>>) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(&path, "music_dir = '/music'\ndb_path = '/database'\nmusicindex_endpoint = 'https://index.test'\n").unwrap();
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::migrate_schema(&conn).unwrap();
        (temp, path, Arc::new(Mutex::new(conn)))
    }

    fn features(snapshot: &ConfigSnapshot) -> FeatureAvailability {
        FeatureAvailability::from_resources(
            &crate::config::MusicIndexEndpoint::from_field(snapshot.musicindex_endpoint.clone()),
            &snapshot.broadcast(),
            true,
        )
    }

    struct Count(Arc<AtomicUsize>);
    impl ApplicationCommand for Count {
        type Output = ();
        fn execute(self, _: &CommandContext) -> CommandResult<()> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(CommandOutcome::without_events(()))
        }
    }

    #[test]
    fn adr_0066_recovery_keeps_multiple_original_queries_and_save_never_retries() {
        let (_temp, path, conn) = fixture();
        let snapshot = ConfigSnapshot::read_existing(&path).unwrap();
        let session = SessionLifecycle::new();
        let mut pending = RecoveryIntents::default();
        let first = pending.retain(
            RecoveryAction::IndexSearch {
                query: "first query".into(),
            },
            Dependency::MusicIndex,
            session.generation(),
        );
        let second = pending.retain(
            RecoveryAction::IndexSearch {
                query: "second query".into(),
            },
            Dependency::MusicIndex,
            session.generation(),
        );
        assert_ne!(first, second);
        assert_eq!(pending.entries().len(), 2);
        assert_eq!(
            pending
                .begin_retry(first, &session, features(&snapshot))
                .unwrap_err(),
            RetryRejection::CheckRequired
        );
        pending.checked(Dependency::MusicIndex, &snapshot);
        let count = Arc::new(AtomicUsize::new(0));
        pending.saved();
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert_eq!(
            pending
                .begin_retry(first, &session, features(&snapshot))
                .unwrap_err(),
            RetryRejection::CheckRequired
        );
        pending.checked(Dependency::MusicIndex, &snapshot);
        let intent = pending
            .begin_retry(first, &session, features(&snapshot))
            .unwrap();
        assert_eq!(
            intent.action,
            RecoveryAction::IndexSearch {
                query: "first query".into()
            }
        );
        assert_eq!(
            pending
                .begin_retry(first, &session, features(&snapshot))
                .unwrap_err(),
            RetryRejection::CheckRequired
        );
        RetryCommand {
            command: Count(count.clone()),
            intent,
            path,
            conn,
            session,
        }
        .execute(&CommandContext::next())
        .unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
        pending.succeed(first, "Original query completed".into());
        assert!(pending
            .entries()
            .iter()
            .find(|entry| entry.id == second)
            .unwrap()
            .checked
            .is_some());
    }

    #[test]
    fn adr_0066_completed_action_cannot_be_rearmed_by_a_setup_check() {
        let (_temp, path, _conn) = fixture();
        let snapshot = ConfigSnapshot::read_existing(&path).unwrap();
        let session = SessionLifecycle::new();
        let mut pending = RecoveryIntents::default();
        let action = RecoveryAction::IndexSearch {
            query: "completed query".into(),
        };
        let id = pending.retain(action.clone(), Dependency::MusicIndex, session.generation());
        pending.succeed(id, "App completed the original search.".into());
        let completed_at = pending.entries()[0].recorded_at;
        pending.saved();
        pending.checking(Dependency::MusicIndex);
        pending.checked(Dependency::MusicIndex, &snapshot);
        pending.finish(id, "An obsolete callback failed.".into());
        assert!(pending.entries()[0].completed());
        assert_eq!(pending.entries()[0].recorded_at, completed_at);
        assert!(pending.entries()[0].checked.is_none());
        assert_eq!(
            pending
                .begin_retry(id, &session, features(&snapshot))
                .unwrap_err(),
            RetryRejection::Completed
        );
        // A separate user action may fail later; it needs its own fresh check.
        pending.retain(action, Dependency::MusicIndex, session.generation());
        assert!(!pending.entries()[0].completed());
        assert_eq!(
            pending
                .begin_retry(id, &session, features(&snapshot))
                .unwrap_err(),
            RetryRejection::CheckRequired
        );
    }

    #[test]
    fn adr_0066_retry_rechecks_configuration_and_session_at_execution() {
        for change in ["configuration", "session", "unrelated workspace"] {
            let (_temp, path, conn) = fixture();
            let snapshot = ConfigSnapshot::read_existing(&path).unwrap();
            let session = SessionLifecycle::new();
            let mut pending = RecoveryIntents::default();
            let id = pending.retain(
                RecoveryAction::IndexSearch {
                    query: "original".into(),
                },
                Dependency::MusicIndex,
                session.generation(),
            );
            pending.checked(Dependency::MusicIndex, &snapshot);
            let intent = pending
                .begin_retry(id, &session, features(&snapshot))
                .unwrap();
            match change {
                "configuration" => {
                    std::fs::write(&path, "musicindex_endpoint = 'https://changed.test'\n").unwrap()
                }
                "session" => {
                    assert!(session.begin_drain());
                }
                _ => {
                    use std::io::Write;
                    writeln!(
                        std::fs::OpenOptions::new()
                            .append(true)
                            .open(&path)
                            .unwrap(),
                        "\n[workspace.layout]\ncontent_pane_width = 700.0"
                    )
                    .unwrap();
                }
            }
            let count = Arc::new(AtomicUsize::new(0));
            let result = RetryCommand {
                command: Count(count.clone()),
                intent,
                path,
                conn,
                session,
            }
            .execute(&CommandContext::next());
            assert_eq!(result.is_ok(), change == "unrelated workspace");
            assert_eq!(
                count.load(Ordering::SeqCst),
                usize::from(change == "unrelated workspace")
            );
        }
    }

    #[test]
    fn adr_0066_runtime_verification_does_not_verify_an_optional_tool() {
        let (_temp, path, _) = fixture();
        let snapshot = ConfigSnapshot::read_existing(&path).unwrap();
        let mut pending = RecoveryIntents::default();
        pending.retain(
            RecoveryAction::IndexSearch {
                query: "query".into(),
            },
            Dependency::BackgroundRuntime,
            1,
        );
        pending.checked(Dependency::BackgroundRuntime, &snapshot);
        assert_eq!(pending.entries()[0].dependency, Dependency::MusicIndex);
        assert!(pending.entries()[0].checked.is_none());
        pending.checked(Dependency::MusicIndex, &snapshot);
        assert!(pending.entries()[0].checked.is_some());
    }

    #[test]
    fn adr_0066_target_retry_rejects_a_new_target_before_transport() {
        let host = BroadcastHostConfig::default_local();
        let action = RecoveryAction::Event {
            operation: crate::view_models::show::EventControlIntent::Detach,
            event_id: Some("original-event".into()),
            selection_revision: 4,
            host: Some(host.clone()),
            target: Some("original-target".into()),
        };
        let snapshot = ConfigSnapshot::from_bytes(Path::new("config.toml"), Vec::new()).unwrap();
        assert_eq!(action.event_target_name().unwrap(), "original-target");
        let context = CommandContext::next().with_retry_subject(RetrySubject { action, snapshot });
        assert!(context
            .validate_retry_event_target(
                "original-event",
                "original-target",
                &host.transport,
                &host.instance_name
            )
            .is_ok());
        for (event, target, instance) in [
            ("original-event", "new-target", host.instance_name.as_str()),
            ("new-event", "original-target", host.instance_name.as_str()),
            ("original-event", "original-target", "another-instance"),
        ] {
            assert!(context
                .validate_retry_event_target(event, target, &host.transport, instance)
                .is_err());
        }
    }

    #[test]
    fn adr_0066_retry_rejects_changed_publisher_and_removed_events() {
        let (_temp, path, conn) = fixture();
        let snapshot = ConfigSnapshot::read_existing(&path).unwrap();
        let mut action = RecoveryAction::Publisher {
            role: BroadcastServiceRole::Publisher,
            operation: PublisherServiceOperation::Start,
            host: snapshot.broadcast().selected_host().ok().cloned(),
            event_id: None,
        };
        let conn = conn.lock().unwrap();
        assert_eq!(action.validate_subject(&conn, &snapshot), Ok(()));
        let changed =
            ConfigSnapshot::from_bytes(&path, b"[broadcast]\nselected_host = 'missing'\n".to_vec())
                .unwrap();
        assert_eq!(
            action.validate_subject(&conn, &changed),
            Err(RetryRejection::PublisherChanged)
        );
        if let RecoveryAction::Publisher { event_id, .. } = &mut action {
            *event_id = Some("removed-event".into());
        }
        assert_eq!(
            action.validate_subject(&conn, &snapshot),
            Err(RetryRejection::SubjectChanged)
        );
        if let RecoveryAction::Publisher { host, .. } = &mut action {
            *host = None;
        }
        assert_eq!(
            action.validate_subject(&conn, &snapshot),
            Err(RetryRejection::PublisherChanged)
        );
    }

    #[test]
    fn adr_0066_retry_rejects_removed_track_and_changed_playlist_position() {
        let (_temp, path, conn) = fixture();
        let snapshot = ConfigSnapshot::read_existing(&path).unwrap();
        let shared = conn.clone();
        let conn = conn.lock().unwrap();
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid) VALUES ('https://feed.test', 'feed-guid')",
            [],
        )
        .unwrap();
        let feed = conn.last_insert_rowid();
        for guid in ["first", "second"] {
            conn.execute("INSERT INTO tracks (feed_id, item_guid, track_title, is_in_library) VALUES (?1, ?2, ?2, 1)", rusqlite::params![feed, guid]).unwrap();
            let id = conn.last_insert_rowid();
            db::mark_track_downloaded(
                &conn,
                id,
                &crate::library_path::LibraryRelativePath::for_test(&format!("{guid}.wav")),
                None,
            )
            .unwrap();
        }
        let playlist = db::playlist_create(&conn, "original playlist").unwrap();
        db::playlist_append(&conn, playlist, 1).unwrap();
        db::playlist_append(&conn, playlist, 2).unwrap();
        let action = RecoveryAction::Playback {
            operation: PlaybackOperation::Playlist {
                playlist_id: playlist,
                position: 0,
            },
            track_id: Some(1),
            queue: Vec::new(),
        };
        assert_eq!(action.validate_subject(&conn, &snapshot), Ok(()));
        conn.execute("DELETE FROM playlist_tracks WHERE track_id = 1", [])
            .unwrap();
        assert_eq!(
            action.validate_subject(&conn, &snapshot),
            Err(RetryRejection::SubjectChanged)
        );
        conn.execute("DELETE FROM tracks WHERE id = 1", []).unwrap();
        assert_eq!(
            action.validate_subject(&conn, &snapshot),
            Err(RetryRejection::SubjectChanged)
        );
        let owner = Arc::new(Mutex::new(crate::playback_owner::PlaybackOwner::new(
            crate::playback_driver::NullDriver::new(),
            playback::DEFAULT_SESSION_ID,
            "/",
        )));
        let context = CommandContext::next().with_retry_subject(RetrySubject { action, snapshot });
        drop(conn);
        // The command rechecks under its own lock, even if state changed after admission.
        let command = crate::application::commands::playback::PlayPlaylistAt::new(
            shared.clone(),
            owner,
            playlist,
            0,
        );
        assert!(command.execute(&context).is_err());
        assert!(
            db::playback_session(&shared.lock().unwrap(), playback::DEFAULT_SESSION_ID)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn adr_0066_retained_inputs_and_revisions_do_not_leak_through_debug() {
        let action = RecoveryAction::IndexSearch {
            query: "https://operator:secret@index.test/?token=private".into(),
        };
        let snapshot = ConfigSnapshot::from_bytes(
            Path::new("config.toml"),
            b"musicindex_endpoint = 'https://operator:secret@index.test/?token=private'".to_vec(),
        )
        .unwrap();
        let rendered = format!(
            "{action:?} {:?}",
            CapabilityRevision::of(&snapshot, Dependency::MusicIndex)
        );
        for secret in ["operator", "secret", "private"] {
            assert!(!rendered.contains(secret));
        }
    }
}
