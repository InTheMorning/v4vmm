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
    Cancel,
    CopyReport,
}

pub(crate) struct DatabaseActionDisplay {
    pub(crate) action: DatabaseAction,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
    pub(crate) availability: StartupAvailability,
}

pub(crate) struct DatabaseVm {
    pub(crate) source: String,
    pub(crate) destination: String,
    pub(crate) report: String,
    pub(crate) worker_available: bool,
    pub(crate) suspended: bool,
    running: Option<Arc<AtomicBool>>,
    generation: u64,
}

impl DatabaseVm {
    pub(crate) const TITLE: &'static str = "Database tools";
    pub(crate) const SCOPE: &'static str = "A database backup covers database records, including committed WAL data. It does not include music files or broadcaster token files. Checking and backing up do not change the selected database or switch the app's library.";
    pub(crate) const SOURCE: &'static str = "Existing database path";
    pub(crate) const DESTINATION: &'static str = "New backup file path";
    pub(crate) const HELP: &'static str = "Use configured database fills the source path, or enter another existing database's absolute path. Enter a new backup filename in an existing folder. Existing files are never overwritten. Each operation has a 60-second limit; Cancel stops it before publication when possible.";

    pub(crate) fn new(worker_available: bool) -> Self {
        Self {
            source: String::new(),
            destination: String::new(),
            report: String::new(),
            worker_available,
            suspended: false,
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
            DatabaseAction::Backup => {
                self.input_enabled()
                    && self.worker_available
                    && PathBuf::from(&self.source).is_absolute()
                    && PathBuf::from(&self.destination).is_absolute()
            }
        };
        DatabaseActionDisplay {
            action,
            label,
            a11y_label,
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
            DatabaseAction::Check => DatabaseOperation::Check,
            DatabaseAction::Backup => DatabaseOperation::Backup {
                destination: self.destination.clone().into(),
            },
            DatabaseAction::Cancel | DatabaseAction::CopyReport => return None,
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
        let text = format!("{} — App could not start or receive the database operation for {}. The maintenance worker is busy or unavailable. Wait for the current operation and check again; no completed backup is confirmed.\n\n", time(recorded_at), self.source);
        self.report
            .push_str(&crate::diagnostics::redact_endpoint_details(&text));
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
        FailureKind::Busy => "SQLite reported a busy or locked database within the bounded wait. Close the conflicting writer, then check again.",
        FailureKind::Corrupt => "SQLite could not read valid database contents. Retain the original files; check a known backup before planning recovery.",
        FailureKind::Io => "Storage could not complete the operation. Check free space, the mounted drive and permissions before retrying.",
        FailureKind::Cancelled => "The operator cancelled the operation before completion. Choose Check database or Back up database to start a new attempt.",
        FailureKind::Deadline => "The operation exceeded its 60-second limit. Check storage and competing database activity before retrying.",
        FailureKind::Destination => "The destination is occupied, aliases the source or its journals, or the private candidate changed. Choose a different new backup filename; existing files were not overwritten.",
        FailureKind::Validation => "The candidate did not pass integrity, foreign-key and supported-schema validation. Check the source database report; retain the original files for recovery.",
        FailureKind::Sql => "SQLite could not complete this check. Check the selected database and its schema before retrying.",
    };
    let mut text = format!("{}: {reason}", failure.operation);
    for path in &failure.remaining {
        let _ = write!(text, "\nApp could not confirm artifact cleanup or publication durability at {}. Retain and inspect this path; it is not a confirmed completed backup.", path.display());
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
