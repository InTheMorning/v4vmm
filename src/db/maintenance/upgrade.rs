//! Guarded database preparation and recorded preservation receipts (ADRs 0016, 0075).
//!
//! Startup and CLI preparation share this owner. Existing records receive a
//! durable snapshot before migration. The exclusive connection holds its locks
//! until schema, records, and committed writes have been checked.

#![warn(clippy::pedantic)]

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use std::time::SystemTime;

use rusqlite::Connection;

use super::{
    build_snapshot, inspect_connection, restore::content_digest, same_file, Budget, Candidate,
    ExclusiveDatabase, Failure, FailureKind, CANDIDATE_SEQUENCE,
};
use crate::db::{startup, upgrades, MigrationBoundary, SchemaCompatibility, CURRENT_VERSION};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparationState {
    Unchanged,
    Created,
    Upgraded,
    Stopped,
    RollbackVerified,
    RollbackUnverified,
    VerificationFailed,
}

/// Actual preparation times and retained artifacts, independent of presentation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct PreparationReceipt {
    pub source: PathBuf,
    pub target: i64,
    pub started_at: SystemTime,
    pub finished_at: SystemTime,
    pub preserved_at: Option<SystemTime>,
    pub snapshot_verified_at: Option<SystemTime>,
    pub preservation: Option<PathBuf>,
    pub manifest: Option<PathBuf>,
    pub snapshot: Option<PathBuf>,
    pub state: PreparationState,
}

#[derive(Debug)]
#[must_use]
pub struct PreparedDatabase {
    pub connection: Connection,
    pub receipt: PreparationReceipt,
}

impl std::ops::Deref for PreparedDatabase {
    type Target = Connection;
    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}
impl std::ops::DerefMut for PreparedDatabase {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

#[derive(Clone, Debug)]
pub struct PreparationError {
    pub check: startup::DbCheckError,
    pub receipt: Box<PreparationReceipt>,
    pub(crate) failure: Failure,
}
impl std::ops::Deref for PreparationError {
    type Target = startup::DbCheckError;
    fn deref(&self) -> &Self::Target {
        &self.check
    }
}
impl std::fmt::Display for PreparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {:?}", self.failure.operation, self.failure.kind)
    }
}
impl std::error::Error for PreparationError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Boundary {
    Preservation,
    Snapshot,
    Sync,
    BeforeMigration,
    Migration(i64, MigrationBoundary),
    AfterCommit,
    BeforeReopen,
}

pub(in crate::db) fn prepare(path: &Path) -> Result<PreparedDatabase, PreparationError> {
    prepare_with(path, &Budget::new(Arc::new(AtomicBool::new(false))), |_| {
        Ok(())
    })
}

fn prepare_with(
    path: &Path,
    budget: &Budget,
    boundary: impl Fn(Boundary) -> Result<(), Failure>,
) -> Result<PreparedDatabase, PreparationError> {
    let started_at = SystemTime::now();
    let mut receipt = PreparationReceipt {
        source: path.to_owned(),
        target: CURRENT_VERSION,
        started_at,
        finished_at: started_at,
        preserved_at: None,
        snapshot_verified_at: None,
        preservation: None,
        manifest: None,
        snapshot: None,
        state: PreparationState::Stopped,
    };
    let result = prepare_inner(path, budget, &boundary, &mut receipt);
    receipt.finished_at = SystemTime::now();
    match result {
        Ok(connection) => Ok(PreparedDatabase { connection, receipt }),
        Err(failure) => Err(PreparationError {
            check: startup::DbCheckError { stage: startup::DbStage::Migrate,
                reason: "Database preparation failed. Retain the recorded artifacts and inspect the database before another attempt." },
            receipt: Box::new(receipt), failure,
        }),
    }
}

fn preparation_probe(
    path: &Path,
    budget: &Budget,
) -> Result<(Connection, SchemaCompatibility), Failure> {
    budget.check("Inspect database before preparation")?;
    let absent = match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(Failure::new(
                    "Inspect regular database file",
                    FailureKind::InvalidFile,
                ));
            }
            false
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(error) => return Err(Failure::io("Inspect database before preparation", &error)),
    };
    if absent {
        fs::create_dir_all(
            path.parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )
        .map_err(|error| Failure::io("Create database parent directory", &error))?;
    }
    let probe = startup::open(path, absent, startup::STARTUP_BUSY_TIMEOUT)
        .map_err(|_| Failure::new("Open database preparation probe", FailureKind::Access))?;
    let compatibility = crate::db::inspect_schema(&probe)
        .map_err(|error| budget.sql("Inspect database preparation schema", &error))?;
    Ok((probe, compatibility))
}

fn prepare_inner(
    path: &Path,
    budget: &Budget,
    boundary: &impl Fn(Boundary) -> Result<(), Failure>,
    receipt: &mut PreparationReceipt,
) -> Result<Connection, Failure> {
    let (mut probe, compatibility) = preparation_probe(path, budget)?;
    if compatibility == SchemaCompatibility::Current {
        verify_readiness(&mut probe)?;
        receipt.state = PreparationState::Unchanged;
        return Ok(probe);
    }
    supported(compatibility)?;
    // Close all source probes before acquiring the exclusive connection.
    probe
        .close()
        .map_err(|(_, error)| budget.sql("Close database preparation probe", &error))?;
    let mut access = ExclusiveDatabase::acquire(path, budget)?;
    access
        .connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|error| budget.sql("Enable guarded database foreign keys", &error))?;
    receipt.source = access.source().to_owned();
    let identity = fs::symlink_metadata(access.source())
        .map_err(|error| Failure::io("Inspect guarded database identity", &error))?;
    let compatibility = crate::db::inspect_schema(&access.connection)
        .map_err(|error| budget.sql("Recheck guarded database schema", &error))?;
    supported(compatibility)?;
    let fresh = compatibility == SchemaCompatibility::Empty;
    if !fresh && compatibility != SchemaCompatibility::Current {
        preserve_original(&mut access, budget, boundary, receipt)?;
    }
    budget.check("Prepare guarded database after preservation")?;
    boundary(Boundary::BeforeMigration)?;
    if fresh {
        crate::db::init_schema(&access.connection)
            .map_err(|_| Failure::new("Initialize empty database schema", FailureKind::Sql))?;
    }
    crate::db::migrate_schema_to(&access.connection, 11)
        .map_err(|_| Failure::new("Apply earlier database migrations", FailureKind::Sql))?;
    let baseline = content_digest(&access.connection, budget)?;
    let migrated =
        crate::db::migrate_schema_with(&access.connection, CURRENT_VERSION, |version, reached| {
            budget
                .check("Apply guarded database migration")
                .map_err(anyhow::Error::new)?;
            boundary(Boundary::Migration(version, reached)).map_err(anyhow::Error::new)
        });
    if let Err(error) = migrated {
        let verification = Budget::new(Arc::new(AtomicBool::new(false)));
        let verified = verification.configure(&access.connection).is_ok()
            && access.connection.is_autocommit()
            && content_digest(&access.connection, &verification)
                .ok()
                .as_ref()
                == Some(&baseline)
            && upgrades::verify_target(&access.connection, 11).is_ok()
            && fs::symlink_metadata(access.source())
                .is_ok_and(|after| same_file(&identity, &after));
        receipt.state = if verified {
            PreparationState::RollbackVerified
        } else {
            PreparationState::RollbackUnverified
        };
        return Err(error.downcast::<Failure>().unwrap_or_else(|_| {
            Failure::new(
                "Apply migration 12 and verify legacy records",
                FailureKind::Validation,
            )
        }));
    }
    // Every failure after commit retains recovery without a rollback claim.
    receipt.state = PreparationState::VerificationFailed;
    boundary(Boundary::AfterCommit)?;
    upgrades::verify_target(&access.connection, CURRENT_VERSION)
        .map_err(|_| Failure::new("Verify committed migration target", FailureKind::Validation))?;
    verify_readiness(&mut access.connection)?;
    if !fs::symlink_metadata(access.source()).is_ok_and(|after| same_file(&identity, &after)) {
        return Err(Failure::new(
            "Verify guarded database path identity",
            FailureKind::UnstableFiles,
        ));
    }
    // ExclusiveDatabase drops SQLite before its retained source descriptors.
    drop(access);
    boundary(Boundary::BeforeReopen)?;
    let mut connection = startup::open_existing(path)
        .map_err(|_| Failure::new("Reopen prepared database", FailureKind::Access))?;
    verify_readiness(&mut connection)?;
    if upgrades::verify_target(&connection, CURRENT_VERSION).is_err()
        || !fs::symlink_metadata(path).is_ok_and(|after| same_file(&identity, &after))
    {
        return Err(Failure::new(
            "Verify reopened database readiness",
            FailureKind::Validation,
        ));
    }
    receipt.state = match compatibility {
        SchemaCompatibility::Empty => PreparationState::Created,
        SchemaCompatibility::Current => PreparationState::Unchanged,
        _ => PreparationState::Upgraded,
    };
    Ok(connection)
}

fn verify_readiness(connection: &mut Connection) -> Result<(), Failure> {
    if startup::check_connection(connection) == Ok(startup::DatabaseReadiness::Ready) {
        Ok(())
    } else {
        Err(Failure::new(
            "Verify prepared database reads and writes",
            FailureKind::Validation,
        ))
    }
}

fn preserve_original(
    access: &mut ExclusiveDatabase,
    budget: &Budget,
    boundary: &impl Fn(Boundary) -> Result<(), Failure>,
    receipt: &mut PreparationReceipt,
) -> Result<(), Failure> {
    if !inspect_connection(&access.connection, budget, true).valid_snapshot() {
        return Err(Failure::new(
            "Verify original database before preservation",
            FailureKind::Validation,
        ));
    }
    let directory = loop {
        budget.check("Reserve database upgrade preservation path")?;
        let sequence = CANDIDATE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let directory = access
            .source()
            .parent()
            .expect("canonical database parent")
            .join(format!(".v4vmm-upgrade-{}-{sequence}", std::process::id()));
        if !directory
            .try_exists()
            .map_err(|error| Failure::io("Inspect preservation destination", &error))?
        {
            break directory;
        }
    };
    boundary(Boundary::Preservation)?;
    let preserved = access.preserve(&directory, budget)?;
    receipt.preserved_at = Some(SystemTime::now());
    receipt.preservation = Some(preserved.directory);
    receipt.manifest = Some(preserved.manifest);
    let candidate = Candidate::reserve(&directory, budget)?;
    receipt.snapshot = Some(candidate.path.clone());
    boundary(Boundary::Snapshot)?;
    build_snapshot(&access.connection, &candidate.path, budget)?;
    receipt.snapshot_verified_at = Some(SystemTime::now());
    boundary(Boundary::Sync)?;
    for directory in [
        &candidate.directory,
        &directory,
        access.source().parent().expect("canonical parent"),
    ] {
        File::open(directory)
            .and_then(|file| file.sync_all())
            .map_err(|error| Failure::io("Sync upgrade preservation before migration", &error))?;
    }
    candidate.verify()?;
    Ok(())
}

fn supported(schema: SchemaCompatibility) -> Result<(), Failure> {
    match schema {
        SchemaCompatibility::Empty
        | SchemaCompatibility::Current
        | SchemaCompatibility::UpgradeRequired { .. } => Ok(()),
        SchemaCompatibility::InterruptedUpgrade => Err(Failure::new(
            "Migration 11 requires explicit interrupted upgrade repair",
            FailureKind::Validation,
        )),
        SchemaCompatibility::Unknown | SchemaCompatibility::Newer { .. } => Err(Failure::new(
            "Refuse unsupported database schema",
            FailureKind::Unsupported,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::maintenance::preservation::tests::Peer;

    fn frozen(mode: &str) -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("library.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        conn.execute_batch(include_str!("../fixtures/adr-0075-schema-11.sql"))
            .unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        conn.pragma_update(None, "journal_mode", mode).unwrap();
        drop(conn);
        (temp, path)
    }
    fn budget() -> Budget {
        Budget::new(Arc::new(AtomicBool::new(false)))
    }

    #[test]
    fn adr_0075_migration_both_entry_points_preserve_frozen_records_and_receipts() {
        for cli in [false, true] {
            let (_temp, path) = frozen("DELETE");
            let before = upgrades::legacy_digest(&Connection::open(&path).unwrap()).unwrap();
            let prepared = if cli {
                crate::db::open_db(&path).unwrap()
            } else {
                startup::prepare_database(&path).unwrap()
            };
            assert_eq!(prepared.receipt.state, PreparationState::Upgraded);
            let receipt = &prepared.receipt;
            assert_eq!(receipt.target, 12);
            assert!(receipt.started_at <= receipt.preserved_at.unwrap());
            assert!(receipt.preserved_at <= receipt.snapshot_verified_at);
            assert!(receipt.snapshot_verified_at.unwrap() <= receipt.finished_at);
            let original = Connection::open(receipt.snapshot.as_ref().unwrap()).unwrap();
            upgrades::verify_target(&original, 11).unwrap();
            assert_eq!(upgrades::legacy_digest(&original).unwrap(), before);
            assert_eq!(upgrades::legacy_digest(&prepared).unwrap(), before);
            assert!(receipt.manifest.as_ref().unwrap().is_file());
            let report = crate::view_models::startup::preparation_report(receipt);
            assert!(report.contains(&receipt.snapshot.as_ref().unwrap().display().to_string()));
            assert!(report.contains("Target schema version: 12"));
        }
    }

    #[test]
    fn adr_0075_migration_preservation_snapshot_sync_and_cancel_fail_before_ddl() {
        for mode in ["DELETE", "WAL"] {
            for stop in [
                Boundary::Preservation,
                Boundary::Snapshot,
                Boundary::Sync,
                Boundary::BeforeMigration,
            ] {
                for cancel in [false, true] {
                    let (_temp, path) = frozen(mode);
                    let before =
                        content_digest(&Connection::open(&path).unwrap(), &budget()).unwrap();
                    let work = budget();
                    let error = prepare_with(&path, &work, |reached| {
                        if reached == stop {
                            if cancel {
                                work.cancelled.store(true, Ordering::Release);
                            }
                            return Err(Failure::new(
                                "Injected preparation failure",
                                if cancel {
                                    FailureKind::Cancelled
                                } else {
                                    FailureKind::Io
                                },
                            ));
                        }
                        Ok(())
                    })
                    .unwrap_err();
                    assert_eq!(error.receipt.state, PreparationState::Stopped);
                    let conn = Connection::open(&path).unwrap();
                    upgrades::verify_target(&conn, 11).unwrap();
                    assert_eq!(content_digest(&conn, &budget()).unwrap(), before);
                    if stop != Boundary::Preservation {
                        assert!(error.receipt.preservation.unwrap().is_dir());
                    }
                }
            }
        }
    }

    #[test]
    fn adr_0075_migration_guard_excludes_competing_writer_and_backs_up_committed_wal() {
        for mode in ["DELETE", "WAL"] {
            let (_temp, path) = frozen(mode);
            if mode == "WAL" {
                Peer::probe(&path, "seed-wal");
            }
            let prepared = prepare_with(&path, &budget(), |boundary| {
                Peer::probe(
                    &path,
                    if boundary == Boundary::BeforeReopen {
                        "open"
                    } else {
                        "blocked"
                    },
                );
                Ok(())
            })
            .unwrap();
            let original = Connection::open(prepared.receipt.snapshot.unwrap()).unwrap();
            assert_eq!(
                upgrades::legacy_digest(&original).unwrap(),
                upgrades::legacy_digest(&prepared.connection).unwrap()
            );
            if mode == "WAL" {
                assert_eq!(
                    original
                        .query_row(
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
    fn adr_0075_migration_precommit_rollback_and_postcommit_verification_are_distinct() {
        for stop in [
            Boundary::Migration(12, MigrationBoundary::BeforeApply),
            Boundary::Migration(12, MigrationBoundary::AfterApply),
            Boundary::Migration(12, MigrationBoundary::AfterRecord),
            Boundary::AfterCommit,
            Boundary::BeforeReopen,
        ] {
            let (_temp, path) = frozen("DELETE");
            let error = prepare_with(&path, &budget(), |reached| {
                if reached == stop {
                    Err(Failure::new(
                        "Injected migration verification failure",
                        FailureKind::Io,
                    ))
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
            let committed = matches!(stop, Boundary::AfterCommit | Boundary::BeforeReopen);
            assert_eq!(
                error.receipt.state,
                if committed {
                    PreparationState::VerificationFailed
                } else {
                    PreparationState::RollbackVerified
                }
            );
            let conn = Connection::open(&path).unwrap();
            upgrades::verify_target(&conn, if committed { 12 } else { 11 }).unwrap();
            assert!(error.receipt.snapshot.as_ref().unwrap().is_file());
            assert!(error.receipt.manifest.as_ref().unwrap().is_file());
            let report = crate::view_models::startup::preparation_failure_report(&error);
            assert_eq!(report.contains("No rollback is claimed"), committed);
            assert_eq!(report.contains("verified rollback"), !committed);
        }
    }

    #[test]
    fn adr_0075_migration_refused_schema_and_interruption_leave_files_unchanged() {
        for change in [
            "CREATE TABLE surprise(value)",
            "CREATE TABLE metadata_bodies(value)",
            "PRAGMA ignore_check_constraints=ON; UPDATE broadcast_event_selection SET revision=0",
        ] {
            let (_temp, path) = frozen("DELETE");
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(change).unwrap();
            drop(conn);
            let before = fs::read(&path).unwrap();
            let error = prepare(&path).unwrap_err();
            assert_eq!(error.receipt.state, PreparationState::Stopped);
            assert!(error.receipt.snapshot.is_none());
            assert_eq!(fs::read(&path).unwrap(), before);
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("interrupted.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        conn.execute_batch(include_str!("../fixtures/adr-0075-interrupted-11.sql"))
            .unwrap();
        drop(conn);
        let before = fs::read(&path).unwrap();
        assert!(prepare(&path)
            .unwrap_err()
            .failure
            .operation
            .contains("explicit"));
        assert_eq!(fs::read(path).unwrap(), before);
    }

    #[test]
    fn adr_0075_migration_fresh_and_repeated_current_preparation_create_no_backup() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("fresh.sqlite");
        let prepared = prepare(&path).unwrap();
        assert_eq!(prepared.receipt.state, PreparationState::Created);
        assert!(prepared.receipt.preservation.is_none());
        drop(prepared);
        for _ in 0..2 {
            let prepared = prepare(&path).unwrap();
            assert_eq!(prepared.receipt.state, PreparationState::Unchanged);
            assert!(prepared.receipt.snapshot.is_none());
            assert_eq!(
                prepared
                    .query_row("SELECT count(*) FROM schema_migrations", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                12
            );
            assert_eq!(
                prepared
                    .query_row("SELECT count(*) FROM metadata_generation", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                1
            );
        }
        assert!(!fs::read_dir(temp.path()).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".v4vmm-upgrade")));
    }
}
