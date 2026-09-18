//! Non-mutating inspection and bounded, verified `SQLite` snapshots (ADRs 0016, 0066).

#![warn(clippy::pedantic)]

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use rusqlite::{
    backup::{Backup, StepResult},
    Connection, ErrorCode, OpenFlags,
};

use super::SchemaCompatibility;

mod preservation;
pub(crate) use preservation::{ExclusiveDatabase, Preservation};
pub(crate) mod restore;

pub(crate) const MAINTENANCE_DEADLINE: Duration = Duration::from_mins(1);
const LOCK_WAIT: Duration = Duration::from_millis(50);
const BACKUP_PAGE_BATCH: i32 = 64;
static CANDIDATE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FailureKind {
    Missing,
    Access,
    InvalidFile,
    Busy,
    Corrupt,
    Io,
    Cancelled,
    Deadline,
    Destination,
    Validation,
    Sql,
    Unsupported,
    UnstableFiles,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    pub(crate) operation: &'static str,
    pub(crate) kind: FailureKind,
    pub(crate) remaining: Vec<PathBuf>,
}

impl Failure {
    fn new(operation: &'static str, kind: FailureKind) -> Self {
        Self {
            operation,
            kind,
            remaining: Vec::new(),
        }
    }
    fn io(operation: &'static str, error: &std::io::Error) -> Self {
        let kind = match error.kind() {
            std::io::ErrorKind::NotFound => FailureKind::Missing,
            std::io::ErrorKind::PermissionDenied => FailureKind::Access,
            std::io::ErrorKind::AlreadyExists => FailureKind::Destination,
            _ => FailureKind::Io,
        };
        Self::new(operation, kind)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Budget {
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
}
impl Budget {
    pub(crate) fn new(cancelled: Arc<AtomicBool>) -> Self {
        Self {
            deadline: Instant::now() + MAINTENANCE_DEADLINE,
            cancelled,
        }
    }
    fn check(&self, operation: &'static str) -> Result<(), Failure> {
        if self.cancelled.load(Ordering::Acquire) {
            Err(Failure::new(operation, FailureKind::Cancelled))
        } else if Instant::now() >= self.deadline {
            Err(Failure::new(operation, FailureKind::Deadline))
        } else {
            Ok(())
        }
    }
    fn sql(&self, operation: &'static str, error: &rusqlite::Error) -> Failure {
        if let Err(failure) = self.check(operation) {
            return failure;
        }
        let kind = match error.sqlite_error_code() {
            Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => FailureKind::Busy,
            Some(ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase) => FailureKind::Corrupt,
            Some(ErrorCode::ReadOnly | ErrorCode::PermissionDenied | ErrorCode::CannotOpen) => {
                FailureKind::Access
            }
            Some(ErrorCode::DiskFull | ErrorCode::SystemIoFailure) => FailureKind::Io,
            _ => FailureKind::Sql,
        };
        Failure::new(operation, kind)
    }
    fn configure(&self, conn: &Connection) -> Result<(), Failure> {
        conn.busy_timeout(LOCK_WAIT)
            .map_err(|e| self.sql("Set database lock wait", &e))?;
        let budget = self.clone();
        conn.progress_handler(
            1000,
            Some(move || budget.check("Check database deadline").is_err()),
        )
        .map_err(|e| self.sql("Set database check deadline", &e))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Integrity {
    Ok,
    Errors(u64),
}

#[derive(Clone, Debug)]
pub(crate) struct Inspection {
    pub(crate) checks_constraints: bool,
    pub(crate) access: Result<(), Failure>,
    pub(crate) integrity: Option<Result<Integrity, Failure>>,
    pub(crate) foreign_keys: Option<Result<u64, Failure>>,
    pub(crate) schema: Option<Result<SchemaCompatibility, Failure>>,
}
impl Inspection {
    pub(crate) fn valid_snapshot(&self) -> bool {
        self.access.is_ok()
            && self.integrity == Some(Ok(Integrity::Ok))
            && self.foreign_keys == Some(Ok(0))
            && matches!(
                self.schema,
                Some(Ok(SchemaCompatibility::Current
                    | SchemaCompatibility::UpgradeRequired { .. }
                    | SchemaCompatibility::InterruptedUpgrade))
            )
    }
}

fn open_source(path: &Path, budget: &Budget) -> Result<Connection, Failure> {
    const OP: &str = "Open existing database for reading";
    budget.check(OP)?;
    let metadata = fs::metadata(path).map_err(|e| Failure::io(OP, &e))?;
    if !metadata.is_file() {
        return Err(Failure::new(OP, FailureKind::InvalidFile));
    }
    // No CREATE and no URI interpretation: an absent path never becomes a database.
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| budget.sql(OP, &e))?;
    budget.configure(&conn)?;
    Ok(conn)
}

pub(crate) fn inspect(path: &Path, budget: &Budget) -> Inspection {
    match open_source(path, budget) {
        Ok(conn) => inspect_connection(&conn, budget, false),
        Err(error) => Inspection {
            checks_constraints: false,
            access: Err(error),
            integrity: None,
            foreign_keys: None,
            schema: None,
        },
    }
}

fn inspect_connection(conn: &Connection, budget: &Budget, checks_constraints: bool) -> Inspection {
    let read = budget.check("Read database schema").and_then(|()| {
        conn.query_row("SELECT count(*) FROM sqlite_schema", [], |r| {
            r.get::<_, i64>(0)
        })
        .map(|_| ())
        .map_err(|e| budget.sql("Read database schema", &e))
    });
    if let Err(error) = read {
        return Inspection {
            checks_constraints: false,
            access: Err(error),
            integrity: None,
            foreign_keys: None,
            schema: None,
        };
    }
    let integrity = check_integrity(conn, budget);
    let foreign_keys = check_foreign_keys(conn, budget);
    let schema = budget
        .check("Inspect database schema and migration ledger")
        .and_then(|()| {
            let schema = super::inspect_schema(conn)
                .map_err(|e| budget.sql("Inspect database schema and migration ledger", &e))?;
            budget.check("Inspect database schema and migration ledger")?;
            Ok(schema)
        });
    Inspection {
        checks_constraints,
        access: Ok(()),
        integrity: Some(integrity),
        foreign_keys: Some(foreign_keys),
        schema: Some(schema),
    }
}

fn check_integrity(conn: &Connection, budget: &Budget) -> Result<Integrity, Failure> {
    const OP: &str = "Check database integrity";
    budget.check(OP)?;
    let run = || -> rusqlite::Result<Integrity> {
        let mut statement = conn.prepare("PRAGMA integrity_check")?;
        let mut rows = statement.query([])?;
        let mut errors = 0;
        let mut ok = false;
        while let Some(row) = rows.next()? {
            // SQLite diagnostics can contain data or schema names. Retain counts only.
            let value: String = row.get(0)?;
            if value == "ok" {
                ok = true;
            } else {
                errors += 1;
            }
        }
        Ok(if ok && errors == 0 {
            Integrity::Ok
        } else {
            Integrity::Errors(errors.max(1))
        })
    };
    run().map_err(|e| budget.sql(OP, &e))
}

fn check_foreign_keys(conn: &Connection, budget: &Budget) -> Result<u64, Failure> {
    const OP: &str = "Check database foreign keys";
    budget.check(OP)?;
    let run = || -> rusqlite::Result<u64> {
        let mut statement = conn.prepare("PRAGMA foreign_key_check")?;
        let mut rows = statement.query([])?;
        let mut count = 0;
        while rows.next()?.is_some() {
            count += 1;
        }
        Ok(count)
    };
    run().map_err(|e| budget.sql(OP, &e))
}

#[derive(Debug)]
pub(crate) struct Snapshot {
    pub(crate) destination: PathBuf,
    pub(crate) inspection: Inspection,
    pub(crate) cleanup_remaining: Vec<PathBuf>,
}

pub(crate) fn backup(
    source: &Path,
    destination: &Path,
    budget: &Budget,
) -> Result<Snapshot, Failure> {
    backup_with(source, destination, budget, || Ok(()))
}

fn backup_with(
    source: &Path,
    destination: &Path,
    budget: &Budget,
    before_publish: impl FnOnce() -> Result<(), Failure>,
) -> Result<Snapshot, Failure> {
    budget.check("Start database backup")?;
    let source_path =
        fs::canonicalize(source).map_err(|e| Failure::io("Locate source database", &e))?;
    let original_parent = source
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let named_source = fs::canonicalize(original_parent)
        .map_err(|e| Failure::io("Locate source database directory", &e))?
        .join(
            source
                .file_name()
                .ok_or_else(|| Failure::new("Locate source database", FailureKind::InvalidFile))?,
        );
    checked_destination(&named_source, destination)?;
    let destination = checked_destination(&source_path, destination)?;
    let conn = open_source(&source_path, budget)?;
    let candidate = Candidate::reserve(destination.parent().expect("canonical parent"), budget)?;
    let result = build_snapshot(&conn, &candidate.path, budget).and_then(|inspection| {
        before_publish()?;
        budget.check("Publish verified database backup")?;
        // Recheck after copying, including competing creation of sidecar files.
        checked_destination(&source_path, &destination)?;
        candidate.verify()?;
        fs::hard_link(&candidate.path, &destination)
            .map_err(|e| Failure::io("Publish database backup without overwriting", &e))?;
        if let Err(error) = File::open(destination.parent().expect("canonical parent"))
            .and_then(|file| file.sync_all())
        {
            let mut failure = Failure::io("Sync published database backup directory", &error);
            failure.remaining.push(destination.clone());
            return Err(failure);
        }
        Ok(Snapshot {
            destination,
            inspection,
            cleanup_remaining: Vec::new(),
        })
    });
    let remaining = candidate.cleanup();
    match result {
        Ok(mut snapshot) => {
            snapshot.cleanup_remaining = remaining;
            Ok(snapshot)
        }
        Err(mut failure) => {
            failure.remaining.extend(remaining);
            Err(failure)
        }
    }
}

fn build_snapshot(
    source: &Connection,
    path: &Path,
    budget: &Budget,
) -> Result<Inspection, Failure> {
    let mut target = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| budget.sql("Open private backup candidate", &e))?;
    budget.configure(&target)?;
    {
        let backup = Backup::new(source, &mut target)
            .map_err(|e| budget.sql("Start SQLite snapshot", &e))?;
        let mut busy = false;
        loop {
            budget
                .check("Copy SQLite snapshot pages")
                .map_err(|mut error| {
                    if busy && error.kind == FailureKind::Deadline {
                        error.kind = FailureKind::Busy;
                    }
                    error
                })?;
            match backup
                .step(BACKUP_PAGE_BATCH)
                .map_err(|e| budget.sql("Copy SQLite snapshot pages", &e))?
            {
                StepResult::Done => break,
                StepResult::More => busy = false,
                StepResult::Busy | StepResult::Locked => {
                    busy = true;
                    std::thread::sleep(LOCK_WAIT);
                }
                _ => return Err(Failure::new("Copy SQLite snapshot pages", FailureKind::Sql)),
            }
        }
    } // Backup::drop finishes the SQLite backup before any destination API call.
    budget.check("Finish database backup")?;
    // Make the completed snapshot a standalone database, including for a WAL source.
    target
        .pragma_update(None, "journal_mode", "DELETE")
        .map_err(|e| budget.sql("Finish standalone database backup", &e))?;
    target
        .close()
        .map_err(|(_, e)| budget.sql("Close database backup candidate", &e))?;
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| Failure::io("Sync database backup candidate", &e))?;
    // A read-only SQLite connection omits CHECK expressions while loading schema.
    // Validate our closed, synced private candidate with write access and query_only.
    let validation = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| budget.sql("Reopen backup candidate for validation", &e))?;
    budget.configure(&validation)?;
    validation
        .pragma_update(None, "query_only", true)
        .map_err(|e| budget.sql("Protect backup validation", &e))?;
    let inspection = inspect_connection(&validation, budget, true);
    validation
        .close()
        .map_err(|(_, e)| budget.sql("Close validated database backup", &e))?;
    if !inspection.valid_snapshot() {
        if let Some(failure) = [
            inspection.access.as_ref().err(),
            inspection.integrity.as_ref().and_then(|r| r.as_ref().err()),
            inspection
                .foreign_keys
                .as_ref()
                .and_then(|r| r.as_ref().err()),
            inspection.schema.as_ref().and_then(|r| r.as_ref().err()),
        ]
        .into_iter()
        .flatten()
        .next()
        {
            return Err(failure.clone());
        }
        return Err(Failure::new(
            "Validate backup integrity, foreign keys and supported schema",
            FailureKind::Validation,
        ));
    }
    Ok(inspection)
}

fn sidecars(path: &Path) -> [PathBuf; 3] {
    ["-wal", "-shm", "-journal"].map(|suffix| {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        PathBuf::from(name)
    })
}

fn checked_destination(source: &Path, destination: &Path) -> Result<PathBuf, Failure> {
    const OP: &str = "Reserve a new backup destination";
    let name = destination
        .file_name()
        .ok_or_else(|| Failure::new(OP, FailureKind::Destination))?;
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let path = fs::canonicalize(parent)
        .map_err(|e| Failure::io(OP, &e))?
        .join(name);
    if path == source || sidecars(source).contains(&path) {
        return Err(Failure::new(OP, FailureKind::Destination));
    }
    for entry in std::iter::once(path.clone()).chain(sidecars(&path)) {
        match fs::symlink_metadata(entry) {
            Ok(_) => return Err(Failure::new(OP, FailureKind::Destination)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Failure::io(OP, &error)),
        }
    }
    Ok(path)
}

#[derive(Debug)]
struct Candidate {
    directory: PathBuf,
    path: PathBuf,
    directory_identity: fs::Metadata,
    identity: fs::Metadata,
}
impl Candidate {
    fn reserve(parent: &Path, budget: &Budget) -> Result<Self, Failure> {
        const OP: &str = "Create private database backup candidate";
        let directory = loop {
            budget.check(OP)?;
            let sequence = CANDIDATE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(".v4vmm-database-{}-{sequence}", std::process::id()));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&path) {
                Ok(()) => break path,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(Failure::io(OP, &e)),
            }
        };
        let directory_identity = fs::symlink_metadata(&directory).map_err(|e| {
            let mut failure = Failure::io(OP, &e);
            failure.remaining.push(directory.clone());
            failure
        })?;
        let path = directory.join("candidate.sqlite");
        let mut options = OpenOptions::new();
        options.create_new(true).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path).and_then(|file| file.metadata()) {
            Ok(identity) => Ok(Self {
                directory,
                path,
                directory_identity,
                identity,
            }),
            Err(error) => {
                let mut failure = Failure::io(OP, &error);
                if fs::remove_dir(&directory).is_err() {
                    failure.remaining.push(directory);
                }
                Err(failure)
            }
        }
    }
    fn verify(&self) -> Result<(), Failure> {
        let directory = fs::symlink_metadata(&self.directory);
        let file = fs::symlink_metadata(&self.path);
        if directory.is_ok_and(|m| m.is_dir() && same_file(&m, &self.directory_identity))
            && file.is_ok_and(|m| m.is_file() && same_file(&m, &self.identity))
        {
            Ok(())
        } else {
            Err(Failure::new(
                "Verify owned backup candidate",
                FailureKind::Destination,
            ))
        }
    }
    fn cleanup(self) -> Vec<PathBuf> {
        if self.verify().is_err() {
            return vec![self.directory];
        }
        let mut remaining = Vec::new();
        // Only this private directory's known SQLite artifacts belong to this operation.
        for path in sidecars(&self.path)
            .into_iter()
            .chain(std::iter::once(self.path))
        {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => remaining.push(path),
            }
        }
        if fs::remove_dir(&self.directory).is_err() {
            remaining.push(self.directory);
        }
        remaining
    }
}

fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        left.dev() == right.dev() && left.ino() == right.ino()
    }
    #[cfg(not(unix))]
    {
        left.created()
            .ok()
            .zip(right.created().ok())
            .is_some_and(|(a, b)| a == b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> Budget {
        Budget::new(Arc::new(AtomicBool::new(false)))
    }
    fn database() -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.sqlite");
        drop(crate::db::open_db(&source).unwrap());
        (temp, source)
    }

    #[test]
    fn adr_0066_inspection_preserves_bytes_and_separates_schema_integrity_and_foreign_keys() {
        let (temp, source) = database();
        let original = fs::read(&source).unwrap();
        let inspection = inspect(&source, &budget());
        assert!(inspection.valid_snapshot());
        assert_eq!(fs::read(&source).unwrap(), original);
        let missing = temp.path().join("absent.sqlite");
        assert_eq!(
            inspect(&missing, &budget()).access.unwrap_err().kind,
            FailureKind::Missing
        );
        assert!(!missing.exists());
        let conn = Connection::open(&source).unwrap();
        conn.execute("DELETE FROM schema_migrations WHERE version=11", [])
            .unwrap();
        conn.execute("DROP TABLE broadcast_event_selection", [])
            .unwrap();
        let before = fs::read(&source).unwrap();
        let inspection = inspect(&source, &budget());
        assert_eq!(
            inspection.schema,
            Some(Ok(SchemaCompatibility::UpgradeRequired {
                applied: 10,
                current: 11
            }))
        );
        assert_eq!(fs::read(&source).unwrap(), before);
        conn.execute(
            "INSERT INTO schema_migrations(version,name) VALUES (999,'future')",
            [],
        )
        .unwrap();
        let inspection = inspect(&source, &budget());
        assert_eq!(inspection.integrity, Some(Ok(Integrity::Ok)));
        assert_eq!(
            inspection.schema,
            Some(Ok(SchemaCompatibility::Newer { version: 999 }))
        );
        assert!(!inspection.valid_snapshot());
        conn.execute(
            "DELETE FROM schema_migrations WHERE version=999 OR version=3",
            [],
        )
        .unwrap();
        assert_eq!(
            inspect(&source, &budget()).schema,
            Some(Ok(SchemaCompatibility::Unknown))
        );
        conn.execute_batch("PRAGMA foreign_keys=OFF; INSERT INTO tracks(feed_id, item_guid) VALUES (900, 'orphan');").unwrap();
        let inspection = inspect(&source, &budget());
        assert_eq!(inspection.foreign_keys, Some(Ok(1)));
        assert_eq!(inspection.integrity, Some(Ok(Integrity::Ok)));
    }

    #[test]
    fn adr_0066_invalid_header_and_integrity_errors_never_become_verified_backups() {
        let (temp, source) = database();
        let mut damaged = fs::read(&source).unwrap();
        // A valid header/schema with a freelist trunk beyond the file's last page.
        damaged[32..36].copy_from_slice(&0x7fff_ffff_u32.to_be_bytes());
        damaged[36..40].copy_from_slice(&1_u32.to_be_bytes());
        fs::write(&source, damaged).unwrap();
        let before = fs::read(&source).unwrap();
        let checked = inspect(&source, &budget());
        assert!(
            matches!(checked.integrity, Some(Ok(Integrity::Errors(_)))),
            "{checked:?}"
        );
        let target = temp.path().join("rejected.sqlite");
        assert_eq!(
            backup(&source, &target, &budget()).unwrap_err().kind,
            FailureKind::Validation
        );
        assert!(!target.exists());
        assert_eq!(fs::read(&source).unwrap(), before);
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
        fs::write(&source, b"invalid sqlite header").unwrap();
        let inspection = inspect(&source, &budget());
        assert_eq!(inspection.access.unwrap_err().kind, FailureKind::Corrupt);
        assert!(inspection.integrity.is_none());
        assert!(backup(&source, &target, &budget()).is_err());
        assert!(!target.exists());
        assert_eq!(fs::read(&source).unwrap(), b"invalid sqlite header");
    }

    #[test]
    fn adr_0066_snapshot_includes_committed_wal_rows_with_source_open() {
        let (temp, source) = database();
        let conn = Connection::open(&source).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;")
            .unwrap();
        let main_before = fs::read(&source).unwrap();
        conn.execute("INSERT INTO playlists(name) VALUES ('WAL-only row')", [])
            .unwrap();
        assert_eq!(fs::read(&source).unwrap(), main_before);
        let wal_before = fs::read(sidecars(&source)[0].clone()).unwrap();
        let target = temp.path().join("backup.sqlite");
        let result = backup(&source, &target, &budget()).unwrap();
        assert!(result.inspection.valid_snapshot());
        assert!(result.cleanup_remaining.is_empty());
        let snapshot = Connection::open(&target).unwrap();
        assert_eq!(
            snapshot
                .query_row("SELECT name FROM playlists", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "WAL-only row"
        );
        assert_eq!(fs::read(&source).unwrap(), main_before);
        assert_eq!(fs::read(sidecars(&source)[0].clone()).unwrap(), wal_before);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(target).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn adr_0066_read_only_source_and_destination_aliases_are_safe() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let (temp, source) = database();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o400)).unwrap();
        let before = fs::read(&source).unwrap();
        assert!(inspect(&source, &budget()).valid_snapshot());
        backup(
            &source,
            &temp.path().join("readonly-backup.sqlite"),
            &budget(),
        )
        .unwrap();
        let link = temp.path().join("link.sqlite");
        let hard = temp.path().join("hard.sqlite");
        symlink(&source, &link).unwrap();
        fs::hard_link(&source, &hard).unwrap();
        let dangling = temp.path().join("dangling.sqlite");
        symlink(temp.path().join("missing"), &dangling).unwrap();
        for target in [&source, &link, &hard, &dangling]
            .into_iter()
            .cloned()
            .chain(sidecars(&source))
        {
            assert_eq!(
                backup(&source, &target, &budget()).unwrap_err().kind,
                FailureKind::Destination
            );
        }
        let occupied = temp.path().join("occupied.sqlite");
        fs::write(&occupied, b"preserve").unwrap();
        assert!(backup(&source, &occupied, &budget()).is_err());
        assert_eq!(fs::read(occupied).unwrap(), b"preserve");
        assert_eq!(fs::read(&source).unwrap(), before);
        let absent_parent = temp.path().join("missing/backup.sqlite");
        assert!(backup(&source, &absent_parent, &budget()).is_err());
        assert!(!absent_parent.parent().unwrap().exists());
    }

    #[test]
    fn adr_0066_busy_cancel_deadline_and_candidate_cleanup_are_bounded() {
        let (temp, source) = database();
        let conn = Connection::open(&source).unwrap();
        conn.execute_batch("BEGIN EXCLUSIVE").unwrap();
        let limited = Budget {
            deadline: Instant::now() + Duration::from_millis(120),
            ..budget()
        };
        let target = temp.path().join("backup.sqlite");
        let start = Instant::now();
        assert_eq!(
            backup(&source, &target, &limited).unwrap_err().kind,
            FailureKind::Busy
        );
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(!target.exists());
        assert_eq!(
            inspect(&source, &budget()).access.unwrap_err().kind,
            FailureKind::Busy
        );
        conn.execute_batch("ROLLBACK").unwrap();
        let cancelled = budget();
        cancelled.cancelled.store(true, Ordering::Release);
        assert_eq!(
            backup(&source, &target, &cancelled).unwrap_err().kind,
            FailureKind::Cancelled
        );
        let expired = Budget {
            deadline: Instant::now(),
            ..budget()
        };
        assert_eq!(
            backup(&source, &target, &expired).unwrap_err().kind,
            FailureKind::Deadline
        );
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
        let candidate = Candidate::reserve(temp.path(), &budget()).unwrap();
        let directory = candidate.directory.clone();
        fs::write(directory.join("unowned-file"), b"keep").unwrap();
        assert_eq!(candidate.cleanup(), vec![directory.clone()]);
        assert_eq!(fs::read(directory.join("unowned-file")).unwrap(), b"keep");
    }

    #[test]
    fn adr_0066_late_cancel_io_failure_and_destination_race_never_publish_a_candidate() {
        let (temp, source) = database();
        let destination = temp.path().join("backup.sqlite");
        let cancelled = budget();
        let result = backup_with(&source, &destination, &cancelled, || {
            cancelled.cancelled.store(true, Ordering::Release);
            Ok(())
        });
        assert_eq!(result.unwrap_err().kind, FailureKind::Cancelled);
        assert!(!destination.exists());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
        let result = backup_with(&source, &destination, &budget(), || {
            Err(Failure::io(
                "Publish snapshot",
                &std::io::Error::from(std::io::ErrorKind::StorageFull),
            ))
        });
        assert_eq!(result.unwrap_err().kind, FailureKind::Io);
        assert!(!destination.exists());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
        let result = backup_with(&source, &destination, &budget(), || {
            fs::write(&destination, b"competing destination").unwrap();
            Ok(())
        });
        assert_eq!(result.unwrap_err().kind, FailureKind::Destination);
        assert_eq!(fs::read(&destination).unwrap(), b"competing destination");
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
    }

    #[test]
    fn adr_0066_backup_validation_checks_constraints_omitted_by_readonly_sqlite() {
        let (temp, source) = database();
        let conn = Connection::open(&source).unwrap();
        conn.execute_batch("CREATE TABLE fixture_check(value INTEGER CHECK(value>0)); PRAGMA ignore_check_constraints=ON; INSERT INTO fixture_check VALUES(-1);").unwrap();
        drop(conn);
        let original = fs::read(&source).unwrap();
        let readonly = inspect(&source, &budget());
        assert!(!readonly.checks_constraints);
        let destination = temp.path().join("backup.sqlite");
        assert_eq!(
            backup(&source, &destination, &budget()).unwrap_err().kind,
            FailureKind::Validation
        );
        assert!(!destination.exists());
        assert_eq!(fs::read(&source).unwrap(), original);
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    fn adr_0066_sql_progress_cancels_a_running_inspection() {
        let (temp, source) = database();
        let conn = Connection::open(&source).unwrap();
        conn.execute_batch("CREATE TABLE large(value); WITH RECURSIVE values_to_check(v) AS (VALUES(1) UNION ALL SELECT v+1 FROM values_to_check WHERE v<200000) INSERT INTO large SELECT v FROM values_to_check;").unwrap();
        let short = Budget {
            deadline: Instant::now(),
            ..budget()
        };
        short.configure(&conn).unwrap();
        assert_eq!(
            check_integrity(&conn, &short).unwrap_err().kind,
            FailureKind::Deadline
        );
        let running = budget();
        running.configure(&conn).unwrap();
        running.cancelled.store(true, Ordering::Release);
        // Run SQL directly to prove the registered SQLite callback interrupts it.
        let result = conn.query_row(
            "SELECT sum(a.value*b.value) FROM large a, large b",
            [],
            |r| r.get::<_, i64>(0),
        );
        assert_eq!(
            result.unwrap_err().sqlite_error_code(),
            Some(ErrorCode::OperationInterrupted)
        );
        assert!(!temp.path().join("backup.sqlite").exists());
    }
}
