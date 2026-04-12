use std::fs::{self, File};
use std::io::copy;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client as ReqwestClient;
use reqwest::Url;

use crate::audio_tags::AudioTags;
use crate::config::Config;
use crate::musicindex::{SourceEnclosure, Track};

const PUBLISHER_TAG_KEY: &str = "V4V_PUBLISHER";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedEnclosure {
    pub url: String,
    pub mime_type: Option<String>,
    pub bytes: Option<i64>,
    pub is_primary: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadedTrack {
    pub path: PathBuf,
    pub enclosure: SelectedEnclosure,
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

pub fn select_mp3_enclosure(track: &Track) -> Option<SelectedEnclosure> {
    let source_enclosures = track.source_enclosures.as_deref().unwrap_or_default();

    source_enclosures
        .iter()
        .filter(|enclosure| enclosure.is_primary == Some(true))
        .find_map(selected_source_enclosure)
        .or_else(|| source_enclosures.iter().find_map(selected_source_enclosure))
        .or_else(|| selected_track_enclosure(track))
}

pub fn local_mp3_path(cfg: &Config, track: &Track) -> PathBuf {
    let feed_dir = sanitize_path_part(
        track
            .feed_title
            .as_deref()
            .or(track.feed_guid.as_deref())
            .unwrap_or("unknown-feed"),
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
        || format!("{title}.mp3"),
        |track_number| format!("{track_number:02} - {title}.mp3"),
    );

    cfg.music_dir.join(feed_dir).join(filename)
}

pub fn download_track_mp3(
    cfg: &Config,
    client: &ReqwestClient,
    track: &Track,
) -> Result<DownloadedTrack> {
    let enclosure =
        select_mp3_enclosure(track).ok_or_else(|| anyhow!("no MP3 enclosure available"))?;
    let path = local_mp3_path(cfg, track);
    download_enclosure(client, &enclosure.url, &path)?;

    Ok(DownloadedTrack { path, enclosure })
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
    vec![
        comparison_row(
            "Title",
            track.title.as_deref().or(track.name.as_deref()),
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
    ]
}

fn selected_source_enclosure(enclosure: &SourceEnclosure) -> Option<SelectedEnclosure> {
    let url = normalized(enclosure.url.as_deref())?;
    if !is_mp3_enclosure(enclosure.mime_type.as_deref(), &url) {
        return None;
    }

    Some(SelectedEnclosure {
        url,
        mime_type: normalized(enclosure.mime_type.as_deref()),
        bytes: enclosure.bytes,
        is_primary: enclosure.is_primary.unwrap_or(false),
    })
}

fn selected_track_enclosure(track: &Track) -> Option<SelectedEnclosure> {
    let url = normalized(track.enclosure_url.as_deref())?;
    if !is_mp3_enclosure(track.enclosure_type.as_deref(), &url) {
        return None;
    }

    Some(SelectedEnclosure {
        url,
        mime_type: normalized(track.enclosure_type.as_deref()),
        bytes: track.enclosure_bytes,
        is_primary: true,
    })
}

fn is_mp3_enclosure(mime_type: Option<&str>, url: &str) -> bool {
    mime_type
        .map(|value| value.eq_ignore_ascii_case("audio/mpeg"))
        .unwrap_or(false)
        || url
            .split_once('?')
            .map_or(url, |(path, _query)| path)
            .to_ascii_lowercase()
            .ends_with(".mp3")
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

fn sanitize_path_part(value: &str) -> String {
    let mut out = String::new();
    for ch in value.trim().chars() {
        match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' if !out.ends_with('-') => {
                out.push('-');
            }
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => {}
            ch if ch.is_control() => {}
            ch => out.push(ch),
        }
    }

    let out = out.trim_matches([' ', '.']);
    if out.is_empty() {
        "unknown".into()
    } else {
        out.into()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use reqwest::blocking::Client as ReqwestClient;

    use super::{
        compare_track_tags, download_track_mp3, local_mp3_path, select_mp3_enclosure,
        ComparisonRow, ComparisonStatus, SelectedEnclosure, PUBLISHER_TAG_KEY,
    };
    use crate::audio_tags::AudioTags;
    use crate::config::Config;
    use crate::musicindex::{SourceEnclosure, Track};

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
    fn prefers_primary_mp3_source_enclosure() {
        let mut track = track();
        track.source_enclosures = Some(vec![
            SourceEnclosure {
                url: Some("https://example.com/song.flac".into()),
                mime_type: Some("audio/flac".into()),
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
                url: Some("https://example.com/alt.mp3".into()),
                mime_type: Some("audio/mpeg".into()),
                is_primary: Some(false),
                ..SourceEnclosure::default()
            },
        ]);

        assert_eq!(
            select_mp3_enclosure(&track),
            Some(SelectedEnclosure {
                url: "https://example.com/song.mp3".into(),
                mime_type: Some("audio/mpeg".into()),
                bytes: Some(123),
                is_primary: true,
            })
        );
    }

    #[test]
    fn falls_back_to_track_enclosure_url() {
        let mut track = track();
        track.enclosure_url = Some("https://example.com/song.mp3?download=1".into());

        assert_eq!(
            select_mp3_enclosure(&track).map(|enclosure| enclosure.url),
            Some("https://example.com/song.mp3?download=1".into())
        );
    }

    #[test]
    fn builds_deterministic_sanitized_local_path() {
        let cfg = Config {
            music_dir: "/tmp/v4vmm-test".into(),
            db_path: "/tmp/v4vmm-test.sqlite".into(),
        };

        assert_eq!(
            local_mp3_path(&cfg, &track()),
            PathBuf::from("/tmp/v4vmm-test")
                .join("Feed - Title")
                .join("04 - Song- Title-.mp3")
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
            date: None,
            custom,
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
        };
        let mut track = track();
        track.enclosure_url = Some(format!("http://{addr}/song.mp3"));
        let downloaded = download_track_mp3(&cfg, &ReqwestClient::new(), &track).expect("download");

        assert_eq!(
            fs::read(downloaded.path).expect("read download"),
            b"mp3data"
        );
    }
}
