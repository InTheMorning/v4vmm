//! Reviewed candidates and preservation-first `SQLite` installation (ADR 0066).
//!
//! The destination stays at its configured inode under task 011's exclusive
//! connection. Only private artifacts are opened outside `SQLite`; opening and
//! closing a raw destination descriptor would release POSIX `SQLite` locks.

#![warn(clippy::pedantic)]

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{atomic::AtomicBool, Arc};

use rusqlite::{backup::Backup, backup::StepResult, types::ValueRef, Connection};
use sha2::{Digest, Sha256};

use super::{
    build_snapshot, checked_destination, inspect_connection, open_source, same_file, sidecars,
    Budget, Candidate, ExclusiveDatabase, Failure, FailureKind, Preservation, SchemaCompatibility,
    BACKUP_PAGE_BATCH, LOCK_WAIT,
};

/// A candidate can only be constructed by successful review. Its private files
/// are retained as evidence, including when the review is abandoned or rejected.
#[derive(Debug)]
pub(crate) struct ValidatedRestore {
    pub(crate) backup: PathBuf,
    chosen_backup: PathBuf,
    pub(crate) destination: PathBuf,
    pub(crate) preservation: PathBuf,
    candidate: Candidate,
    backup_revision: FileRevision,
    candidate_revision: FileRevision,
    destination_identity: fs::Metadata,
    destination_contents: Option<String>,
    contents: String,
}

#[derive(Debug)]
struct FileRevision {
    identity: fs::Metadata,
    digest: String,
}

#[derive(Debug)]
pub(crate) enum InstallState {
    NotInstalled(Failure),
    Failed {
        failure: Failure,
        rollback_verified: bool,
    },
    VerificationFailed(Failure),
    Verified,
}

#[derive(Debug)]
pub(crate) struct RestoreResult {
    pub(crate) preservation: Option<Preservation>,
    pub(crate) verified_original: Option<PathBuf>,
    pub(crate) state: InstallState,
}

impl RestoreResult {
    pub(crate) fn refused(failure: Failure) -> Self {
        Self {
            preservation: None,
            verified_original: None,
            state: InstallState::NotInstalled(failure),
        }
    }
}

impl ValidatedRestore {
    pub(crate) fn review(
        backup: &Path,
        destination: &Path,
        preservation: &Path,
        budget: &Budget,
    ) -> Result<Self, Failure> {
        Self::review_with(backup, destination, preservation, budget, false)
    }

    pub(crate) fn upgrade_backup(
        backup: &Path,
        destination: &Path,
        preservation: &Path,
        budget: &Budget,
    ) -> Result<Self, Failure> {
        Self::review_with(backup, destination, preservation, budget, true)
    }

    fn review_with(
        backup: &Path,
        destination: &Path,
        preservation: &Path,
        budget: &Budget,
        upgrade: bool,
    ) -> Result<Self, Failure> {
        let destination = fs::canonicalize(destination)
            .map_err(|e| Failure::io("Locate configured restore destination", &e))?;
        let destination_identity = regular_identity(&destination)?;
        let chosen_backup = backup.to_owned();
        let backup = fs::canonicalize(backup)
            .map_err(|e| Failure::io("Locate chosen restore backup", &e))?;
        let backup_identity = regular_identity(&backup)?;
        if same_file(&backup_identity, &destination_identity)
            || sidecars(&destination).contains(&backup)
        {
            return Err(changed(
                "Reject backup alias of the configured database or journals",
            ));
        }
        let backup_revision = FileRevision::read(&backup, budget)?;
        let preservation = checked_destination(&destination, preservation)?;
        checked_destination(&backup, &preservation)?;
        let destination_contents = match open_source(&destination, budget) {
            Ok(conn) => match content_digest(&conn, budget) {
                Ok(value) => Some(value),
                Err(failure) if failure.kind == FailureKind::Corrupt => None,
                Err(failure) => return Err(failure),
            },
            Err(failure) if failure.kind == FailureKind::Corrupt => None,
            Err(failure) => return Err(failure),
        };
        let candidate = Candidate::reserve(preservation.parent().expect("checked parent"), budget)?;
        let result = (|| {
            let source = open_source(&backup, budget)?;
            match super::super::inspect_schema(&source)
                .map_err(|e| budget.sql("Read chosen backup schema", &e))?
            {
                SchemaCompatibility::Current if !upgrade => {}
                SchemaCompatibility::UpgradeRequired { .. } if upgrade => {}
                SchemaCompatibility::InterruptedUpgrade | SchemaCompatibility::Current => {
                    return Err(Failure::new("Upgrade backup requires a valid older schema; interrupted migration 11 uses Repair interrupted upgrade", FailureKind::Validation));
                }
                SchemaCompatibility::UpgradeRequired { .. } => {
                    return Err(Failure::new(
                        "Older backup requires explicit migration preparation before restore",
                        FailureKind::Validation,
                    ))
                }
                SchemaCompatibility::Newer { .. } => {
                    return Err(Failure::new(
                        "Backup schema is newer than this app supports",
                        FailureKind::Validation,
                    ))
                }
                SchemaCompatibility::Unknown | SchemaCompatibility::Empty => {
                    return Err(Failure::new(
                        "Backup schema or migration ledger is unknown or empty",
                        FailureKind::Validation,
                    ))
                }
            }
            let mut inspection = build_snapshot(&source, &candidate.path, budget)?;
            if upgrade {
                migrate_candidate(&candidate.path, budget)?;
                inspection =
                    inspect_connection(&open_source(&candidate.path, budget)?, budget, false);
            }
            if inspection.schema != Some(Ok(SchemaCompatibility::Current)) {
                return Err(Failure::new("Restore requires the current schema; older backups need explicit migration preparation", FailureKind::Validation));
            }
            backup_revision.verify(&backup, budget)?;
            let candidate_revision = FileRevision::read(&candidate.path, budget)?;
            let contents = content_digest(&open_source(&candidate.path, budget)?, budget)?;
            Ok((candidate_revision, contents))
        })();
        match result {
            Ok((candidate_revision, contents)) => Ok(Self {
                backup,
                chosen_backup,
                destination,
                preservation,
                candidate,
                backup_revision,
                candidate_revision,
                destination_identity,
                destination_contents,
                contents,
            }),
            Err(mut failure) => {
                failure.remaining.extend(candidate.cleanup());
                Err(failure)
            }
        }
    }

    pub(crate) fn candidate_path(&self) -> &Path {
        &self.candidate.path
    }

    /// Recheck private inputs before acquiring destination access. Never hash
    /// the live destination through a raw file descriptor.
    pub(crate) fn revalidate(&self, budget: &Budget) -> Result<(), Failure> {
        self.revalidate_inputs(budget)?;
        checked_destination(&self.destination, &self.preservation)?;
        Ok(())
    }

    fn revalidate_inputs(&self, budget: &Budget) -> Result<(), Failure> {
        if fs::canonicalize(&self.chosen_backup).ok().as_ref() != Some(&self.backup) {
            return Err(changed(
                "Chosen backup path changed since review; review again",
            ));
        }
        self.backup_revision.verify(&self.backup, budget)?;
        self.candidate.verify()?;
        self.candidate_revision
            .verify(&self.candidate.path, budget)?;
        self.verify_destination_identity()
    }

    fn verify_destination_identity(&self) -> Result<(), Failure> {
        let now = regular_identity(&self.destination)?;
        if !same_file(&now, &self.destination_identity) {
            return Err(changed(
                "Configured database changed since review; review again",
            ));
        }
        Ok(())
    }

    pub(crate) fn install(
        &self,
        access: ExclusiveDatabase,
        budget: &Budget,
        interrupt_after_step: bool,
    ) -> RestoreResult {
        self.install_with(access, budget, || {
            if interrupt_after_step {
                Err(Failure::new(
                    "Fixture interrupted SQLite installation between page batches",
                    FailureKind::Io,
                ))
            } else {
                Ok(())
            }
        })
    }

    fn install_with(
        &self,
        access: ExclusiveDatabase,
        budget: &Budget,
        after_step: impl Fn() -> Result<(), Failure>,
    ) -> RestoreResult {
        self.install_prepared(
            access,
            budget,
            after_step,
            RestoreResult::refused(changed("Restore prerequisites were not completed")),
        )
    }

    fn install_prepared(
        &self,
        mut access: ExclusiveDatabase,
        budget: &Budget,
        after_step: impl Fn() -> Result<(), Failure>,
        mut result: RestoreResult,
    ) -> RestoreResult {
        let prepare = (|| {
            if access.source() != self.destination {
                return Err(changed(
                    "Exclusive access belongs to a different destination",
                ));
            }
            self.verify_destination_identity()?;
            self.revalidate_inputs(budget)?;
            // A changed database, including work completed during drain, needs a
            // new review in recovery. Checkpoint-only changes keep the same digest.
            let before = match content_digest(&access.connection, budget) {
                Ok(value) => Some(value),
                Err(failure) if failure.kind == FailureKind::Corrupt => None,
                Err(failure) => return Err(failure),
            };
            if before != self.destination_contents {
                return Err(changed(
                    "Database records changed since review; review again in recovery",
                ));
            }
            let source = open_source(&self.candidate.path, budget)?;
            source
                .execute_batch("BEGIN")
                .map_err(|e| budget.sql("Hold reviewed candidate snapshot", &e))?;
            if content_digest(&source, budget)? != self.contents {
                return Err(changed(
                    "Candidate records changed since review; review again",
                ));
            }
            if result.preservation.is_none() {
                result.preservation = Some(access.preserve(&self.preservation, budget)?);
            }
            if result.verified_original.is_none()
                && needs_original_snapshot(&access.connection, budget)?
            {
                let original = Candidate::reserve(&self.preservation, budget)?;
                // A readable original must have a verified SQLite snapshot as well
                // as its labelled files. A failed snapshot prevents installation.
                build_snapshot(&access.connection, &original.path, budget).map_err(|mut e| {
                    e.remaining.push(original.directory.clone());
                    e
                })?;
                for directory in [&original.directory, &self.preservation] {
                    File::open(directory)
                        .and_then(|file| file.sync_all())
                        .map_err(|error| {
                            let mut failure = Failure::io(
                                "Sync verified original snapshot directory before installation",
                                &error,
                            );
                            failure.remaining.push(original.directory.clone());
                            failure
                        })?;
                }
                result.verified_original = Some(original.path);
            }
            self.verify_destination_identity()?;
            budget.check("Install reviewed database after preservation")?;
            Ok((source, before))
        })();
        let (source, before) = match prepare {
            Ok(value) => value,
            Err(failure) => {
                result.state = InstallState::NotInstalled(failure);
                return result;
            }
        };
        let installed = copy_into(&source, &mut access.connection, budget, after_step);
        // Backup has finished/dropped before any destination verification. Use a
        // fresh finite budget even when cancellation/deadline ended installation.
        let verification = Budget::new(Arc::new(AtomicBool::new(false)));
        if let Err(failure) = verification.configure(&access.connection) {
            result.state = if installed.is_ok() {
                InstallState::VerificationFailed(failure)
            } else {
                InstallState::Failed {
                    failure,
                    rollback_verified: false,
                }
            };
            return result;
        }
        if let Err(failure) = installed {
            let after = content_digest(&access.connection, &verification).ok();
            let rollback_verified = before.is_some()
                && before == after
                && access.connection.is_autocommit()
                && self.verify_destination_identity().is_ok()
                && inspect_connection(&access.connection, &verification, true).valid_snapshot();
            result.state = InstallState::Failed {
                failure,
                rollback_verified,
            };
            return result;
        }
        result.state = match self.verify_installed(&mut access, &verification) {
            Ok(()) => InstallState::Verified,
            Err(failure) => InstallState::VerificationFailed(failure),
        };
        result
    }

    fn verify_installed(
        &self,
        access: &mut ExclusiveDatabase,
        budget: &Budget,
    ) -> Result<(), Failure> {
        self.verify_destination_identity()?;
        let inspection = inspect_connection(&access.connection, budget, true);
        if !inspection.valid_snapshot()
            || inspection.schema != Some(Ok(SchemaCompatibility::Current))
            || content_digest(&access.connection, budget)? != self.contents
        {
            return Err(Failure::new(
                "Verify installed database integrity, schema and records",
                FailureKind::Validation,
            ));
        }
        if crate::db::startup::check_connection(&mut access.connection)
            != Ok(crate::db::startup::DatabaseReadiness::Ready)
        {
            return Err(Failure::new(
                "Verify installed database reads and rolled-back writes",
                FailureKind::Validation,
            ));
        }
        budget.check("Finish installed database verification")
    }
}

/// Preserve the exclusive original before any candidate migration. Installation
/// uses the same verified `SQLite` path as an ordinary reviewed restore.
pub(crate) fn repair_interrupted_upgrade(
    mut access: ExclusiveDatabase,
    preservation: &Path,
    budget: &Budget,
    interrupt_after_step: bool,
) -> RestoreResult {
    let mut result = RestoreResult::refused(Failure::new(
        "Recognize migration 11 broadcast_event_selection before repair",
        FailureKind::Validation,
    ));
    let prepared = (|| {
        let inspection = inspect_connection(&access.connection, budget, true);
        if !inspection.valid_snapshot()
            || inspection.schema != Some(Ok(SchemaCompatibility::InterruptedUpgrade))
        {
            return Err(Failure::new("Only migration 11 broadcast_event_selection after table creation and before recording is supported; preserve files or review a known backup", FailureKind::Validation));
        }
        let destination = access.source().to_owned();
        let destination_identity = regular_identity(&destination)?;
        let destination_contents = Some(content_digest(&access.connection, budget)?);
        let preserved_data = database_digest(&access.connection, budget, true)?;
        let copy = access.preserve(preservation, budget)?;
        let preservation = copy.directory.clone();
        result.preservation = Some(copy);
        let original = Candidate::reserve(&preservation, budget)?;
        build_snapshot(&access.connection, &original.path, budget).map_err(|mut failure| {
            failure.remaining.push(original.directory.clone());
            failure
        })?;
        result.verified_original = Some(original.path.clone());
        let backup_revision = FileRevision::read(&original.path, budget)?;
        let candidate = Candidate::reserve(&preservation, budget)?;
        let prepare = (|| {
            build_snapshot(&access.connection, &candidate.path, budget)?;
            // Both the manifest and verified original are durable before migration.
            for directory in [&original.directory, &preservation] {
                File::open(directory)
                    .and_then(|file| file.sync_all())
                    .map_err(|e| Failure::io("Sync preserved original before migration", &e))?;
            }
            migrate_candidate(&candidate.path, budget)?;
            let candidate_connection = open_source(&candidate.path, budget)?;
            if database_digest(&candidate_connection, budget, true)? != preserved_data {
                return Err(Failure::new("Verify repair preserved all rows, event selections and prior migration records", FailureKind::Validation));
            }
            Ok((
                FileRevision::read(&candidate.path, budget)?,
                content_digest(&candidate_connection, budget)?,
            ))
        })();
        let (candidate_revision, contents) = prepare.map_err(|mut failure: Failure| {
            failure.remaining.push(candidate.directory.clone());
            failure
        })?;
        Ok(ValidatedRestore {
            chosen_backup: original.path.clone(),
            backup: original.path,
            destination,
            preservation,
            candidate,
            backup_revision,
            candidate_revision,
            destination_identity,
            destination_contents,
            contents,
        })
    })();
    match prepared {
        Ok(review) => review.install_prepared(
            access,
            budget,
            || {
                if interrupt_after_step {
                    Err(Failure::new(
                        "Fixture interrupted repair installation between page batches",
                        FailureKind::Io,
                    ))
                } else {
                    Ok(())
                }
            },
            result,
        ),
        Err(failure) => {
            result.state = InstallState::NotInstalled(failure);
            result
        }
    }
}

fn migrate_candidate(path: &Path, budget: &Budget) -> Result<(), Failure> {
    let mut candidate =
        Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(|e| budget.sql("Open private upgrade candidate", &e))?;
    budget.configure(&candidate)?;
    budget.check("Apply normal migrations to private candidate")?;
    super::super::migrate_schema(&candidate).map_err(|_| {
        budget
            .check("Apply normal migration registry to private candidate")
            .err()
            .unwrap_or_else(|| {
                Failure::new(
                    "Apply normal migration registry to private candidate",
                    FailureKind::Validation,
                )
            })
    })?;
    let inspection = inspect_connection(&candidate, budget, true);
    if !inspection.valid_snapshot()
        || inspection.schema != Some(Ok(SchemaCompatibility::Current))
        || !super::super::upgrades::recognize_migration_11(&candidate)
            .map_err(|e| budget.sql("Verify upgraded schema against the normal authority", &e))?
        || crate::db::startup::check_connection(&mut candidate)
            != Ok(crate::db::startup::DatabaseReadiness::Ready)
    {
        return Err(Failure::new(
            "Validate upgraded candidate schema, integrity, foreign keys and read/write access",
            FailureKind::Validation,
        ));
    }
    candidate
        .close()
        .map_err(|(_, e)| budget.sql("Close upgraded candidate", &e))?;
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| Failure::io("Sync upgraded candidate", &e))
}

fn needs_original_snapshot(connection: &Connection, budget: &Budget) -> Result<bool, Failure> {
    let inspection = inspect_connection(connection, budget, true);
    for failure in [
        inspection.access.as_ref().err(),
        inspection
            .integrity
            .as_ref()
            .and_then(|value| value.as_ref().err()),
        inspection
            .foreign_keys
            .as_ref()
            .and_then(|value| value.as_ref().err()),
        inspection
            .schema
            .as_ref()
            .and_then(|value| value.as_ref().err()),
    ]
    .into_iter()
    .flatten()
    {
        if failure.kind != FailureKind::Corrupt {
            return Err(failure.clone());
        }
    }
    Ok(inspection.valid_snapshot())
}

fn copy_into(
    source: &Connection,
    destination: &mut Connection,
    budget: &Budget,
    after_step: impl Fn() -> Result<(), Failure>,
) -> Result<(), Failure> {
    const OP: &str = "Install candidate through SQLite backup";
    let backup = Backup::new(source, destination).map_err(|e| budget.sql(OP, &e))?;
    loop {
        budget.check(OP)?;
        match backup
            .step(BACKUP_PAGE_BATCH)
            .map_err(|e| budget.sql(OP, &e))?
        {
            StepResult::Done => return Ok(()),
            StepResult::More => after_step()?,
            StepResult::Busy | StepResult::Locked => std::thread::sleep(LOCK_WAIT),
            _ => return Err(Failure::new(OP, FailureKind::Sql)),
        }
    }
} // Backup::drop calls sqlite3_backup_finish, rolling back an incomplete install.

fn changed(operation: &'static str) -> Failure {
    Failure::new(operation, FailureKind::UnstableFiles)
}

fn regular_identity(path: &Path) -> Result<fs::Metadata, Failure> {
    let metadata =
        fs::symlink_metadata(path).map_err(|e| Failure::io("Inspect restore file identity", &e))?;
    if !metadata.is_file() {
        return Err(changed("Restore requires a regular file"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(changed("Restore refuses hard-link aliases"));
        }
    }
    Ok(metadata)
}

impl FileRevision {
    fn read(path: &Path, budget: &Budget) -> Result<Self, Failure> {
        let identity = regular_identity(path)?;
        // A chosen backup is a closed standalone snapshot. Reading live WAL or
        // hot journals could modify its sidecars; require an explicit backup first.
        for sidecar in sidecars(path) {
            match fs::symlink_metadata(sidecar) {
                Ok(_) => {
                    return Err(changed(
                        "Choose a standalone backup without SQLite journals",
                    ))
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(Failure::io("Inspect backup journals", &e)),
            }
        }
        let mut file =
            File::open(path).map_err(|e| Failure::io("Read reviewed backup fingerprint", &e))?;
        if !same_file(
            &identity,
            &file
                .metadata()
                .map_err(|e| Failure::io("Inspect opened backup", &e))?,
        ) {
            return Err(changed("Backup identity changed during review"));
        }
        let mut digest = Sha256::new();
        let mut buffer = vec![0; 64 * 1024];
        let mut first = true;
        loop {
            budget.check("Fingerprint reviewed backup")?;
            let count = file
                .read(&mut buffer)
                .map_err(|e| Failure::io("Fingerprint reviewed backup", &e))?;
            if count == 0 {
                break;
            }
            if first
                && count >= 20
                && buffer.starts_with(b"SQLite format 3\0")
                && (buffer[18] == 2 || buffer[19] == 2)
            {
                return Err(Failure::new("Choose a standalone verified backup; WAL databases first need Back up database", FailureKind::Unsupported));
            }
            first = false;
            digest.update(&buffer[..count]);
        }
        Ok(Self {
            identity,
            digest: format!("{:x}", digest.finalize()),
        })
    }
    fn verify(&self, path: &Path, budget: &Budget) -> Result<(), Failure> {
        if !same_file(&regular_identity(path)?, &self.identity) {
            return Err(changed(
                "Backup or candidate identity changed since review; review again",
            ));
        }
        let now = Self::read(path, budget)?;
        if same_file(&now.identity, &self.identity) && now.digest == self.digest {
            Ok(())
        } else {
            Err(changed(
                "Backup or candidate changed since review; review again",
            ))
        }
    }
}

/// Compare schema, row identities, all stored values and the migration ledger
/// independent of journal mode, checkpointing and `SQLite` header counters.
fn content_digest(conn: &Connection, budget: &Budget) -> Result<String, Failure> {
    database_digest(conn, budget, false)
}

fn database_digest(
    conn: &Connection,
    budget: &Budget,
    omit_migration_11: bool,
) -> Result<String, Failure> {
    const OP: &str = "Fingerprint database records and schema";
    budget.check(OP)?;
    let _snapshot = if conn.is_autocommit() {
        Some(
            conn.unchecked_transaction()
                .map_err(|e| budget.sql(OP, &e))?,
        )
    } else {
        None
    };
    let run = || -> rusqlite::Result<String> {
        let mut hash = Sha256::new();
        let mut schema = conn
            .prepare("SELECT type, name, tbl_name, sql FROM sqlite_schema ORDER BY type, name")?;
        let mut rows = schema.query([])?;
        let mut tables = Vec::new();
        while let Some(row) = rows.next()? {
            for index in 0..4 {
                hash_value(&mut hash, row.get_ref(index)?);
            }
            if row.get::<_, String>(0)? == "table" {
                tables.push(row.get::<_, String>(1)?);
            }
        }
        for table in tables {
            let quoted = table.replace('"', "\"\"");
            let filter = if omit_migration_11 && table == "schema_migrations" {
                " WHERE version != 11"
            } else {
                ""
            };
            let mut statement =
                conn.prepare(&format!("SELECT rowid, * FROM \"{quoted}\"{filter}"))?;
            let columns = statement.column_count();
            let mut rows = statement.query([])?;
            let mut records = Vec::new();
            while let Some(row) = rows.next()? {
                if budget.check(OP).is_err() {
                    return Err(rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERRUPT),
                        None,
                    ));
                }
                let mut record = Sha256::new();
                for index in 0..columns {
                    hash_value(&mut record, row.get_ref(index)?);
                }
                records.push(record.finalize());
            }
            records.sort_unstable();
            hash.update((records.len() as u64).to_le_bytes());
            for record in records {
                hash.update(record);
            }
        }
        Ok(format!("{:x}", hash.finalize()))
    };
    run().map_err(|e| budget.sql(OP, &e))
}

fn hash_value(hash: &mut Sha256, value: ValueRef<'_>) {
    match value {
        ValueRef::Null => hash.update([0]),
        ValueRef::Integer(value) => {
            hash.update([1]);
            hash.update(value.to_le_bytes());
        }
        ValueRef::Real(value) => {
            hash.update([2]);
            hash.update(value.to_bits().to_le_bytes());
        }
        ValueRef::Text(value) => {
            hash.update([3]);
            hash.update((value.len() as u64).to_le_bytes());
            hash.update(value);
        }
        ValueRef::Blob(value) => {
            hash.update([4]);
            hash.update((value.len() as u64).to_le_bytes());
            hash.update(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::preservation::tests::Peer;
    use super::*;
    use std::cell::Cell;
    use std::sync::atomic::Ordering;

    fn budget() -> Budget {
        Budget::new(Arc::new(AtomicBool::new(false)))
    }

    fn files(mode: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("library.sqlite");
        let backup = temp.path().join("chosen.sqlite");
        let conn = crate::db::open_db(&destination).unwrap();
        conn.pragma_update(None, "journal_mode", mode).unwrap();
        conn.execute(
            "INSERT INTO playlists(name) VALUES ('previous library')",
            [],
        )
        .unwrap();
        drop(conn);
        let conn = crate::db::open_db(&backup).unwrap();
        conn.execute_batch("WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i+1 FROM n WHERE i<1500) INSERT INTO playlists(name) SELECT printf('%04d ', i) || hex(zeroblob(1024)) FROM n;").unwrap();
        drop(conn);
        (temp, backup, destination)
    }

    fn interrupted(path: &Path) {
        let conn = crate::db::open_db(path).unwrap();
        crate::db::upgrades::interrupt_fixture(&conn, crate::db::MigrationBoundary::AfterApply)
            .unwrap();
        conn.execute(
            "INSERT INTO playlists(name,description) VALUES ('retained',hex(zeroblob(524288)))",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO broadcast_event_selection VALUES (1,'selected-event',9)",
            [],
        )
        .unwrap();
    }

    #[test]
    fn adr_0066_upgrade_repair_preserves_before_install_and_verifies_failure_rollback() {
        for interrupt in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("db.sqlite");
            interrupted(&path);
            assert!(crate::db::startup::check_database(&path).is_err());
            let before = fs::read(&path).unwrap();
            let before_contents =
                content_digest(&open_source(&path, &budget()).unwrap(), &budget()).unwrap();
            let before_data =
                database_digest(&open_source(&path, &budget()).unwrap(), &budget(), true).unwrap();
            let identity = fs::metadata(&path).unwrap();
            let preserved = temp.path().join("preserved");
            let result = repair_interrupted_upgrade(
                ExclusiveDatabase::acquire(&path, &budget()).unwrap(),
                &preserved,
                &budget(),
                interrupt,
            );
            assert!(result.preservation.as_ref().unwrap().manifest.exists());
            let original = result.verified_original.as_ref().unwrap();
            assert_eq!(
                content_digest(&open_source(original, &budget()).unwrap(), &budget()).unwrap(),
                before_contents
            );
            assert_eq!(fs::read(preserved.join("database.sqlite")).unwrap(), before);
            assert!(same_file(&identity, &fs::metadata(&path).unwrap()));
            let connection = open_source(&path, &budget()).unwrap();
            assert_eq!(
                database_digest(&connection, &budget(), true).unwrap(),
                before_data
            );
            if interrupt {
                assert!(
                    matches!(
                        result.state,
                        InstallState::Failed {
                            rollback_verified: true,
                            ..
                        }
                    ),
                    "{result:?}"
                );
                assert_eq!(
                    content_digest(&connection, &budget()).unwrap(),
                    before_contents
                );
                assert!(crate::db::startup::check_database(&path).is_err());
            } else {
                assert!(matches!(result.state, InstallState::Verified), "{result:?}");
                assert_eq!(
                    crate::db::startup::check_database(&path).unwrap(),
                    crate::db::startup::DatabaseReadiness::Ready
                );
            }
        }
    }

    #[test]
    fn adr_0066_upgrade_failed_preservation_and_unsupported_schema_do_not_mutate() {
        for unsupported in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("db.sqlite");
            interrupted(&path);
            let preserved = temp.path().join("preserved");
            if unsupported {
                Connection::open(&path)
                    .unwrap()
                    .execute("DELETE FROM schema_migrations WHERE version=5", [])
                    .unwrap();
            } else {
                fs::write(&preserved, "occupied").unwrap();
            }
            let before = fs::read(&path).unwrap();
            let result = repair_interrupted_upgrade(
                ExclusiveDatabase::acquire(&path, &budget()).unwrap(),
                &preserved,
                &budget(),
                false,
            );
            assert!(
                matches!(result.state, InstallState::NotInstalled(_)),
                "{result:?}"
            );
            assert!(result.verified_original.is_none());
            assert_eq!(fs::read(&path).unwrap(), before);
            if unsupported {
                assert!(!preserved.exists());
            }
        }
    }

    #[test]
    fn adr_0066_upgrade_backup_requires_explicit_candidate_and_preserves_chosen_source() {
        let (temp, backup, destination) = files("DELETE");
        let conn = Connection::open(&backup).unwrap();
        crate::db::upgrades::interrupt_fixture(&conn, crate::db::MigrationBoundary::BeforeApply)
            .unwrap();
        drop(conn);
        let bytes = fs::read(&backup).unwrap();
        let destination_bytes = fs::read(&destination).unwrap();
        let preserved = temp.path().join("preserved");
        assert!(ValidatedRestore::review(&backup, &destination, &preserved, &budget()).is_err());
        let review =
            ValidatedRestore::upgrade_backup(&backup, &destination, &preserved, &budget()).unwrap();
        assert_eq!(fs::read(&backup).unwrap(), bytes);
        assert_eq!(fs::read(&destination).unwrap(), destination_bytes);
        let candidate = open_source(review.candidate_path(), &budget()).unwrap();
        assert_eq!(
            crate::db::inspect_schema(&candidate).unwrap(),
            SchemaCompatibility::Current
        );
        assert_eq!(
            candidate
                .query_row("SELECT count(*) FROM playlists", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1500
        );
        drop(candidate);
        let result = review.install(
            ExclusiveDatabase::acquire(&destination, &budget()).unwrap(),
            &budget(),
            false,
        );
        assert!(matches!(result.state, InstallState::Verified), "{result:?}");
        assert_eq!(fs::read(&backup).unwrap(), bytes);
    }

    #[test]
    fn adr_0066_restore_preserves_installs_and_verifies_without_replacing_inode() {
        for mode in ["DELETE", "WAL"] {
            let (temp, backup, destination) = files(mode);
            let original_backup = fs::read(&backup).unwrap();
            let identity = fs::metadata(&destination).unwrap();
            let old =
                content_digest(&open_source(&destination, &budget()).unwrap(), &budget()).unwrap();
            let review = ValidatedRestore::review(
                &backup,
                &destination,
                &temp.path().join("preservation"),
                &budget(),
            )
            .unwrap();
            let access = ExclusiveDatabase::acquire(&destination, &budget()).unwrap();
            let steps = Cell::new(0);
            let result = review.install_with(access, &budget(), || {
                steps.set(steps.get() + 1);
                Peer::probe(&destination, "blocked");
                Ok(())
            });
            assert!(matches!(result.state, InstallState::Verified), "{result:?}");
            assert!(steps.get() > 1);
            assert!(same_file(&identity, &fs::metadata(&destination).unwrap()));
            assert_eq!(
                content_digest(&open_source(&destination, &budget()).unwrap(), &budget()).unwrap(),
                review.contents
            );
            assert_eq!(
                content_digest(
                    &open_source(&result.verified_original.unwrap(), &budget()).unwrap(),
                    &budget()
                )
                .unwrap(),
                old
            );
            assert!(result.preservation.unwrap().manifest.exists());
            assert_eq!(fs::read(&backup).unwrap(), original_backup);
            Peer::probe(&destination, "open");
        }
    }

    #[test]
    fn adr_0066_restore_interrupted_and_cancelled_steps_verify_sqlite_rollback() {
        for mode in ["DELETE", "WAL"] {
            for cancel in [false, true] {
                let (temp, backup, destination) = files(mode);
                let review = ValidatedRestore::review(
                    &backup,
                    &destination,
                    &temp.path().join("preserved"),
                    &budget(),
                )
                .unwrap();
                let work = budget();
                let access = ExclusiveDatabase::acquire(&destination, &work).unwrap();
                let result = review.install_with(access, &work, || {
                    Peer::probe(&destination, "blocked");
                    if cancel {
                        work.cancelled.store(true, Ordering::Release);
                        Ok(())
                    } else {
                        Err(Failure::new("Injected step failure", FailureKind::Io))
                    }
                });
                assert!(
                    matches!(
                        result.state,
                        InstallState::Failed {
                            rollback_verified: true,
                            ..
                        }
                    ),
                    "{result:?}"
                );
                assert!(result.verified_original.unwrap().exists());
                assert!(result.preservation.unwrap().manifest.exists());
                assert!(review.candidate.path.exists());
                assert_eq!(
                    content_digest(&open_source(&destination, &budget()).unwrap(), &budget()).ok(),
                    review.destination_contents
                );
                Peer::probe(&destination, "open");
            }
        }
    }

    #[test]
    fn adr_0066_restore_rejects_invalid_newer_older_and_aliased_candidates() {
        let (temp, backup, destination) = files("DELETE");
        let preservation = temp.path().join("preserved");
        let owner = Connection::open(&destination).unwrap();
        owner.execute_batch("BEGIN EXCLUSIVE").unwrap();
        assert!(
            ValidatedRestore::review(&destination, &destination, &preservation, &budget()).is_err()
        );
        // No raw descriptor may close and release this process's SQLite lock.
        Peer::probe(&destination, "blocked");
        drop(owner);
        let invalid = temp.path().join("invalid.sqlite");
        fs::write(&invalid, b"invalid SQLite").unwrap();
        assert!(
            ValidatedRestore::review(&invalid, &destination, &preservation, &budget()).is_err()
        );
        assert_eq!(fs::read(invalid).unwrap(), b"invalid SQLite");
        let conn = Connection::open(&backup).unwrap();
        conn.execute(
            "INSERT INTO schema_migrations(version, name) VALUES (999, 'fixture')",
            [],
        )
        .unwrap();
        drop(conn);
        let newer = fs::read(&backup).unwrap();
        assert!(ValidatedRestore::review(&backup, &destination, &preservation, &budget()).is_err());
        assert_eq!(fs::read(&backup).unwrap(), newer);
        let conn = Connection::open(&backup).unwrap();
        conn.execute("DELETE FROM schema_migrations WHERE version>=11", [])
            .unwrap();
        drop(conn);
        assert!(ValidatedRestore::review(&backup, &destination, &preservation, &budget()).is_err());
        let alias = temp.path().join("alias.sqlite");
        fs::hard_link(&destination, &alias).unwrap();
        assert!(ValidatedRestore::review(&alias, &destination, &preservation, &budget()).is_err());
        assert!(!preservation.exists());
    }

    #[test]
    fn adr_0066_restore_wal_backups_require_a_standalone_snapshot_without_touching_source() {
        let (temp, backup, destination) = files("DELETE");
        let conn = Connection::open(&backup).unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        drop(conn);
        let before = fs::read(&backup).unwrap();
        let failure = ValidatedRestore::review(
            &backup,
            &destination,
            &temp.path().join("preserved"),
            &budget(),
        )
        .unwrap_err();
        assert_eq!(failure.kind, FailureKind::Unsupported);
        assert_eq!(fs::read(&backup).unwrap(), before);
        assert!(sidecars(&backup).iter().all(|path| !path.exists()));
    }

    #[test]
    fn adr_0066_restore_changed_files_or_records_require_new_review() {
        for changed_input in ["backup", "candidate", "destination", "records"] {
            let (temp, backup, destination) = files("DELETE");
            let review = ValidatedRestore::review(
                &backup,
                &destination,
                &temp.path().join("preserved"),
                &budget(),
            )
            .unwrap();
            let path = match changed_input {
                "backup" => &backup,
                "candidate" => &review.candidate.path,
                _ => &destination,
            };
            if changed_input == "destination" {
                fs::rename(path, temp.path().join("retained.sqlite")).unwrap();
                drop(crate::db::open_db(path).unwrap());
            } else {
                Connection::open(path)
                    .unwrap()
                    .execute(
                        "INSERT INTO playlists(name) VALUES ('changed after review')",
                        [],
                    )
                    .unwrap();
            }
            if changed_input == "records" {
                review.revalidate(&budget()).unwrap();
                let result = review.install(
                    ExclusiveDatabase::acquire(&destination, &budget()).unwrap(),
                    &budget(),
                    false,
                );
                assert!(matches!(result.state, InstallState::NotInstalled(_)));
            } else {
                assert!(review.revalidate(&budget()).is_err());
            }
            assert!(!review.preservation.exists());
        }
    }

    #[test]
    fn adr_0066_restore_failed_preservation_never_installs() {
        let (temp, backup, destination) = files("DELETE");
        let review = ValidatedRestore::review(
            &backup,
            &destination,
            &temp.path().join("preserved"),
            &budget(),
        )
        .unwrap();
        let before = fs::read(&destination).unwrap();
        fs::write(&review.preservation, b"occupied preservation location").unwrap();
        let access = ExclusiveDatabase::acquire(&destination, &budget()).unwrap();
        let result = review.install_with(access, &budget(), || {
            panic!("preservation failure entered install")
        });
        assert!(matches!(result.state, InstallState::NotInstalled(_)));
        assert_eq!(fs::read(&destination).unwrap(), before);
        assert_eq!(
            fs::read(&review.preservation).unwrap(),
            b"occupied preservation location"
        );
    }

    #[test]
    fn adr_0066_restore_verification_requires_candidate_records_and_usable_writes() {
        let (temp, backup, destination) = files("DELETE");
        let review = ValidatedRestore::review(
            &backup,
            &destination,
            &temp.path().join("preserved"),
            &budget(),
        )
        .unwrap();
        let mut access = ExclusiveDatabase::acquire(&destination, &budget()).unwrap();
        assert!(review.verify_installed(&mut access, &budget()).is_err());
        let source = open_source(review.candidate_path(), &budget()).unwrap();
        copy_into(&source, &mut access.connection, &budget(), || Ok(())).unwrap();
        access
            .connection
            .pragma_update(None, "query_only", true)
            .unwrap();
        assert!(review.verify_installed(&mut access, &budget()).is_err());
        access
            .connection
            .pragma_update(None, "query_only", false)
            .unwrap();
        review.verify_installed(&mut access, &budget()).unwrap();
        Peer::probe(&destination, "blocked");
    }

    #[test]
    fn adr_0066_restore_lockable_damage_keeps_file_evidence() {
        let (temp, backup, destination) = files("DELETE");
        let mut damaged = fs::read(&destination).unwrap();
        damaged[32..36].copy_from_slice(&0x7fff_ffff_u32.to_be_bytes());
        damaged[36..40].copy_from_slice(&1_u32.to_be_bytes());
        fs::write(&destination, &damaged).unwrap();
        assert!(
            crate::db::startup::check_database(&destination).is_err(),
            "fixture damage must enter core recovery"
        );
        let review = ValidatedRestore::review(
            &backup,
            &destination,
            &temp.path().join("preserved"),
            &budget(),
        )
        .unwrap();
        let result = review.install(
            ExclusiveDatabase::acquire(&destination, &budget()).unwrap(),
            &budget(),
            false,
        );
        assert!(matches!(result.state, InstallState::Verified), "{result:?}");
        assert!(result.verified_original.is_none());
        assert_eq!(
            fs::read(
                result
                    .preservation
                    .unwrap()
                    .directory
                    .join("database.sqlite")
            )
            .unwrap(),
            damaged
        );
    }
}
