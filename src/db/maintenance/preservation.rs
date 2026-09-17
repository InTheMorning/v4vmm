//! Exclusive `SQLite` access and journal-file preservation (ADR 0066, invariant 6).
//!
//! The connection holds `SQLite`'s exclusive locking mode before its first database
//! access. Source descriptors stay open until AFTER that connection closes: on
//! POSIX, closing any descriptor for the same inode can release `SQLite`'s locks.
//! Copies describe the files after `SQLite`'s lock-acquisition/recovery work, never
//! a verified snapshot or a promise about bytes before `SQLite` opened the source.

#![warn(clippy::pedantic)]

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{same_file, sidecars, Budget, Failure, FailureKind, LOCK_WAIT};

pub(crate) const EXCLUSIVE_ACCESS_DEADLINE: Duration = Duration::from_secs(5);
const COPY_BUFFER_BYTES: usize = 64 * 1024;

/// Field order is significant: `SQLite` closes before ANY raw source descriptor.
#[derive(Debug)]
pub(crate) struct ExclusiveDatabase {
    connection: Connection,
    files: Vec<SourceFile>,
    source: PathBuf,
    mode: String,
    acquired_at: SystemTime,
    changed_during_access: Vec<PathBuf>,
}

#[derive(Debug)]
struct SourceFile {
    path: PathBuf,
    copied_name: String,
    file: File,
    identity: fs::Metadata,
}

#[derive(Debug)]
pub(crate) struct Preservation {
    pub(crate) directory: PathBuf,
    pub(crate) manifest: PathBuf,
    pub(crate) journal_mode: String,
    pub(crate) acquired_at: SystemTime,
    pub(crate) file_count: usize,
    pub(crate) changed_during_access: Vec<PathBuf>,
}

#[derive(Serialize)]
struct Manifest {
    kind: &'static str,
    source: PathBuf,
    acquired_at_utc: String,
    copied_at_utc: String,
    sqlite_version: String,
    journal_mode: String,
    sqlite_access: &'static str,
    changed_during_access: Vec<PathBuf>,
    files: Vec<FileRecord>,
}

#[derive(Serialize)]
struct FileRecord {
    source: PathBuf,
    copied_name: String,
    length: u64,
    sha256: String,
}

impl ExclusiveDatabase {
    /// Called by the maintenance command only while it owns a drained session.
    pub(crate) fn acquire(source: &Path, budget: &Budget) -> Result<Self, Failure> {
        Self::acquire_with_timeout(source, budget, EXCLUSIVE_ACCESS_DEADLINE)
    }

    fn acquire_with_timeout(
        source: &Path,
        budget: &Budget,
        timeout: Duration,
    ) -> Result<Self, Failure> {
        const OP: &str = "Acquire exclusive SQLite maintenance access";
        budget.check(OP)?;
        let source = fs::canonicalize(source).map_err(|e| Failure::io(OP, &e))?;
        let before = fs::symlink_metadata(&source).map_err(|e| Failure::io(OP, &e))?;
        check_regular(&before)?;
        let observed = observe_files(&source, before.clone())?;
        // SQLite must resolve all journals through one pathname. Hard-link aliases
        // do not share those names, so this maintenance protocol refuses them.
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if before.nlink() != 1 {
                return Err(Failure::new(
                    "Refuse database with hard-link aliases",
                    FailureKind::Unsupported,
                ));
            }
        }
        let connection = Connection::open_with_flags(
            &source,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|e| budget.sql(OP, &e))?;
        budget.configure(&connection)?;
        let mode: String = connection
            .query_row("PRAGMA main.locking_mode=EXCLUSIVE", [], |r| r.get(0))
            .map_err(|e| budget.sql(OP, &e))?;
        if mode != "exclusive" {
            return Err(Failure::new(OP, FailureKind::Unsupported));
        }
        let deadline = Instant::now() + timeout;
        loop {
            budget.check(OP)?;
            match connection.execute_batch("BEGIN EXCLUSIVE") {
                Ok(()) => break,
                Err(e) => {
                    let failure = budget.sql(OP, &e);
                    if failure.kind != FailureKind::Busy || Instant::now() >= deadline {
                        return Err(failure);
                    }
                    std::thread::sleep(
                        LOCK_WAIT.min(deadline.saturating_duration_since(Instant::now())),
                    );
                }
            }
        }
        // End the empty transaction, retaining the exclusive connection's lock.
        // Task 012 can use SQLite's backup API on this autocommit connection.
        connection
            .execute_batch("ROLLBACK")
            .map_err(|e| budget.sql(OP, &e))?;
        let mode: String = connection
            .query_row("PRAGMA main.journal_mode", [], |r| r.get(0))
            .map_err(|e| budget.sql("Observe SQLite journal mode", &e))?;
        if !matches!(mode.as_str(), "delete" | "truncate" | "persist" | "wal") {
            return Err(Failure::new(
                "SQLite journal mode does not support file preservation",
                FailureKind::Unsupported,
            ));
        }
        let after = fs::symlink_metadata(&source).map_err(|e| Failure::io(OP, &e))?;
        if !same_file(&before, &after) {
            return Err(Failure::new(
                "Database path changed during lock acquisition",
                FailureKind::Unsupported,
            ));
        }
        let mut access = Self {
            connection,
            files: Vec::new(),
            source,
            mode,
            acquired_at: SystemTime::now(),
            changed_during_access: Vec::new(),
        };
        access.open_files(budget)?;
        access.changed_during_access = observed
            .into_iter()
            .filter_map(|(path, before)| {
                let after = access
                    .files
                    .iter()
                    .find(|file| file.path == path)
                    .map(|file| &file.identity);
                let unchanged = match (before, after) {
                    (Some(a), Some(b)) => {
                        same_file(&a, b)
                            && a.len() == b.len()
                            && a.modified().ok() == b.modified().ok()
                    }
                    (None, None) => true,
                    _ => false,
                };
                (!unchanged).then_some(path)
            })
            .collect();
        Ok(access)
    }

    fn open_files(&mut self, budget: &Budget) -> Result<(), Failure> {
        const OP: &str = "Open stable database and journal files for preservation";
        let paths = std::iter::once(self.source.clone()).chain(sidecars(&self.source));
        for (index, path) in paths.enumerate() {
            budget.check(OP)?;
            let identity = match fs::symlink_metadata(&path) {
                Ok(value) => value,
                Err(e) if index != 0 && e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(Failure::io(OP, &e)),
            };
            check_regular(&identity)?;
            let suffix = ["", "-wal", "-shm", "-journal"][index];
            let file = File::open(&path).map_err(|e| Failure::io(OP, &e))?;
            // Transfer every opened descriptor immediately, including failure paths.
            self.files.push(SourceFile {
                path,
                copied_name: format!("database.sqlite{suffix}"),
                file,
                identity,
            });
            let source = self.files.last().expect("just inserted source");
            let opened = source.file.metadata().map_err(|e| Failure::io(OP, &e))?;
            if !same_file(&opened, &source.identity) {
                return Err(Failure::new(
                    "Source file changed while opening preservation inputs",
                    FailureKind::Unsupported,
                ));
            }
        }
        Ok(())
    }

    /// Requires the held guard; no free function can copy an unlocked source.
    pub(crate) fn preserve(
        &mut self,
        destination: &Path,
        budget: &Budget,
    ) -> Result<Preservation, Failure> {
        self.preserve_with(destination, budget, |_| Ok(()))
    }

    fn preserve_with(
        &mut self,
        destination: &Path,
        budget: &Budget,
        after_file: impl Fn(usize) -> Result<(), Failure>,
    ) -> Result<Preservation, Failure> {
        const OP: &str = "Create owner-only preservation directory";
        budget.check(OP)?;
        let destination = super::checked_destination(&self.source, destination)?;
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(&destination)
            .map_err(|e| Failure::io(OP, &e))?;
        let result = self.copy_files(&destination, budget, after_file);
        match result {
            Ok(manifest) => Ok(Preservation {
                directory: destination,
                manifest,
                journal_mode: self.mode.clone(),
                acquired_at: self.acquired_at,
                file_count: self.files.len(),
                changed_during_access: self.changed_during_access.clone(),
            }),
            Err(mut failure) => {
                // Keep partial evidence. It is never a completed preservation copy.
                failure.remaining.push(destination);
                Err(failure)
            }
        }
    }

    fn copy_files(
        &mut self,
        directory: &Path,
        budget: &Budget,
        after_file: impl Fn(usize) -> Result<(), Failure>,
    ) -> Result<PathBuf, Failure> {
        let mut records = Vec::new();
        for source in &mut self.files {
            budget.check("Copy database preservation files")?;
            let destination = directory.join(&source.copied_name);
            let mut output = private_file(&destination)?;
            source
                .file
                .seek(SeekFrom::Start(0))
                .map_err(|e| Failure::io("Read preservation source", &e))?;
            let (length, checksum) = copy_and_hash(&mut source.file, &mut output, budget)?;
            output
                .sync_all()
                .map_err(|e| Failure::io("Sync preserved database file", &e))?;
            output
                .seek(SeekFrom::Start(0))
                .map_err(|e| Failure::io("Recheck preserved database file", &e))?;
            let verified = copy_and_hash(&mut output, &mut std::io::sink(), budget)?;
            let current = source
                .file
                .metadata()
                .map_err(|e| Failure::io("Recheck preservation source", &e))?;
            if verified != (length, checksum.clone())
                || length != source.identity.len()
                || current.len() != source.identity.len()
                || current.modified().ok() != source.identity.modified().ok()
            {
                return Err(Failure::new(
                    "Preservation file changed or failed checksum verification",
                    FailureKind::UnstableFiles,
                ));
            }
            records.push(FileRecord {
                source: source.path.clone(),
                copied_name: source.copied_name.clone(),
                length,
                sha256: checksum,
            });
            after_file(records.len())?;
        }
        budget.check("Write completed preservation manifest")?;
        let manifest = Manifest {
            kind: "database_file_preservation_not_verified_backup",
            source: self.source.clone(),
            acquired_at_utc: chrono::DateTime::<chrono::Utc>::from(self.acquired_at).to_rfc3339(),
            copied_at_utc: chrono::Utc::now().to_rfc3339(),
            sqlite_version: rusqlite::version().into(),
            journal_mode: self.mode.clone(),
            sqlite_access: "SQLite acquired BEGIN EXCLUSIVE in exclusive locking mode and rolled back the empty transaction while retaining its lock. SQLite may perform automatic journal recovery during access and journal cleanup/checkpoint on close. These files were copied after lock acquisition, before connection close; no pre-access byte identity or verified restore is claimed. App requested no manual journal replay, deletion or checkpoint.",
            changed_during_access: self.changed_during_access.clone(),
            files: records,
        };
        let bytes = serde_json::to_vec_pretty(&manifest).map_err(|_| {
            Failure::new("Encode exact preservation paths", FailureKind::Unsupported)
        })?;
        let path = directory.join("manifest.json");
        let mut file = private_file(&path)?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| Failure::io("Write and sync preservation manifest", &e))?;
        File::open(directory)
            .and_then(|f| f.sync_all())
            .map_err(|e| Failure::io("Sync preservation directory", &e))?;
        File::open(directory.parent().expect("checked destination parent"))
            .and_then(|f| f.sync_all())
            .map_err(|e| Failure::io("Sync preservation parent directory", &e))?;
        Ok(path)
    }
}

impl Drop for ExclusiveDatabase {
    fn drop(&mut self) {
        // Read this field to make the ownership purpose explicit. The field drop
        // order then closes it before files, including cancellation and errors.
        debug_assert!(self.connection.is_autocommit());
    }
}

fn check_regular(metadata: &fs::Metadata) -> Result<(), Failure> {
    if metadata.is_file() {
        Ok(())
    } else {
        Err(Failure::new(
            "Refuse non-regular preservation source",
            FailureKind::InvalidFile,
        ))
    }
}

fn observe_files(
    source: &Path,
    main: fs::Metadata,
) -> Result<Vec<(PathBuf, Option<fs::Metadata>)>, Failure> {
    let mut observed = vec![(source.to_owned(), Some(main))];
    for path in sidecars(source) {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                check_regular(&metadata)?;
                Some(metadata)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(Failure::io("Observe SQLite journals before access", &e)),
        };
        observed.push((path, metadata));
    }
    Ok(observed)
}

fn private_file(path: &Path) -> Result<File, Failure> {
    let mut options = OpenOptions::new();
    options.create_new(true).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .map_err(|e| Failure::io("Create private preservation file without overwriting", &e))
}

fn copy_and_hash(
    reader: &mut impl Read,
    writer: &mut impl Write,
    budget: &Budget,
) -> Result<(u64, String), Failure> {
    let mut buffer = vec![0; COPY_BUFFER_BYTES];
    let mut digest = Sha256::new();
    let mut length = 0;
    loop {
        budget.check("Copy and verify preservation bytes")?;
        let count = reader
            .read(&mut buffer)
            .map_err(|e| Failure::io("Read preservation bytes", &e))?;
        if count == 0 {
            break;
        }
        writer
            .write_all(&buffer[..count])
            .map_err(|e| Failure::io("Write preservation bytes", &e))?;
        digest.update(&buffer[..count]);
        length += count as u64;
    }
    Ok((length, format!("{:x}", digest.finalize())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    fn budget() -> Budget {
        Budget::new(Arc::new(AtomicBool::new(false)))
    }

    fn database(mode: &str) -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.sqlite");
        let conn = crate::db::open_db(&source).unwrap();
        conn.pragma_update(None, "journal_mode", mode).unwrap();
        conn.execute("INSERT INTO playlists(name) VALUES('preserved')", [])
            .unwrap();
        drop(conn);
        (temp, source)
    }

    struct Peer(Child);
    impl Peer {
        fn start(source: &Path, mode: &str) -> Self {
            let child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "db::maintenance::preservation::tests::adr_0066_external_sqlite_peer",
                    "--nocapture",
                ])
                .env("V4VMM_TEST_EXCLUSIVE_SOURCE", source)
                .env("V4VMM_TEST_EXCLUSIVE_MODE", mode)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();
            Self(child)
        }
        fn ready(&mut self) {
            let mut reader = BufReader::new(self.0.stdout.as_mut().unwrap());
            let mut line = String::new();
            loop {
                assert_ne!(
                    reader.read_line(&mut line).unwrap(),
                    0,
                    "peer exited without readiness: {line}"
                );
                if line.contains("PEER_READY") {
                    break;
                }
                line.clear();
            }
        }
        fn finish(mut self) {
            self.0
                .stdin
                .take()
                .unwrap()
                .write_all(b"release\n")
                .unwrap();
            assert!(self.0.wait().unwrap().success());
        }
        fn probe(source: &Path, expected: &str) {
            let mut peer = Self::start(source, expected);
            assert!(
                peer.0.wait().unwrap().success(),
                "separate process lock probe failed: {expected}"
            );
        }
    }
    impl Drop for Peer {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    // This child is the SAME bundled SQLite as the application, in another OS process.
    #[test]
    fn adr_0066_external_sqlite_peer() {
        let Some(source) = std::env::var_os("V4VMM_TEST_EXCLUSIVE_SOURCE") else {
            return;
        };
        let mode = std::env::var("V4VMM_TEST_EXCLUSIVE_MODE").unwrap();
        let conn =
            Connection::open_with_flags(PathBuf::from(source), OpenFlags::SQLITE_OPEN_READ_WRITE)
                .unwrap();
        conn.busy_timeout(Duration::from_millis(80)).unwrap();
        if mode == "seed-wal" {
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; INSERT INTO playlists(name) VALUES('committed WAL');").unwrap();
            // Simulate a process exit without SQLite's final checkpoint/sidecar cleanup.
            std::process::exit(0);
        }
        if mode == "hot-journal" {
            conn.execute_batch("BEGIN IMMEDIATE; UPDATE playlists SET name='uncommitted change';")
                .unwrap();
            conn.cache_flush().unwrap();
            std::process::exit(0);
        }
        if matches!(mode.as_str(), "blocked" | "open") {
            for sql in ["SELECT count(*) FROM sqlite_schema", "BEGIN IMMEDIATE"] {
                let result = conn
                    .prepare(sql)
                    .and_then(|mut statement| statement.raw_query().next().map(|_| ()));
                if mode == "blocked" {
                    assert!(matches!(
                        result.unwrap_err().sqlite_error_code(),
                        Some(
                            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                        )
                    ));
                } else {
                    result.unwrap();
                }
            }
            return;
        }
        conn.execute_batch(if mode == "reader" {
            "BEGIN; SELECT count(*) FROM sqlite_schema;"
        } else {
            "BEGIN IMMEDIATE;"
        })
        .unwrap();
        println!("PEER_READY");
        std::io::stdout().flush().unwrap();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        conn.execute_batch("ROLLBACK").unwrap();
    }

    #[test]
    fn adr_0066_exclusive_modes_exclude_processes_through_each_copy_and_release() {
        for mode in ["DELETE", "TRUNCATE", "PERSIST", "WAL"] {
            let (temp, source) = database(mode);
            if mode == "WAL" {
                Peer::probe(&source, "seed-wal");
            }
            let mut access = ExclusiveDatabase::acquire(&source, &budget()).unwrap();
            let observed_mode = if mode == "WAL" { "wal" } else { "delete" };
            assert_eq!(access.mode, observed_mode);
            let existing: Vec<_> = std::iter::once(source.clone())
                .chain(sidecars(&source))
                .filter(|path| fs::symlink_metadata(path).is_ok())
                .collect();
            assert_eq!(
                access
                    .files
                    .iter()
                    .map(|file| file.path.clone())
                    .collect::<Vec<_>>(),
                existing
            );
            Peer::probe(&source, "blocked");
            let destination = temp.path().join("preserved");
            let result = access
                .preserve_with(&destination, &budget(), |_| {
                    // Opening/copying then closing a raw main-file descriptor here
                    // would break the lock. Probe after EVERY copied file.
                    Peer::probe(&source, "blocked");
                    Ok(())
                })
                .unwrap();
            let manifest: serde_json::Value =
                serde_json::from_slice(&fs::read(result.manifest).unwrap()).unwrap();
            assert_eq!(manifest["journal_mode"], observed_mode);
            assert_eq!(
                manifest["kind"],
                "database_file_preservation_not_verified_backup"
            );
            assert_eq!(
                manifest["files"].as_array().unwrap().len(),
                access.files.len()
            );
            for entry in manifest["files"].as_array().unwrap() {
                let bytes =
                    fs::read(destination.join(entry["copied_name"].as_str().unwrap())).unwrap();
                assert_eq!(entry["length"].as_u64().unwrap(), bytes.len() as u64);
                assert_eq!(entry["sha256"], format!("{:x}", Sha256::digest(&bytes)));
                // Source handles stay owned by access; compare through those
                // descriptors, never fs::read(source) under a POSIX SQLite lock.
                let input = access
                    .files
                    .iter_mut()
                    .find(|s| s.path.to_str() == entry["source"].as_str())
                    .unwrap();
                input.file.seek(SeekFrom::Start(0)).unwrap();
                let mut original = Vec::new();
                input.file.read_to_end(&mut original).unwrap();
                assert_eq!(original, bytes);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
                    0o700
                );
                for entry in fs::read_dir(&destination).unwrap() {
                    assert_eq!(
                        entry.unwrap().metadata().unwrap().permissions().mode() & 0o777,
                        0o600
                    );
                }
            }
            Peer::probe(&source, "blocked");
            drop(access);
            Peer::probe(&source, "open");
            if mode == "WAL" {
                assert!(destination.join("database.sqlite-wal").is_file());
                let conn = Connection::open(destination.join("database.sqlite")).unwrap();
                assert_eq!(
                    conn.query_row(
                        "SELECT count(*) FROM playlists WHERE name='committed WAL'",
                        [],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                    1
                );
            }
        }
    }

    #[test]
    fn adr_0066_real_readers_and_writers_bound_acquisition_without_copying() {
        for mode in ["DELETE", "WAL"] {
            for activity in ["reader", "writer"] {
                let (temp, source) = database(mode);
                let mut peer = Peer::start(&source, activity);
                peer.ready();
                let start = Instant::now();
                let result = ExclusiveDatabase::acquire_with_timeout(
                    &source,
                    &budget(),
                    Duration::from_millis(180),
                );
                assert_eq!(result.unwrap_err().kind, FailureKind::Busy);
                assert!(start.elapsed() < Duration::from_secs(2));
                assert!(!temp.path().join("preserved").exists());
                peer.finish();
                drop(ExclusiveDatabase::acquire(&source, &budget()).unwrap());
            }
        }
        // Also cover a second connection inside this process.
        let (_temp, source) = database("DELETE");
        let reader = Connection::open(&source).unwrap();
        reader
            .execute_batch("BEGIN; SELECT count(*) FROM sqlite_schema;")
            .unwrap();
        assert_eq!(
            ExclusiveDatabase::acquire_with_timeout(&source, &budget(), Duration::ZERO)
                .unwrap_err()
                .kind,
            FailureKind::Busy
        );
    }

    #[test]
    fn adr_0066_preservation_failure_and_cancel_keep_partial_artifacts_and_originals() {
        let (temp, source) = database("DELETE");
        let original = fs::read(&source).unwrap();
        let mut access = ExclusiveDatabase::acquire(&source, &budget()).unwrap();
        let cancelled = budget();
        cancelled.cancelled.store(true, Ordering::Release);
        let destination = temp.path().join("cancel-before");
        assert_eq!(
            access.preserve(&destination, &cancelled).unwrap_err().kind,
            FailureKind::Cancelled
        );
        assert!(!destination.exists());
        let destination = temp.path().join("partial");
        let failure = access
            .preserve_with(&destination, &budget(), |_| {
                Err(Failure::new(
                    "Injected destination failure",
                    FailureKind::Io,
                ))
            })
            .unwrap_err();
        assert_eq!(failure.remaining, [destination.clone()]);
        assert!(destination.join("database.sqlite").exists());
        assert!(!destination.join("manifest.json").exists());
        Peer::probe(&source, "blocked");
        let cancelled = budget();
        let destination = temp.path().join("cancel-during");
        assert_eq!(
            access
                .preserve_with(&destination, &cancelled, |_| {
                    cancelled.cancelled.store(true, Ordering::Release);
                    Ok(())
                })
                .unwrap_err()
                .kind,
            FailureKind::Cancelled
        );
        assert!(!destination.join("manifest.json").exists());
        assert_eq!(
            access
                .preserve(&temp.path().join("partial"), &budget())
                .unwrap_err()
                .kind,
            FailureKind::Destination
        );
        drop(access);
        Peer::probe(&source, "open");
        assert_eq!(fs::read(&source).unwrap(), original);
    }

    #[test]
    fn adr_0066_lockable_damage_preserves_but_invalid_header_and_missing_sources_refuse() {
        let (temp, source) = database("DELETE");
        let mut bytes = fs::read(&source).unwrap();
        bytes[32..36].copy_from_slice(&0x7fff_ffff_u32.to_be_bytes());
        bytes[36..40].copy_from_slice(&1_u32.to_be_bytes());
        fs::write(&source, &bytes).unwrap();
        assert!(!super::super::inspect(&source, &budget()).valid_snapshot());
        let mut access = ExclusiveDatabase::acquire(&source, &budget()).unwrap();
        let destination = temp.path().join("damaged");
        access.preserve(&destination, &budget()).unwrap();
        drop(access);
        assert_eq!(
            fs::read(destination.join("database.sqlite")).unwrap(),
            bytes
        );
        assert_eq!(fs::read(&source).unwrap(), bytes);
        let invalid = temp.path().join("invalid.sqlite");
        fs::write(&invalid, b"invalid database header").unwrap();
        assert_eq!(
            ExclusiveDatabase::acquire(&invalid, &budget())
                .unwrap_err()
                .kind,
            FailureKind::Corrupt
        );
        assert_eq!(fs::read(invalid).unwrap(), b"invalid database header");
        let missing = temp.path().join("missing.sqlite");
        assert_eq!(
            ExclusiveDatabase::acquire(&missing, &budget())
                .unwrap_err()
                .kind,
            FailureKind::Missing
        );
        assert!(!missing.exists());
    }

    #[test]
    fn adr_0066_sqlite_journal_recovery_is_observed_before_preservation() {
        let (temp, source) = database("DELETE");
        let original = fs::read(&source).unwrap();
        Peer::probe(&source, "hot-journal");
        assert_ne!(fs::read(&source).unwrap(), original);
        assert!(sidecars(&source)[2].exists());
        let mut access = ExclusiveDatabase::acquire(&source, &budget()).unwrap();
        assert!(access.changed_during_access.contains(&source));
        assert!(access.changed_during_access.contains(&sidecars(&source)[2]));
        let result = access
            .preserve(&temp.path().join("after-recovery"), &budget())
            .unwrap();
        let copied = fs::read(result.directory.join("database.sqlite")).unwrap();
        assert_eq!(copied, original);
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(result.manifest).unwrap()).unwrap();
        assert!(manifest["changed_during_access"].as_array().unwrap().len() >= 2);
        assert!(manifest["sqlite_access"]
            .as_str()
            .unwrap()
            .contains("no pre-access byte identity"));
        Peer::probe(&source, "blocked");
        drop(access);
        Peer::probe(&source, "open");
    }

    #[cfg(unix)]
    #[test]
    fn adr_0066_preservation_refuses_hardlink_aliases_and_nonregular_journals() {
        let (temp, source) = database("DELETE");
        let alias = temp.path().join("alias.sqlite");
        fs::hard_link(&source, &alias).unwrap();
        assert_eq!(
            ExclusiveDatabase::acquire(&source, &budget())
                .unwrap_err()
                .kind,
            FailureKind::Unsupported
        );
        fs::remove_file(alias).unwrap();
        let unrelated = temp.path().join("unrelated");
        fs::write(&unrelated, b"retain unrelated contents").unwrap();
        std::os::unix::fs::symlink(&unrelated, &sidecars(&source)[2]).unwrap();
        assert_eq!(
            ExclusiveDatabase::acquire(&source, &budget())
                .unwrap_err()
                .kind,
            FailureKind::InvalidFile
        );
        assert_eq!(fs::read(unrelated).unwrap(), b"retain unrelated contents");
    }
}
