//! Configured SQLite verification and explicit preparation (ADRs 0016, 0066).

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rusqlite::{Connection, Error, ErrorCode, OpenFlags};

/// A finite wait for another process to release the configured database.
pub const STARTUP_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
static PROBE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DbStage {
    Inspect,
    Open,
    Configure,
    Schema,
    Read,
    WriteProbe,
    Rollback,
    Initialize,
    Migrate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DbCheckError {
    pub stage: DbStage,
    pub reason: &'static str,
}

impl DbCheckError {
    fn sqlite(stage: DbStage, error: &Error) -> Self {
        let reason = match error.sqlite_error_code() {
            Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => {
                "Another process kept the database locked beyond the allowed wait."
            }
            Some(ErrorCode::ReadOnly) => "SQLite could not write to this database.",
            Some(ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase) => {
                "SQLite could not read this file as a valid database."
            }
            Some(ErrorCode::DiskFull) => "The database storage is full.",
            Some(ErrorCode::CannotOpen | ErrorCode::PermissionDenied) => {
                "SQLite could not access this database with the required permissions."
            }
            Some(ErrorCode::SystemIoFailure) => "SQLite reported a storage input/output failure.",
            _ => {
                "SQLite could not complete this operation. Check the database file and its schema."
            }
        };
        Self { stage, reason }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseReadiness {
    Ready,
    NeedsPreparation,
}

fn open(path: &Path, create: bool, wait: Duration) -> Result<Connection, DbCheckError> {
    let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if create {
        flags |= OpenFlags::SQLITE_OPEN_CREATE;
    }
    let conn = Connection::open_with_flags(path, flags)
        .map_err(|e| DbCheckError::sqlite(DbStage::Open, &e))?;
    conn.busy_timeout(wait)
        .map_err(|e| DbCheckError::sqlite(DbStage::Configure, &e))?;
    conn.pragma_update(None, "foreign_keys", true)
        .map_err(|e| DbCheckError::sqlite(DbStage::Configure, &e))?;
    Ok(conn)
}

/// Check existing main-database storage. This never initializes or migrates it.
pub fn check_database(path: &Path) -> Result<DatabaseReadiness, DbCheckError> {
    check_with_wait(path, STARTUP_BUSY_TIMEOUT)
}

fn check_with_wait(path: &Path, wait: Duration) -> Result<DatabaseReadiness, DbCheckError> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DatabaseReadiness::NeedsPreparation)
        }
        Err(_) => {
            return Err(DbCheckError {
                stage: DbStage::Inspect,
                reason: "App could not inspect the database path.",
            })
        }
        Ok(_) => {}
    }
    let mut conn = open(path, false, wait)?;
    let state = check_schema(&conn)?;
    probe_main_database(&mut conn)?;
    Ok(state)
}

/// Prepare a new/recognized database through the normal migration authority.
pub fn prepare_database(path: &Path) -> Result<Connection, DbCheckError> {
    let absent = match fs::symlink_metadata(path) {
        Ok(_) => false,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(_) => {
            return Err(DbCheckError {
                stage: DbStage::Inspect,
                reason: "App could not inspect the database path.",
            })
        }
    };
    if absent {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|_| DbCheckError {
            stage: DbStage::Open,
            reason: "App could not create the new database's parent directory.",
        })?;
    }
    let mut conn = open(path, absent, STARTUP_BUSY_TIMEOUT)?;
    check_schema(&conn)?;
    super::init_schema(&conn).map_err(|_| DbCheckError { stage: DbStage::Initialize, reason: "App could not prepare the database tables. Earlier changes may remain; no reset was performed." })?;
    super::migrate_schema(&conn).map_err(|_| DbCheckError { stage: DbStage::Migrate, reason: "App could not finish the database migrations. Earlier changes may remain; preserve the file before repair." })?;
    check_schema(&conn)?;
    probe_main_database(&mut conn)?;
    Ok(conn)
}

fn check_schema(conn: &Connection) -> Result<DatabaseReadiness, DbCheckError> {
    let tables = conn
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'")
        .and_then(|mut s| {
            s.query_map([], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|e| DbCheckError::sqlite(DbStage::Schema, &e))?;
    if tables.is_empty() {
        return Ok(DatabaseReadiness::NeedsPreparation);
    }
    // These queries describe read compatibility, not a second schema mutation registry.
    for sql in BASE_READS {
        conn.prepare(sql).map_err(|_| DbCheckError {
            stage: DbStage::Schema,
            reason: "The database does not have the expected library tables and columns.",
        })?;
    }
    if !tables.iter().any(|t| t == "schema_migrations") {
        return Ok(DatabaseReadiness::NeedsPreparation);
    }
    let versions = conn
        .prepare("SELECT version, name FROM schema_migrations ORDER BY version")
        .and_then(|mut s| {
            s.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|e| DbCheckError::sqlite(DbStage::Schema, &e))?;
    if versions.len() > super::MIGRATIONS.len()
        || versions
            .iter()
            .zip(super::MIGRATIONS)
            .any(|((version, name), expected)| {
                *version != expected.version || name != expected.name
            })
    {
        return Err(DbCheckError { stage: DbStage::Schema, reason: "The migration record has an unsupported version, name, or gap. App did not change it." });
    }
    if versions.len() < super::MIGRATIONS.len() {
        return Ok(DatabaseReadiness::NeedsPreparation);
    }
    for (table, columns) in CURRENT_COLUMNS {
        let sql = format!("SELECT * FROM main.{table} LIMIT 0");
        let statement = conn.prepare(&sql).map_err(|_| DbCheckError {
            stage: DbStage::Schema,
            reason: "The recorded database version does not match its tables.",
        })?;
        if !columns
            .iter()
            .all(|column| statement.column_names().contains(column))
        {
            return Err(DbCheckError {
                stage: DbStage::Schema,
                reason: "The recorded database version does not match its columns.",
            });
        }
    }
    conn.query_row("SELECT count(*) FROM local_files", [], |r| {
        r.get::<_, i64>(0)
    })
    .map_err(|e| DbCheckError::sqlite(DbStage::Read, &e))?;
    Ok(DatabaseReadiness::Ready)
}

const BASE_READS: &[&str] = &[
    "SELECT id, feed_url, feed_guid, title, extra_json FROM feeds LIMIT 0",
    "SELECT id, feed_id, item_guid, track_title, is_in_library FROM tracks LIMIT 0",
    "SELECT id, path, track_id, file_size_bytes FROM local_files LIMIT 0",
    "SELECT id, name, description FROM playlists LIMIT 0",
    "SELECT playlist_id, track_id, position FROM playlist_tracks LIMIT 0",
    "SELECT session_id, local_track_id, sequence, state, position_ms FROM playback_sessions LIMIT 0",
];
const CURRENT_COLUMNS: &[(&str, &[&str])] = &[
    ("schema_migrations", &["version", "name", "applied_at"]),
    ("schema_version", &["version"]),
    (
        "feeds",
        &[
            "id",
            "feed_url",
            "feed_guid",
            "title",
            "link",
            "language",
            "description",
            "podcast_medium",
            "album_image_href",
            "album_image_mime",
            "people_json",
            "podcast_value_json",
            "is_subscribed",
            "last_fetched_at",
            "extra_json",
            "musicindex_updated_at",
        ],
    ),
    (
        "tracks",
        &[
            "id",
            "feed_id",
            "item_guid",
            "enclosure_url",
            "enclosure_type",
            "link",
            "pub_date",
            "track_title",
            "artist_name",
            "album_title",
            "album_artist_name",
            "disc_number",
            "track_number",
            "duration_seconds",
            "itunes_duration_raw",
            "itunes_explicit",
            "track_image_href",
            "track_image_mime",
            "people_json",
            "item_value_json",
            "is_in_library",
            "extra_json",
        ],
    ),
    (
        "local_files",
        &[
            "id",
            "path",
            "track_id",
            "added_at",
            "file_size_bytes",
            "audio_duration_sec",
            "checksum",
            "extra_json",
        ],
    ),
    (
        "playlists",
        &["id", "name", "description", "created_at", "updated_at"],
    ),
    ("playlist_tracks", &["playlist_id", "track_id", "position"]),
    (
        "playback_sessions",
        &[
            "session_id",
            "sequence",
            "local_track_id",
            "playlist_id",
            "playlist_position",
            "started_at",
            "position_ms",
            "state",
            "updated_at",
        ],
    ),
    (
        "entity_identity_links",
        &[
            "id",
            "owner_kind",
            "feed_id",
            "track_id",
            "contributor_position",
            "entity_type",
            "entity_id",
            "position",
            "link_type",
            "url",
            "source",
            "extraction_path",
            "observed_at",
            "raw_json",
            "updated_at",
        ],
    ),
    (
        "entity_identity_ids",
        &[
            "id",
            "owner_kind",
            "feed_id",
            "track_id",
            "contributor_position",
            "entity_type",
            "entity_id",
            "position",
            "scheme",
            "value",
            "source",
            "extraction_path",
            "observed_at",
            "raw_json",
            "updated_at",
        ],
    ),
    (
        "entity_contributors",
        &[
            "id",
            "owner_kind",
            "feed_id",
            "track_id",
            "position",
            "name",
            "role",
            "group_name",
            "href",
            "image_url",
            "nostr_npub",
            "source",
            "raw_json",
            "observed_at",
            "updated_at",
        ],
    ),
    (
        "entity_metadata_facts",
        &[
            "id",
            "owner_kind",
            "feed_id",
            "track_id",
            "fact_key",
            "value_text",
            "value_integer",
            "value_boolean",
            "source",
            "extraction_path",
            "observed_at",
            "raw_json",
            "updated_at",
        ],
    ),
    (
        "artist_source_facts",
        &[
            "id",
            "source",
            "source_artist_id",
            "name",
            "sort_name",
            "image_url",
            "website_url",
            "aliases_json",
            "tags_json",
            "area",
            "begin_year",
            "end_year",
            "observed_at",
            "raw_json",
            "updated_at",
        ],
    ),
    (
        "artist_source_links",
        &[
            "id",
            "artist_source_fact_id",
            "entity_type",
            "entity_id",
            "position",
            "link_type",
            "url",
            "extraction_path",
            "observed_at",
            "raw_json",
            "updated_at",
        ],
    ),
    (
        "artist_source_ids",
        &[
            "id",
            "artist_source_fact_id",
            "entity_type",
            "entity_id",
            "position",
            "scheme",
            "value",
            "extraction_path",
            "observed_at",
            "raw_json",
            "updated_at",
        ],
    ),
    (
        "track_artist_source_bindings",
        &[
            "id",
            "track_id",
            "role",
            "source",
            "source_artist_id",
            "confidence",
            "provenance",
            "observed_at",
            "updated_at",
        ],
    ),
    (
        "broadcast_events",
        &[
            "id",
            "event_id",
            "label",
            "endpoint",
            "token_path",
            "created_at",
            "last_checked_at",
            "last_status",
        ],
    ),
    (
        "broadcast_event_selection",
        &["singleton", "event_id", "revision"],
    ),
    (
        "local_path_repairs",
        &["id", "track_id", "old_path", "reason", "recorded_at"],
    ),
];

fn probe_main_database(conn: &mut Connection) -> Result<(), DbCheckError> {
    probe_with(conn, |conn, name| {
        conn.execute_batch(&format!("CREATE TABLE main.{name}(value TEXT NOT NULL); INSERT INTO main.{name} VALUES ('v4vmm startup probe');"))?;
        let value: String =
            conn.query_row(&format!("SELECT value FROM main.{name}"), [], |r| r.get(0))?;
        if value != "v4vmm startup probe" {
            return Err(Error::InvalidQuery);
        }
        conn.execute_batch(&format!("DROP TABLE main.{name}"))
    })
}

fn probe_with(
    conn: &mut Connection,
    work: impl FnOnce(&Connection, &str) -> rusqlite::Result<()>,
) -> Result<(), DbCheckError> {
    let name = format!(
        "v4vmm_startup_probe_{}_{}",
        std::process::id(),
        PROBE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    let mut transaction = conn
        .transaction()
        .map_err(|e| DbCheckError::sqlite(DbStage::WriteProbe, &e))?;
    let mut savepoint = transaction
        .savepoint()
        .map_err(|e| DbCheckError::sqlite(DbStage::WriteProbe, &e))?;
    let result = work(&savepoint, &name).map_err(|e| DbCheckError::sqlite(DbStage::WriteProbe, &e));
    savepoint
        .rollback()
        .map_err(|e| DbCheckError::sqlite(DbStage::Rollback, &e))?;
    savepoint
        .finish()
        .map_err(|e| DbCheckError::sqlite(DbStage::Rollback, &e))?;
    transaction
        .rollback()
        .map_err(|e| DbCheckError::sqlite(DbStage::Rollback, &e))?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn adr_0066_read_only_configured_database_rejects_the_write_probe() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("readonly.sqlite");
        drop(prepare_database(&path).unwrap());
        let before = fs::read(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        let result = check_database(&path);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        if let Err(error) = result {
            assert!(matches!(error.stage, DbStage::WriteProbe | DbStage::Open));
        }
        // Root may bypass mode bits. The genuine competing-writer test and
        // injected query_only check still prove that a main write is required.
        assert!(fs::read(path).unwrap() == before);
    }

    #[test]
    fn adr_0066_schema_read_contract_covers_the_current_registry() {
        let conn = Connection::open_in_memory().unwrap();
        super::super::init_schema(&conn).unwrap();
        super::super::migrate_schema(&conn).unwrap();
        let count: i64 = conn.query_row("SELECT count(*) FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'", [], |r| r.get(0)).unwrap();
        assert_eq!(CURRENT_COLUMNS.len(), usize::try_from(count).unwrap());
        for (table, columns) in CURRENT_COLUMNS {
            let statement = conn.prepare(&format!("SELECT * FROM {table}")).unwrap();
            assert_eq!(
                statement.column_names(),
                *columns,
                "schema contract for {table}"
            );
        }
    }

    #[test]
    fn adr_0066_database_checks_leave_schema_rows_and_file_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("library.sqlite");
        let conn = prepare_database(&path).unwrap();
        conn.execute("INSERT INTO playlists(name) VALUES ('keep')", [])
            .unwrap();
        drop(conn);
        let before = fs::read(&path).unwrap();
        assert_eq!(check_database(&path), Ok(DatabaseReadiness::Ready));
        assert!(
            fs::read(&path).unwrap() == before,
            "database bytes changed during check"
        );
        let conn = Connection::open(&path).unwrap();
        assert_eq!(
            conn.query_row("SELECT count(*) FROM playlists", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name LIKE 'v4vmm_startup_probe_%'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn adr_0066_missing_database_check_does_not_create_or_upgrade() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("not-created/library.sqlite");
        assert_eq!(
            check_database(&path),
            Ok(DatabaseReadiness::NeedsPreparation)
        );
        assert!(!path.parent().unwrap().exists());
        let conn = prepare_database(&path).unwrap();
        conn.execute("DELETE FROM schema_migrations WHERE version=11", [])
            .unwrap();
        conn.execute("DROP TABLE broadcast_event_selection", [])
            .unwrap();
        drop(conn);
        let before = fs::read(&path).unwrap();
        assert_eq!(
            check_database(&path),
            Ok(DatabaseReadiness::NeedsPreparation)
        );
        assert!(
            fs::read(&path).unwrap() == before,
            "database bytes changed during check"
        );
        drop(prepare_database(&path).unwrap());
        assert_eq!(check_database(&path), Ok(DatabaseReadiness::Ready));
    }

    #[test]
    fn adr_0066_corrupt_wrong_schema_and_migration_gaps_are_named() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("broken.sqlite");
        fs::write(&path, b"not sqlite").unwrap();
        assert_eq!(check_database(&path).unwrap_err().stage, DbStage::Schema);
        assert_eq!(fs::read(&path).unwrap(), b"not sqlite");
        fs::remove_file(&path).unwrap();
        let conn = prepare_database(&path).unwrap();
        conn.execute("DELETE FROM schema_migrations WHERE version=3", [])
            .unwrap();
        assert_eq!(check_database(&path).unwrap_err().stage, DbStage::Schema);
        conn.execute("DROP TABLE feeds", []).unwrap();
        assert_eq!(check_database(&path).unwrap_err().stage, DbStage::Schema);
    }

    #[test]
    fn adr_0066_main_database_lock_is_bounded_and_reported() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("locked.sqlite");
        let owner = prepare_database(&path).unwrap();
        owner.execute_batch("BEGIN IMMEDIATE").unwrap();
        let start = std::time::Instant::now();
        let error = check_with_wait(&path, Duration::from_millis(25)).unwrap_err();
        assert_eq!(error.stage, DbStage::WriteProbe);
        assert!(error.reason.contains("locked"));
        assert!(start.elapsed() < Duration::from_secs(2));
        assert_eq!(STARTUP_BUSY_TIMEOUT, Duration::from_secs(5));
        owner.execute_batch("ROLLBACK").unwrap();
        assert_eq!(check_database(&path), Ok(DatabaseReadiness::Ready));
    }

    #[test]
    fn adr_0066_failed_probe_rolls_back_its_main_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        let error = probe_with(&mut conn, |conn, name| {
            conn.execute_batch(&format!(
                "CREATE TABLE main.{name}(value); INSERT INTO main.{name} VALUES ('partial');"
            ))?;
            Err(Error::InvalidQuery)
        })
        .unwrap_err();
        assert_eq!(error.stage, DbStage::WriteProbe);
        assert_eq!(
            conn.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(conn.is_autocommit());
        conn.pragma_update(None, "query_only", true).unwrap();
        assert_eq!(
            probe_main_database(&mut conn).unwrap_err().stage,
            DbStage::WriteProbe
        );
    }
}
