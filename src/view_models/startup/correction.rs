//! Shared inert drafts, action admission and recorded correction reports (ADR 0066).

#![warn(clippy::pedantic)]

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;

use crate::application::commands::maintenance::{
    CorrectionAccess, CorrectionCommand, CorrectionOperation, CorrectionResult,
};
use crate::config::correction::{CorrectionDraft, CorrectionField, CorrectionSource};
use crate::config::ConfigSnapshot;

use super::StartupAvailability;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CorrectionAction {
    Load,
    Reload,
    Select(CorrectionField),
    Validate,
    Save,
    EndSession,
    CopyDraft,
    CopyReport,
}

#[derive(Debug)]
pub(crate) struct CorrectionActionDisplay {
    pub(crate) action: CorrectionAction,
    pub(crate) label: String,
    pub(crate) a11y_label: String,
    pub(crate) availability: StartupAvailability,
}

pub(crate) struct CorrectionVm {
    pub(crate) source: Option<Arc<CorrectionSource>>,
    pub(crate) selected: Option<CorrectionField>,
    draft: CorrectionDraft,
    pub(crate) access: CorrectionAccess,
    generation: u64,
    working: Option<CorrectionOperation>,
    pub(crate) report: String,
    pub(crate) worker_available: bool,
    pub(crate) suspended: bool,
    saved: bool,
}

impl CorrectionVm {
    pub(crate) const TITLE: &'static str = "Configuration repair";
    pub(crate) const EXPLANATION: &'static str = "Load the current file to correct it. Editing keeps a draft only. Save preserves the original in a separate backup; it does not retry an operation. Core changes require ending this app session, then Check again and Open app.";

    pub(crate) fn new(worker_available: bool) -> Self {
        Self {
            source: None,
            selected: None,
            draft: CorrectionDraft::default(),
            access: CorrectionAccess::CoreRecovery,
            generation: 0,
            working: None,
            report: String::new(),
            worker_available,
            suspended: false,
            saved: false,
        }
    }

    pub(crate) fn is_working(&self) -> bool {
        self.working.is_some()
    }

    pub(crate) fn work_message(&self) -> Option<&'static str> {
        self.working.map(|operation| match operation {
            CorrectionOperation::Load => {
                "App is reading the configuration file and recording its revision."
            }
            CorrectionOperation::Validate => {
                "App is validating the draft and testing any changed music and database paths."
            }
            CorrectionOperation::Save => {
                "App is validating the draft, preserving the original and saving the correction."
            }
        })
    }

    pub(crate) fn needs_maintenance(&self) -> bool {
        !self.saved
            && self.access == CorrectionAccess::OptionalOnly
            && (self.selected.is_some_and(CorrectionField::core)
                || self.draft.raw.is_some()
                || self.draft.fields.iter().any(|(field, _)| field.core()))
    }

    pub(crate) fn action(&self, action: CorrectionAction) -> CorrectionActionDisplay {
        let label: String = match action {
            CorrectionAction::Load => "Edit configuration".into(),
            CorrectionAction::Reload => "Reload file (discard draft)".into(),
            CorrectionAction::Select(field) => field.0.into(),
            CorrectionAction::Validate => "Test draft and paths".into(),
            CorrectionAction::Save => "Save correction".into(),
            CorrectionAction::EndSession => "End session to edit core paths".into(),
            CorrectionAction::CopyDraft => "Copy draft (redacted)".into(),
            CorrectionAction::CopyReport => "Copy repair report".into(),
        };
        let allowed = match action {
            CorrectionAction::CopyReport => !self.report.is_empty(),
            CorrectionAction::CopyDraft => self.source.is_some(),
            CorrectionAction::Load | CorrectionAction::Reload => self.worker_available,
            CorrectionAction::Select(_) => {
                self.source
                    .as_ref()
                    .is_some_and(|source| source.snapshot.is_some())
                    && !self.saved
            }
            CorrectionAction::EndSession => self.worker_available && self.needs_maintenance(),
            CorrectionAction::Save | CorrectionAction::Validate => {
                self.worker_available
                    && self.source.is_some()
                    && !self.saved
                    && !self.needs_maintenance()
                    && (self.draft.raw.is_some() || !self.draft.fields.is_empty())
            }
        };
        let independent = matches!(
            action,
            CorrectionAction::CopyReport | CorrectionAction::CopyDraft
        );
        let availability = if !independent && (self.is_working() || self.suspended) {
            StartupAvailability::Working
        } else if allowed {
            StartupAvailability::Available
        } else {
            StartupAvailability::Unavailable
        };
        CorrectionActionDisplay {
            action,
            a11y_label: label.clone(),
            label,
            availability,
        }
    }

    pub(crate) fn fields(&self) -> Vec<CorrectionField> {
        let Some(snapshot) = self
            .source
            .as_ref()
            .and_then(|source| source.snapshot.as_ref())
        else {
            return Vec::new();
        };
        // Whole-table editors appear only when field extraction is impossible.
        CorrectionField::ALL
            .into_iter()
            .filter(|field| {
                !matches!(
                    field.0,
                    "playback" | "broadcast" | "workspace" | "workspace.layout"
                ) || snapshot.issues().iter().any(|issue| issue.field == field.0)
            })
            .collect()
    }

    pub(crate) fn select(&mut self, field: CorrectionField) -> bool {
        if self.action(CorrectionAction::Select(field)).availability
            != StartupAvailability::Available
        {
            return false;
        }
        self.selected = Some(field);
        true
    }

    pub(crate) fn value(&self) -> String {
        let Some(source) = &self.source else {
            return String::new();
        };
        match self.selected {
            None => self
                .draft
                .raw
                .clone()
                .unwrap_or_else(|| source.raw().into()),
            Some(field) => self
                .draft
                .fields
                .iter()
                .find(|(key, _)| *key == field)
                .map_or_else(|| source.value(field), |(_, value)| value.clone()),
        }
    }

    pub(crate) fn edit(&mut self, value: String) {
        if self.saved
            || self.suspended
            || self.working == Some(CorrectionOperation::Save)
            || self.working == Some(CorrectionOperation::Load)
        {
            return;
        }
        if let Some(field) = self.selected {
            self.draft.fields.retain(|(key, _)| *key != field);
            if self
                .source
                .as_ref()
                .is_some_and(|source| source.value(field) != value)
            {
                self.draft.fields.push((field, value));
            }
        } else if let Some(source) = &self.source {
            self.draft.raw = (value != source.raw()).then_some(value);
        }
        if self.working == Some(CorrectionOperation::Validate) {
            self.generation += 1;
            self.working = None;
        }
    }

    pub(crate) fn input_enabled(&self) -> bool {
        self.source.is_some() && !self.saved && !self.suspended && !self.is_working()
    }

    pub(crate) fn copy_draft(&self) -> String {
        self.source
            .as_ref()
            .map_or_else(String::new, |source| source.copy_draft(&self.draft))
    }

    pub(crate) fn input_help(&self) -> String {
        match self.selected {
            None => "Document text. Correct the TOML syntax at the reported line and column.".into(),
            Some(field) if field.text() => format!("{}: enter the value as text, without quotes. Other edited fields stay in this draft.", field.0),
            Some(field) => format!("{}: edit the TOML value assignment below. Leave blank to remove this optional field. Other edited fields stay in this draft.", field.0),
        }
    }

    pub(crate) fn begin(
        &mut self,
        action: CorrectionAction,
        path: PathBuf,
    ) -> Option<(u64, CorrectionCommand)> {
        if self.action(action).availability != StartupAvailability::Available {
            return None;
        }
        let operation = match action {
            CorrectionAction::Load | CorrectionAction::Reload => CorrectionOperation::Load,
            CorrectionAction::Validate => CorrectionOperation::Validate,
            CorrectionAction::Save => CorrectionOperation::Save,
            _ => return None,
        };
        self.generation += 1;
        self.working = Some(operation);
        Some((
            self.generation,
            CorrectionCommand {
                path,
                source: self.source.clone(),
                draft: self.draft.clone(),
                access: self.access,
                operation,
            },
        ))
    }

    pub(crate) fn complete(
        &mut self,
        generation: u64,
        result: CorrectionResult,
    ) -> Option<Arc<ConfigSnapshot>> {
        if generation != self.generation || self.working.is_none() {
            return None;
        }
        let operation = self.working.take().expect("current operation");
        let mut fresh = None;
        let message = match result {
            CorrectionResult::Loaded(source) => {
                self.draft = CorrectionDraft::default();
                self.saved = false;
                self.selected = source.snapshot.as_ref().map(|snapshot| {
                    Self::fields_for_issue(snapshot).unwrap_or(CorrectionField("musicindex_endpoint"))
                });
                let mut message = format!("App loaded configuration {}. Resolved destination: {}.", source.path.display(), source.destination().display());
                if let Some(parse) = &source.parse_report { write!(message, "\n{parse}").expect("string write"); }
                if let Some(snapshot) = &source.snapshot { append_issues(&mut message, snapshot); }
                self.source = Some(source);
                message
            }
            CorrectionResult::Validated => "App validated the draft and tested any changed core paths. No configuration was saved. Save correction performs validation again before preserving and replacing the file.".into(),
            CorrectionResult::Saved(receipt) => {
                self.saved = true;
                let mut message = format!("App saved the configuration correction. Original bytes are preserved in {}. Save did not retry an operation. Core changes need Check again, then Open app. Optional tools retain their current session state until reinitialized.", receipt.backup.display());
                append_issues(&mut message, &receipt.fresh);
                message.push_str(if receipt.fresh.issues().is_empty() { "\nA fresh read found no configuration issues; ordinary persistence is permitted again." } else { "\nOrdinary persistence remains paused until all configuration issues are corrected." });
                fresh = Some(Arc::new(receipt.fresh));
                message
            }
            CorrectionResult::Failed(message) => {
                if operation == CorrectionOperation::Load { self.source = None; self.draft = CorrectionDraft::default(); self.selected = None; }
                format!("App could not complete configuration {operation:?}. {message}")
            }
        };
        let at = chrono::Utc::now();
        write!(
            self.report,
            "{} — {message}\n\n",
            at.format("%Y-%m-%d %H:%M:%S UTC")
        )
        .expect("string write");
        fresh
    }

    fn fields_for_issue(snapshot: &ConfigSnapshot) -> Option<CorrectionField> {
        snapshot.issues().iter().find_map(|issue| {
            CorrectionField::ALL
                .into_iter()
                .find(|field| field.0 == issue.field)
        })
    }
}

fn append_issues(message: &mut String, snapshot: &ConfigSnapshot) {
    for issue in snapshot.issues() {
        write!(message, "\n{issue}").expect("string write");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded() -> (tempfile::TempDir, PathBuf, CorrectionVm) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(&path, "music_dir = '/music'\ndb_path = '/library.sqlite'\nmusicindex_endpoint = 42\nflac_path = false\n").unwrap();
        let mut vm = CorrectionVm::new(true);
        vm.access = CorrectionAccess::OptionalOnly;
        let (generation, command) = vm.begin(CorrectionAction::Load, path.clone()).unwrap();
        vm.complete(generation, command.execute());
        (temp, path, vm)
    }

    #[test]
    fn adr_0066_editing_is_inert_and_save_is_single_flight_without_retry() {
        let (_temp, path, mut vm) = loaded();
        let original = std::fs::read(&path).unwrap();
        vm.select(CorrectionField("musicindex_endpoint"));
        vm.edit("https://example.test".into());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let (generation, command) = vm.begin(CorrectionAction::Save, path.clone()).unwrap();
        assert!(vm.begin(CorrectionAction::Save, path.clone()).is_none());
        let snapshot = vm.complete(generation, command.execute()).unwrap();
        assert_eq!(snapshot.issues().len(), 1);
        assert_eq!(snapshot.issues()[0].field, "flac_path");
        assert!(vm.report.contains("Ordinary persistence remains paused"));
        assert!(vm.report.contains("UTC"));
        assert!(vm.begin(CorrectionAction::Save, path).is_none());
        assert!(!vm.input_enabled());
        assert!(vm.report.contains("Save did not retry an operation"));
    }

    #[test]
    fn adr_0066_validation_generations_reject_old_drafts_and_core_edits_need_drain() {
        let (_temp, path, mut vm) = loaded();
        vm.select(CorrectionField("musicindex_endpoint"));
        vm.edit("https://first.test".into());
        let (generation, command) = vm.begin(CorrectionAction::Validate, path.clone()).unwrap();
        let prior = vm.report.clone();
        vm.edit("https://second.test".into());
        assert!(vm.complete(generation, command.execute()).is_none());
        assert_eq!(vm.report, prior);
        assert_eq!(vm.value(), "https://second.test");
        vm.select(CorrectionField("music_dir"));
        vm.edit("/new/music".into());
        assert!(vm.needs_maintenance());
        assert!(vm.begin(CorrectionAction::Save, path.clone()).is_none());
        assert_eq!(
            vm.action(CorrectionAction::EndSession).availability,
            StartupAvailability::Available
        );
        vm.access = CorrectionAccess::CoreRecovery;
        assert!(vm.begin(CorrectionAction::Save, path).is_some());
    }

    #[test]
    fn adr_0066_conflict_preserves_draft_and_reload_never_invents_unreadable_contents() {
        let (_temp, path, mut vm) = loaded();
        vm.select(CorrectionField("musicindex_endpoint"));
        vm.edit("https://draft.test".into());
        std::fs::write(&path, "external = 'keep'\n").unwrap();
        let (generation, command) = vm.begin(CorrectionAction::Save, path.clone()).unwrap();
        vm.complete(generation, command.execute());
        assert_eq!(vm.value(), "https://draft.test");
        assert!(vm.report.contains("kept your draft"));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "external = 'keep'\n"
        );
        std::fs::remove_file(&path).unwrap();
        let (generation, command) = vm.begin(CorrectionAction::Reload, path).unwrap();
        vm.complete(generation, command.execute());
        assert!(vm.source.is_none());
        assert!(!vm.input_enabled());
        assert_eq!(vm.value(), "");
    }
}
