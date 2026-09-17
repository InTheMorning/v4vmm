//! Fresh, independent optional-resource preparation without action replay (ADR 0066).

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::config::{ConfigSnapshot, MusicIndexEndpoint};
use crate::playback_driver::ConfiguredPlaybackDriver;
use crate::playback_owner::PlaybackOwner;

use super::super::capability::Dependency;

pub(crate) type Player = Arc<Mutex<PlaybackOwner<ConfiguredPlaybackDriver>>>;

pub(crate) enum PreparedCapability {
    Endpoint(MusicIndexEndpoint),
    Playback(Player),
    Configuration,
}

pub(crate) struct CapabilityCheck {
    pub(crate) snapshot: Arc<ConfigSnapshot>,
    pub(crate) prepared: PreparedCapability,
    pub(crate) observation: &'static str,
    // Keep in-app configuration stable through resource installation.
    _configuration: crate::config::ConfigWriteLease,
}

/// Runs on the independent maintenance worker; never starts a remote service.
pub(crate) fn check(
    path: &Path,
    dependency: Dependency,
    player: Option<Player>,
    music_dir: &Path,
    conn: &Arc<Mutex<rusqlite::Connection>>,
    producer: Option<crate::broadcast::producer::DropFileProducer>,
) -> Result<CapabilityCheck, &'static str> {
    let configuration = crate::config::ConfigWriteLease::acquire(path)
        .map_err(|_| "App could not reserve the configuration for a tool check. Wait for its current save or check, then check again.")?;
    let snapshot = ConfigSnapshot::read_existing(path).map_err(|_| "App could not read the current configuration. Use Configuration repair, then check the tool again.")?;
    let db_path = conn
        .lock()
        .ok()
        .and_then(|conn| conn.path().map(std::path::PathBuf::from))
        .ok_or("App could not identify its current database path.")?;
    if snapshot.music_dir.as_deref() != Ok(music_dir)
        || snapshot.db_path.as_deref() != Ok(db_path.as_path())
    {
        return Err("Core paths changed. End this app session and use core recovery before checking an optional tool.");
    }
    let mut observation =
        "App verified local configuration and setup. No external service response was checked.";
    let prepared = match dependency {
        Dependency::MusicIndex => {
            let endpoint = snapshot.musicindex_endpoint.as_ref().map_err(|_| {
                "App could not validate musicindex_endpoint. Correct its URL, then check again."
            })?;
            PreparedCapability::Endpoint(endpoint.into())
        }
        Dependency::Playback => {
            let config = snapshot.playback().map_err(|_| {
                "App could not validate playback settings. Correct them, then check again."
            })?;
            let driver = crate::startup::prepare_playback_driver(&config, path).map_err(|_| "App could not prepare the configured player. Check its runtime directory and permissions, then check again.")?;
            let player = if let Some(player) = player {
                player.lock().map_err(|_| "App could not access its player.")?.replace_idle_driver(driver)
                    .map_err(|_| "App kept its loaded player unchanged. End the app session before changing playback settings.")?;
                player
            } else {
                Arc::new(Mutex::new(
                    PlaybackOwner::new(driver, crate::playback::DEFAULT_SESSION_ID, music_dir)
                        .with_drop_file_producer(producer),
                ))
            };
            PreparedCapability::Playback(player)
        }
        Dependency::Publisher => {
            let broadcast = snapshot.broadcast();
            let host = broadcast.selected_host().map_err(|_| "App could not validate the selected publisher host. Correct broadcast.hosts and broadcast.selected_host, then check again.")?;
            let unit = crate::broadcast::control::UnitRef::publisher(&host.instance_name)
                .map_err(|_| "App could not identify the publisher unit.")?;
            let state = crate::broadcast::control::show(&host.transport, &unit).map_err(|_| "App could not read the selected publisher's service state. Check its host and connection, then check again.")?;
            if matches!(
                state,
                crate::broadcast::control::ServiceState::NotReachable
                    | crate::broadcast::control::ServiceState::Unknown
            ) {
                return Err("App could not obtain the selected publisher's service state. Check its host and connection, then check again.");
            }
            observation = "App read the selected publisher's service state. This check did not start or restart any service.";
            PreparedCapability::Configuration
        }
        Dependency::Producer => {
            let producer = snapshot.broadcast().drop_file_producer().map_err(|_| "App could not validate drop-file settings. Correct the directory and target, then check again.")?;
            if let Some(producer) = &producer {
                producer.prepare_directory().map_err(|_| "App could not prepare the drop-file directory. Check the path and permissions, then check again.")?;
            }
            if let Some(player) = player {
                player.lock().map_err(|_| "App could not access its player.")?.replace_idle_producer(producer)
                    .map_err(|_| "App kept its loaded player's publication target unchanged. End the app session before changing publication settings.")?;
            }
            PreparedCapability::Configuration
        }
        Dependency::Encoder => {
            let target = snapshot.encoder.as_ref().map_err(|_| "App could not validate broadcast.encoder. Correct it, then check again.")?
                .as_ref().ok_or("No stream encoder is configured. Configure broadcast.encoder before retrying a stream action.")?
                .target().map_err(|_| "App could not prepare the encoder target. Correct its settings, then check again.")?;
            crate::broadcast::encoder::status(&target).map_err(|_| "App could not read the configured encoder's status. Check its executable and connection, then check again.")?;
            observation = "App read the configured encoder's status. This check did not connect, disconnect or restart it.";
            PreparedCapability::Configuration
        }
        Dependency::Presentation => {
            if snapshot.issues().iter().any(|issue| {
                super::super::capability::configuration_dependency(issue.field)
                    == Dependency::Presentation
            }) {
                return Err("App still found invalid presentation settings. Correct the named fields, then check again.");
            }
            PreparedCapability::Configuration
        }
        Dependency::Converter => {
            snapshot.flac_path.as_ref().map_err(|_| {
                "App could not validate flac_path. Correct its value before using conversion."
            })?;
            PreparedCapability::Configuration
        }
        _ => return Err("This resource uses its existing dedicated check."),
    };
    Ok(CapabilityCheck {
        snapshot: Arc::new(snapshot),
        prepared,
        observation,
        _configuration: configuration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::capability::{
        CapabilityFailure, CapabilityObservation, CapabilityObservations,
    };

    #[test]
    fn adr_0066_endpoint_check_preserves_player_producer_and_other_issues() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let music = temp.path().join("music");
        let database = temp.path().join("library.sqlite");
        std::fs::create_dir(&music).unwrap();
        let conn = Arc::new(Mutex::new(rusqlite::Connection::open(&database).unwrap()));
        let original = format!("music_dir = {music:?}\ndb_path = {database:?}\nmusicindex_endpoint = 'https://new.test'\nflac_path = false\n");
        std::fs::write(&path, &original).unwrap();
        let producer =
            crate::broadcast::producer::DropFileProducer::new(temp.path().join("drop"), "default")
                .unwrap();
        let owner = PlaybackOwner::new(
            ConfiguredPlaybackDriver::from_config(&crate::config::PlaybackConfig::default())
                .unwrap(),
            crate::playback::DEFAULT_SESSION_ID,
            &music,
        )
        .with_drop_file_producer(Some(producer));
        let player = Arc::new(Mutex::new(owner));
        let before = Arc::as_ptr(&player);
        let observation = CapabilityObservations::default();
        observation.record(CapabilityObservation::new(
            Dependency::ThumbnailMaintenance,
            Some(CapabilityFailure::MaintenanceUnavailable),
        ));
        let checked = check(
            &path,
            Dependency::MusicIndex,
            Some(player.clone()),
            &music,
            &conn,
            None,
        )
        .unwrap();
        assert!(matches!(checked.prepared, PreparedCapability::Endpoint(_)));
        assert_eq!(before, Arc::as_ptr(&player));
        assert!(player.lock().unwrap().drop_file_producer().is_some());
        assert!(!player.lock().unwrap().driver().is_live_driver());
        observation.checked_configuration(&checked.snapshot, Dependency::MusicIndex);
        assert!(observation.snapshot()[&Dependency::ThumbnailMaintenance]
            .failure
            .is_some());
        assert_eq!(checked.snapshot.issues()[0].field, "flac_path");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert!(!temp.path().join("drop").exists());
        assert!(crate::config::ConfigWriteLease::acquire(&path).is_err());
        drop(checked);
        assert!(crate::config::ConfigWriteLease::acquire(&path).is_ok());
    }

    #[test]
    fn adr_0066_player_recovery_keeps_owner_and_does_not_materialize_a_session() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let music = temp.path().join("music");
        let database = temp.path().join("library.sqlite");
        std::fs::create_dir(&music).unwrap();
        let connection = rusqlite::Connection::open(&database).unwrap();
        crate::db::init_schema(&connection).unwrap();
        crate::db::migrate_schema(&connection).unwrap();
        let conn = Arc::new(Mutex::new(connection));
        let original =
            format!("music_dir = {music:?}\ndb_path = {database:?}\n[playback]\ndriver = 'null'\n");
        std::fs::write(&path, &original).unwrap();
        let prepared = check(&path, Dependency::Playback, None, &music, &conn, None).unwrap();
        let PreparedCapability::Playback(player) = prepared.prepared else {
            panic!("player expected");
        };
        drop(prepared._configuration);
        let again = check(
            &path,
            Dependency::Playback,
            Some(player.clone()),
            &music,
            &conn,
            None,
        )
        .unwrap();
        let PreparedCapability::Playback(same) = again.prepared else {
            panic!("player expected");
        };
        drop(again._configuration);
        assert!(Arc::ptr_eq(&player, &same));
        assert!(crate::db::playback_session(
            &conn.lock().unwrap(),
            crate::playback::DEFAULT_SESSION_ID
        )
        .unwrap()
        .is_none());
        std::fs::write(&path, original.replace("'null'", "false")).unwrap();
        assert!(check(
            &path,
            Dependency::Playback,
            Some(player.clone()),
            &music,
            &conn,
            None
        )
        .is_err());
        assert!(Arc::ptr_eq(&player, &same));
        assert!(!same.lock().unwrap().driver().is_live_driver());
    }

    #[test]
    fn adr_0066_producer_configuration_check_does_not_require_a_player() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let database = temp.path().join("library.sqlite");
        let drop_dir = temp.path().join("drop");
        let conn = Arc::new(Mutex::new(rusqlite::Connection::open(&database).unwrap()));
        let source = format!("music_dir = {:?}\ndb_path = {database:?}\n[playback]\ndriver = false\n[broadcast]\ndrop_directory = {drop_dir:?}\n", temp.path());
        std::fs::write(&path, &source).unwrap();
        let result = check(&path, Dependency::Producer, None, temp.path(), &conn, None).unwrap();
        assert!(matches!(result.prepared, PreparedCapability::Configuration));
        assert_eq!(std::fs::read_dir(&drop_dir).unwrap().count(), 0);
        assert!(result.snapshot.playback().is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
    }

    #[test]
    fn adr_0066_failed_check_does_not_resolve_an_issue_or_modify_a_core_path() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let database = temp.path().join("library.sqlite");
        let conn = Arc::new(Mutex::new(rusqlite::Connection::open(&database).unwrap()));
        let wrong_music = temp.path().join("unmounted");
        std::fs::write(&path, format!("music_dir = {wrong_music:?}\ndb_path = {database:?}\nmusicindex_endpoint = 'https://valid.test'\n")).unwrap();
        let original = std::fs::read(&path).unwrap();
        assert!(check(
            &path,
            Dependency::MusicIndex,
            None,
            temp.path(),
            &conn,
            None
        )
        .err()
        .unwrap()
        .contains("Core paths changed"));
        assert!(!wrong_music.exists());
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
}
