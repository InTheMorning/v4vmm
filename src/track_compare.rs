use std::fs::{self, File};
use std::io::copy;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client as ReqwestClient;
use reqwest::Url;

use crate::api::{SourceEnclosure, Track};
use crate::audio_format::AudioFormat;
use crate::audio_tags::AudioTags;
use crate::config::Config;

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
    staging_dir: Option<PathBuf>,
}

impl DownloadedTrack {
    /// Move the staged file to its final destination under `music_dir` and
    /// remove the staging directory. Returns the final path on success.
    pub fn finalize(mut self) -> Result<PathBuf> {
        if self.path != self.final_path {
            if let Some(parent) = self.final_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create final directory {}", parent.display()))?;
            }
            fs::rename(&self.path, &self.final_path).with_context(|| {
                format!(
                    "promote staged download {} -> {}",
                    self.path.display(),
                    self.final_path.display()
                )
            })?;
            self.path = self.final_path.clone();
        }
        if let Some(dir) = self.staging_dir.take() {
            let _ = fs::remove_dir_all(&dir);
        }
        Ok(self.final_path.clone())
    }

    /// Drop the staged download without promoting it to `music_dir`.
    pub fn discard(mut self) {
        if let Some(dir) = self.staging_dir.take() {
            let _ = fs::remove_dir_all(&dir);
        }
    }
}

impl Drop for DownloadedTrack {
    fn drop(&mut self) {
        if let Some(dir) = self.staging_dir.take() {
            let _ = fs::remove_dir_all(&dir);
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

pub fn local_track_path(cfg: &Config, track: &Track, extension: &str) -> PathBuf {
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
/// override first, then `$PATH`), re-encode it in place and return the new
/// FLAC path. Otherwise return `path` unchanged. Used by subscribe flows
/// that reuse a pre-existing local file so tag writes land on a taggable
/// container.
pub fn ensure_taggable_local_path(cfg: &Config, path: &Path) -> PathBuf {
    if !matches!(AudioFormat::detect_from_file(path), Ok(AudioFormat::Wav)) {
        return path.to_path_buf();
    }
    let flac_override = cfg.flac_path.as_deref();
    let resolved_override = if flac_override
        .is_some_and(|p| crate::audio_format::flac_cli_available(Some(p)))
    {
        flac_override
    } else if crate::audio_format::flac_cli_available(None) {
        None
    } else {
        eprintln!(
            "ensure_taggable_local_path: flac CLI not reachable (override: {:?}); leaving {} as WAV",
            flac_override,
            path.display()
        );
        return path.to_path_buf();
    };
    match crate::audio_format::transcode_wav_to_flac(path, resolved_override) {
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

pub fn download_track(
    cfg: &Config,
    client: &ReqwestClient,
    track: &Track,
) -> Result<DownloadedTrack> {
    let enclosure =
        select_audio_enclosure(track).ok_or_else(|| anyhow!("no supported audio enclosure"))?;
    let declared_format = enclosure.format;
    let final_path_initial = local_track_path(cfg, track, declared_format.canonical_extension());
    let staging_dir = create_staging_dir(cfg)?;

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
        let _ = fs::remove_dir_all(&staging_dir);
        err
    };

    if let Err(err) = download_enclosure(client, &enclosure.url, &staged) {
        return Err(cleanup_on_err(err));
    }

    let detected_format = AudioFormat::detect_from_file(&staged).unwrap_or(declared_format);

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
    let mut current_path = if detected_format != declared_format {
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
    let mut current_format = detected_format;

    // WAV → FLAC silent upgrade (still inside the staging dir).
    if current_format == AudioFormat::Wav {
        let flac_override = cfg.flac_path.as_deref();
        let have_flac = crate::audio_format::flac_cli_available(flac_override);
        let have_ffmpeg_fallback = !have_flac; // transcode_wav_to_flac probes ffmpeg internally
        if have_flac || have_ffmpeg_fallback {
            match crate::audio_format::transcode_wav_to_flac(&current_path, flac_override) {
                Ok(flac_path) => {
                    current_path = flac_path;
                    current_format = AudioFormat::Flac;
                    warnings.push("upgraded WAV to FLAC so tags can be written".to_string());
                }
                Err(err) => warnings.push(format!("WAV→FLAC transcode failed: {err:#}")),
            }
        } else {
            warnings.push(
                "install the `flac` CLI (or `ffmpeg`) to enable tagging of WAV downloads"
                    .to_string(),
            );
        }
    }

    // Recompute the final path from the (possibly upgraded) format so the
    // caller can move the staged file into music_dir at finalize time.
    let final_path = local_track_path(cfg, track, current_format.canonical_extension());

    let format_warning = if warnings.is_empty() {
        None
    } else {
        Some(warnings.join("; "))
    };

    Ok(DownloadedTrack {
        path: current_path,
        final_path,
        enclosure,
        detected_format: current_format,
        format_warning,
        staging_dir: Some(staging_dir),
    })
}

fn create_staging_dir(cfg: &Config) -> Result<PathBuf> {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let staging_root = cfg.music_dir.join(".v4vmm-staging");
    let dir = staging_root.join(format!("{pid}-{nanos}-{seq}"));
    fs::create_dir_all(&dir)
        .with_context(|| format!("create staging directory {}", dir.display()))?;
    Ok(dir)
}

pub fn download_enclosure(client: &ReqwestClient, url: &str, path: &Path) -> Result<()> {
    let parsed = Url::parse(url).with_context(|| format!("parse enclosure URL {url}"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => return Err(anyhow!("unsupported enclosure URL scheme: {scheme}")),
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create download directory {}", parent.display()))?;
    }

    let mut response = client
        .get(parsed)
        .send()
        .with_context(|| format!("download enclosure {url}"))?
        .error_for_status()
        .with_context(|| format!("download enclosure {url}"))?;
    let mut output =
        File::create(path).with_context(|| format!("create download {}", path.display()))?;
    copy(&mut response, &mut output)
        .with_context(|| format!("write download {}", path.display()))?;

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
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use reqwest::blocking::Client as ReqwestClient;

    use super::{
        compare_track_tags, download_track, local_track_path, select_audio_enclosure,
        ComparisonRow, ComparisonStatus, SelectedEnclosure, MAX_PATH_PART_CHARS, PUBLISHER_TAG_KEY,
    };
    use crate::api::{SourceEnclosure, Track};
    use crate::audio_format::AudioFormat;
    use crate::audio_tags::AudioTags;
    use crate::config::{Config, PlaybackConfig};
    use crate::theme_profile::ThemeProfile;

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
        let cfg = Config {
            music_dir: "/tmp/v4vmm-test".into(),
            db_path: "/tmp/v4vmm-test.sqlite".into(),
            flac_path: None,
            playback: PlaybackConfig::default(),
            ui_scale: Default::default(),
            theme_profile: ThemeProfile::default(),
            workspace_layout: None,
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
        let cfg = Config {
            music_dir: "/tmp/v4vmm-test".into(),
            db_path: "/tmp/v4vmm-test.sqlite".into(),
            flac_path: None,
            playback: PlaybackConfig::default(),
            ui_scale: Default::default(),
            theme_profile: ThemeProfile::default(),
            workspace_layout: None,
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
        let cfg = Config {
            music_dir: "/tmp/v4vmm-test".into(),
            db_path: "/tmp/v4vmm-test.sqlite".into(),
            flac_path: None,
            playback: PlaybackConfig::default(),
            ui_scale: Default::default(),
            theme_profile: ThemeProfile::default(),
            workspace_layout: None,
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
                b"HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\nContent-Length: 7\r\nConnection: close\r\n\r\nmp3data",
            )
            .expect("write response");
        });

        let temp = tempfile::tempdir().expect("tempdir");
        let cfg = Config {
            music_dir: temp.path().join("music"),
            db_path: temp.path().join("db.sqlite"),
            flac_path: None,
            playback: PlaybackConfig::default(),
            ui_scale: Default::default(),
            theme_profile: ThemeProfile::default(),
            workspace_layout: None,
        };
        let mut track = track();
        track.enclosure_url = Some(format!("http://{addr}/song.mp3"));
        let downloaded = download_track(&cfg, &ReqwestClient::new(), &track).expect("download");

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
        assert_eq!(fs::read(&downloaded.path).expect("read staged"), b"mp3data");
        let final_path = downloaded.finalize().expect("finalize");
        assert_eq!(final_path, expected_final);
        assert_eq!(fs::read(&final_path).expect("read final"), b"mp3data");
    }
}
