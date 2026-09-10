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
            cause: redact_endpoint_details(&cause.into()),
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

// Redact before storing, so Debug, UI, clipboard and stderr share safe data.
fn redact_endpoint_details(detail: &str) -> String {
    detail
        .split_inclusive(char::is_whitespace)
        .map(|part| {
            let Some(start) = part.find("https://").or_else(|| part.find("http://")) else {
                return part.to_owned();
            };
            let raw = part[start..].trim_end_matches(|c: char| {
                c.is_whitespace() || matches!(c, '\'' | '"' | ')' | ',' | ';')
            });
            let suffix = &part[start + raw.len()..];
            let endpoint = match reqwest::Url::parse(raw) {
                Ok(mut url) => {
                    let _ = url.set_username("");
                    let _ = url.set_password(None);
                    url.set_query(None);
                    url.set_fragment(None);
                    url.to_string()
                }
                Err(_) => "[unreadable endpoint]".to_owned(),
            };
            format!("{}{endpoint}{suffix}", &part[..start])
        })
        .collect()
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

#[cfg(test)]
mod tests {
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
