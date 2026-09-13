//! Explicit configuration correction commands on the independent worker (ADR 0066).

#![warn(clippy::pedantic)]

use std::path::PathBuf;
use std::sync::Arc;

use crate::config::correction::{CorrectionDraft, CorrectionReceipt, CorrectionSource};
use crate::db::startup::{check_database, DatabaseReadiness};
use crate::startup::storage::check_music;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CorrectionAccess {
    OptionalOnly,
    CoreRecovery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CorrectionOperation {
    Load,
    Validate,
    Save,
}

#[derive(Debug)]
pub(crate) struct CorrectionCommand {
    pub(crate) path: PathBuf,
    pub(crate) source: Option<Arc<CorrectionSource>>,
    pub(crate) draft: CorrectionDraft,
    pub(crate) access: CorrectionAccess,
    pub(crate) operation: CorrectionOperation,
}

pub(crate) enum CorrectionResult {
    Loaded(Arc<CorrectionSource>),
    Validated,
    Saved(Box<CorrectionReceipt>),
    Failed(String),
}

impl CorrectionCommand {
    pub(crate) fn execute(self) -> CorrectionResult {
        let path = self.path.clone();
        match self.run() {
            Ok(result) => result,
            Err(error) => CorrectionResult::Failed(crate::diagnostics::redact_endpoint_details(
                &format!("Configuration {}: {error:#}", path.display()),
            )),
        }
    }

    fn run(self) -> anyhow::Result<CorrectionResult> {
        if self.operation == CorrectionOperation::Load {
            return CorrectionSource::read(&self.path)
                .map(|source| CorrectionResult::Loaded(Arc::new(source)));
        }
        let source = self.source.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Reload the configuration before validating or saving.")
        })?;
        let proposed = source.propose(&self.draft)?;
        if source.core_changed(&proposed) {
            if self.access != CorrectionAccess::CoreRecovery {
                return Err(anyhow::anyhow!("End the current app session before correcting core paths. App did not save or retarget its open resources."));
            }
            let music = proposed
                .music_dir
                .as_ref()
                .map_err(|issue| anyhow::anyhow!("{issue}"))?;
            check_music(music, false, false).map_err(|issue| {
                anyhow::anyhow!(
                    "App could not verify proposed music folder {}: {} {}",
                    music.display(),
                    issue.cause,
                    issue.next_action
                )
            })?;
            let database = proposed
                .db_path
                .as_ref()
                .map_err(|issue| anyhow::anyhow!("{issue}"))?;
            match check_database(database) {
                Ok(DatabaseReadiness::Ready) => {},
                Ok(DatabaseReadiness::NeedsPreparation) => return Err(anyhow::anyhow!("The proposed database {} is missing, empty, or needs schema preparation. Choose an existing prepared library; configuration correction cannot create a replacement library.", database.display())),
                Err(error) => return Err(anyhow::anyhow!("App could not verify proposed database {}: {}", database.display(), error.reason)),
            }
        }
        if self.operation == CorrectionOperation::Validate {
            Ok(CorrectionResult::Validated)
        } else {
            source
                .save(&proposed)
                .map(|receipt| CorrectionResult::Saved(Box::new(receipt)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::correction::CorrectionField;

    #[test]
    fn adr_0066_path_correction_tests_existing_locations_and_never_creates_a_database() {
        let temp = tempfile::tempdir().unwrap();
        let music = temp.path().join("music");
        std::fs::create_dir(&music).unwrap();
        let audio = music.join("keep.mp3");
        std::fs::write(&audio, b"existing music").unwrap();
        let database = temp.path().join("existing.sqlite");
        drop(crate::db::open_db(&database).unwrap());
        let path = temp.path().join("config.toml");
        let original = "music_dir = ''\ndb_path = ''\nfuture = 'preserve'\n";
        std::fs::write(&path, original).unwrap();
        let source = Arc::new(CorrectionSource::read(&path).unwrap());
        let make = |music: &std::path::Path, database: &std::path::Path, access, operation| {
            CorrectionCommand {
                path: path.clone(),
                source: Some(Arc::clone(&source)),
                access,
                operation,
                draft: CorrectionDraft {
                    raw: None,
                    fields: vec![
                        (CorrectionField("music_dir"), music.display().to_string()),
                        (CorrectionField("db_path"), database.display().to_string()),
                    ],
                },
            }
        };
        assert!(matches!(
            make(
                &music,
                &database,
                CorrectionAccess::OptionalOnly,
                CorrectionOperation::Save
            )
            .execute(),
            CorrectionResult::Failed(_)
        ));
        let missing = temp.path().join("missing.sqlite");
        assert!(matches!(
            make(
                &music,
                &missing,
                CorrectionAccess::CoreRecovery,
                CorrectionOperation::Save
            )
            .execute(),
            CorrectionResult::Failed(_)
        ));
        assert!(!missing.exists());
        assert!(matches!(
            make(
                &temp.path().join("absent-music"),
                &database,
                CorrectionAccess::CoreRecovery,
                CorrectionOperation::Save
            )
            .execute(),
            CorrectionResult::Failed(_)
        ));
        assert!(!temp.path().join("absent-music").exists());
        let empty = temp.path().join("empty.sqlite");
        std::fs::write(&empty, []).unwrap();
        assert!(matches!(
            make(
                &music,
                &empty,
                CorrectionAccess::CoreRecovery,
                CorrectionOperation::Save
            )
            .execute(),
            CorrectionResult::Failed(_)
        ));
        assert_eq!(std::fs::metadata(&empty).unwrap().len(), 0);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert!(matches!(
            make(
                &music,
                &database,
                CorrectionAccess::CoreRecovery,
                CorrectionOperation::Validate
            )
            .execute(),
            CorrectionResult::Validated
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert!(matches!(
            make(
                &music,
                &database,
                CorrectionAccess::CoreRecovery,
                CorrectionOperation::Save
            )
            .execute(),
            CorrectionResult::Saved(_)
        ));
        assert_eq!(std::fs::read(&audio).unwrap(), b"existing music");
        assert_eq!(std::fs::read_dir(&music).unwrap().count(), 1);
    }
}
