//! One subscription/materialization path for original work and explicit retry (ADR 0066).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;

use super::{PreparedTrack, SubscribeTrackOutcome};
use crate::audio_format::{AudioFormat, ConversionOutcome};
use crate::audio_tags::{write_id3v24_edits, Id3v24Edit};
use crate::config::DownloadConfig;
use crate::db::{self, TrackRow};
use crate::library_path::LibraryRelativePath;
use crate::metadata::TrackContext;
use crate::track_compare::retained::{stage_existing, RetainedArtifact};
use crate::track_compare::{download_track, select_audio_enclosure, SelectedEnclosure};

pub(crate) struct Materialization {
    row: TrackRow,
    context: TrackContext,
    edits: Vec<Id3v24Edit>,
    music_dir: PathBuf,
    enclosure: Option<SelectedEnclosure>,
    prepared: Option<PreparedTrack>,
    binding: Option<RetainedArtifact>,
    pub(super) return_tag_compare: bool,
    pub(super) reconcile_feed: Option<String>,
}

impl std::fmt::Debug for Materialization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Materialization")
            .field("track_id", &self.row.id)
            .finish_non_exhaustive()
    }
}

impl Materialization {
    pub(super) fn new(
        row: TrackRow,
        context: TrackContext,
        edits: Vec<Id3v24Edit>,
        music_dir: PathBuf,
    ) -> Self {
        let enclosure = select_audio_enclosure(&context.track);
        Self {
            row,
            context,
            edits,
            music_dir,
            enclosure,
            prepared: None,
            binding: None,
            return_tag_compare: false,
            reconcile_feed: None,
        }
    }

    pub(crate) fn track_id(&self) -> i64 {
        self.row.id
    }
    pub(crate) fn title(&self) -> String {
        crate::diagnostics::redact_endpoint_details(
            self.context
                .track
                .title
                .as_deref()
                .unwrap_or(&self.row.item_guid),
        )
    }

    pub(crate) fn validate_subject(&self, conn: &Connection, cfg: &DownloadConfig) -> Result<()> {
        anyhow::ensure!(
            cfg.music_dir == self.music_dir,
            "music storage changed; start a new download of the intended track"
        );
        let row = db::track_row_by_id(conn, self.row.id)?.context("original track was removed")?;
        anyhow::ensure!(row.item_guid == self.row.item_guid && row.feed_id == self.row.feed_id
            && row.enclosure_url == self.row.enclosure_url && row.local_path == self.row.local_path
            && row.is_in_library == self.row.is_in_library,
            "original track identity or library binding changed; start a new download of the intended track");
        anyhow::ensure!(
            select_audio_enclosure(&self.context.track) == self.enclosure,
            "original enclosure changed"
        );
        Ok(())
    }

    pub(crate) fn validate_input(&self) -> Result<()> {
        if let Some(binding) = &self.binding {
            binding.validate()?;
        }
        match &self.prepared {
            Some(PreparedTrack::Downloaded(download)) => download.validate_input(),
            _ if self.binding.is_some() => Ok(()),
            _ => Err(anyhow!("downloaded input is no longer available")),
        }
    }

    pub(crate) fn cleanup(&mut self) -> Result<()> {
        if let Some(PreparedTrack::Downloaded(download)) = &mut self.prepared {
            download.cleanup()?;
        }
        self.prepared = None;
        Ok(())
    }

    pub(crate) fn has_owned_staging(&self) -> bool {
        matches!(self.prepared, Some(PreparedTrack::Downloaded(_)))
    }

    /// Retry passes retained input explicitly; source lookup and metadata selection do not run again.
    pub(crate) fn run(
        &mut self,
        conn: &Arc<Mutex<Connection>>,
        cfg: &DownloadConfig,
        retry: bool,
        redownload: bool,
    ) -> Result<SubscribeTrackOutcome> {
        {
            let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
            self.validate_subject(&db, cfg)?;
        }
        if retry && !redownload {
            self.validate_input()?;
        }
        if redownload {
            self.cleanup()?;
            self.binding = None;
            self.prepared = Some(PreparedTrack::Downloaded(Box::new(download_track(
                cfg,
                &self.context.track,
            )?)));
        } else if let Some(PreparedTrack::Downloaded(download)) = &mut self.prepared {
            download.retry_conversion(cfg)?;
        } else {
            let existing = self
                .row
                .local_path
                .as_ref()
                .map(|path| path.resolve(&cfg.music_dir));
            self.prepared = Some(super::prepare_track_for_subscription_internal(
                cfg,
                &self.context.track,
                existing.as_deref(),
            )?);
            if let Some(PreparedTrack::Existing { path }) = &self.prepared {
                if AudioFormat::detect_from_file(path)? == AudioFormat::Wav {
                    let binding = RetainedArtifact::capture(&cfg.music_dir, path, None)?;
                    let enclosure = self
                        .enclosure
                        .clone()
                        .context("original track has no supported enclosure")?;
                    let staged = stage_existing(cfg, &binding, enclosure)?;
                    self.binding = Some(binding);
                    self.prepared = Some(PreparedTrack::Downloaded(Box::new(staged)));
                }
            }
        }

        let prepared = self.prepared.as_ref().expect("prepared materialization");
        let conversion = match prepared {
            PreparedTrack::Existing { .. } => ConversionOutcome::NotRequired,
            PreparedTrack::Downloaded(download) => download.conversion,
        };
        let warning = prepared.format_warning();
        if conversion == ConversionOutcome::WavRetained {
            if let Some(binding) = &self.binding {
                binding.validate()?;
                let path = binding.path().to_path_buf();
                self.cleanup()?;
                return self.outcome(path, conversion, warning, 0);
            }
            // On retry, a previously materialized file (even now invalid) is never overwritten.
            anyhow::ensure!(!retry || self.row.local_path.is_none(), "conversion failed; original library file was preserved and downloaded WAV input is retained for another retry");
        }

        let working_path = prepared.working_path().to_path_buf();
        let applied_edits = if conversion == ConversionOutcome::WavRetained {
            0
        } else if retry || self.binding.is_some() {
            if !self.edits.is_empty() {
                write_id3v24_edits(&working_path, &self.edits)?;
            }
            self.edits.len()
        } else {
            super::apply_id3_edits_nonfatal(&working_path, &self.edits)
        };

        let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        self.validate_subject(&db, cfg)?;
        if let Some(binding) = &self.binding {
            binding.validate()?;
        }
        let prepared = self.prepared.as_mut().expect("prepared materialization");
        let path = match prepared {
            PreparedTrack::Existing { path } => path.clone(),
            PreparedTrack::Downloaded(download) => {
                download.promote()?;
                download.path.clone()
            }
        };
        let relative_path = LibraryRelativePath::from_absolute(&cfg.music_dir, &path)?;
        let save = (|| -> Result<()> {
            let transaction = db.transaction()?;
            let size = std::fs::metadata(&path)?.len().try_into().ok();
            crate::library_service::mark_track_downloaded(
                &transaction,
                self.row.id,
                &relative_path,
                size,
            )?;
            if let Some(previous) = self
                .row
                .local_path
                .as_ref()
                .filter(|previous| *previous != &relative_path)
            {
                // ADR 0066 replaces this binding inside the same transaction; the original WAV remains a preserved source file.
                db::delete_local_file(&transaction, previous)?;
            }
            if let Some(feed_url) = &self.reconcile_feed {
                db::reconcile_feed_subscription_by_url(&transaction, feed_url)?;
            }
            transaction.commit()?;
            Ok(())
        })();
        if let Err(error) = save {
            if let PreparedTrack::Downloaded(download) = prepared {
                download.undo_promotion(&working_path)?;
            }
            return Err(error);
        }
        self.row.local_path = Some(relative_path);
        self.row.is_in_library = true;
        drop(db);
        self.binding = if conversion == ConversionOutcome::WavRetained {
            Some(RetainedArtifact::capture(&cfg.music_dir, &path, None)?)
        } else {
            None
        };
        let cleanup = self.cleanup().err();
        let warning = cleanup.map_or(warning.clone(), |error| {
            let message = format!("App saved the track but could not release its staging: {error:#}. Dismiss retries cleanup of owned staging only.");
            Some(warning.map_or_else(|| message.clone(), |warning| format!("{warning}; {message}")))
        });
        self.outcome(path, conversion, warning, applied_edits)
    }

    fn outcome(
        &self,
        path: PathBuf,
        conversion: ConversionOutcome,
        mut format_warning: Option<String>,
        applied_edits: usize,
    ) -> Result<SubscribeTrackOutcome> {
        let compare = if self.return_tag_compare {
            match super::compare_downloaded_track_path(&path, &self.context) {
                Ok(compare) => Some(compare),
                Err(error) => {
                    let message = crate::diagnostics::redact_endpoint_details(&format!(
                        "App saved the track but could not read its tag comparison: {error:#}"
                    ));
                    format_warning = Some(format_warning.map_or_else(
                        || message.clone(),
                        |warning| format!("{warning}; {message}"),
                    ));
                    None
                }
            }
        } else {
            None
        };
        Ok(SubscribeTrackOutcome {
            relative_path: Some(LibraryRelativePath::from_absolute(&self.music_dir, &path)?),
            path,
            format_warning,
            conversion,
            applied_edits,
            marked_downloaded: true,
            compare,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    fn fixture() -> (
        tempfile::TempDir,
        DownloadConfig,
        Arc<Mutex<Connection>>,
        Materialization,
    ) {
        let temp = tempfile::tempdir().unwrap();
        let cfg = crate::config::ConfigSnapshot::from_bytes(
            Path::new("test.toml"),
            format!("music_dir = {:?}\nflac_path = false\n", temp.path()).into_bytes(),
        )
        .unwrap()
        .downloads()
        .unwrap();
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::migrate_schema(&conn).unwrap();
        conn.execute("INSERT INTO feeds (id, feed_url, title) VALUES (1, 'https://example.test/rss', 'Feed')", []).unwrap();
        conn.execute("INSERT INTO tracks (id, feed_id, item_guid, track_title, enclosure_url, enclosure_type, is_in_library) VALUES (1, 1, 'original', 'Original', 'https://example.test/original.wav', 'audio/wav', 1)", []).unwrap();
        conn.execute(
            "INSERT INTO local_files (path, track_id) VALUES ('original.wav', 1)",
            [],
        )
        .unwrap();
        fs::write(temp.path().join("original.wav"), b"RIFF\x24\0\0\0WAVEfmt ").unwrap();
        let row = db::track_row_by_id(&conn, 1).unwrap().unwrap();
        let context = TrackContext {
            track: super::super::track_row_to_api_track(&row),
            feed: None,
        };
        let operation = Materialization::new(
            row,
            context,
            vec![Id3v24Edit {
                frame_label: "TIT2".into(),
                value: "Retained edit".into(),
            }],
            cfg.music_dir.clone(),
        );
        (temp, cfg, Arc::new(Mutex::new(conn)), operation)
    }

    fn converter(directory: &Path) -> PathBuf {
        let binary = directory.join("flac-stub");
        // Real 80-sample silent FLAC, encoded once; tests need no installed encoder.
        let flac = include_bytes!("../../docs/runbooks/fixtures/conversion.flac");
        fs::write(directory.join("encoded.flac"), flac).unwrap();
        fs::write(&binary, format!("#!/bin/sh\n[ \"$1\" = --version ] && exit 0\nwhile [ \"$1\" != -o ]; do shift; done\nshift\n/bin/cp '{}' \"$1\"\n", directory.join("encoded.flac").display())).unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
        binary
    }

    #[test]
    fn adr_0066_conversion_retry_keeps_edits_one_binding_and_original_wav() {
        let (temp, mut cfg, conn, mut operation) = fixture();
        let original = fs::read(temp.path().join("original.wav")).unwrap();
        let first = operation.run(&conn, &cfg, false, false).unwrap();
        assert_eq!(first.conversion, ConversionOutcome::WavRetained);
        assert_eq!(first.applied_edits, 0);
        operation.validate_input().unwrap();
        cfg.flac_path = Ok(Some(converter(temp.path())));
        let result = operation.run(&conn, &cfg, true, false).unwrap();
        assert_eq!(result.conversion, ConversionOutcome::Flac);
        assert_eq!(result.applied_edits, 1);
        assert_eq!(
            crate::audio_tags::read_audio_tags(&result.path)
                .unwrap()
                .title
                .as_deref(),
            Some("Retained edit")
        );
        assert_eq!(
            fs::read(temp.path().join("original.wav")).unwrap(),
            original
        );
        let db = conn.lock().unwrap();
        assert_eq!(db::library_tracks(&db).unwrap().len(), 1);
        assert_eq!(
            db::track_row_by_id(&db, 1)
                .unwrap()
                .unwrap()
                .local_path
                .unwrap()
                .as_stored(),
            "original.flac"
        );
        assert!(fs::read_dir(temp.path().join(".v4vmm-staging"))
            .unwrap()
            .next()
            .is_none());
        assert!(operation.validate_input().is_err());
    }

    #[test]
    fn adr_0066_conversion_retry_rejects_changed_binding_context_and_input() {
        let (temp, cfg, conn, mut operation) = fixture();
        operation.run(&conn, &cfg, false, false).unwrap();
        let mut moved = cfg.clone();
        moved.music_dir = temp.path().join("elsewhere");
        assert!(operation
            .run(&conn, &moved, true, false)
            .err()
            .unwrap()
            .to_string()
            .contains("storage changed"));
        fs::write(temp.path().join("original.wav"), b"RIFF\x24\0\0\0WAVEbad!").unwrap();
        assert!(operation.validate_input().is_err());
        assert!(operation.run(&conn, &cfg, true, false).is_err());
        conn.lock()
            .unwrap()
            .execute("UPDATE tracks SET item_guid = 'different' WHERE id = 1", [])
            .unwrap();
        assert!(operation
            .run(&conn, &cfg, true, true)
            .err()
            .unwrap()
            .to_string()
            .contains("identity or library binding changed"));
    }

    #[test]
    fn adr_0066_failed_replacement_keeps_file_binding_and_retained_staging() {
        let (temp, mut cfg, conn, mut operation) = fixture();
        operation.run(&conn, &cfg, false, false).unwrap();
        cfg.flac_path = Ok(Some(converter(temp.path())));
        fs::write(
            temp.path().join("original.flac"),
            b"unrelated existing music",
        )
        .unwrap();
        assert!(operation
            .run(&conn, &cfg, true, false)
            .err()
            .unwrap()
            .to_string()
            .contains("existing destination"));
        operation.validate_input().unwrap();
        assert_eq!(
            fs::read(temp.path().join("original.flac")).unwrap(),
            b"unrelated existing music"
        );
        assert_eq!(
            db::track_row_by_id(&conn.lock().unwrap(), 1)
                .unwrap()
                .unwrap()
                .local_path
                .unwrap()
                .as_stored(),
            "original.wav"
        );
        operation.cleanup().unwrap();
        assert!(temp.path().join("original.wav").exists());
        assert!(temp.path().join("original.flac").exists());
    }

    #[test]
    fn adr_0066_database_rejection_restores_staging_and_original_binding() {
        let (temp, mut cfg, conn, mut operation) = fixture();
        operation.run(&conn, &cfg, false, false).unwrap();
        cfg.flac_path = Ok(Some(converter(temp.path())));
        conn.lock().unwrap().execute_batch("CREATE TRIGGER reject_binding BEFORE INSERT ON local_files BEGIN SELECT RAISE(FAIL, 'fixture write denied'); END;").unwrap();
        assert!(operation.run(&conn, &cfg, true, false).is_err());
        assert!(!temp.path().join("original.flac").exists());
        assert!(temp.path().join("original.wav").exists());
        operation.validate_input().unwrap();
        assert_eq!(
            db::track_row_by_id(&conn.lock().unwrap(), 1)
                .unwrap()
                .unwrap()
                .local_path
                .unwrap()
                .as_stored(),
            "original.wav"
        );
    }
}
