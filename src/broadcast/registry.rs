#![warn(clippy::pedantic)]

//! Broadcast event registry service for ADR 0059.
//!
//! This module owns the GPUI-free service boundary around relay event creation,
//! liveness checks, local rows, and token files. It calls the relay through
//! `api::Client`; it never sends metadata and never returns broadcaster token
//! text to callers.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;
use serde::Serialize;

use crate::broadcast::tokens;
use crate::{api, config, db};

type Clock = fn() -> Result<i64>;

/// Local registry for relay broadcast events.
pub struct BroadcastRegistry<'a> {
    conn: &'a Connection,
    endpoint: String,
    client: api::Client,
    token_directory: PathBuf,
    clock: Clock,
}

/// A created broadcast event without broadcaster token text.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CreatedBroadcastEvent {
    pub event: db::BroadcastEventRow,
    pub token_path: String,
}

/// The result of a definite broadcast event liveness check.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CheckedBroadcastEvent {
    pub event: db::BroadcastEventRow,
    pub status: db::BroadcastEventStatus,
}

/// A relay answer arrived, but storing or reading back that answer failed (ADR 0059).
#[derive(Debug)]
pub(crate) struct EventCheckSaveError {
    pub(crate) observed_status: db::BroadcastEventStatus,
    source: anyhow::Error,
}

impl std::fmt::Display for EventCheckSaveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "save relay answer: {:#}", self.source)
    }
}

impl std::error::Error for EventCheckSaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Forget a local registration and its token without requiring a remote endpoint.
///
/// # Errors
/// Reports database or token-file removal errors.
pub fn forget_local_event(
    conn: &Connection,
    event_id: &str,
) -> Result<Option<db::BroadcastEventRow>> {
    let Some(event) = db::broadcast_event_by_event_id(conn, event_id)? else {
        return Ok(None);
    };

    remove_token_file(Path::new(&event.token_path))?;
    anyhow::ensure!(
        db::delete_broadcast_event(conn, event.id)?,
        "broadcast event disappeared before delete: {}",
        event.event_id
    );
    Ok(Some(event))
}

impl<'a> BroadcastRegistry<'a> {
    /// Build a registry using the default token directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint or token directory cannot be resolved.
    pub fn new(conn: &'a Connection, endpoint: &str) -> Result<Self> {
        Self::with_token_directory(conn, endpoint, tokens::default_token_directory()?)
    }

    /// Build a registry using an explicit token directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint is not a valid `MusicIndex` endpoint.
    pub fn with_token_directory(
        conn: &'a Connection,
        endpoint: &str,
        token_directory: PathBuf,
    ) -> Result<Self> {
        Self::with_clock(conn, endpoint, token_directory, unix_timestamp)
    }

    /// Create a relay event and store its token path.
    ///
    /// The relay returns the broadcaster token one time only. This method writes
    /// that token to disk before it inserts the registry row, then returns only
    /// the row and token path.
    ///
    /// # Errors
    ///
    /// Returns an error if relay creation, token storage, or row insertion
    /// fails. If row insertion fails, the newly written token file is removed.
    pub fn create_event(&self, label: Option<&str>) -> Result<CreatedBroadcastEvent> {
        let response = self.client.create_live_item().context("create live item")?;
        if db::broadcast_event_by_event_id(self.conn, &response.event_id)?.is_some() {
            return Err(anyhow!(
                "broadcast event already exists: {}",
                response.event_id
            ));
        }

        let token_path =
            tokens::token_path_in_directory(&self.token_directory, &response.event_id)?;
        tokens::write_token_file(&token_path, &response.broadcaster_token)
            .context("write broadcast token file")?;

        let input = db::BroadcastEventInput {
            event_id: response.event_id,
            label: normalized_label(label),
            endpoint: self.endpoint.clone(),
            token_path: path_to_string(&token_path),
            created_at: (self.clock)()?,
            last_checked_at: None,
            last_status: Some(db::BroadcastEventStatus::Unknown),
        };
        let row_id = match db::insert_broadcast_event(self.conn, &input) {
            Ok(row_id) => row_id,
            Err(error) => {
                remove_token_after_failed_insert(&token_path)?;
                return Err(error).context("insert broadcast event");
            }
        };
        let event = db::broadcast_event_by_id(self.conn, row_id)?
            .context("created broadcast event disappeared")?;

        Ok(CreatedBroadcastEvent {
            event,
            token_path: path_to_string(&token_path),
        })
    }

    /// List stored broadcast events.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn list_events(&self) -> Result<Vec<db::BroadcastEventRow>> {
        db::broadcast_events(self.conn)
    }

    /// Forget one stored broadcast event.
    ///
    /// This removes the local row and token file. It sends no request to the
    /// relay, because the relay has no delete route.
    ///
    /// # Errors
    ///
    /// Returns an error if the event identifier is invalid, the token file
    /// cannot be removed, or the database delete fails.
    pub fn forget_event(&self, event_id: &str) -> Result<Option<db::BroadcastEventRow>> {
        forget_local_event(self.conn, event_id)
    }

    /// Check one stored broadcast event for relay liveness.
    ///
    /// A successful metadata read marks the event live. A metadata-read `404`
    /// marks it dead. Other transport errors leave the stored status unchanged.
    ///
    /// # Errors
    ///
    /// Returns an error if the event is not stored, if the relay read has a
    /// transport failure, or if the status update fails.
    pub fn check_event(&self, event_id: &str) -> Result<CheckedBroadcastEvent> {
        let event = self.require_event(event_id)?;
        let client = api::Client::new_with_base_url(event.endpoint.clone());
        let status = match client
            .fetch_live_metadata_optional(&event.event_id)
            .with_context(|| format!("check broadcast event {}", event.event_id))?
        {
            Some(_) => db::BroadcastEventStatus::Live,
            None => db::BroadcastEventStatus::Dead,
        };
        self.store_checked_event(&event, status).map_err(|source| {
            EventCheckSaveError {
                observed_status: status,
                source,
            }
            .into()
        })
    }

    fn store_checked_event(
        &self,
        event: &db::BroadcastEventRow,
        status: db::BroadcastEventStatus,
    ) -> Result<CheckedBroadcastEvent> {
        let checked_at = (self.clock)()?;
        anyhow::ensure!(
            db::update_broadcast_event_status(self.conn, event.id, status, Some(checked_at))?,
            "broadcast event disappeared before status update: {}",
            event.event_id
        );
        let event = db::broadcast_event_by_id(self.conn, event.id)?
            .context("checked broadcast event disappeared")?;
        Ok(CheckedBroadcastEvent { event, status })
    }

    fn require_event(&self, event_id: &str) -> Result<db::BroadcastEventRow> {
        db::broadcast_event_by_event_id(self.conn, event_id)?
            .with_context(|| format!("broadcast event not found: {event_id}"))
    }

    fn with_clock(
        conn: &'a Connection,
        endpoint: &str,
        token_directory: PathBuf,
        clock: Clock,
    ) -> Result<Self> {
        let endpoint = config::normalize_musicindex_endpoint(endpoint)?;
        Ok(Self {
            conn,
            endpoint: endpoint.clone(),
            client: api::Client::new_with_base_url(endpoint),
            token_directory,
            clock,
        })
    }
}

fn normalized_label(label: Option<&str>) -> Option<String> {
    label
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .map(str::to_owned)
}

fn path_to_string(path: &Path) -> String {
    path.display().to_string()
}

fn remove_token_after_failed_insert(path: &Path) -> Result<()> {
    remove_token_file(path).with_context(|| {
        format!(
            "remove broadcast token file after failed row insert: {}",
            path.display()
        )
    })
}

fn remove_token_file(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("remove broadcast token file {}", path.display()))
        }
    }
}

fn unix_timestamp() -> Result<i64> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("read system time for broadcast event")?
        .as_secs();
    i64::try_from(seconds).context("convert broadcast event timestamp")
}

#[cfg(test)]
mod tests {
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;

    use anyhow::Result;

    use super::*;

    const FIXED_TIME: i64 = 1_778_284_900;

    fn fixed_time() -> Result<i64> {
        Ok(FIXED_TIME)
    }

    fn test_db(temp: &tempfile::TempDir) -> Result<Connection> {
        db::open_db(&temp.path().join("v4vmm.sqlite"))
    }

    fn registry<'a>(
        conn: &'a Connection,
        endpoint: &str,
        temp: &tempfile::TempDir,
    ) -> Result<BroadcastRegistry<'a>> {
        BroadcastRegistry::with_clock(conn, endpoint, temp.path().join("tokens"), fixed_time)
    }

    fn serve_once(status: &str, body: impl Into<String>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        let status = status.to_owned();
        let body = body.into();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            let headers = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(headers.as_bytes()).expect("write headers");
            stream.write_all(body.as_bytes()).expect("write body");
        });
        format!("http://{addr}")
    }

    fn serve_disconnect_once() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
        });
        format!("http://{addr}")
    }

    fn create_response(event_id: &str, token: &str) -> String {
        serde_json::json!({
            "event_id": event_id,
            "broadcaster_token": token,
            "metadata_url": format!("https://relay.example.test/v1/liveitems/{event_id}/metadata"),
            "events_url": "https://relay.example.test/events",
        })
        .to_string()
    }

    fn insert_event(
        conn: &Connection,
        event_id: &str,
        endpoint: &str,
        temp: &tempfile::TempDir,
        status: db::BroadcastEventStatus,
        checked_at: Option<i64>,
    ) -> Result<i64> {
        let token_path = temp.path().join(format!("{event_id}.token"));
        tokens::write_token_file(&token_path, "stored-token")?;
        db::insert_broadcast_event(
            conn,
            &db::BroadcastEventInput {
                event_id: event_id.to_owned(),
                label: Some("Sunday set".to_owned()),
                endpoint: endpoint.to_owned(),
                token_path: path_to_string(&token_path),
                created_at: FIXED_TIME - 1,
                last_checked_at: checked_at,
                last_status: Some(status),
            },
        )
    }

    #[cfg(unix)]
    fn file_mode(path: &Path) -> Result<u32> {
        use std::os::unix::fs::PermissionsExt as _;

        Ok(fs::metadata(path)
            .with_context(|| format!("read mode for {}", path.display()))?
            .permissions()
            .mode()
            & 0o777)
    }

    #[test]
    fn create_event_writes_token_file_and_registry_row_without_returning_token_text() -> Result<()>
    {
        let temp = tempfile::tempdir()?;
        let conn = test_db(&temp)?;
        let body = create_response("event-one", "secret-token");
        let endpoint = serve_once("200 OK", body);
        let registry = registry(&conn, &endpoint, &temp)?;

        let created = registry.create_event(Some(" Sunday set "))?;

        assert_eq!(created.event.event_id, "event-one");
        assert_eq!(created.event.label.as_deref(), Some("Sunday set"));
        assert_eq!(created.event.endpoint, endpoint);
        assert_eq!(created.event.created_at, FIXED_TIME);
        assert_eq!(
            created.event.last_status,
            Some(db::BroadcastEventStatus::Unknown)
        );
        assert_eq!(
            tokens::read_token_file(Path::new(&created.token_path))?,
            "secret-token"
        );
        let json = serde_json::to_string(&created)?;
        assert!(
            !json.contains("secret-token"),
            "created event JSON must not expose token text"
        );
        #[cfg(unix)]
        assert_eq!(
            file_mode(Path::new(&created.token_path))?,
            0o600,
            "created token file should be owner-readable only"
        );
        assert_eq!(
            registry.list_events()?,
            vec![created.event],
            "created event should be listable"
        );
        Ok(())
    }

    #[test]
    fn create_event_cleans_up_token_file_when_insert_fails() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let conn = test_db(&temp)?;
        conn.execute_batch(
            "CREATE TRIGGER fail_broadcast_event_insert
             BEFORE INSERT ON broadcast_events
             BEGIN
                 SELECT RAISE(FAIL, 'forced insert failure');
             END;",
        )?;
        let body = create_response("event-insert-fails", "secret-token");
        let endpoint = serve_once("200 OK", body);
        let registry = registry(&conn, &endpoint, &temp)?;

        let error = registry
            .create_event(None)
            .expect_err("forced database insert failure should bubble up");

        assert!(
            error.to_string().contains("insert broadcast event"),
            "error should name the failed row insert: {error}"
        );
        let token_path =
            tokens::token_path_in_directory(&temp.path().join("tokens"), "event-insert-fails")?;
        assert!(
            !token_path.exists(),
            "failed insert should remove the newly written token file"
        );
        Ok(())
    }

    #[test]
    fn create_event_does_not_replace_existing_token_file_for_duplicate_event_id() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let conn = test_db(&temp)?;
        let token_path = tokens::token_path_in_directory(&temp.path().join("tokens"), "event-one")?;
        tokens::write_token_file(&token_path, "original-token")?;
        db::insert_broadcast_event(
            &conn,
            &db::BroadcastEventInput {
                event_id: "event-one".to_owned(),
                label: None,
                endpoint: "http://127.0.0.1:9".to_owned(),
                token_path: path_to_string(&token_path),
                created_at: FIXED_TIME,
                last_checked_at: None,
                last_status: Some(db::BroadcastEventStatus::Dead),
            },
        )?;
        let body = create_response("event-one", "replacement-token");
        let endpoint = serve_once("200 OK", body);
        let registry = registry(&conn, &endpoint, &temp)?;

        let error = registry
            .create_event(None)
            .expect_err("duplicate event id should not replace a stored token");

        assert!(
            error.to_string().contains("already exists"),
            "duplicate error should explain the local event conflict: {error}"
        );
        assert_eq!(
            tokens::read_token_file(&token_path)?,
            "original-token",
            "existing token file must not be overwritten for a duplicate event"
        );
        Ok(())
    }

    #[test]
    fn check_event_maps_live_metadata_response_to_live_status() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let conn = test_db(&temp)?;
        let endpoint = serve_once(
            "200 OK",
            r#"{"event_id":"event-one","seq":1,"updated_at":"2026-09-07T00:00:00Z","metadata":{}}"#,
        );
        let id = insert_event(
            &conn,
            "event-one",
            &endpoint,
            &temp,
            db::BroadcastEventStatus::Unknown,
            None,
        )?;
        let registry = registry(&conn, &endpoint, &temp)?;

        let checked = registry.check_event("event-one")?;

        assert_eq!(checked.status, db::BroadcastEventStatus::Live);
        assert_eq!(
            checked.event.last_status,
            Some(db::BroadcastEventStatus::Live)
        );
        assert_eq!(checked.event.last_checked_at, Some(FIXED_TIME));
        assert_eq!(
            db::broadcast_event_by_id(&conn, id)?
                .context("event should still exist")?
                .last_status,
            Some(db::BroadcastEventStatus::Live)
        );
        Ok(())
    }

    #[test]
    fn check_event_maps_any_metadata_404_to_dead_status() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let conn = test_db(&temp)?;
        let endpoint = serve_once("404 Not Found", r#"{"error":"liveitem_not_found"}"#);
        insert_event(
            &conn,
            "event-one",
            &endpoint,
            &temp,
            db::BroadcastEventStatus::Live,
            Some(1),
        )?;
        let registry = registry(&conn, &endpoint, &temp)?;

        let checked = registry.check_event("event-one")?;

        assert_eq!(checked.status, db::BroadcastEventStatus::Dead);
        assert_eq!(
            checked.event.last_status,
            Some(db::BroadcastEventStatus::Dead)
        );
        assert_eq!(checked.event.last_checked_at, Some(FIXED_TIME));
        Ok(())
    }

    #[test]
    fn check_event_leaves_status_unchanged_on_transport_failure() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let conn = test_db(&temp)?;
        let endpoint = serve_disconnect_once();
        let id = insert_event(
            &conn,
            "event-one",
            &endpoint,
            &temp,
            db::BroadcastEventStatus::Live,
            Some(123),
        )?;
        let registry = registry(&conn, &endpoint, &temp)?;

        let error = registry
            .check_event("event-one")
            .expect_err("transport failure should return an error");

        let response = error
            .downcast_ref::<crate::api::LiveMetadataReadError>()
            .expect("ADR 0059: retain response facts through registry context");
        assert_eq!(response.response_status, None);

        assert!(
            error
                .to_string()
                .contains("check broadcast event event-one"),
            "error should name the liveness check without token text: {error}"
        );
        let row = db::broadcast_event_by_id(&conn, id)?.context("event should still exist")?;
        assert_eq!(row.last_status, Some(db::BroadcastEventStatus::Live));
        assert_eq!(row.last_checked_at, Some(123));
        Ok(())
    }

    /// Situational ADR 0059: rejected and unreadable HTTP responses remain distinct from no response.
    #[test]
    fn check_event_retains_http_status_without_changing_saved_facts() -> Result<()> {
        for (status, expected) in [("503 Service Unavailable", 503), ("200 OK", 200)] {
            let temp = tempfile::tempdir()?;
            let conn = test_db(&temp)?;
            let endpoint = serve_once(status, "not metadata JSON");
            let id = insert_event(
                &conn,
                "event-one",
                &endpoint,
                &temp,
                db::BroadcastEventStatus::Live,
                Some(123),
            )?;
            let registry = registry(&conn, &endpoint, &temp)?;
            let error = registry
                .check_event("event-one")
                .expect_err("metadata request cannot answer");
            let response = error
                .downcast_ref::<crate::api::LiveMetadataReadError>()
                .expect("retain HTTP status through registry context");
            assert_eq!(response.response_status, Some(expected));
            let row = db::broadcast_event_by_id(&conn, id)?.context("stored event")?;
            assert_eq!(row.last_status, Some(db::BroadcastEventStatus::Live));
            assert_eq!(row.last_checked_at, Some(123));
        }
        Ok(())
    }

    #[test]
    fn forget_event_removes_row_and_token_file_without_relay_call() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let conn = test_db(&temp)?;
        let endpoint = "http://127.0.0.1:9";
        let token_path = temp.path().join("event-one.token");
        tokens::write_token_file(&token_path, "stored-token")?;
        db::insert_broadcast_event(
            &conn,
            &db::BroadcastEventInput {
                event_id: "event-one".to_owned(),
                label: None,
                endpoint: endpoint.to_owned(),
                token_path: path_to_string(&token_path),
                created_at: FIXED_TIME,
                last_checked_at: None,
                last_status: Some(db::BroadcastEventStatus::Unknown),
            },
        )?;
        let forgotten =
            forget_local_event(&conn, "event-one")?.context("event should be forgotten")?;

        assert_eq!(forgotten.event_id, "event-one");
        assert!(
            db::broadcast_event_by_event_id(&conn, "event-one")?.is_none(),
            "forgotten event should no longer be in the registry"
        );
        assert!(
            !token_path.exists(),
            "forget should remove the local token file"
        );
        assert!(
            forget_local_event(&conn, "missing")?.is_none(),
            "forget should report missing events without relay calls"
        );
        Ok(())
    }
}
