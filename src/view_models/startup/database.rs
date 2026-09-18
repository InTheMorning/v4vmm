//! Shared database maintenance intent, availability and safe recorded reports (ADR 0066).

#![warn(clippy::pedantic)]

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::application::commands::maintenance::{
    DatabaseCommand, DatabaseOperation, DatabaseOutcome, DatabaseResult,
};
use crate::db::{
    maintenance::{Failure, FailureKind, Inspection, Integrity, Snapshot},
    SchemaCompatibility,
};

use super::StartupAvailability;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DatabaseAction {
    ConfiguredSource,
    Check,
    Backup,
    EndSession,
    Preserve,
    ReviewRestore,
    UpgradeBackup,
    RepairUpgrade,
    Restore,
    Cancel,
    CopyReport,
}

pub(crate) struct DatabaseActionDisplay {
    pub(crate) action: DatabaseAction,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
    pub(crate) availability: StartupAvailability,
    pub(crate) destructive: bool,
    pub(crate) visible: bool,
}

pub(crate) struct DatabaseVm {
    pub(crate) source: String,
    pub(crate) destination: String,
    pub(crate) restore_source: String,
    pub(crate) report: String,
    pub(crate) worker_available: bool,
    pub(crate) suspended: bool,
    pub(crate) maintenance_ready: bool,
    pub(crate) session_generation: u64,
    review: Option<Arc<crate::application::commands::maintenance::RestoreReview>>,
    reviewed_inputs: Option<(String, String)>,
    repair_source: Option<PathBuf>,
    running: Option<Arc<AtomicBool>>,
    generation: u64,
}

impl DatabaseVm {
    pub(crate) const TITLE: &'static str = "Database tools";
    pub(crate) const SCOPE: &'static str = "A database backup covers database records, including committed WAL data. It does not include music files or broadcaster token files. Checking and backing up do not change the selected database or switch the app's library.";
    pub(crate) const SOURCE: &'static str = "Existing database path";
    pub(crate) const DESTINATION: &'static str = "New backup file or preservation directory path";
    pub(crate) const HELP: &'static str = "Use configured database fills the source path, or enter another existing database's absolute path. For backup, enter a new filename in an existing folder. For preservation, enter a new directory name. Existing paths are never overwritten. Checks, backups and file preservation each have a 60-second limit; Cancel waits for a known result before releasing database access.";
    pub(crate) const PRESERVATION: &'static str = "If a verified backup cannot be made, end the app session, then choose Preserve database files. App waits up to five seconds for exclusive SQLite access and copies the database and journals to the new private directory. SQLite may recover journals while acquiring access and clean up or checkpoint them on close. The copy records files after access was acquired; it is not a verified restorable backup. Afterward, Check again and Open app use fresh core verification.";
    pub(crate) const RESTORE_SOURCE: &'static str = "Chosen restore backup path";
    pub(crate) const RESTORE_HELP: &'static str = "To restore, enter a standalone backup and a new preservation directory above, then choose Review restore. Restore replaces only the configured database, regardless of the inspection source field. Review does not replace data. The separate Restore database action ends the current app session, preserves its database, installs the reviewed candidate and reopens the app after fresh verification. Music files and broadcaster token files are not restored or changed. Older valid backups need the explicit Upgrade backup action, which migrates a separate candidate and returns it to Restore review. The chosen backup is preserved. Restore has a 60-second work limit; final verification can take another 60 seconds, including after cancellation.";

    pub(crate) const UPGRADE_HELP: &'static str = "For an interrupted upgrade, use the configured database and Check database. Repair interrupted upgrade is offered only when migration 11 broadcast_event_selection has a compatible table, migrations 1–10 are valid and integrity checks pass. Enter a new preservation directory and end the app session first. Repair preserves the original, records migration 11 through the normal migration registry in a separate candidate, verifies all existing records and event selections, then installs it. Unsupported schemas require preservation and a known backup.";

    pub(crate) fn new(worker_available: bool) -> Self {
        Self {
            source: String::new(),
            destination: String::new(),
            restore_source: String::new(),
            report: String::new(),
            worker_available,
            suspended: false,
            maintenance_ready: false,
            session_generation: 0,
            review: None,
            reviewed_inputs: None,
            repair_source: None,
            running: None,
            generation: 0,
        }
    }
    pub(crate) fn is_working(&self) -> bool {
        self.running.is_some()
    }
    pub(crate) fn input_enabled(&self) -> bool {
        !self.is_working() && !self.suspended
    }
    pub(crate) fn action(&self, action: DatabaseAction) -> DatabaseActionDisplay {
        let (label, a11y_label) = match action {
            DatabaseAction::ConfiguredSource => (
                "Use configured database",
                "Read the configured database path without checking or changing the database",
            ),
            DatabaseAction::Check => (
                "Check database",
                "Inspect the selected database without changing its contents or schema",
            ),
            DatabaseAction::Backup => (
                "Back up database",
                "Save a verified database snapshot at the new backup path",
            ),
            DatabaseAction::EndSession => (
                "End app session for preservation",
                "Stop app work and close database connections before preserving files",
            ),
            DatabaseAction::Preserve => (
                "Preserve database files",
                "Preserve database and journal files under exclusive access; this is not a verified backup",
            ),
            DatabaseAction::UpgradeBackup => ("Upgrade backup", "Upgrade a separate candidate from the chosen older backup using the normal migrations, then review restore"),
            DatabaseAction::RepairUpgrade => ("Repair interrupted upgrade", "Preserve the configured database and complete migration 11 broadcast_event_selection in a validated candidate before installation"),
            DatabaseAction::ReviewRestore => ("Review restore", "Validate the chosen backup and review the configured destination and preservation directory"),
            DatabaseAction::Restore => ("Restore database", "Replace the reviewed configured database after ending the app session and preserving its original data"),
            DatabaseAction::Cancel => (
                "Cancel",
                "Request cancellation of the running database operation",
            ),
            DatabaseAction::CopyReport => (
                "Copy database report",
                "Copy the complete database maintenance report",
            ),
        };
        let available = match action {
            DatabaseAction::CopyReport => !self.report.is_empty(),
            DatabaseAction::Cancel => self
                .running
                .as_ref()
                .is_some_and(|c| !c.load(Ordering::Acquire)),
            DatabaseAction::ConfiguredSource => self.input_enabled() && self.worker_available,
            DatabaseAction::Check => {
                self.input_enabled()
                    && self.worker_available
                    && PathBuf::from(&self.source).is_absolute()
            }
            DatabaseAction::EndSession => {
                self.input_enabled() && self.worker_available && !self.maintenance_ready
            }
            DatabaseAction::Backup | DatabaseAction::Preserve => {
                self.input_enabled()
                    && self.worker_available
                    && (action != DatabaseAction::Preserve || self.maintenance_ready)
                    && PathBuf::from(&self.source).is_absolute()
                    && PathBuf::from(&self.destination).is_absolute()
            }
            DatabaseAction::RepairUpgrade => {
                self.input_enabled()
                    && self.worker_available
                    && self.maintenance_ready
                    && self.session_generation != 0
                    && self.repair_source.as_ref() == Some(&PathBuf::from(&self.source))
                    && PathBuf::from(&self.destination).is_absolute()
            }
            DatabaseAction::ReviewRestore | DatabaseAction::UpgradeBackup => {
                self.input_enabled()
                    && self.worker_available
                    && self.session_generation != 0
                    && PathBuf::from(&self.restore_source).is_absolute()
                    && PathBuf::from(&self.destination).is_absolute()
            }
            DatabaseAction::Restore => {
                self.input_enabled() && self.worker_available && self.current_review().is_some()
            }
        };
        DatabaseActionDisplay {
            action,
            label,
            a11y_label,
            visible: action != DatabaseAction::RepairUpgrade
                || self.repair_source.as_ref() == Some(&PathBuf::from(&self.source)),
            destructive: matches!(
                action,
                DatabaseAction::Restore | DatabaseAction::RepairUpgrade
            ),
            availability: if available {
                StartupAvailability::Available
            } else {
                StartupAvailability::Unavailable
            },
        }
    }
    pub(crate) fn begin(
        &mut self,
        action: DatabaseAction,
        config_path: PathBuf,
    ) -> Option<(u64, DatabaseCommand)> {
        if self.action(action).availability != StartupAvailability::Available {
            return None;
        }
        let operation = match action {
            DatabaseAction::ConfiguredSource => DatabaseOperation::ConfiguredSource(config_path),
            DatabaseAction::Check => {
                self.repair_source = None;
                DatabaseOperation::Check
            }
            DatabaseAction::Backup => DatabaseOperation::Backup {
                destination: self.destination.clone().into(),
            },
            DatabaseAction::Preserve => DatabaseOperation::Preserve {
                destination: self.destination.clone().into(),
            },
            DatabaseAction::ReviewRestore | DatabaseAction::UpgradeBackup => {
                self.review = None;
                self.reviewed_inputs =
                    Some((self.restore_source.clone(), self.destination.clone()));
                if action == DatabaseAction::UpgradeBackup {
                    DatabaseOperation::UpgradeBackup {
                        backup: self.restore_source.clone().into(),
                        preservation: self.destination.clone().into(),
                        config_path,
                        session_generation: self.session_generation,
                    }
                } else {
                    DatabaseOperation::ReviewRestore {
                        backup: self.restore_source.clone().into(),
                        preservation: self.destination.clone().into(),
                        config_path,
                        session_generation: self.session_generation,
                    }
                }
            }
            DatabaseAction::RepairUpgrade => {
                self.repair_source = None;
                self.review = None;
                DatabaseOperation::RepairUpgrade {
                    preservation: self.destination.clone().into(),
                    config_path,
                    session_generation: self.session_generation,
                }
            }
            DatabaseAction::Restore => DatabaseOperation::Restore(self.review.take()?),
            DatabaseAction::EndSession | DatabaseAction::Cancel | DatabaseAction::CopyReport => {
                return None
            }
        };
        let cancelled = Arc::new(AtomicBool::new(false));
        self.running = Some(cancelled.clone());
        self.generation += 1;
        Some((
            self.generation,
            DatabaseCommand {
                source: self.source.clone().into(),
                operation,
                cancelled,
            },
        ))
    }
    pub(crate) fn cancel(&self) {
        if let Some(cancelled) = &self.running {
            cancelled.store(true, Ordering::Release);
        }
    }
    fn current_review(
        &self,
    ) -> Option<&Arc<crate::application::commands::maintenance::RestoreReview>> {
        self.review.as_ref().filter(|review| {
            review.session_generation == self.session_generation
                && self
                    .reviewed_inputs
                    .as_ref()
                    .is_some_and(|(backup, preservation)| {
                        backup == &self.restore_source && preservation == &self.destination
                    })
        })
    }
    pub(crate) fn restore_confirmation(&self) -> Option<String> {
        self.current_review().map(|review| {
            let candidate = &review.candidate;
            crate::diagnostics::redact_endpoint_details(&format!(
                "Ready for explicit Restore database.\nChosen backup: {}\nConfigured database to replace: {}\nPreservation directory: {}\nValidated candidate: {}\nThis replaces database records only. Keep the preservation directory to recover the previous database. Changes to the backup, destination, configuration or session require a new review.",
                candidate.backup.display(), candidate.destination.display(), candidate.preservation.display(), candidate.candidate_path().display()))
        })
    }
    pub(crate) fn working_message(&self) -> Option<&'static str> {
        if !self.worker_available {
            return Some("The independent maintenance worker is unavailable. Copy the available report and restart the app after freeing system resources.");
        }
        if self.suspended {
            return Some("App is checking core resources. Database tools will be available when that operation finishes.");
        }
        self.running.as_ref().map(|c| if c.load(Ordering::Acquire) { "App requested cancellation. Waiting for the database operation to finish and report cleanup." } else { "App is working on the selected database. Navigation remains available." })
    }
    pub(crate) fn unavailable(&mut self, generation: u64, recorded_at: std::time::SystemTime) {
        if generation != self.generation || self.running.take().is_none() {
            return;
        }
        let text = format!("{} — App could not start or receive the database operation for {}. App-session work or the maintenance worker is busy or unavailable. Wait for the current operation, then review or check again; no completed database operation is confirmed.\n\n", time(recorded_at), self.source);
        self.report
            .push_str(&crate::diagnostics::redact_endpoint_details(&text));
    }
    fn review_report(
        &mut self,
        result_review: Result<
            Arc<crate::application::commands::maintenance::RestoreReview>,
            Failure,
        >,
        operation: &DatabaseOperation,
    ) -> String {
        let mut text = String::new();
        if matches!(operation, DatabaseOperation::UpgradeBackup { .. }) {
            text.push_str("App attempted normal migration preparation on a separate candidate. The chosen backup was not migrated.\n");
        }
        if let DatabaseOperation::ReviewRestore {
            backup,
            preservation,
            config_path,
            ..
        }
        | DatabaseOperation::UpgradeBackup {
            backup,
            preservation,
            config_path,
            ..
        } = operation
        {
            let _ = writeln!(&mut text, "App reviewed backup {} for the database configured in {}. Preservation requested at {}.", backup.display(), config_path.display(), preservation.display());
        }

        match result_review {
            Ok(review) => {
                self.review = Some(review);
                if let Some(confirmation) = self.restore_confirmation() {
                    let _ = writeln!(&mut text, "App validated a separate restore candidate. The chosen backup and configured database were not replaced.\n{confirmation}");
                }
            }
            Err(failure) => {
                let _ = writeln!(&mut text, "App refused restore review. No database was installed. {}\nChoose a valid standalone backup and Review restore again. For an older valid backup, choose Upgrade backup to prepare a separate candidate.", failure_report(&failure));
            }
        }
        text
    }

    pub(crate) fn complete(&mut self, generation: u64, result: DatabaseResult) -> bool {
        if generation != self.generation || self.running.take().is_none() {
            return false;
        }
        let mut text = format!("{} — ", time(result.recorded_at));
        let source_changed = match result.outcome {
            DatabaseOutcome::ConfiguredSource(result_path) => {
                if let DatabaseOperation::ConfiguredSource(path) = result.operation {
                    let _ = writeln!(
                        text,
                        "App checked configuration {} for the database path.",
                        path.display()
                    );
                }
                match result_path {
                    Ok(path) => {
                        self.source = path.display().to_string();
                        let _ = writeln!(text, "Selected database: {}. Choose Check database or enter a new backup path. App did not open or retarget the library.", path.display());
                        true
                    }
                    Err(message) => {
                        text.push_str(message);
                        false
                    }
                }
            }
            DatabaseOutcome::Checked(inspection) => {
                self.repair_source = (inspection.valid_snapshot()
                    && inspection.schema == Some(Ok(SchemaCompatibility::InterruptedUpgrade)))
                .then(|| result.source.clone());
                let _ = writeln!(text, "App checked database {}.\n{}\nApp did not initialize, migrate or repair this database.", result.source.display(), inspection_report(&inspection));
                false
            }
            DatabaseOutcome::BackedUp(result_backup) => {
                let requested = match result.operation {
                    DatabaseOperation::Backup { destination } => destination,
                    _ => PathBuf::new(),
                };
                let _ = writeln!(
                    text,
                    "App attempted a database backup from {} to {}.",
                    result.source.display(),
                    requested.display()
                );
                match result_backup {
                    Ok(snapshot) => text.push_str(&snapshot_report(&snapshot)),
                    Err(failure) => {
                        let _ = writeln!(
                            text,
                            "No completed backup was confirmed. {}",
                            failure_report(&failure)
                        );
                    }
                }
                text.push_str(Self::SCOPE);
                false
            }
            DatabaseOutcome::Preserved(result_copy) => {
                text.push_str(&preservation_report(
                    &result.source,
                    &result.operation,
                    result_copy,
                ));
                false
            }
            DatabaseOutcome::RestoreReviewed(result_review) => {
                text.push_str(&self.review_report(result_review, &result.operation));
                false
            }
            DatabaseOutcome::Restored(result_restore) => {
                if matches!(result.operation, DatabaseOperation::RepairUpgrade { .. }) {
                    let _ = writeln!(
                        text,
                        "Configured database selected for repair: {}.",
                        result.source.display()
                    );
                }
                text.push_str(&restore_report(result.operation, result_restore));
                false
            }
        };
        self.report
            .push_str(&crate::diagnostics::redact_endpoint_details(&text));
        self.report.push_str("\n\n");
        source_changed
    }
}
impl Drop for DatabaseVm {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn preservation_report(
    source: &std::path::Path,
    operation: &DatabaseOperation,
    result_copy: Result<crate::db::maintenance::Preservation, Failure>,
) -> String {
    let mut text = String::new();
    let requested = match operation {
        DatabaseOperation::Preserve { destination } => destination,
        _ => &PathBuf::new(),
    };
    let _ = writeln!(
        &mut text,
        "App attempted file preservation from {} to {}.",
        source.display(),
        requested.display()
    );
    match result_copy {
        Ok(copy) => {
            let _ = writeln!(&mut text, "App saved a preservation copy of {} database/journal file(s) in {}. This is not a verified restorable backup. Manifest: {}.\nSQLite acquired exclusive access at {} with journal mode {}. Files were copied after SQLite acquired access and before it closed the connection. SQLite may perform automatic journal recovery during access and cleanup/checkpoint on close. No pre-access byte identity is claimed. Keep the manifest and copied files together.", copy.file_count, copy.directory.display(), copy.manifest.display(), time(copy.acquired_at), copy.journal_mode);
            for path in copy.changed_during_access {
                let _ = writeln!(&mut text, "App observed a file identity, size, modification time or presence change while SQLite acquired access: {}. The preservation copy reflects the state after that access, including any SQLite journal recovery.", path.display());
            }
        }
        Err(failure) => {
            let _ = writeln!(&mut text, "App did not complete a preservation copy. {}\nIn-app preservation cannot proceed without safe exclusive access and a completed copy. App did not install or replace a database. Retain original files and inspect a known backup for recovery.", failure_report(&failure));
        }
    }
    text.push_str("The preservation operation has finished and released its database access. Normal work has not resumed. Choose Check again, then Open app only when fresh core checks pass.");
    text
}

fn restore_report(
    operation: DatabaseOperation,
    result_restore: crate::db::maintenance::restore::RestoreResult,
) -> String {
    use crate::db::maintenance::restore::InstallState;
    let mut text = String::new();
    if let DatabaseOperation::RepairUpgrade { preservation, .. } = &operation {
        let _ = writeln!(&mut text, "App attempted migration 11 broadcast_event_selection repair of the checked configured database. Preservation requested at {}.", preservation.display());
        if matches!(result_restore.state, InstallState::Verified) {
            text.push_str("App recorded migration 11 broadcast_event_selection through the normal migration registry. All existing rows, event selections, schema objects and prior migration records were preserved.\n");
        }
    }
    if let DatabaseOperation::Restore(review) = operation {
        let _ = writeln!(&mut text, "App attempted restore from {} into configured database {}. Validated candidate retained at {}. Preservation requested at {}.",
                review.candidate.backup.display(), review.candidate.destination.display(), review.candidate.candidate_path().display(), review.candidate.preservation.display());
    }
    if let Some(copy) = result_restore.preservation {
        let _ = writeln!(&mut text, "App preserved {} database/journal files after acquiring exclusive access at {}. File-preservation manifest: {}. This file copy is not a verified backup.", copy.file_count, time(copy.acquired_at), copy.manifest.display());
    }
    if let Some(path) = result_restore.verified_original {
        let _ = writeln!(&mut text, "Verified snapshot of the previous database: {}. Retain this backup for an explicit restore if needed.", path.display());
    }

    match result_restore.state {
            InstallState::Verified => text.push_str("App installed and verified the reviewed database, including its schema, records and rolled-back write probe. App will reopen one fresh session after checking configuration and music storage. The report remains in Settings > Diagnostics > Database tools."),
            InstallState::NotInstalled(failure) => { let _ = writeln!(&mut text, "App stopped before installation. {}\nReview restore again after resolving the reported prerequisite. In recovery, Check again and Open app can reopen the current database if core checks pass.", failure_report(&failure)); }
            InstallState::Failed { failure, rollback_verified } => {
                let _ = writeln!(&mut text, "App could not complete SQLite installation. {}\n{}", failure_report(&failure), if rollback_verified {
                    "App verified that SQLite rolled back installation: the destination schema and records match their pre-installation fingerprint. Recovery remains open. Resolve the failure, then Review restore again, or Check again and Open app to use the previous database."
                } else { "App could not verify complete rollback. Recovery remains open. Retain the original files, candidate and preservation artifacts; check the destination and explicitly review a known backup before further recovery." });
            }
            InstallState::VerificationFailed(failure) => { let _ = writeln!(&mut text, "SQLite completed installation, but verification failed. {}\nRecovery remains open. No rollback is claimed. Retain all artifacts and explicitly review the preserved original backup for recovery.", failure_report(&failure)); }
        }

    text
}

fn time(recorded: std::time::SystemTime) -> String {
    chrono::DateTime::<chrono::Utc>::from(recorded)
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string()
}
fn failure_report(failure: &Failure) -> String {
    let reason = match failure.kind {
        FailureKind::Missing => "The file or its parent folder does not exist. Check the path and mounted storage, then check again.",
        FailureKind::Access => "Access was denied. Check file and folder permissions, then check again.",
        FailureKind::InvalidFile => "The selected path is not a regular database file. Select an existing SQLite database.",
        FailureKind::Busy => "SQLite reported a busy or locked database within the bounded wait. Close the conflicting SQLite reader or writer, then check again.",
        FailureKind::Corrupt => "SQLite could not read valid database contents. Retain the original files; check a known backup before planning recovery.",
        FailureKind::Io => "Storage could not complete the operation. Check free space, the mounted drive and permissions before retrying.",
        FailureKind::Cancelled => "The operator cancelled the operation before completion. Choose Check database or Back up database to start a new attempt.",
        FailureKind::Deadline => "The operation exceeded its 60-second limit. Check storage and competing database activity before retrying.",
        FailureKind::Destination => "The destination is occupied, aliases the source or its journals, or the private candidate changed. Choose a new backup filename or preservation directory; existing files were not overwritten.",
        FailureKind::Validation => "The candidate did not pass integrity, foreign-key and supported-schema validation. Check the source database report; retain the original files for recovery.",
        FailureKind::Sql => "SQLite could not complete this check. Check the selected database and its schema before retrying.",
        FailureKind::Unsupported => "App cannot establish the supported maintenance protocol for this source or session. Retain the original files, use Check again, and validate a known backup before planning recovery. No raw-copy bypass is available.",
        FailureKind::UnstableFiles => "A source file changed or its copied bytes did not match. App cannot confirm a complete preservation copy. Retain the original and partial artifacts; check storage before retrying.",
    };
    let mut text = format!("{}: {reason}", failure.operation);
    for path in &failure.remaining {
        let _ = write!(text, "\nA partial or unsynced artifact remains at {}. Retain and inspect this path; no completed result is confirmed there.", path.display());
    }
    text
}
fn inspection_report(inspection: &Inspection) -> String {
    let access = match &inspection.access {
        Ok(()) => if inspection.checks_constraints {
            "App read the private backup candidate's schema with writes disabled."
        } else {
            "SQLite opened the existing file read-only and read its schema."
        }
        .into(),
        Err(e) => failure_report(e),
    };
    let integrity = match &inspection.integrity { None => "Not checked because database access failed.".into(), Some(Ok(Integrity::Ok)) => if inspection.checks_constraints { "PRAGMA integrity_check returned ok, including CHECK constraints." } else { "Read-only PRAGMA integrity_check returned ok. SQLite omits CHECK constraints on a read-only source; backup validation checks them on its private candidate." }.into(), Some(Ok(Integrity::Errors(count))) => format!("PRAGMA integrity_check reported {count} error(s). Retain the original; check a known backup before recovery. Raw database text is omitted."), Some(Err(e)) => failure_report(e) };
    let foreign = match &inspection.foreign_keys { None => "Not checked because database access failed.".into(), Some(Ok(0)) => "No foreign-key violations found.".into(), Some(Ok(count)) => format!("Found {count} foreign-key violation(s). Retain the original; this database cannot be labelled a verified backup."), Some(Err(e)) => failure_report(e) };
    let schema = match &inspection.schema {
        None => "Not checked because database access failed.".into(),
        Some(Err(e)) => failure_report(e),
        Some(Ok(SchemaCompatibility::InterruptedUpgrade)) => "Migration 11 broadcast_event_selection created its compatible table but has no completion record. Migrations 1–10 match the normal registry. If integrity and foreign-key checks pass, enter a new preservation directory, end the app session and choose Repair interrupted upgrade. Repair preserves the original and completes the migration on a separate validated candidate before installation.".into(),
        Some(Ok(SchemaCompatibility::Current)) => "Current schema and migration ledger are compatible. No schema action is needed.".into(),
        Some(Ok(SchemaCompatibility::UpgradeRequired { applied, current })) => format!("Supported older migration ledger ({applied} of {current} migrations). An explicit supported upgrade is needed before normal use; this check did not apply it."),
        Some(Ok(SchemaCompatibility::Newer { version })) => format!("Migration version {version} is newer than this app supports. Use a compatible app version; unfamiliar schema alone is not corruption."),
        Some(Ok(SchemaCompatibility::Unknown)) => "The schema or migration ledger is unrecognized. Retain the original and use a compatible app or inspect a known backup; unfamiliar schema alone is not corruption.".into(),
        Some(Ok(SchemaCompatibility::Empty)) => "No application tables found. Select an existing library; this check did not initialize the file.".into(),
    };
    format!("Access: {access}\nIntegrity: {integrity}\nForeign keys: {foreign}\nSchema: {schema}")
}
fn snapshot_report(snapshot: &Snapshot) -> String {
    let mut text = format!(
        "App saved a verified SQLite backup at {}.\n{}\nKeep this backup for recovery.\n",
        snapshot.destination.display(),
        inspection_report(&snapshot.inspection)
    );
    for path in &snapshot.cleanup_remaining {
        let _ = writeln!(text, "App could not remove its temporary backup artifact at {}. The completed backup above remains valid; inspect the temporary path before cleanup.", path.display());
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0066_upgrade_actions_require_fresh_recognition_and_explicit_candidate_preparation() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("library.sqlite");
        let conn = crate::db::open_db(&source).unwrap();
        crate::db::upgrades::interrupt_fixture(&conn, crate::db::MigrationBoundary::AfterApply)
            .unwrap();
        drop(conn);
        let mut vm = DatabaseVm::new(true);
        vm.source = source.display().to_string();
        vm.destination = temp.path().join("preserved").display().to_string();
        vm.session_generation = 42;
        assert!(!vm.action(DatabaseAction::RepairUpgrade).visible);
        let (generation, command) = vm.begin(DatabaseAction::Check, PathBuf::new()).unwrap();
        vm.complete(generation, command.execute());
        assert!(vm.action(DatabaseAction::RepairUpgrade).visible);
        assert!(vm.report.contains("Migration 11 broadcast_event_selection"));
        assert!(vm
            .begin(DatabaseAction::RepairUpgrade, PathBuf::new())
            .is_none());
        vm.maintenance_ready = true;
        vm.source.push('x');
        assert!(!vm.action(DatabaseAction::RepairUpgrade).visible);
        vm.source.pop();
        let (_, command) = vm
            .begin(DatabaseAction::RepairUpgrade, PathBuf::from("/config"))
            .unwrap();
        assert!(matches!(
            command.operation,
            DatabaseOperation::RepairUpgrade {
                session_generation: 42,
                ..
            }
        ));
        assert!(!vm.action(DatabaseAction::RepairUpgrade).visible);
        assert!(vm
            .begin(DatabaseAction::RepairUpgrade, PathBuf::new())
            .is_none());
    }

    #[test]
    fn adr_0066_upgrade_backup_returns_to_review_without_installing() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("library.sqlite");
        drop(crate::db::open_db(&source).unwrap());
        let backup = temp.path().join("older.sqlite");
        let conn = crate::db::open_db(&backup).unwrap();
        crate::db::upgrades::interrupt_fixture(&conn, crate::db::MigrationBoundary::BeforeApply)
            .unwrap();
        drop(conn);
        let config = temp.path().join("config.toml");
        std::fs::write(
            &config,
            format!(
                "music_dir = '{}'\ndb_path = '{}'\n",
                temp.path().display(),
                source.display()
            ),
        )
        .unwrap();
        let before = std::fs::read(&source).unwrap();
        let mut vm = DatabaseVm::new(true);
        vm.session_generation = 7;
        vm.restore_source = backup.display().to_string();
        vm.destination = temp.path().join("preserved").display().to_string();
        let (generation, command) = vm.begin(DatabaseAction::UpgradeBackup, config).unwrap();
        vm.complete(generation, command.execute());
        assert!(vm.restore_confirmation().is_some());
        assert_eq!(
            vm.action(DatabaseAction::Restore).availability,
            StartupAvailability::Available
        );
        assert_eq!(std::fs::read(source).unwrap(), before);
        assert!(vm.report.contains("chosen backup was not migrated"));
    }

    #[test]
    fn adr_0066_restore_review_is_explicit_bound_to_inputs_and_one_session() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("configured.sqlite");
        let chosen = temp.path().join("chosen.sqlite");
        let config = temp.path().join("config.toml");
        drop(crate::db::open_db(&source).unwrap());
        drop(crate::db::open_db(&chosen).unwrap());
        std::fs::write(
            &config,
            format!(
                "music_dir = '{}'\ndb_path = '{}'\n",
                temp.path().display(),
                source.display()
            ),
        )
        .unwrap();
        let mut vm = DatabaseVm::new(true);
        vm.session_generation = 42;
        vm.restore_source = chosen.display().to_string();
        vm.destination = temp.path().join("preserved").display().to_string();
        assert!(vm.begin(DatabaseAction::Restore, config.clone()).is_none());
        let (generation, command) = vm
            .begin(DatabaseAction::ReviewRestore, config.clone())
            .unwrap();
        vm.complete(generation, command.execute());
        let summary = vm.restore_confirmation().unwrap();
        for path in [&source, &chosen, &PathBuf::from(&vm.destination)] {
            assert!(summary.contains(path.to_str().unwrap()));
        }
        assert!(summary.contains("database records only"));
        assert!(!PathBuf::from(&vm.destination).exists());
        assert!(vm.action(DatabaseAction::Restore).destructive);
        vm.restore_source.push('x');
        assert!(vm.restore_confirmation().is_none());
        assert!(vm.begin(DatabaseAction::Restore, config.clone()).is_none());
        vm.restore_source.pop();
        vm.session_generation += 1;
        assert!(vm.restore_confirmation().is_none());
        vm.session_generation -= 1;
        let (_, command) = vm.begin(DatabaseAction::Restore, config.clone()).unwrap();
        assert!(matches!(command.operation, DatabaseOperation::Restore(_)));
        assert!(vm.begin(DatabaseAction::Restore, config).is_none());
        assert!(vm.restore_confirmation().is_none());
        vm.cancel();
        assert!(command.cancelled.load(Ordering::Acquire));
    }

    #[test]
    fn adr_0066_restore_reports_distinguish_rollback_and_failed_verification() {
        use crate::db::maintenance::restore::{InstallState, RestoreResult};
        let failure = || Failure {
            operation: "Install reviewed candidate",
            kind: FailureKind::Io,
            remaining: vec!["/retained/candidate.sqlite".into()],
        };
        for state in [
            InstallState::Failed {
                failure: failure(),
                rollback_verified: true,
            },
            InstallState::Failed {
                failure: failure(),
                rollback_verified: false,
            },
            InstallState::VerificationFailed(failure()),
        ] {
            let verified = matches!(
                state,
                InstallState::Failed {
                    rollback_verified: true,
                    ..
                }
            );
            let text = restore_report(
                DatabaseOperation::Check,
                RestoreResult {
                    preservation: None,
                    verified_original: None,
                    state,
                },
            );
            assert!(text.contains("Recovery remains open"));
            assert!(text.contains("/retained/candidate.sqlite"));
            assert_eq!(
                text.contains("App verified that SQLite rolled back"),
                verified
            );
            assert!(!text.contains("will reopen one fresh session"));
        }
    }

    #[test]
    fn adr_0066_preservation_requires_drained_authority_and_keeps_report_through_resumption() {
        use crate::application::session_lifecycle::{SessionDrain, SessionLifecycle, SessionPhase};
        use std::sync::Mutex;
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("library.sqlite");
        let connection = Arc::new(Mutex::new(crate::db::open_db(&source).unwrap()));
        let held = connection.clone();
        let session = SessionLifecycle::new();
        let mut drain = SessionDrain::new(session.clone(), connection, None);
        let mut vm = DatabaseVm::new(true);
        vm.source = source.display().to_string();
        vm.destination = temp.path().join("preserved").display().to_string();
        assert!(vm.begin(DatabaseAction::Preserve, PathBuf::new()).is_none());
        assert_eq!(
            vm.action(DatabaseAction::EndSession).availability,
            StartupAvailability::Available
        );
        session.begin_drain();
        assert!(drain.finish().is_err());
        drop(held);
        let maintenance = drain.finish().unwrap();
        vm.maintenance_ready = maintenance.is_ready();
        let (generation, command) = vm.begin(DatabaseAction::Preserve, PathBuf::new()).unwrap();
        // The ordinary worker path cannot accidentally bypass the drained token.
        let bypass = DatabaseCommand {
            source: source.clone(),
            operation: command.operation.clone(),
            cancelled: command.cancelled.clone(),
        }
        .execute();
        assert!(matches!(
            bypass.outcome,
            DatabaseOutcome::Preserved(Err(Failure {
                kind: FailureKind::Unsupported,
                ..
            }))
        ));
        assert!(!PathBuf::from(&vm.destination).exists());
        let worker = crate::presentation::maintenance_executor::MaintenanceWorker::start().unwrap();
        let (mut maintenance, result) = worker
            .client
            .submit(move || command.execute_preservation(maintenance))
            .unwrap()
            .blocking_recv()
            .unwrap();
        assert_eq!(session.phase(), SessionPhase::Maintenance);
        assert!(!session.accepts(session.generation()));
        vm.complete(generation, result);
        assert!(vm.report.contains("preservation copy"));
        assert!(vm.report.contains("not a verified restorable backup"));
        assert!(vm.report.contains("Normal work has not resumed"));
        let report = vm.report.clone();
        assert!(maintenance.begin_resume());
        assert!(!maintenance.begin_resume());
        assert!(matches!(
            crate::db::startup::check_database(&source),
            Ok(crate::db::startup::DatabaseReadiness::Ready)
        ));
        let fresh = SessionLifecycle::new();
        assert!(!fresh.accepts(session.generation()));
        assert_eq!(vm.report, report);
        worker.finish();
    }

    #[test]
    fn adr_0066_preservation_cancel_and_invalid_source_keep_reports_and_reject_stale_results() {
        use crate::application::session_lifecycle::SessionDrain;
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("invalid.sqlite");
        std::fs::write(&source, b"invalid header").unwrap();
        let mut vm = DatabaseVm::new(true);
        vm.source = source.display().to_string();
        vm.destination = temp.path().join("copy").display().to_string();
        vm.maintenance_ready = true;
        vm.report = "Earlier database report\n".into();
        let (generation, command) = vm.begin(DatabaseAction::Preserve, PathBuf::new()).unwrap();
        vm.cancel();
        assert!(vm.is_working());
        assert_eq!(
            vm.action(DatabaseAction::Preserve).availability,
            StartupAvailability::Unavailable
        );
        let authority = SessionDrain::core_recovery().finish().unwrap();
        let (authority, result) = command.execute_preservation(authority);
        vm.complete(generation, result);
        assert!(vm.report.starts_with("Earlier database report\n"));
        assert!(vm.report.contains("cancelled"));
        assert!(!PathBuf::from(&vm.destination).exists());
        let (generation, command) = vm.begin(DatabaseAction::Preserve, PathBuf::new()).unwrap();
        let (authority, result) = command.execute_preservation(authority);
        let earlier = vm.report.clone();
        assert!(!vm.complete(generation - 1, result));
        assert!(vm.is_working());
        assert_eq!(vm.report, earlier);
        vm.unavailable(generation, std::time::SystemTime::now());
        let (generation, command) = vm.begin(DatabaseAction::Preserve, PathBuf::new()).unwrap();
        let (authority, result) = command.execute_preservation(authority);
        vm.complete(generation, result);
        assert!(authority.is_ready());
        assert!(vm.report.contains("In-app preservation cannot proceed"));
        assert!(vm.report.contains("Check again"));
        assert_eq!(std::fs::read(source).unwrap(), b"invalid header");
        assert!(!PathBuf::from(&vm.destination).exists());
    }

    #[test]
    fn adr_0066_database_actions_use_worker_without_normal_runtime_and_report_recorded_paths() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("library.sqlite");
        drop(crate::db::open_db(&source).unwrap());
        let config = temp.path().join("config.toml");
        std::fs::write(
            &config,
            format!("music_dir = ''\ndb_path = '{}'\n", source.display()),
        )
        .unwrap();
        let worker = crate::presentation::maintenance_executor::MaintenanceWorker::start().unwrap();
        let mut vm = DatabaseVm::new(true);
        assert_eq!(
            vm.action(DatabaseAction::Check).availability,
            StartupAvailability::Unavailable
        );
        let (generation, command) = vm
            .begin(DatabaseAction::ConfiguredSource, config.clone())
            .unwrap();
        assert!(vm.begin(DatabaseAction::Check, config.clone()).is_none());
        let result = worker
            .client
            .submit(move || command.execute())
            .unwrap()
            .blocking_recv()
            .unwrap();
        assert!(vm.complete(generation, result));
        assert_eq!(vm.source, source.display().to_string());
        let (generation, command) = vm.begin(DatabaseAction::Check, config.clone()).unwrap();
        let mut result = worker
            .client
            .submit(move || command.execute())
            .unwrap()
            .blocking_recv()
            .unwrap();
        let recorded = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_800_000_000);
        result.recorded_at = recorded;
        vm.complete(generation, result);
        assert!(vm.report.contains(&time(recorded)));
        for expected in [
            "Access:",
            "Integrity:",
            "Foreign keys:",
            "Schema:",
            "did not initialize",
            &vm.source,
        ] {
            assert!(vm.report.contains(expected));
        }
        vm.destination = temp.path().join("backup.sqlite").display().to_string();
        let (generation, command) = vm.begin(DatabaseAction::Backup, config).unwrap();
        let result = worker
            .client
            .submit(move || command.execute())
            .unwrap()
            .blocking_recv()
            .unwrap();
        vm.complete(generation, result);
        assert!(vm.report.contains(&vm.destination));
        assert!(vm.report.contains("App saved a verified SQLite backup"));
        assert!(vm
            .report
            .contains("does not include music files or broadcaster token files"));
        worker.finish();
    }

    #[test]
    fn adr_0066_database_cancel_and_stale_completion_preserve_current_admission() {
        let mut vm = DatabaseVm::new(false);
        vm.source = "/tmp/source.sqlite".into();
        assert!(vm.begin(DatabaseAction::Check, PathBuf::new()).is_none());
        vm.worker_available = true;
        let (generation, command) = vm.begin(DatabaseAction::Check, PathBuf::new()).unwrap();
        vm.cancel();
        assert!(command.cancelled.load(Ordering::Acquire));
        assert!(vm.working_message().unwrap().contains("Waiting"));
        vm.unavailable(generation + 1, std::time::UNIX_EPOCH);
        assert!(vm.is_working());
        vm.unavailable(generation, std::time::UNIX_EPOCH);
        assert!(!vm.is_working());
        vm.suspended = true;
        assert!(vm.begin(DatabaseAction::Check, PathBuf::new()).is_none());
        assert_eq!(
            vm.action(DatabaseAction::CopyReport).availability,
            StartupAvailability::Available
        );
    }

    #[test]
    fn adr_0066_database_report_keeps_newer_schema_separate_from_corruption() {
        let inspection = Inspection {
            checks_constraints: false,
            access: Ok(()),
            integrity: Some(Ok(Integrity::Ok)),
            foreign_keys: Some(Ok(0)),
            schema: Some(Ok(SchemaCompatibility::Newer { version: 999 })),
        };
        let text = inspection_report(&inspection);
        assert!(text.contains("integrity_check returned ok"));
        assert!(text.contains("Use a compatible app version"));
        assert!(text.contains("unfamiliar schema alone is not corruption"));
    }
}
