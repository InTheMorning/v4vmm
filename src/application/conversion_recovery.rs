//! Session-owned conversion requests, staging and explicit retry admission (ADR 0066).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;
use tokio::sync::watch;

use crate::audio_format::ConversionOutcome;
use crate::config::ConfigSnapshot;
use crate::subscribe_service::materialization::Materialization;
use crate::subscribe_service::{self, SubscribeTrackOutcome, SubscribeTrackRequest};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConversionState {
    WavRetained,
    Failed,
    RedownloadRequired,
    Running,
    Completed,
}

#[derive(Clone, Debug)]
pub(crate) struct ConversionReport {
    pub(crate) id: u64,
    pub(crate) track_id: i64,
    pub(crate) title: String,
    pub(crate) state: ConversionState,
    pub(crate) message: String,
    pub(crate) recorded_at: SystemTime,
}

struct Entry {
    operation: Materialization,
    original_request: SubscribeTrackRequest,
    db_path: PathBuf,
    key: String,
    playlist: Option<i64>,
    playlist_appended: bool,
}

#[derive(Default)]
struct State {
    sequence: u64,
    entries: HashMap<u64, Entry>,
    reports: Vec<ConversionReport>,
    running: HashSet<String>,
    closed: bool,
}

pub(crate) struct ConversionRecovery {
    state: Mutex<State>,
    updates: watch::Sender<Vec<ConversionReport>>,
}

impl std::fmt::Debug for ConversionRecovery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ConversionRecovery(..)")
    }
}

impl Default for ConversionRecovery {
    fn default() -> Self {
        Self {
            state: Mutex::new(State::default()),
            updates: watch::channel(Vec::new()).0,
        }
    }
}

impl ConversionRecovery {
    pub(crate) fn subscribe(&self) -> watch::Receiver<Vec<ConversionReport>> {
        self.updates.subscribe()
    }

    pub(crate) fn subscribe_track(
        &self,
        conn: Arc<Mutex<Connection>>,
        request: SubscribeTrackRequest,
    ) -> Result<SubscribeTrackOutcome> {
        self.subscribe_to(conn, request, None)
    }

    pub(crate) fn subscribe_to(
        &self,
        conn: Arc<Mutex<Connection>>,
        request: SubscribeTrackRequest,
        playlist: Option<i64>,
    ) -> Result<SubscribeTrackOutcome> {
        self.subscribe_to_at(conn, request, playlist, &crate::config::config_path()?)
    }

    fn subscribe_to_at(
        &self,
        conn: Arc<Mutex<Connection>>,
        request: SubscribeTrackRequest,
        playlist: Option<i64>,
        config_path: &Path,
    ) -> Result<SubscribeTrackOutcome> {
        let key = request_key(&request);
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("conversion owner unavailable"))?;
            anyhow::ensure!(!state.closed, "original app session ended");
            anyhow::ensure!(
                !state.running.contains(&key),
                "this track already has a running download"
            );
            anyhow::ensure!(
                !state.entries.values().any(|entry| entry.key == key),
                "this track has retained input; use its conversion action in Settings"
            );
            state.running.insert(key.clone());
        }
        let mut retained = None;
        let original_request = request.clone();
        let snapshot = ConfigSnapshot::read_existing(config_path);
        let result = (|| {
            let snapshot = snapshot.as_ref().map_err(|error| anyhow!("{error:#}"))?;
            let cfg = snapshot.downloads()?;
            crate::config::prepare_artists_directory(&cfg.music_dir)?;
            subscribe_service::subscribe_track_retaining(conn, &cfg, request, &mut retained)
        })();
        self.state
            .lock()
            .map_err(|_| anyhow!("conversion owner unavailable"))?
            .running
            .remove(&key);
        if let (Some(operation), Ok(snapshot)) = (retained, snapshot) {
            self.record_entry(
                Entry {
                    operation,
                    original_request,
                    db_path: snapshot.db_path?,
                    key,
                    playlist,
                    playlist_appended: false,
                },
                &result,
            )?;
        }
        result
    }

    fn record_entry(&self, entry: Entry, result: &Result<SubscribeTrackOutcome>) -> Result<()> {
        let keep = entry.operation.has_owned_staging()
            || result.as_ref().map_or(true, |outcome| {
                outcome.conversion == ConversionOutcome::WavRetained
            });
        if !keep
            && result
                .as_ref()
                .is_ok_and(|outcome| outcome.conversion == ConversionOutcome::NotRequired)
        {
            return Ok(());
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("conversion owner unavailable"))?;
        state.sequence += 1;
        let id = state.sequence;
        state.reports.push(report(id, &entry.operation, result));
        if keep {
            state.entries.insert(id, entry);
        }
        self.updates.send_replace(state.reports.clone());
        Ok(())
    }

    pub(crate) fn subscribe_feed(
        &self,
        conn: Arc<Mutex<Connection>>,
        request: subscribe_service::SubscribeFeedRequest,
    ) -> Result<subscribe_service::SubscribeFeedOutcome> {
        let snapshot = ConfigSnapshot::read_existing(&crate::config::config_path()?)?;
        let cfg = snapshot.downloads()?;
        crate::config::prepare_artists_directory(&cfg.music_dir)?;
        let db_path = snapshot.db_path?;
        subscribe_service::subscribe_feed_retaining(
            conn,
            &cfg,
            request,
            |original_request, retained, result| {
                if let Some(operation) = retained {
                    let key = request_key(&original_request);
                    self.record_entry(
                        Entry {
                            operation,
                            original_request,
                            db_path: db_path.clone(),
                            key,
                            playlist: None,
                            playlist_appended: false,
                        },
                        result,
                    )?;
                }
                Ok(())
            },
        )
    }

    pub(crate) fn retry(
        &self,
        id: u64,
        redownload: bool,
        conn: &Arc<Mutex<Connection>>,
        path: &Path,
    ) -> Result<SubscribeTrackOutcome> {
        let mut entry = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("conversion owner unavailable"))?;
            anyhow::ensure!(!state.closed, "original app session ended");
            anyhow::ensure!(
                !state
                    .reports
                    .iter()
                    .any(|row| row.id == id && row.state == ConversionState::Completed),
                "this conversion already completed; Dismiss can retry pending staging cleanup"
            );
            anyhow::ensure!(!redownload || state.reports.iter().any(|row| row.id == id && row.state == ConversionState::RedownloadRequired), "App did not redownload. Validate the retained input with Retry before choosing an explicit redownload");
            let entry = state
                .entries
                .remove(&id)
                .context("this conversion is running, completed or discarded")?;
            state.running.insert(entry.key.clone());
            update_report(
                &mut state,
                id,
                ConversionState::Running,
                "App is retrying the original track with its retained edits.".into(),
            );
            self.updates.send_replace(state.reports.clone());
            entry
        };
        let mut needs_redownload = false;
        let result = (|| {
            let snapshot = ConfigSnapshot::read_existing(path)?;
            anyhow::ensure!(
                snapshot.db_path.as_ref().ok() == Some(&entry.db_path),
                "database destination changed; start a new download"
            );
            let cfg = snapshot.downloads()?;
            {
                let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
                entry.operation.validate_subject(&db, &cfg)?;
                if let Some(playlist) = entry.playlist {
                    anyhow::ensure!(
                        crate::db::playlists_list(&db)?
                            .iter()
                            .any(|item| item.id == playlist),
                        "original playlist was removed"
                    );
                }
            }
            // The immutable original request also anchors the operation's key.
            anyhow::ensure!(
                request_key(&entry.original_request) == entry.key,
                "original download request changed"
            );
            if !redownload {
                if let Err(error) = entry.operation.validate_input() {
                    needs_redownload = true;
                    return Err(error.context("App did not retry. Choose Redownload original track to fetch the same enclosure again"));
                }
            }
            let outcome = entry.operation.run(conn, &cfg, true, redownload)?;
            if let Some(playlist) = entry.playlist.filter(|_| !entry.playlist_appended) {
                let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
                let present = crate::db::playlist_tracks(&db, playlist)?
                    .iter()
                    .any(|track| track.id == entry.operation.track_id());
                if !present {
                    crate::playlist_service::append_track(
                        &db,
                        playlist,
                        entry.operation.track_id(),
                    )?;
                }
                entry.playlist_appended = true;
            }
            Ok(outcome)
        })();
        let complete = result
            .as_ref()
            .is_ok_and(|outcome| outcome.conversion != ConversionOutcome::WavRetained);
        let mut current = report(id, &entry.operation, &result);
        if needs_redownload {
            current.state = ConversionState::RedownloadRequired;
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("conversion owner unavailable"))?;
        state.running.remove(&entry.key);
        if let Some(row) = state.reports.iter_mut().find(|row| row.id == id) {
            *row = current;
        }
        if !complete || entry.operation.has_owned_staging() {
            state.entries.insert(id, entry);
        }
        self.updates.send_replace(state.reports.clone());
        result
    }

    pub(crate) fn discard(&self, id: u64) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("conversion owner unavailable"))?;
        if let Some(entry) = state.entries.get_mut(&id) {
            entry.operation.cleanup()?;
        }
        anyhow::ensure!(
            !state
                .reports
                .iter()
                .any(|row| row.id == id && row.state == ConversionState::Running),
            "conversion is still running"
        );
        state.entries.remove(&id);
        state.reports.retain(|row| row.id != id);
        self.updates.send_replace(state.reports.clone());
        Ok(())
    }

    pub(crate) fn playlist_appended(&self, track_id: i64, playlist_id: i64) {
        if let Ok(mut state) = self.state.lock() {
            for entry in state.entries.values_mut() {
                if entry.operation.track_id() == track_id && entry.playlist == Some(playlist_id) {
                    entry.playlist_appended = true;
                }
            }
        }
    }

    pub(crate) fn close(&self) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("conversion owner unavailable"))?;
        state.closed = true;
        anyhow::ensure!(state.running.is_empty(), "conversion work has not drained");
        for entry in state.entries.values_mut() {
            entry.operation.cleanup()?;
        }
        state.entries.clear();
        state.reports.clear();
        self.updates.send_replace(Vec::new());
        Ok(())
    }
}

impl Drop for ConversionRecovery {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            eprintln!(
                "[{}] App could not release conversion input: {error:#}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
    }
}

fn request_key(request: &SubscribeTrackRequest) -> String {
    match request {
        SubscribeTrackRequest::LibraryTrack { track } => format!("library:{}", track.id),
        SubscribeTrackRequest::SearchTrack { track_context, .. } => format!(
            "index:{:?}:{:?}:{:?}",
            track_context.track.feed_url,
            track_context.track.track_guid,
            track_context.track.enclosure_url
        ),
    }
}

fn report(
    id: u64,
    operation: &Materialization,
    outcome: &Result<SubscribeTrackOutcome>,
) -> ConversionReport {
    let (state, mut message) = match outcome {
        Ok(outcome) => match outcome.conversion {
            ConversionOutcome::WavRetained => (ConversionState::WavRetained, format!("App kept usable WAV {} in the library. {} Open converter setup, check it, then retry this track.", outcome.path.display(), outcome.format_warning.as_deref().unwrap_or("Conversion did not complete."))),
            ConversionOutcome::Flac => (ConversionState::Completed, format!("FLAC converted the original track. App updated its existing library entry: {}.", outcome.path.display())),
            ConversionOutcome::FfmpegFallback => (ConversionState::Completed, format!("ffmpeg fallback converted the original track. App updated its existing library entry: {}.", outcome.path.display())),
            ConversionOutcome::NotRequired => (ConversionState::Completed, format!("App completed the original track: {}.", outcome.path.display())),
        },
        Err(error) => (ConversionState::Failed, format!("App could not finish materializing this track: {error:#}. Available input is retained for this session.")),
    };
    if let Ok(outcome) = outcome {
        if outcome.conversion != ConversionOutcome::WavRetained {
            if let Some(warning) = &outcome.format_warning {
                message.push(' ');
                message.push_str(warning);
            }
        }
    }
    ConversionReport {
        id,
        track_id: operation.track_id(),
        title: operation.title(),
        state,
        message: crate::diagnostics::redact_endpoint_details(&message),
        recorded_at: SystemTime::now(),
    }
}

fn update_report(state: &mut State, id: u64, status: ConversionState, message: String) {
    if let Some(row) = state.reports.iter_mut().find(|row| row.id == id) {
        row.state = status;
        row.message = message;
        row.recorded_at = SystemTime::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn fixture() -> (
        tempfile::TempDir,
        PathBuf,
        Arc<Mutex<Connection>>,
        SubscribeTrackRequest,
    ) {
        let temp = tempfile::tempdir().unwrap();
        let config = temp.path().join("config.toml");
        fs::write(
            &config,
            format!(
                "music_dir = {:?}\ndb_path = {:?}\nflac_path = false\n",
                temp.path(),
                temp.path().join("state.sqlite")
            ),
        )
        .unwrap();
        let db = Connection::open(temp.path().join("state.sqlite")).unwrap();
        crate::db::init_schema(&db).unwrap();
        crate::db::migrate_schema(&db).unwrap();
        db.execute(
            "INSERT INTO feeds (id, feed_url) VALUES (1, 'https://example.test/feed')",
            [],
        )
        .unwrap();
        db.execute("INSERT INTO tracks (id, feed_id, item_guid, track_title, enclosure_url, enclosure_type, is_in_library) VALUES (1, 1, 'original', 'Original', 'https://example.test/original.wav', 'audio/wav', 1)", []).unwrap();
        db.execute(
            "INSERT INTO local_files (path, track_id) VALUES ('original.wav', 1)",
            [],
        )
        .unwrap();
        crate::db::playlist_create(&db, "Original destination").unwrap();
        fs::write(temp.path().join("original.wav"), b"RIFF\x24\0\0\0WAVEfmt ").unwrap();
        let request = SubscribeTrackRequest::LibraryTrack {
            track: Box::new(crate::db::track_row_by_id(&db, 1).unwrap().unwrap()),
        };
        (temp, config, Arc::new(Mutex::new(db)), request)
    }

    fn working_converter(config: &Path, hold: bool) {
        let root = config.parent().unwrap();
        fs::write(
            root.join("sample.flac"),
            include_bytes!("../../docs/runbooks/fixtures/conversion.flac"),
        )
        .unwrap();
        let binary = root.join("flac");
        let wait = if hold {
            format!(
                "while [ ! -f '{}' ]; do /bin/sleep 0.01; done\n",
                root.join("release").display()
            )
        } else {
            String::new()
        };
        fs::write(&binary, format!("#!/bin/sh\n[ \"$1\" = --version ] && exit 0\n{wait}while [ \"$1\" != -o ]; do shift; done\nshift\n/bin/cp '{}' \"$1\"\n", root.join("sample.flac").display())).unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
        let document = fs::read_to_string(config)
            .unwrap()
            .replace("flac_path = false", &format!("flac_path = {:?}", binary));
        fs::write(config, document).unwrap();
    }

    #[test]
    fn adr_0066_conversion_single_flight_save_is_inert_and_playlist_append_is_not_repeated() {
        let (temp, config, conn, request) = fixture();
        let recovery = Arc::new(ConversionRecovery::default());
        recovery
            .subscribe_to_at(Arc::clone(&conn), request.clone(), Some(1), &config)
            .unwrap();
        let id = recovery.subscribe().borrow()[0].id;
        crate::db::playlist_append(&conn.lock().unwrap(), 1, 1).unwrap();
        recovery.playlist_appended(1, 1);
        // An intentional subsequent playlist edit is not replayed by conversion retry.
        conn.lock()
            .unwrap()
            .execute("DELETE FROM playlist_tracks", [])
            .unwrap();
        working_converter(&config, true);
        assert_eq!(
            recovery.subscribe().borrow()[0].state,
            ConversionState::WavRetained
        );
        assert!(!temp.path().join("original.flac").exists());
        let worker_recovery = Arc::clone(&recovery);
        let worker_conn = Arc::clone(&conn);
        let worker_config = config.clone();
        let worker = std::thread::spawn(move || {
            worker_recovery
                .retry(id, false, &worker_conn, &worker_config)
                .map(|result| result.conversion)
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while recovery.subscribe().borrow()[0].state != ConversionState::Running
            && std::time::Instant::now() < deadline
        {
            std::thread::yield_now();
        }
        assert!(recovery.retry(id, false, &conn, &config).is_err());
        assert!(recovery
            .subscribe_to_at(Arc::clone(&conn), request, None, &config)
            .is_err());
        fs::write(temp.path().join("release"), b"release").unwrap();
        assert_eq!(worker.join().unwrap().unwrap(), ConversionOutcome::Flac);
        assert!(recovery.retry(id, false, &conn, &config).is_err());
        assert_eq!(
            crate::db::library_tracks(&conn.lock().unwrap())
                .unwrap()
                .len(),
            1
        );
        assert!(crate::db::playlist_tracks(&conn.lock().unwrap(), 1)
            .unwrap()
            .is_empty());
        assert_eq!(
            recovery.subscribe().borrow()[0].state,
            ConversionState::Completed
        );
    }

    #[test]
    fn adr_0066_conversion_missing_input_requires_explicit_redownload_of_original_enclosure() {
        use std::io::{Read, Write};
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        let (temp, config, conn, _) = fixture();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/original.wav", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let count = Arc::clone(&requests);
        let ended = Arc::clone(&stop);
        let server = std::thread::spawn(move || {
            while !ended.load(Ordering::SeqCst) {
                if let Ok((mut stream, _)) = listener.accept() {
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                        .unwrap();
                    let mut buffer = [0; 2048];
                    let size = stream.read(&mut buffer).unwrap();
                    assert!(
                        String::from_utf8_lossy(&buffer[..size]).starts_with("GET /original.wav ")
                    );
                    count.fetch_add(1, Ordering::SeqCst);
                    let body = b"RIFF\x24\0\0\0WAVEfmt ";
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .unwrap();
                    stream.write_all(body).unwrap();
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        });
        conn.lock()
            .unwrap()
            .execute("UPDATE tracks SET enclosure_url=?1 WHERE id=1", [&url])
            .unwrap();
        let request = SubscribeTrackRequest::LibraryTrack {
            track: Box::new(
                crate::db::track_row_by_id(&conn.lock().unwrap(), 1)
                    .unwrap()
                    .unwrap(),
            ),
        };
        let recovery = ConversionRecovery::default();
        recovery
            .subscribe_to_at(Arc::clone(&conn), request, None, &config)
            .unwrap();
        let id = recovery.subscribe().borrow()[0].id;
        assert!(recovery.retry(id, true, &conn, &config).is_err());
        fs::rename(
            temp.path().join("original.wav"),
            temp.path().join("preserved.wav"),
        )
        .unwrap();
        assert!(recovery.retry(id, false, &conn, &config).is_err());
        assert_eq!(requests.load(Ordering::SeqCst), 0);
        assert_eq!(
            recovery.subscribe().borrow()[0].state,
            ConversionState::RedownloadRequired
        );
        working_converter(&config, false);
        assert_eq!(
            recovery.retry(id, true, &conn, &config).unwrap().conversion,
            ConversionOutcome::Flac
        );
        assert_eq!(requests.load(Ordering::SeqCst), 1);
        assert!(temp.path().join("preserved.wav").exists());
        stop.store(true, Ordering::SeqCst);
        server.join().unwrap();
    }

    #[test]
    fn adr_0066_conversion_discard_and_session_close_keep_existing_music() {
        let (temp, config, conn, request) = fixture();
        let recovery = ConversionRecovery::default();
        recovery
            .subscribe_to_at(Arc::clone(&conn), request.clone(), None, &config)
            .unwrap();
        let id = recovery.subscribe().borrow()[0].id;
        recovery.discard(id).unwrap();
        assert!(recovery.subscribe().borrow().is_empty());
        recovery
            .subscribe_to_at(Arc::clone(&conn), request.clone(), None, &config)
            .unwrap();
        recovery.close().unwrap();
        assert!(recovery
            .subscribe_to_at(conn, request, None, &config)
            .is_err());
        assert_eq!(
            fs::read(temp.path().join("original.wav")).unwrap(),
            b"RIFF\x24\0\0\0WAVEfmt "
        );
        assert!(fs::read_dir(temp.path().join(".v4vmm-staging"))
            .unwrap()
            .next()
            .is_none());
    }
}
