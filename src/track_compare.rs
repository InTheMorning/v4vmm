//! Validated download staging and comparison (ADRs 0056, 0064 and 0066).

pub(crate) mod retained;

use std::fs::{self, File};
use std::io::copy;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};

use crate::api::{SourceEnclosure, Track};
use crate::audio_format::{AudioFormat, ConversionOutcome};
use crate::audio_tags::AudioTags;
use crate::config::DownloadConfig;
use crate::remote_media;

const PUBLISHER_TAG_KEY: &str = "V4V_PUBLISHER";
const MAX_PATH_PART_CHARS: usize = 120;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedEnclosure {
    pub url: String,
    pub mime_type: Option<String>,
    pub bytes: Option<i64>,
    pub is_primary: bool,
    pub format: AudioFormat,
}

#[derive(Debug)]
pub struct DownloadedTrack {
    /// Current on-disk location. Points at a staging directory until
    /// [`DownloadedTrack::finalize`] is called.
    pub path: PathBuf,
    /// Final destination under `music_dir`. Use this for DB writes that need
    /// the post-finalize path; the file may not exist there yet.
    pub final_path: PathBuf,
    pub enclosure: SelectedEnclosure,
    pub detected_format: AudioFormat,
    pub format_warning: Option<String>,
    pub conversion: ConversionOutcome,
    source_warning: Option<String>,
    input: Option<retained::RetainedArtifact>,
    staging_dir: Option<retained::OwnedStaging>,
}

impl DownloadedTrack {
    /// Move the staged file to its final destination under `music_dir` and
    /// remove the staging directory. Returns the final path on success.
    pub fn finalize(mut self) -> Result<PathBuf> {
        self.publish()
    }

    pub(crate) fn publish(&mut self) -> Result<PathBuf> {
        self.promote()?;
        self.cleanup()?;
        Ok(self.final_path.clone())
    }

    pub(crate) fn promote(&mut self) -> Result<()> {
        if self.path != self.final_path {
            if let Some(input) = &self.input {
                retained::validate_destination(input.music_dir(), &self.final_path)?;
            }
            if let Some(parent) = self.final_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create final directory {}", parent.display()))?;
            }
            if let Some(input) = &self.input {
                retained::validate_destination(input.music_dir(), &self.final_path)?;
            }
            anyhow::ensure!(
                !self.final_path.exists(),
                "App kept the existing destination file {}",
                self.final_path.display()
            );
            // Both paths are on music storage. Link publication refuses an existing entry atomically.
            fs::hard_link(&self.path, &self.final_path).with_context(|| {
                format!(
                    "promote staged download {} -> {}",
                    self.path.display(),
                    self.final_path.display()
                )
            })?;
            self.path = self.final_path.clone();
        }
        Ok(())
    }

    pub(crate) fn undo_promotion(&mut self, staged: &Path) -> Result<()> {
        fs::remove_file(&self.path).with_context(|| {
            format!(
                "restore uncommitted download to staging {}",
                staged.display()
            )
        })?;
        self.path = staged.to_path_buf();
        Ok(())
    }

    /// Drop the staged download without promoting it to `music_dir`.
    pub fn discard(mut self) {
        if let Err(error) = self.cleanup() {
            eprintln!(
                "[{}] {error:#}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
    }

    pub(crate) fn cleanup(&mut self) -> Result<()> {
        if let Some(dir) = &self.staging_dir {
            dir.cleanup()?;
            self.staging_dir = None;
        }
        Ok(())
    }

    pub(crate) fn validate_input(&self) -> Result<()> {
        self.input
            .as_ref()
            .ok_or_else(|| anyhow!("downloaded input was already released"))?
            .validate()
    }

    pub(crate) fn retry_conversion(&mut self, cfg: &DownloadConfig) -> Result<()> {
        self.validate_input()?;
        let input = self.input.as_ref().expect("validated retained input");
        if self.path != input.path() && self.path.exists() {
            fs::remove_file(&self.path).with_context(|| {
                format!("remove previous conversion output {}", self.path.display())
            })?;
        }
        self.path = input.path().to_path_buf();
        let partial = self.path.with_extension("flac");
        if partial != self.path && partial.exists() {
            fs::remove_file(&partial).with_context(|| {
                format!("remove failed conversion output {}", partial.display())
            })?;
        }
        self.detected_format = AudioFormat::detect_from_file(&self.path)?;
        self.final_path
            .set_extension(self.detected_format.canonical_extension());
        self.convert(cfg);
        Ok(())
    }

    fn convert(&mut self, cfg: &DownloadConfig) {
        if self.detected_format != AudioFormat::Wav {
            return;
        }
        let result = cfg
            .flac_path
            .as_ref()
            .map_err(|_| anyhow!("flac_path is invalid; correct the converter setting"))
            .and_then(|path| {
                crate::audio_format::convert_retaining_input(&self.path, path.as_deref())
            });
        match result {
            Ok((path, outcome)) => {
                self.path = path;
                self.final_path.set_extension("flac");
                self.detected_format = AudioFormat::Flac;
                self.conversion = outcome;
                self.format_warning.clone_from(&self.source_warning);
            }
            Err(error) => {
                self.conversion = ConversionOutcome::WavRetained;
                let conversion =
                    format!("App retained usable WAV input after conversion failed: {error:#}");
                self.format_warning = Some(self.source_warning.as_ref().map_or_else(
                    || conversion.clone(),
                    |source| format!("{source}; {conversion}"),
                ));
            }
        }
    }
}

impl Drop for DownloadedTrack {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            eprintln!(
                "[{}] {error:#}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComparisonStatus {
    Match,
    Different,
    MissingSource,
    MissingTag,
    MissingBoth,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparisonRow {
    pub field: &'static str,
    pub source_value: Option<String>,
    pub tag_value: Option<String>,
    pub status: ComparisonStatus,
}

pub fn select_audio_enclosure(track: &Track) -> Option<SelectedEnclosure> {
    let source_enclosures = track.source_enclosures.as_deref().unwrap_or_default();

    source_enclosures
        .iter()
        .filter(|enclosure| enclosure.is_primary == Some(true))
        .find_map(selected_source_enclosure)
        .or_else(|| source_enclosures.iter().find_map(selected_source_enclosure))
        .or_else(|| selected_track_enclosure(track))
}

pub fn local_track_path(cfg: &DownloadConfig, track: &Track, extension: &str) -> PathBuf {
    let artist_dir = sanitize_path_part(
        track
            .track_artist
            .as_deref()
            .or(track.publisher_text.as_deref())
            .or(track.feed_guid.as_deref())
            .unwrap_or("unknown-artist"),
    );
    let album_dir = sanitize_path_part(
        track
            .feed_title
            .as_deref()
            .or(track.feed_guid.as_deref())
            .unwrap_or("unknown-album"),
    );
    let title = sanitize_path_part(
        track
            .title
            .as_deref()
            .or(track.name.as_deref())
            .or(track.track_guid.as_deref())
            .unwrap_or("unknown-track"),
    );

    let filename = track.track_number.map_or_else(
        || format!("{title}.{extension}"),
        |track_number| format!("{track_number:02} - {title}.{extension}"),
    );

    cfg.music_dir
        .join("artists")
        .join(artist_dir)
        .join(album_dir)
        .join(filename)
}

/// If `path` is a WAV file and the `flac` CLI is reachable (configured
/// executable, or `$PATH` when unset), re-encode it in place and return the new
/// FLAC path. Otherwise return `path` unchanged. Used by subscribe flows
/// that reuse a pre-existing local file so tag writes land on a taggable
/// container.
pub fn ensure_taggable_local_path(cfg: &DownloadConfig, path: &Path) -> PathBuf {
    if !matches!(AudioFormat::detect_from_file(path), Ok(AudioFormat::Wav)) {
        return path.to_path_buf();
    }
    let Ok(flac_path) = &cfg.flac_path else {
        eprintln!("App retained WAV file {} because flac_path is invalid. Correct the converter setting before converting this file.", path.display());
        return path.to_path_buf();
    };
    let flac_override = flac_path.as_deref();
    if !crate::audio_format::flac_cli_available(flac_override) {
        eprintln!(
            "ensure_taggable_local_path: flac CLI not reachable (override: {:?}); leaving {} as WAV",
            flac_override,
            path.display()
        );
        return path.to_path_buf();
    }
    match crate::audio_format::transcode_wav_to_flac(path, flac_override) {
        Ok(flac_path) => flac_path,
        Err(err) => {
            eprintln!(
                "ensure_taggable_local_path: WAV→FLAC transcode failed for {}: {:#}",
                path.display(),
                err
            );
            path.to_path_buf()
        }
    }
}

pub fn download_track(cfg: &DownloadConfig, track: &Track) -> Result<DownloadedTrack> {
    let enclosure =
        select_audio_enclosure(track).ok_or_else(|| anyhow!("no supported audio enclosure"))?;
    let declared_format = enclosure.format;
    let final_path_initial = local_track_path(cfg, track, declared_format.canonical_extension());
    let staging = create_staging_dir(cfg)?;
    let staging_dir = staging.path();

    let filename = final_path_initial
        .file_name()
        .map(|n| n.to_owned())
        .unwrap_or_else(|| {
            std::ffi::OsString::from(format!("track.{}", declared_format.canonical_extension()))
        });
    let staged = staging_dir.join(&filename);

    // Helper that cleans the staging dir if we bail out before constructing
    // DownloadedTrack (which would otherwise own the cleanup).
    let cleanup_on_err = |err: anyhow::Error| -> anyhow::Error {
        if let Err(cleanup) = staging.cleanup() {
            return err.context(format!("{cleanup:#}"));
        }
        err
    };

    if let Err(err) = download_enclosure(&enclosure.url, &staged) {
        return Err(cleanup_on_err(err));
    }
    if let Err(err) = validate_downloaded_size(&staged, enclosure.bytes) {
        return Err(cleanup_on_err(err));
    }

    // The bytes must be a container we support. Falling back to the declared
    // format here would relabel a redirect landing page as the expected audio
    // format, and the mismatch warning below would never fire (ADR 0056).
    let detected_format = match AudioFormat::detect_from_file(&staged) {
        Ok(format) => format,
        Err(err) => {
            return Err(cleanup_on_err(err.context(format!(
                "downloaded enclosure for {} is not a supported audio container",
                enclosure.url
            ))))
        }
    };

    let mut warnings: Vec<String> = Vec::new();
    if detected_format != declared_format {
        warnings.push(format!(
            "RSS declared {} but file is {}",
            declared_format.display_label(),
            detected_format.display_label()
        ));
    }

    // Rename within the staging dir so the extension matches the detected
    // container before we hand the file to the tagging pipeline.
    let current_path = if detected_format != declared_format {
        let stem = staged
            .file_stem()
            .map(|s| s.to_os_string())
            .unwrap_or_default();
        let mut renamed_name = stem;
        renamed_name.push(".");
        renamed_name.push(detected_format.canonical_extension());
        let renamed = staging_dir.join(&renamed_name);
        if renamed != staged {
            if let Err(err) = fs::rename(&staged, &renamed)
                .with_context(|| format!("rename {} -> {}", staged.display(), renamed.display()))
            {
                return Err(cleanup_on_err(err));
            }
        }
        renamed
    } else {
        staged
    };

    // Recompute the final path from the (possibly upgraded) format so the
    // caller can move the staged file into music_dir at finalize time.
    let final_path = local_track_path(cfg, track, detected_format.canonical_extension());

    let format_warning = if warnings.is_empty() {
        None
    } else {
        Some(warnings.join("; "))
    };

    let input = retained::RetainedArtifact::capture(&cfg.music_dir, &current_path, enclosure.bytes)
        .map_err(cleanup_on_err)?;
    let mut downloaded = DownloadedTrack {
        path: current_path,
        final_path,
        enclosure,
        detected_format,
        source_warning: format_warning.clone(),
        format_warning,
        conversion: ConversionOutcome::NotRequired,
        input: Some(input),
        staging_dir: Some(staging),
    };
    downloaded.convert(cfg);
    Ok(downloaded)
}

fn create_staging_dir(cfg: &DownloadConfig) -> Result<retained::OwnedStaging> {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    fs::create_dir_all(&cfg.music_dir)
        .with_context(|| format!("prepare download storage {}", cfg.music_dir.display()))?;
    let staging_root = cfg.music_dir.join(".v4vmm-staging");
    retained::validate_destination(&cfg.music_dir, &staging_root.join("new"))?;
    let dir = staging_root.join(format!("{pid}-{nanos}-{seq}"));
    fs::create_dir_all(&dir)
        .with_context(|| format!("create staging directory {}", dir.display()))?;
    retained::OwnedStaging::capture(&cfg.music_dir, dir)
}

pub fn download_enclosure(url: &str, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create download directory {}", parent.display()))?;
    }

    let mut response = remote_media::fetch(url, "enclosure")?;
    let mut output =
        File::create(path).with_context(|| format!("create download {}", path.display()))?;
    copy(&mut response, &mut output)
        .with_context(|| format!("write download {}", path.display()))?;

    Ok(())
}

fn validate_downloaded_size(path: &Path, expected_bytes: Option<i64>) -> Result<()> {
    let Some(expected_bytes) = expected_bytes.filter(|bytes| *bytes > 0) else {
        return Ok(());
    };
    let expected_bytes =
        u64::try_from(expected_bytes).context("convert expected enclosure bytes")?;
    let actual_bytes = fs::metadata(path)
        .with_context(|| format!("stat downloaded enclosure {}", path.display()))?
        .len();
    anyhow::ensure!(
        actual_bytes == expected_bytes,
        "downloaded enclosure size mismatch for {}: expected {} bytes, got {} bytes",
        path.display(),
        expected_bytes,
        actual_bytes
    );
    Ok(())
}

pub fn compare_track_tags(track: &Track, tags: &AudioTags) -> Vec<ComparisonRow> {
    let mut rows = vec![
        comparison_row(
            "Title",
            track
                .title
                .as_deref()
                .or(track.name.as_deref())
                .map(sanitize_title_text),
            tags.title.as_deref(),
        ),
        comparison_row(
            "Artist",
            track.track_artist.as_deref(),
            tags.artist.as_deref(),
        ),
        comparison_row(
            "Album/Feed",
            track.feed_title.as_deref(),
            tags.album.as_deref(),
        ),
        comparison_row(
            "Track #",
            track.track_number.map(|number| number.to_string()),
            tags.track_number.clone(),
        ),
        comparison_row(
            "Publisher",
            track.publisher_text.as_deref(),
            tags.custom.get(PUBLISHER_TAG_KEY).map(String::as_str),
        ),
    ];
    if normalized(track.release_artist.as_deref()).is_some() {
        rows.push(comparison_row(
            "Album artist",
            track.release_artist.as_deref(),
            tags.fields
                .iter()
                .find(|field| field.frame_id == "TPE2")
                .map(|field| field.value.as_str()),
        ));
    }
    rows
}

fn selected_source_enclosure(enclosure: &SourceEnclosure) -> Option<SelectedEnclosure> {
    let url = normalized(enclosure.url.as_deref())?;
    let format = classify_enclosure(enclosure.mime_type.as_deref(), &url)?;

    Some(SelectedEnclosure {
        url,
        mime_type: normalized(enclosure.mime_type.as_deref()),
        bytes: enclosure.bytes,
        is_primary: enclosure.is_primary.unwrap_or(false),
        format,
    })
}

fn selected_track_enclosure(track: &Track) -> Option<SelectedEnclosure> {
    let url = normalized(track.enclosure_url.as_deref())?;
    let format = classify_enclosure(track.enclosure_type.as_deref(), &url)?;

    Some(SelectedEnclosure {
        url,
        mime_type: normalized(track.enclosure_type.as_deref()),
        bytes: track.enclosure_bytes,
        is_primary: true,
        format,
    })
}

fn classify_enclosure(mime_type: Option<&str>, url: &str) -> Option<AudioFormat> {
    let from_mime = mime_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(AudioFormat::from_declared_mime);
    let from_ext = AudioFormat::from_url_extension(url);
    // URL extension trumps declared mime on disagreement (feeds frequently lie).
    match (from_mime, from_ext) {
        (Some(mime), Some(ext)) if mime != ext => Some(ext),
        (Some(mime), _) => Some(mime),
        (None, Some(ext)) => Some(ext),
        (None, None) => None,
    }
}

fn comparison_row(
    field: &'static str,
    source_value: Option<impl AsRef<str>>,
    tag_value: Option<impl AsRef<str>>,
) -> ComparisonRow {
    let source_value = source_value.and_then(|value| normalized(Some(value.as_ref())));
    let tag_value = tag_value.and_then(|value| normalized(Some(value.as_ref())));
    let status = match (&source_value, &tag_value) {
        (Some(source), Some(tag)) if source == tag => ComparisonStatus::Match,
        (Some(_), Some(_)) => ComparisonStatus::Different,
        (Some(_), None) => ComparisonStatus::MissingTag,
        (None, Some(_)) => ComparisonStatus::MissingSource,
        (None, None) => ComparisonStatus::MissingBoth,
    };

    ComparisonRow {
        field,
        source_value,
        tag_value,
        status,
    }
}

fn normalized(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn sanitize_title_text(value: &str) -> String {
    value
        .trim_start()
        .strip_prefix("- ")
        .map(str::trim_start)
        .unwrap_or(value)
        .to_string()
}

fn sanitize_path_part(value: &str) -> String {
    let mut out = String::new();
    for ch in value.trim().chars() {
        match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' if !out.ends_with('-') => {
                out.push('-');
            }
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => {}
            ch if ch.is_control() => {
                if !out.ends_with(' ') && !out.is_empty() {
                    out.push(' ');
                }
            }
            ch => out.push(ch),
        }
    }

    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    let out = out.trim_matches([' ', '.']);
    let out = truncate_path_part(out, MAX_PATH_PART_CHARS);
    if out.is_empty() {
        "unknown".into()
    } else if is_reserved_path_part(&out) {
        format!("_{out}")
    } else {
        out
    }
}

fn truncate_path_part(value: &str, max_chars: usize) -> String {
    let truncated = value.chars().take(max_chars).collect::<String>();
    truncated.trim_matches([' ', '.']).to_string()
}

fn is_reserved_path_part(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "." | ".."
            | "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

#[cfg(test)]
mod tests {

    #[test]
    fn adr_0066_invalid_converter_preserves_downloads_and_retains_wav() {
        use std::io::{Read, Write};
        for (extension, body) in [
            ("mp3", b"ID3\x04\x00\x00\x00\x00\x00\x00mp3data".as_slice()),
            ("wav", b"RIFF\x24\x00\x00\x00WAVEfmt ".as_slice()),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!(
                "http://{}/audio.{extension}",
                listener.local_addr().unwrap()
            );
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                stream.read(&mut [0; 2048]).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(body).unwrap();
            });
            let temp = tempfile::tempdir().unwrap();
            let text = format!("music_dir = {:?}\nflac_path = false\n", temp.path());
            let cfg = crate::config::ConfigSnapshot::from_bytes(
                std::path::Path::new("fixture.toml"),
                text.into_bytes(),
            )
            .unwrap()
            .downloads()
            .unwrap();
            let mut item = track();
            item.enclosure_url = Some(url);
            item.enclosure_bytes = Some(i64::try_from(body.len()).unwrap());
            let downloaded = download_track(&cfg, &item).unwrap();
            assert_eq!(fs::read(&downloaded.path).unwrap(), body);
            if extension == "wav" {
                assert!(downloaded
                    .format_warning
                    .as_ref()
                    .unwrap()
                    .contains("flac_path is invalid"));
            }
            let final_path = downloaded.finalize().unwrap();
            assert_eq!(
                super::ensure_taggable_local_path(&cfg, &final_path),
                final_path
            );
            assert_eq!(fs::read(&final_path).unwrap(), body);
            server.join().unwrap();
        }
    }

    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use super::{
        compare_track_tags, download_track, local_track_path, select_audio_enclosure,
        ComparisonRow, ComparisonStatus, SelectedEnclosure, MAX_PATH_PART_CHARS, PUBLISHER_TAG_KEY,
    };
    use crate::api::{SourceEnclosure, Track};
    use crate::audio_format::AudioFormat;
    use crate::audio_tags::AudioTags;
    use crate::config::DownloadConfig;

    fn track() -> Track {
        Track {
            track_guid: Some("track-guid".into()),
            feed_guid: Some("feed-guid".into()),
            feed_title: Some("Feed / Title".into()),
            title: Some("Song: Title?".into()),
            track_number: Some(4),
            track_artist: Some("Artist".into()),
            publisher_text: Some("Wavlake".into()),
            ..Track::default()
        }
    }

    #[test]
    fn prefers_first_primary_audio_source_enclosure() {
        let mut track = track();
        track.source_enclosures = Some(vec![
            SourceEnclosure {
                url: Some("https://example.com/notes.txt".into()),
                mime_type: Some("text/plain".into()),
                is_primary: Some(true),
                ..SourceEnclosure::default()
            },
            SourceEnclosure {
                url: Some("https://example.com/song.mp3".into()),
                mime_type: Some("audio/mpeg".into()),
                bytes: Some(123),
                is_primary: Some(true),
                ..SourceEnclosure::default()
            },
            SourceEnclosure {
                url: Some("https://example.com/alt.flac".into()),
                mime_type: Some("audio/flac".into()),
                is_primary: Some(false),
                ..SourceEnclosure::default()
            },
        ]);

        assert_eq!(
            select_audio_enclosure(&track),
            Some(SelectedEnclosure {
                url: "https://example.com/song.mp3".into(),
                mime_type: Some("audio/mpeg".into()),
                bytes: Some(123),
                is_primary: true,
                format: AudioFormat::Mp3,
            })
        );
    }

    #[test]
    fn falls_back_to_track_enclosure_url() {
        let mut track = track();
        track.enclosure_url = Some("https://example.com/song.mp3?download=1".into());

        assert_eq!(
            select_audio_enclosure(&track).map(|enclosure| enclosure.url),
            Some("https://example.com/song.mp3?download=1".into())
        );
    }

    #[test]
    fn url_extension_overrides_lying_mime() {
        let mut track = track();
        track.source_enclosures = Some(vec![SourceEnclosure {
            url: Some("https://example.com/song.wav".into()),
            mime_type: Some("audio/mpeg".into()),
            is_primary: Some(true),
            ..SourceEnclosure::default()
        }]);

        let selected = select_audio_enclosure(&track).expect("selected");
        assert_eq!(selected.format, AudioFormat::Wav);
    }

    #[test]
    fn builds_deterministic_sanitized_local_path() {
        let cfg = DownloadConfig {
            music_dir: "/tmp/v4vmm-test".into(),
            flac_path: Ok(None),
        };

        assert_eq!(
            local_track_path(&cfg, &track(), "mp3"),
            PathBuf::from("/tmp/v4vmm-test")
                .join("artists")
                .join("Artist")
                .join("Feed - Title")
                .join("04 - Song- Title-.mp3")
        );
    }

    #[test]
    fn sanitizes_ntfs_reserved_names_and_trailing_dots() {
        let cfg = DownloadConfig {
            music_dir: "/tmp/v4vmm-test".into(),
            flac_path: Ok(None),
        };
        let mut track = track();
        track.track_artist = Some("CON".into());
        track.feed_title = Some("AUX.".into());
        track.title = Some("NUL ".into());

        assert_eq!(
            local_track_path(&cfg, &track, "mp3"),
            PathBuf::from("/tmp/v4vmm-test")
                .join("artists")
                .join("_CON")
                .join("_AUX")
                .join("04 - _NUL.mp3")
        );
    }

    #[test]
    fn sanitizes_control_chars_and_caps_segment_length() {
        let cfg = DownloadConfig {
            music_dir: "/tmp/v4vmm-test".into(),
            flac_path: Ok(None),
        };
        let mut track = track();
        track.track_artist = Some("Artist\tName".into());
        track.feed_title = Some("Feed\0Title".into());
        track.title = Some("a".repeat(150));

        let path = local_track_path(&cfg, &track, "mp3");

        assert_eq!(
            path.parent().expect("parent"),
            PathBuf::from("/tmp/v4vmm-test")
                .join("artists")
                .join("Artist Name")
                .join("Feed Title")
        );
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("utf8 filename");
        assert_eq!(
            filename.len(),
            "04 - ".len() + MAX_PATH_PART_CHARS + ".mp3".len()
        );
    }

    #[test]
    fn compares_source_fields_to_tag_fields() {
        let mut custom = BTreeMap::new();
        custom.insert(PUBLISHER_TAG_KEY.into(), "Wavlake".into());
        let tags = AudioTags {
            title: Some("Song: Title?".into()),
            artist: Some("Other Artist".into()),
            album: None,
            track_number: Some("4".into()),
            custom,
            ..AudioTags::default()
        };

        assert_eq!(
            compare_track_tags(&track(), &tags),
            vec![
                ComparisonRow {
                    field: "Title",
                    source_value: Some("Song: Title?".into()),
                    tag_value: Some("Song: Title?".into()),
                    status: ComparisonStatus::Match,
                },
                ComparisonRow {
                    field: "Artist",
                    source_value: Some("Artist".into()),
                    tag_value: Some("Other Artist".into()),
                    status: ComparisonStatus::Different,
                },
                ComparisonRow {
                    field: "Album/Feed",
                    source_value: Some("Feed / Title".into()),
                    tag_value: None,
                    status: ComparisonStatus::MissingTag,
                },
                ComparisonRow {
                    field: "Track #",
                    source_value: Some("4".into()),
                    tag_value: Some("4".into()),
                    status: ComparisonStatus::Match,
                },
                ComparisonRow {
                    field: "Publisher",
                    source_value: Some("Wavlake".into()),
                    tag_value: Some("Wavlake".into()),
                    status: ComparisonStatus::Match,
                },
            ]
        );
    }

    #[test]
    fn downloads_selected_mp3_to_local_path() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut buf = [0_u8; 1024];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\nContent-Length: 17\r\nConnection: close\r\n\r\nID3\x04\x00\x00\x00\x00\x00\x00mp3data",
            )
            .expect("write response");
        });

        let temp = tempfile::tempdir().expect("tempdir");
        let cfg = DownloadConfig {
            music_dir: temp.path().join("music"),
            flac_path: Ok(None),
        };
        let mut track = track();
        track.enclosure_url = Some(format!("http://{addr}/song.mp3"));
        let downloaded = download_track(&cfg, &track).expect("download");

        let expected_final = temp
            .path()
            .join("music")
            .join("artists")
            .join("Artist")
            .join("Feed - Title")
            .join("04 - Song- Title-.mp3");
        assert_eq!(downloaded.final_path, expected_final);
        assert_ne!(
            downloaded.path, expected_final,
            "staged path should differ from final until finalize()"
        );
        assert_eq!(
            fs::read(&downloaded.path).expect("read staged"),
            b"ID3\x04\x00\x00\x00\x00\x00\x00mp3data"
        );
        let final_path = downloaded.finalize().expect("finalize");
        assert_eq!(final_path, expected_final);
        assert_eq!(
            fs::read(&final_path).expect("read final"),
            b"ID3\x04\x00\x00\x00\x00\x00\x00mp3data"
        );
    }

    #[test]
    fn follows_enclosure_redirect_instead_of_saving_redirect_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        std::thread::spawn(move || {
            let (mut redirect_stream, _) = listener.accept().expect("accept redirect request");
            let mut buf = [0_u8; 1024];
            let _ = std::io::Read::read(&mut redirect_stream, &mut buf);
            let location = format!("http://{addr}/Music/song file.mp3");
            let response = format!(
                "HTTP/1.1 301 Moved Permanently\r\nLocation: {location}\r\nContent-Type: text/html\r\nContent-Length: 8\r\nConnection: close\r\n\r\nredirect"
            );
            std::io::Write::write_all(&mut redirect_stream, response.as_bytes())
                .expect("write redirect response");

            let (mut audio_stream, _) = listener.accept().expect("accept audio request");
            let mut buf = [0_u8; 1024];
            let _ = std::io::Read::read(&mut audio_stream, &mut buf);
            std::io::Write::write_all(
                &mut audio_stream,
                b"HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\nContent-Length: 17\r\nConnection: close\r\n\r\nID3\x04\x00\x00\x00\x00\x00\x00mp3data",
            )
            .expect("write audio response");
        });

        let temp = tempfile::tempdir().expect("tempdir");
        let cfg = DownloadConfig {
            music_dir: temp.path().join("music"),
            flac_path: Ok(None),
        };
        let mut track = track();
        track.enclosure_url = Some(format!("http://{addr}/song.mp3"));
        track.enclosure_bytes = Some(17);
        let downloaded = download_track(&cfg, &track).expect("download");

        assert_eq!(
            fs::read(&downloaded.path).expect("read staged"),
            b"ID3\x04\x00\x00\x00\x00\x00\x00mp3data"
        );
    }

    /// A feed that declares no byte count still cannot promote a redirect
    /// landing page as a playable track: the staged bytes have to be a
    /// container we support (ADR 0056).
    #[test]
    fn rejects_downloaded_enclosure_that_is_not_a_supported_container() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut buf = [0_u8; 1024];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\nContent-Length: 18\r\nConnection: close\r\n\r\n<html>moved</html>",
            )
            .expect("write response");
        });

        let temp = tempfile::tempdir().expect("tempdir");
        let cfg = DownloadConfig {
            music_dir: temp.path().join("music"),
            flac_path: Ok(None),
        };
        let mut track = track();
        track.enclosure_url = Some(format!("http://{addr}/song.mp3"));
        track.enclosure_bytes = None;

        let error = download_track(&cfg, &track)
            .expect_err("a non-audio body must not be promoted as a track");

        assert!(
            error
                .to_string()
                .contains("not a supported audio container"),
            "error should explain the container rejection: {error}"
        );
        assert!(
            fs::read_dir(cfg.music_dir.join(".v4vmm-staging"))
                .expect("read staging root")
                .next()
                .is_none(),
            "staging must be cleaned after a rejected download"
        );
    }

    #[test]
    fn rejects_downloaded_enclosure_when_advertised_size_does_not_match() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut buf = [0_u8; 1024];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\nContent-Length: 7\r\nConnection: close\r\n\r\nmp3data",
            )
            .expect("write response");
        });

        let temp = tempfile::tempdir().expect("tempdir");
        let cfg = DownloadConfig {
            music_dir: temp.path().join("music"),
            flac_path: Ok(None),
        };
        let mut track = track();
        track.enclosure_url = Some(format!("http://{addr}/song.mp3"));
        track.enclosure_bytes = Some(8);

        let error = download_track(&cfg, &track)
            .expect_err("size mismatch should reject partial downloads");

        assert!(
            error.to_string().contains("size mismatch"),
            "error should explain the byte-count mismatch: {error}"
        );
        assert!(
            fs::read_dir(cfg.music_dir.join(".v4vmm-staging"))
                .expect("read staging root")
                .next()
                .is_none(),
            "rejected downloads should clean up per-download staging files"
        );
    }
}
