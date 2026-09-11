//! Core startup checks and preparation, independent of GPUI (ADR 0066).

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rusqlite::Connection;

use crate::config::{self, ConfigSnapshot};
use crate::db::startup::{self as database, DatabaseReadiness, DbStage};

#[cfg(debug_assertions)]
pub mod fixture;
mod storage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupStage {
    ConfigPath,
    ConfigRead,
    ConfigField,
    MusicInspect,
    MusicList,
    MusicCreateProbe,
    MusicWriteProbe,
    MusicReadProbe,
    MusicRemoveProbe,
    MusicSetup,
    Artists,
    Database(DbStage),
    Worker,
    Window,
    Activation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueSeverity {
    Blocked,
    NeedsPreparation,
    Notice,
}

/// Safe report data: no raw TOML, token values, URLs with credentials, or SQL errors.
#[derive(Clone, Debug)]
pub struct StartupIssue {
    pub stage: StartupStage,
    pub resource: Option<PathBuf>,
    pub observed_at: SystemTime,
    pub severity: IssueSeverity,
    pub cause: String,
    pub next_action: &'static str,
}

impl StartupIssue {
    pub fn new(
        stage: StartupStage,
        path: Option<&Path>,
        cause: impl Into<String>,
        next_action: &'static str,
    ) -> Self {
        Self {
            stage,
            resource: path.map(Path::to_path_buf),
            observed_at: SystemTime::now(),
            severity: IssueSeverity::Blocked,
            cause: crate::diagnostics::redact_endpoint_details(&cause.into()),
            next_action,
        }
    }

    pub fn io(stage: StartupStage, path: &Path, error: &std::io::Error) -> Self {
        let cause = match error.kind() {
            std::io::ErrorKind::NotFound => "The configured location does not exist.",
            std::io::ErrorKind::PermissionDenied => "The operating system denied access.",
            std::io::ErrorKind::AlreadyExists => "Another entry already occupies this path.",
            std::io::ErrorKind::InvalidData => {
                "App read back probe data that did not match the bytes it wrote."
            }
            std::io::ErrorKind::NotADirectory | std::io::ErrorKind::IsADirectory => {
                "This path has the wrong file type."
            }
            _ => "The operating system could not complete this storage operation.",
        };
        Self::new(
            stage,
            Some(path),
            cause,
            "Check the path, mounted storage and permissions, then choose Check again.",
        )
    }
}

pub struct CoreCheckOutcome {
    pub observed_at: SystemTime,
    pub issues: Vec<StartupIssue>,
    pub observations: Vec<CoreObservation>,
    checked_bytes: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct CoreObservation {
    pub observed_at: SystemTime,
    pub resource: PathBuf,
    pub description: &'static str,
}
impl fmt::Debug for CoreCheckOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CoreCheckOutcome")
            .field("issues", &self.issues)
            .finish_non_exhaustive()
    }
}
impl CoreCheckOutcome {
    pub fn pending() -> Self {
        Self {
            observed_at: SystemTime::now(),
            issues: Vec::new(),
            observations: Vec::new(),
            checked_bytes: None,
        }
    }
    pub fn blocked(issue: StartupIssue) -> Self {
        Self {
            observed_at: issue.observed_at,
            issues: vec![issue],
            observations: Vec::new(),
            checked_bytes: None,
        }
    }
    pub fn can_open(&self) -> bool {
        self.checked_bytes.is_some()
            && !self
                .issues
                .iter()
                .any(|i| i.severity == IssueSeverity::Blocked)
    }
    pub fn checked_bytes(&self) -> Option<&[u8]> {
        self.checked_bytes.as_deref()
    }
}

/// Verified resources are moved once into normal startup; a failure has none.
pub struct PreparedCore {
    pub config_path: PathBuf,
    pub snapshot: ConfigSnapshot,
    pub connection: Connection,
    pub notices: Vec<StartupIssue>,
}

pub enum CoreResult {
    Checked(CoreCheckOutcome),
    Prepared(Box<PreparedCore>),
}

#[derive(Clone)]
pub enum CheckIntent {
    Initial,
    Check,
    Open { checked_bytes: Vec<u8> },
}

/// Session-owned first-run entitlement; a changed document cannot reuse it.
pub struct StartupBackend {
    config_path: Option<PathBuf>,
    first_run: Option<(PathBuf, Vec<u8>)>,
}
impl StartupBackend {
    pub fn new(config_path: Option<PathBuf>) -> Self {
        Self {
            config_path,
            first_run: None,
        }
    }

    pub fn execute(&mut self, intent: CheckIntent) -> CoreResult {
        let path = match self
            .config_path
            .clone()
            .map_or_else(config::config_path, Ok)
        {
            Ok(path) => path,
            Err(_) => {
                return CoreResult::Checked(CoreCheckOutcome::blocked(StartupIssue::new(
                    StartupStage::ConfigPath,
                    None,
                    "App could not determine the configuration file location.",
                    "Correct the user configuration directory environment and relaunch the app.",
                )))
            }
        };
        self.config_path = Some(path.clone());
        let snapshot = match if matches!(intent, CheckIntent::Initial) {
            config::load_config_snapshot(&path)
        } else {
            ConfigSnapshot::read_existing(&path)
        } {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return CoreResult::Checked(CoreCheckOutcome::blocked(StartupIssue::new(
                    StartupStage::ConfigRead,
                    Some(&path),
                    format!("{error:#}"),
                    "Correct this file without replacing your settings, then choose Check again.",
                )))
            }
        };
        if snapshot.created_defaults() {
            self.first_run = Some((path.clone(), snapshot.original_bytes().to_vec()));
        }
        let first_run = self
            .first_run
            .as_ref()
            .is_some_and(|(p, bytes)| p == &path && bytes == snapshot.original_bytes());
        // A changed input earns a new check, never preparation under old consent.
        let prepare = match &intent {
            CheckIntent::Initial => true,
            CheckIntent::Open { checked_bytes } => checked_bytes == snapshot.original_bytes(),
            CheckIntent::Check => false,
        };
        let mut outcome = CoreCheckOutcome {
            observed_at: SystemTime::now(),
            issues: Vec::new(),
            observations: Vec::new(),
            checked_bytes: Some(snapshot.original_bytes().to_vec()),
        };
        for field in [&snapshot.music_dir, &snapshot.db_path] {
            if let Err(issue) = field {
                outcome.issues.push(StartupIssue::new(
                    StartupStage::ConfigField,
                    Some(&path),
                    issue.to_string(),
                    "Correct the named path in this file, then choose Check again.",
                ));
            }
        }
        // Optional issues remain visible but do not participate in core admission.
        for field in snapshot
            .issues()
            .into_iter()
            .filter(|i| !matches!(i.field, "music_dir" | "db_path"))
        {
            let mut issue = StartupIssue::new(StartupStage::ConfigField, Some(&path), field.to_string(), "Correct this optional setting. Full in-app correction is delivered by a later recovery packet.");
            issue.severity = IssueSeverity::Notice;
            outcome.issues.push(issue);
        }
        if let Ok(music) = &snapshot.music_dir {
            match storage::check_music(music, first_run, prepare) {
                Ok(Some(issue)) => outcome.issues.push(issue),
                Err(issue) => outcome.issues.push(issue),
                Ok(None) => outcome.observations.push(CoreObservation {
                    observed_at: SystemTime::now(), resource: music.clone(),
                    description: "App listed the music directory and wrote, read back and removed its own probe. App did not open existing audio files.",
                }),
            }
        }
        if let Ok(db_path) = &snapshot.db_path {
            match database::check_database(db_path) {
                Ok(DatabaseReadiness::NeedsPreparation) => {
                    let mut issue = StartupIssue::new(StartupStage::Database(DbStage::Schema), Some(db_path), "The database needs initial setup or a supported schema upgrade.", "Choose Open app to prepare this database through the normal migration path.");
                    issue.severity = IssueSeverity::NeedsPreparation;
                    outcome.issues.push(issue);
                }
                Err(error) => outcome.issues.push(database_issue(db_path, error)),
                Ok(DatabaseReadiness::Ready) => outcome.observations.push(CoreObservation {
                    observed_at: SystemTime::now(), resource: db_path.clone(),
                    description: "App verified the SQLite schema, read the database and tested a main-database write. App rolled back its probe.",
                }),
            }
        }
        if !prepare || !outcome.can_open() {
            return CoreResult::Checked(outcome);
        }
        self.prepare(path, snapshot, outcome)
    }

    fn prepare(
        &self,
        path: PathBuf,
        snapshot: ConfigSnapshot,
        mut outcome: CoreCheckOutcome,
    ) -> CoreResult {
        let db_path = snapshot
            .db_path
            .as_ref()
            .expect("core admission verified database path");
        let connection = match database::prepare_database(db_path) {
            Ok(conn) => conn,
            Err(error) => {
                outcome.issues.push(database_issue(db_path, error));
                return CoreResult::Checked(outcome);
            }
        };
        let music = snapshot
            .music_dir
            .as_ref()
            .expect("core admission verified music path");
        if let Err(error) = config::prepare_artists_directory(music) {
            let mut issue = StartupIssue::io(StartupStage::Artists, &music.join("artists"), &error);
            issue.severity = IssueSeverity::Notice;
            issue.next_action = "Correct the artists directory before downloading music. Other library work can continue.";
            outcome.issues.push(issue);
        }
        outcome
            .issues
            .retain(|issue| issue.severity == IssueSeverity::Notice);
        CoreResult::Prepared(Box::new(PreparedCore {
            config_path: path,
            snapshot,
            connection,
            notices: outcome.issues,
        }))
    }
}

fn database_issue(path: &Path, error: database::DbCheckError) -> StartupIssue {
    StartupIssue::new(StartupStage::Database(error.stage), Some(path), error.reason,
        "Close other database users or correct access, then choose Check again. Preserve this file before attempting database repair.")
}

/// Prepare optional resources after core admission (ADR 0066).
pub(crate) fn prepare_playback(
    snapshot: &ConfigSnapshot,
    config_path: &Path,
    observations: &crate::application::capability::CapabilityObservations,
) -> Option<crate::playback_owner::PlaybackOwner<crate::playback_driver::ConfiguredPlaybackDriver>>
{
    prepare_playback_with(snapshot, config_path, observations, |config| {
        #[cfg(all(debug_assertions, unix))]
        if config.driver == config::PlaybackDriver::Mpv {
            if let Some(directory) = fixture::playback_runtime_directory(config_path) {
                return crate::playback_driver::ConfiguredPlaybackDriver::from_config_in_directory(
                    config, directory,
                );
            }
        }
        crate::playback_driver::ConfiguredPlaybackDriver::from_config(config)
    })
}

fn prepare_playback_with(
    snapshot: &ConfigSnapshot,
    config_path: &Path,
    observations: &crate::application::capability::CapabilityObservations,
    driver: impl FnOnce(
        &config::PlaybackConfig,
    ) -> anyhow::Result<crate::playback_driver::ConfiguredPlaybackDriver>,
) -> Option<crate::playback_owner::PlaybackOwner<crate::playback_driver::ConfiguredPlaybackDriver>>
{
    use crate::application::capability::{CapabilityFailure, CapabilityObservation, Dependency};
    for issue in snapshot
        .issues()
        .into_iter()
        .filter(|issue| !matches!(issue.field, "music_dir" | "db_path"))
    {
        observations.record(
            CapabilityObservation::new(
                Dependency::Configuration(issue.field),
                Some(CapabilityFailure::Configuration(issue)),
            )
            .at_path(config_path),
        );
    }
    let producer = match snapshot
        .broadcast()
        .drop_file_producer()
        .and_then(|producer| {
            if let Some(producer) = &producer {
                producer.prepare_directory()?;
            }
            Ok(producer)
        }) {
        Ok(producer) => producer,
        Err(_) => {
            let location = snapshot
                .drop_directory
                .as_ref()
                .ok()
                .and_then(Option::as_deref)
                .unwrap_or(config_path);
            observations.record(
                CapabilityObservation::new(
                    Dependency::Producer,
                    Some(CapabilityFailure::Preparation),
                )
                .at_path(location),
            );
            None
        }
    };
    let config = snapshot.playback().ok()?;
    let driver = match driver(&config) {
        Ok(driver) => driver,
        Err(_) => {
            observations.record(
                CapabilityObservation::new(
                    Dependency::Playback,
                    Some(CapabilityFailure::Preparation),
                )
                .at_path(config_path),
            );
            return None;
        }
    };
    Some(
        crate::playback_owner::PlaybackOwner::new(
            driver,
            crate::playback::DEFAULT_SESSION_ID,
            snapshot
                .music_dir
                .as_ref()
                .expect("core admission verified music path")
                .clone(),
        )
        .with_drop_file_producer(producer),
    )
}

/// Contain an incomplete path repair without discarding committed bindings.
pub(crate) fn prepare_local_paths(
    connection: &Connection,
    music_dir: &Path,
    db_path: &Path,
) -> Result<bool, CoreCheckOutcome> {
    let result = crate::db::repair_local_file_paths(connection, music_dir);
    if result.as_ref().is_ok_and(|repair| repair.skipped.is_none()) {
        return Ok(false);
    }
    if let Err(issue) = storage::check_music(music_dir, false, false) {
        return Err(CoreCheckOutcome::blocked(issue));
    }
    match database::check_database(db_path) {
        Ok(DatabaseReadiness::Ready) => Ok(result.is_err()),
        Ok(DatabaseReadiness::NeedsPreparation) => Err(CoreCheckOutcome::blocked(StartupIssue::new(
            StartupStage::Database(DbStage::Schema), Some(db_path),
            "App could not verify the database after local path repair. Completed changes remain recorded.",
            "Preserve this database and correct its schema before choosing Check again.",
        ))),
        Err(error) => Err(CoreCheckOutcome::blocked(database_issue(db_path, error))),
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn adr_0066_failed_player_and_producer_preserve_independent_resources() {
        use crate::application::capability::{
            CapabilityObservations, Dependency, FeatureAvailability,
        };
        let temp = tempfile::tempdir().unwrap();
        let blocked = temp.path().join("occupied");
        fs::write(&blocked, b"preserve").unwrap();
        let text = format!("music_dir = {:?}\ndb_path = {:?}\n[playback]\ndriver = 'mpv'\n[broadcast]\ndrop_directory = {:?}\n", temp.path(), temp.path().join("db"), blocked);
        let config_path = temp.path().join("config.toml");
        let snapshot = ConfigSnapshot::from_bytes(&config_path, text.into_bytes()).unwrap();
        let observations = CapabilityObservations::default();
        let owner = prepare_playback_with(&snapshot, &config_path, &observations, |config| {
            assert_eq!(config.driver, config::PlaybackDriver::Mpv);
            crate::playback_driver::MpvDriver::with_runtime_dir("not-launched-mpv", blocked.clone())
                .map(crate::playback_driver::ConfiguredPlaybackDriver::Mpv)
        });
        assert!(owner.is_none());
        let observations = observations.snapshot();
        assert!(observations[&Dependency::Playback].failure.is_some());
        assert!(observations[&Dependency::Producer].failure.is_some());
        let features = FeatureAvailability::from_resources(
            &config::MusicIndexEndpoint::from_field(snapshot.musicindex_endpoint.clone()),
            &snapshot.broadcast(),
            owner.is_some(),
        )
        .with_observations(&observations);
        assert!(features.require(Dependency::Publisher).is_ok());
        assert!(features.require(Dependency::Playback).is_err());
        assert!(features.require(Dependency::Producer).is_err());
        assert_eq!(fs::read(&blocked).unwrap(), b"preserve");

        let null = ConfigSnapshot::from_bytes(
            &config_path,
            format!(
                "music_dir = {:?}\ndb_path = {:?}\n[broadcast]\ndrop_directory = {:?}\n",
                temp.path(),
                temp.path().join("db"),
                blocked
            )
            .into_bytes(),
        )
        .unwrap();
        let owner =
            prepare_playback(&null, &config_path, &CapabilityObservations::default()).unwrap();
        assert!(!owner.driver().is_live_driver());
        assert_eq!(
            null.playback().unwrap().driver,
            config::PlaybackDriver::Null
        );
    }

    #[test]
    fn adr_0066_partial_repair_keeps_committed_and_unvalidated_bindings() {
        let temp = tempfile::tempdir().unwrap();
        let music = temp.path().join("music");
        fs::create_dir(&music).unwrap();
        let path = temp.path().join("library.sqlite");
        let conn = crate::db::open_db(&path).unwrap();
        conn.execute(
            "INSERT INTO feeds (id, feed_url, feed_guid) VALUES (1, 'fixture:rss', 'fixture-feed')",
            [],
        )
        .unwrap();
        for (id, name, stored) in [
            (1, "a.mp3", "/old/music/a.mp3"),
            (2, "b.mp3", "/old/music/b.mp3"),
            (3, "c.mp3", "c.mp3"),
        ] {
            fs::write(music.join(name), name.as_bytes()).unwrap();
            conn.execute("INSERT INTO tracks (id, feed_id, item_guid, track_title, is_in_library) VALUES (?1, 1, ?2, ?2, 1)", rusqlite::params![id, name]).unwrap();
            conn.execute(
                "INSERT INTO local_files (path, track_id) VALUES (?1, ?2)",
                rusqlite::params![stored, id],
            )
            .unwrap();
        }
        conn.execute_batch("CREATE TRIGGER stop_second_path BEFORE UPDATE OF path ON local_files WHEN OLD.track_id = 2 BEGIN SELECT RAISE(FAIL, 'fixture path repair failure'); END;").unwrap();
        assert!(prepare_local_paths(&conn, &music, &path)
            .unwrap_or_else(|_| panic!("core must remain usable")));
        let stored: Vec<String> = conn
            .prepare("SELECT path FROM local_files ORDER BY track_id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(stored, ["a.mp3", "/old/music/b.mp3", "c.mp3"]);
        let rows = crate::library_service::library_tracks(&conn).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows
            .iter()
            .find(|row| row.id == 2)
            .unwrap()
            .local_path
            .is_none());
        for id in [1, 3] {
            assert!(rows
                .iter()
                .find(|row| row.id == id)
                .unwrap()
                .local_path
                .as_ref()
                .unwrap()
                .resolve(&music)
                .is_file());
        }
        for name in ["a.mp3", "b.mp3", "c.mp3"] {
            assert_eq!(fs::read(music.join(name)).unwrap(), name.as_bytes());
        }
        assert!(fs::read_dir(&music).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("probe")));
        conn.execute_batch("DROP TRIGGER stop_second_path").unwrap();
        assert!(!prepare_local_paths(&conn, &music, &path)
            .unwrap_or_else(|_| panic!("explicit repair should finish")));
        assert!(crate::db::track_row_by_id(&conn, 2)
            .unwrap()
            .unwrap()
            .local_path
            .is_some());
    }

    #[test]
    fn adr_0066_repair_failure_rechecks_core_storage_and_schema() {
        let temp = tempfile::tempdir().unwrap();
        let music = temp.path().join("music");
        fs::create_dir(&music).unwrap();
        let path = temp.path().join("library.sqlite");
        let conn = crate::db::open_db(&path).unwrap();
        conn.execute_batch("DROP TABLE local_files; DROP TABLE tracks;")
            .unwrap();
        assert!(prepare_local_paths(&conn, &music, &path).is_err());
        fs::remove_dir(&music).unwrap();
        assert!(prepare_local_paths(&conn, &music, &path).is_err());
        assert!(!music.exists());
    }

    use super::*;
    use std::fs;

    fn fixture(temp: &Path) -> PathBuf {
        fs::create_dir(temp.join("music")).unwrap();
        let path = temp.join("config.toml");
        fs::write(
            &path,
            format!(
                "music_dir = {}\ndb_path = {}\n",
                toml::Value::String(temp.join("music").display().to_string()),
                toml::Value::String(temp.join("data/library.sqlite").display().to_string())
            ),
        )
        .unwrap();
        path
    }
    fn checked(result: CoreResult) -> CoreCheckOutcome {
        match result {
            CoreResult::Checked(outcome) => outcome,
            CoreResult::Prepared(_) => panic!("unexpected preparation"),
        }
    }

    #[test]
    fn adr_0066_checks_do_not_prepare_and_changed_inputs_require_new_consent() {
        let temp = tempfile::tempdir().unwrap();
        let path = fixture(temp.path());
        let mut backend = StartupBackend::new(Some(path.clone()));
        let outcome = checked(backend.execute(CheckIntent::Check));
        assert!(outcome.can_open());
        assert!(!temp.path().join("data").exists());
        assert!(!temp.path().join("music/artists").exists());
        let bytes = outcome.checked_bytes().unwrap().to_vec();
        fs::write(
            &path,
            format!(
                "{}\n# external edit\n",
                String::from_utf8(bytes.clone()).unwrap()
            ),
        )
        .unwrap();
        let new_outcome = checked(backend.execute(CheckIntent::Open {
            checked_bytes: bytes,
        }));
        assert!(!temp.path().join("data").exists());
        match backend.execute(CheckIntent::Open {
            checked_bytes: new_outcome.checked_bytes().unwrap().to_vec(),
        }) {
            CoreResult::Prepared(core) => {
                assert!(core.connection.is_autocommit());
                assert!(temp.path().join("music/artists").is_dir());
            }
            CoreResult::Checked(_) => panic!("explicit open must prepare"),
        }
        let before = fs::read(temp.path().join("data/library.sqlite")).unwrap();
        assert!(checked(backend.execute(CheckIntent::Check)).can_open());
        assert!(
            fs::read(temp.path().join("data/library.sqlite")).unwrap() == before,
            "check changed database bytes"
        );
    }

    #[test]
    fn adr_0066_core_failure_produces_no_prepared_connection_and_preserves_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let path = fixture(temp.path());
        let bytes = fs::read(&path).unwrap();
        fs::remove_dir(temp.path().join("music")).unwrap();
        let mut backend = StartupBackend::new(Some(path.clone()));
        let outcome = checked(backend.execute(CheckIntent::Initial));
        assert!(!outcome.can_open());
        assert!(!temp.path().join("data").exists());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::write(&path, "music_dir = 4\ndb_path = false").unwrap();
        let outcome = checked(backend.execute(CheckIntent::Check));
        assert_eq!(
            outcome
                .issues
                .iter()
                .filter(|i| i.stage == StartupStage::ConfigField)
                .count(),
            2
        );
    }

    #[test]
    fn adr_0066_artists_failure_and_optional_errors_are_scoped() {
        let temp = tempfile::tempdir().unwrap();
        let path = fixture(temp.path());
        fs::write(temp.path().join("music/artists"), b"existing file").unwrap();
        let mut text = fs::read_to_string(&path).unwrap();
        text.push_str("musicindex_endpoint = 7\n");
        fs::write(&path, text).unwrap();
        let mut backend = StartupBackend::new(Some(path));
        match backend.execute(CheckIntent::Initial) {
            CoreResult::Prepared(core) => {
                assert_eq!(core.notices.len(), 2);
                assert!(core
                    .notices
                    .iter()
                    .all(|i| i.severity == IssueSeverity::Notice));
            }
            CoreResult::Checked(_) => panic!("optional failures must not reject core admission"),
        }
        assert_eq!(
            fs::read(temp.path().join("music/artists")).unwrap(),
            b"existing file"
        );
    }
}
