//! Explicit revision-checked configuration correction and preservation (ADR 0066).

#![warn(clippy::pedantic)]

use std::fmt;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use anyhow::{anyhow, Context, Result};

use super::{ConfigSnapshot, DEFAULT_TEMP_ATTEMPTS, DEFAULT_TEMP_SEQUENCE};

/// The editable unit is a known field or an invalid whole optional table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CorrectionField(pub(crate) &'static str);

impl CorrectionField {
    pub(crate) const ALL: [Self; 20] = [
        Self("music_dir"),
        Self("db_path"),
        Self("musicindex_endpoint"),
        Self("flac_path"),
        Self("ui_scale"),
        Self("theme_profile"),
        Self("playback.driver"),
        Self("playback.mpv_path"),
        Self("broadcast.hosts"),
        Self("broadcast.selected_host"),
        Self("broadcast.drop_directory"),
        Self("broadcast.drop_file_target"),
        Self("broadcast.encoder"),
        Self("workspace_layout"),
        Self("workspace.layout.content_pane_width"),
        Self("workspace.layout.content_list_view_mode"),
        Self("playback"),
        Self("broadcast"),
        Self("workspace"),
        Self("workspace.layout"),
    ];

    pub(crate) fn core(self) -> bool {
        matches!(self.0, "music_dir" | "db_path")
    }

    pub(crate) fn text(self) -> bool {
        matches!(
            self.0,
            "music_dir"
                | "db_path"
                | "musicindex_endpoint"
                | "flac_path"
                | "ui_scale"
                | "theme_profile"
                | "playback.driver"
                | "playback.mpv_path"
                | "broadcast.selected_host"
                | "broadcast.drop_directory"
                | "broadcast.drop_file_target"
                | "workspace.layout.content_list_view_mode"
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileIdentity {
    len: u64,
    modified: Option<std::time::SystemTime>,
    #[cfg(unix)]
    unix: (u64, u64, i64, i64),
}

impl FileIdentity {
    fn of(metadata: &Metadata) -> Self {
        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            unix: {
                use std::os::unix::fs::MetadataExt;
                (
                    metadata.dev(),
                    metadata.ino(),
                    metadata.ctime(),
                    metadata.ctime_nsec(),
                )
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Revision {
    entry: FileIdentity,
    target: FileIdentity,
    destination: PathBuf,
}

impl Revision {
    fn read(path: &Path) -> Result<Self> {
        let entry = fs::symlink_metadata(path).context("App could not inspect the configuration entry. Restore access, then reload the editor.")?;
        let destination = fs::canonicalize(path).context("App could not resolve the configuration destination. Repair a dangling link or restore access, then reload the editor.")?;
        let target = fs::metadata(&destination).context("App could not inspect the configuration destination. Restore access, then reload the editor.")?;
        if !target.is_file() {
            return Err(anyhow!("The configuration destination is not a regular file. Select the configuration file, then reload the editor."));
        }
        Ok(Self {
            entry: FileIdentity::of(&entry),
            target: FileIdentity::of(&target),
            destination,
        })
    }
}

/// Source and draft bytes never appear in Debug or diagnostic reports.
pub(crate) struct CorrectionSource {
    pub(crate) path: PathBuf,
    revision: Revision,
    bytes: Vec<u8>,
    pub(crate) snapshot: Option<ConfigSnapshot>,
    pub(crate) parse_report: Option<String>,
}

impl fmt::Debug for CorrectionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CorrectionSource")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl CorrectionSource {
    pub(crate) fn read(path: &Path) -> Result<Self> {
        let revision = Revision::read(path)?;
        let bytes = fs::read(&revision.destination).context("App could not read the configuration document. Restore read access, then reload the editor.")?;
        std::str::from_utf8(&bytes).context("The configuration document is not UTF-8. Preserve and correct its encoding externally, then reload the editor.")?;
        let parsed = ConfigSnapshot::from_bytes(path, bytes.clone());
        let (snapshot, parse_report) = match parsed {
            Ok(snapshot) => (Some(snapshot), None),
            Err(error) => (None, Some(error.to_string())),
        };
        let source = Self {
            path: path.to_owned(),
            revision,
            bytes,
            snapshot,
            parse_report,
        };
        source.check_revision()?;
        Ok(source)
    }

    pub(crate) fn destination(&self) -> &Path {
        &self.revision.destination
    }

    pub(crate) fn raw(&self) -> &str {
        std::str::from_utf8(&self.bytes).expect("correction source is UTF-8")
    }

    pub(crate) fn value(&self, field: CorrectionField) -> String {
        let value = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| get(&snapshot.document, field.0));
        match value {
            Some(toml::Value::String(text)) if field.text() => text.clone(),
            Some(value) => {
                let mut wrapper = toml::Table::new();
                wrapper.insert("value".into(), value.clone());
                toml::to_string_pretty(&wrapper).expect("TOML value serializes")
            }
            None => String::new(),
        }
    }

    fn check_revision(&self) -> Result<()> {
        let current = Revision::read(&self.path);
        if current.as_ref().ok() != Some(&self.revision)
            || fs::read(&self.revision.destination).ok().as_deref() != Some(&self.bytes)
        {
            return Err(anyhow!("Configuration changed, disappeared, or its link changed after editing began. App kept your draft and did not replace the file. Copy the draft, then reload the editor; no automatic merge is available."));
        }
        Ok(())
    }

    pub(crate) fn propose(&self, draft: &CorrectionDraft) -> Result<ConfigSnapshot> {
        let bytes = self.proposed_bytes(draft)?;
        self.validate_bytes(draft, bytes)
    }

    /// Extract only converter settings; a version test does not require usable core resources.
    pub(crate) fn converter_path(&self, draft: &CorrectionDraft) -> Result<Option<PathBuf>> {
        let snapshot = ConfigSnapshot::from_bytes(&self.path, self.proposed_bytes(draft)?)?;
        snapshot.flac_path.map_err(|issue| anyhow!("{issue}"))
    }

    fn proposed_bytes(&self, draft: &CorrectionDraft) -> Result<Vec<u8>> {
        let bytes = if let Some(raw) = &draft.raw {
            raw.as_bytes().to_vec()
        } else {
            let mut document = self
                .snapshot
                .as_ref()
                .context("Reload the document before editing a field.")?
                .document
                .clone();
            for (field, draft) in &draft.fields {
                let value = if draft.is_empty() && !field.core() {
                    None
                } else if field.text() {
                    Some(toml::Value::String(draft.to_owned()))
                } else {
                    let mut wrapper = draft.parse::<toml::Table>().map_err(|_| anyhow!("App could not parse the proposed field. Enter a TOML assignment named value, or leave the field blank to remove it."))?;
                    let value = wrapper
                        .remove("value")
                        .context("The proposed field needs a TOML assignment named value.")?;
                    if !wrapper.is_empty() {
                        return Err(anyhow!(
                            "The proposed field may contain only the value assignment."
                        ));
                    }
                    Some(value)
                };
                set(&mut document, field.0, value)?;
            }
            toml::to_string_pretty(&document)
                .context("App could not serialize the proposed configuration.")?
                .into_bytes()
        };
        Ok(bytes)
    }

    fn validate_bytes(&self, draft: &CorrectionDraft, bytes: Vec<u8>) -> Result<ConfigSnapshot> {
        let proposed = ConfigSnapshot::from_bytes(&self.path, bytes)?;
        proposed
            .music_dir
            .as_ref()
            .map_err(|issue| anyhow!("{issue}"))?;
        proposed
            .db_path
            .as_ref()
            .map_err(|issue| anyhow!("{issue}"))?;
        for issue in proposed.issues() {
            let edited = draft.raw.is_some()
                || draft
                    .fields
                    .iter()
                    .any(|(field, _)| overlaps(field.0, issue.field));
            let new_issue = self
                .snapshot
                .as_ref()
                .is_none_or(|old| !old.issues().contains(&issue));
            if edited || new_issue {
                return Err(anyhow!("App did not validate the proposed configuration. {issue}. Correct this setting before saving."));
            }
        }
        Ok(proposed)
    }

    pub(crate) fn copy_draft(&self, draft: &CorrectionDraft) -> String {
        match self.proposed_bytes(draft) {
            Ok(bytes) => redacted_draft(std::str::from_utf8(&bytes).expect("draft is UTF-8"), None),
            Err(_) => "App could not copy the whole draft because a field is not valid TOML. The draft remains in the editor; correct the field syntax before copying.".into(),
        }
    }

    pub(crate) fn core_changed(&self, proposed: &ConfigSnapshot) -> bool {
        self.snapshot.as_ref().is_none_or(|old| {
            old.music_dir != proposed.music_dir || old.db_path != proposed.db_path
        })
    }

    pub(crate) fn save(&self, proposed: &ConfigSnapshot) -> Result<CorrectionReceipt> {
        self.save_with(
            proposed,
            |file, bytes| {
                file.write_all(bytes)?;
                file.sync_all()
            },
            || {},
        )
    }

    fn save_with(
        &self,
        proposed: &ConfigSnapshot,
        backup_write: impl FnOnce(&mut File, &[u8]) -> std::io::Result<()>,
        before_replace: impl FnOnce(),
    ) -> Result<CorrectionReceipt> {
        self.check_revision()?;
        let _write = super::ConfigWriteLease::acquire(&self.path)?;
        self.check_revision()?;
        let (backup, mut backup_file) = artifact(self.destination(), "backup")?;
        backup_write(&mut backup_file, &self.bytes).with_context(|| format!("App could not write and sync the original backup {}. The incomplete backup remains; configuration was not replaced.", backup.display()))?;
        drop(backup_file);
        let directory = File::open(self.destination().parent().context("Configuration has no parent directory.")?)
            .with_context(|| format!("App could not open the configuration directory for sync. Original backup: {}. Configuration was not replaced.", backup.display()))?;
        directory.sync_all().with_context(|| format!("App could not sync the backup directory entry. Original backup: {}. Configuration was not replaced.", backup.display()))?;
        let result = (|| {
            let (candidate, mut file) = artifact(self.destination(), "candidate")?;
            let result = (|| {
                file.write_all(proposed.original_bytes())
                    .context("App could not write the configuration candidate.")?;
                file.sync_all()
                    .context("App could not sync the configuration candidate.")?;
                before_replace();
                self.check_revision()?;
                fs::rename(&candidate, self.destination())
                    .context("App could not install the configuration candidate.")?;
                Ok(())
            })();
            drop(file);
            if let Err(previous) = &result {
                if let Err(error) = fs::remove_file(&candidate) {
                    return Err(anyhow!(
                        "{} App could not remove candidate {}: {error}. The candidate remains.",
                        previous,
                        candidate.display()
                    ));
                }
            }
            result
        })();
        result.with_context(|| {
            format!(
                "App preserved the original configuration in {}.",
                backup.display()
            )
        })?;
        // Persist the rename and the separately named backup directory entries.
        directory.sync_all().with_context(|| format!("App installed the configuration, but could not sync its directory. Original backup: {}. Reload and check the file before continuing.", backup.display()))?;
        let fresh = ConfigSnapshot::read_existing(&self.path).with_context(|| format!("App installed the configuration, but could not read it back. Original backup: {}. Reload and check the file before continuing.", backup.display()))?;
        if fresh.original_bytes() != proposed.original_bytes() {
            return Err(anyhow!("App installed the correction but another writer changed the configuration before verification. Original backup: {}. Reload and check the current file.", backup.display()));
        }
        Ok(CorrectionReceipt { backup, fresh })
    }
}

#[derive(Clone, Default)]
pub(crate) struct CorrectionDraft {
    pub(crate) raw: Option<String>,
    pub(crate) fields: Vec<(CorrectionField, String)>,
}

impl fmt::Debug for CorrectionDraft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CorrectionDraft").finish_non_exhaustive()
    }
}

pub(crate) struct CorrectionReceipt {
    pub(crate) backup: PathBuf,
    pub(crate) fresh: ConfigSnapshot,
}

fn overlaps(left: &str, right: &str) -> bool {
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('.'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

fn redact(value: &mut toml::Value) {
    match value {
        toml::Value::Table(table) => {
            for (key, value) in table {
                let key = key.to_ascii_lowercase();
                if ["token", "secret", "password", "credential"]
                    .iter()
                    .any(|word| key.contains(word))
                {
                    *value = toml::Value::String("[redacted]".into());
                } else {
                    redact(value);
                }
            }
        }
        toml::Value::Array(values) => {
            for value in values {
                redact(value);
            }
        }
        toml::Value::String(text) => *text = crate::diagnostics::redact_endpoint_details(text),
        _ => {}
    }
}
pub(crate) fn redacted_draft(value: &str, field: Option<CorrectionField>) -> String {
    if field.is_some_and(CorrectionField::text) {
        return crate::diagnostics::redact_endpoint_details(value);
    }
    let Ok(mut table) = value.parse::<toml::Table>() else {
        return "App did not copy the malformed draft because it cannot safely identify credential fields. The full draft remains in the editor; correct its syntax before copying.".into();
    };
    for (key, value) in &mut table {
        if ["token", "secret", "password", "credential"]
            .iter()
            .any(|word| key.to_ascii_lowercase().contains(word))
        {
            *value = toml::Value::String("[redacted]".into());
        } else {
            redact(value);
        }
    }
    toml::to_string_pretty(&table).expect("TOML values serialize")
}

fn get<'a>(table: &'a toml::Table, path: &str) -> Option<&'a toml::Value> {
    let (head, rest) = path
        .split_once('.')
        .map_or((path, None), |(head, rest)| (head, Some(rest)));
    let value = table.get(head)?;
    rest.map_or(Some(value), |rest| get(value.as_table()?, rest))
}

fn set(table: &mut toml::Table, path: &str, value: Option<toml::Value>) -> Result<()> {
    if let Some((head, rest)) = path.split_once('.') {
        let child = table
            .entry(head)
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        return set(
            child.as_table_mut().context(
                "The containing optional table is invalid. Select and correct that table first.",
            )?,
            rest,
            value,
        );
    }
    if let Some(value) = value {
        table.insert(path.to_owned(), value);
    } else {
        table.remove(path);
    }
    Ok(())
}

fn artifact(destination: &Path, kind: &str) -> Result<(PathBuf, File)> {
    for _ in 0..DEFAULT_TEMP_ATTEMPTS {
        let sequence = DEFAULT_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = destination.with_file_name(format!(
            ".v4vmm-config-{}-{sequence}.{kind}",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                    "App could not create configuration {kind} {}. Configuration was not replaced.",
                    path.display()
                )
                })
            }
        }
    }
    Err(anyhow!(
        "App could not reserve a configuration {kind} beside {}. Configuration was not replaced.",
        destination.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGINAL: &str = "# preserve bytes\nmusic_dir = '/music'\ndb_path = '/library.sqlite'\nmusicindex_endpoint = 42\nflac_path = false\nfuture = { nested = ['retain', 8] }\n";

    fn source() -> (tempfile::TempDir, CorrectionSource) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        fs::write(&path, ORIGINAL).unwrap();
        let source = CorrectionSource::read(&path).unwrap();
        (temp, source)
    }

    fn endpoint_draft() -> CorrectionDraft {
        CorrectionDraft {
            raw: None,
            fields: vec![(
                CorrectionField("musicindex_endpoint"),
                "https://example.test".into(),
            )],
        }
    }

    #[test]
    fn adr_0066_correction_preserves_exact_original_unknown_values_and_invalid_siblings() {
        let (_temp, source) = source();
        let proposed = source.propose(&endpoint_draft()).unwrap();
        let receipt = source.save(&proposed).unwrap();
        assert_eq!(fs::read(&receipt.backup).unwrap(), ORIGINAL.as_bytes());
        assert_eq!(
            receipt.fresh.document["future"],
            source.snapshot.as_ref().unwrap().document["future"]
        );
        assert_eq!(
            receipt.fresh.document["flac_path"],
            toml::Value::Boolean(false)
        );
        assert_eq!(receipt.fresh.issues().len(), 1);
        assert!(super::super::read_config_for_save(&source.path).is_err());
        let source = CorrectionSource::read(&source.path).unwrap();
        let proposed = source
            .propose(&CorrectionDraft {
                raw: None,
                fields: vec![(CorrectionField("flac_path"), String::new())],
            })
            .unwrap();
        let second = source.save(&proposed).unwrap();
        assert!(super::super::read_config_for_save(&source.path).is_ok());
        assert_ne!(receipt.backup, second.backup);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(receipt.backup).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(&source.path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn adr_0066_rejected_and_unreadable_corrections_do_not_write_or_disclose_values() {
        let (_temp, source) = source();
        let secret = "sensitive-credential";
        let draft = CorrectionDraft {
            raw: Some(format!("token = '{secret}'\ninvalid = [")),
            fields: Vec::new(),
        };
        let error = source.propose(&draft).unwrap_err();
        let report = format!("{error:#} {draft:?} {source:?}");
        assert!(!report.contains(secret));
        assert!(report.contains("line 2"));
        assert_eq!(fs::read(&source.path).unwrap(), ORIGINAL.as_bytes());
        fs::remove_file(&source.path).unwrap();
        fs::create_dir(&source.path).unwrap();
        assert!(CorrectionSource::read(&source.path).is_err());
        fs::remove_dir(&source.path).unwrap();
        fs::write(&source.path, [0xff, 0xfe]).unwrap();
        assert!(CorrectionSource::read(&source.path).is_err());
        assert_eq!(fs::read(&source.path).unwrap(), [0xff, 0xfe]);
    }

    #[test]
    fn adr_0066_failed_backup_and_concurrent_changes_prevent_replacement() {
        let (_temp, source) = source();
        let proposed = source.propose(&endpoint_draft()).unwrap();
        let result = source.save_with(
            &proposed,
            |file, _| {
                file.write_all(b"partial")?;
                Err(std::io::ErrorKind::PermissionDenied.into())
            },
            || {},
        );
        let error = result.err().unwrap().to_string();
        assert!(error.contains("incomplete backup remains"));
        assert_eq!(fs::read(&source.path).unwrap(), ORIGINAL.as_bytes());
        let result = source.save_with(
            &proposed,
            |file, bytes| {
                file.write_all(bytes)?;
                file.sync_all()
            },
            || fs::write(&source.path, b"external revision").unwrap(),
        );
        let error = format!("{:#}", result.err().unwrap());
        assert!(error.contains("preserved the original"));
        assert!(error.contains("kept your draft"));
        assert_eq!(fs::read(&source.path).unwrap(), b"external revision");
        fs::remove_file(&source.path).unwrap();
        assert!(source.save(&proposed).is_err());
        assert!(!source.path.exists());
        fs::write(&source.path, ORIGINAL).unwrap();
        assert!(
            source.save(&proposed).is_err(),
            "same-byte replacement is a conflict"
        );
    }

    #[cfg(unix)]
    #[test]
    fn adr_0066_symlink_correction_preserves_link_and_rejects_link_or_target_changes() {
        use std::os::unix::fs::{symlink, MetadataExt};
        let (temp, original) = source();
        let link = temp.path().join("config-link");
        symlink(&original.path, &link).unwrap();
        let link_inode = fs::symlink_metadata(&link).unwrap().ino();
        let source = CorrectionSource::read(&link).unwrap();
        let proposed = source.propose(&endpoint_draft()).unwrap();
        let receipt = source.save(&proposed).unwrap();
        assert_eq!(fs::read(receipt.backup).unwrap(), ORIGINAL.as_bytes());
        assert_eq!(fs::symlink_metadata(&link).unwrap().ino(), link_inode);
        assert_eq!(fs::read(&link).unwrap(), fs::read(&original.path).unwrap());
        let source = CorrectionSource::read(&link).unwrap();
        fs::remove_file(&link).unwrap();
        let other = temp.path().join("other.toml");
        fs::write(&other, ORIGINAL).unwrap();
        symlink(&other, &link).unwrap();
        assert!(source.save(&proposed).is_err());
        assert_eq!(fs::read(&other).unwrap(), ORIGINAL.as_bytes());
        fs::remove_file(&other).unwrap();
        assert!(CorrectionSource::read(&link).is_err());
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn adr_0066_draft_copy_redacts_nested_credentials_and_malformed_documents() {
        let secret = "private-marker";
        for raw in [
            format!("token = '{secret}'"),
            format!("[nested]\npassword = '{secret}'"),
            format!("[broken\ntoken = '{secret}'"),
        ] {
            assert!(!redacted_draft(&raw, None).contains(secret));
        }
        assert!(!redacted_draft(
            "https://user:private-marker@example.test?token=private-marker",
            Some(CorrectionField("musicindex_endpoint"))
        )
        .contains(secret));
    }
}
