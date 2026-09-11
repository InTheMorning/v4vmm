#![warn(clippy::pedantic)]
#![expect(
    clippy::module_name_repetitions,
    reason = "drop-file producer names mirror the ADR 0059 contract"
)]

//! Local mpv now-playing drop-file producer for ADR 0059.
//!
//! The publisher owns relay payload delivery. This module only writes and
//! removes the local `musicindex.nowplaying/1` input file for the built-in mpv
//! source.

use std::fs;
use std::io::ErrorKind;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::Serialize;

use crate::api::PaymentRoute;
use crate::audio_tags::{read_audio_tags, AudioTags};
use crate::playback::NowPlayingUpdate;

/// Supported now-playing producer schema.
pub const DROP_FILE_SCHEMA: &str = "musicindex.nowplaying/1";
/// Target-scoped final drop files end with this suffix.
pub const DROP_FILE_SUFFIX: &str = ".nowplaying.json";
/// Route provenance marker for embedded audio tags.
pub const EMBEDDED_ID3_ROUTES_SOURCE: &str = "embedded-id3";
/// Readiness reason retained when a playing track has no embedded route tag.
pub const MISSING_VALUE_ROUTES_TAG_REASON: &str = "missing embedded MusicIndex Value Routes tag";
/// Operator warning emitted before app shutdown clears the mpv source file.
pub const SHUTDOWN_WARNING: &str = "v4vmm::broadcast: closing v4vmm removes the mpv now-playing drop file; publisher output for the built-in player stops.";

const FEED_GUID_KEY: &str = "Feed Guid";
const IMAGE_KEY: &str = "Image";
const TRACK_GUID_KEY: &str = "Track Guid";
const VALUE_ROUTES_KEY: &str = "Value Routes";
static PREPARATION_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const MUSICINDEX_VOCABULARY: &[(&str, &str)] = &[
    (FEED_GUID_KEY, "TXXX:MusicIndex Feed Guid"),
    (IMAGE_KEY, "TXXX:MusicIndex Image"),
    (TRACK_GUID_KEY, "TXXX:MusicIndex Track Guid"),
    (VALUE_ROUTES_KEY, "TXXX:MusicIndex Value Routes"),
];

/// Active producer for one mpv now-playing drop file.
#[derive(Debug)]
pub struct DropFileProducer {
    drop_directory: PathBuf,
    target: String,
    path: PathBuf,
    last_content: Option<String>,
    last_readiness_reason: Option<String>,
}

/// Result of publishing one now-playing update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DropFilePublication {
    /// Final drop-file path.
    pub path: PathBuf,
    /// Number of serialized payment routes.
    pub value_routes_count: usize,
    /// Readiness note for task 012 when embedded route data is missing.
    pub readiness_reason: Option<String>,
    /// Whether this call wrote a new file.
    pub wrote_file: bool,
}

#[derive(Serialize)]
struct DropFile<'a> {
    schema: &'static str,
    target: &'a str,
    artist: &'a str,
    title: &'a str,
    duration_secs: Option<f64>,
    image: Option<String>,
    feed_guid: Option<String>,
    track_guid: Option<String>,
    value_routes: &'a [PaymentRoute],
    value_routes_source: Option<&'static str>,
}

impl DropFileProducer {
    /// Verify that the producer can write its configured directory (ADR 0066).
    ///
    /// # Errors
    /// Reports directory/probe errors without touching an existing drop file.
    pub(crate) fn prepare_directory(&self) -> Result<()> {
        fs::create_dir_all(&self.drop_directory).context("prepare producer directory")?;
        let probe = self.drop_directory.join(format!(
            ".v4vmm-producer-probe-{}-{}",
            std::process::id(),
            PREPARATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)
            .context("create producer write probe")?;
        let written = file
            .write_all(b"v4vmm producer probe")
            .context("write producer probe");
        drop(file);
        fs::remove_file(&probe)
            .with_context(|| format!("remove producer probe {}", probe.display()))?;
        written
    }
    /// Create an active producer for one watched directory and target.
    ///
    /// # Errors
    ///
    /// Returns an error when the directory path is empty or the target cannot
    /// be used as a visible file name.
    pub fn new(drop_directory: impl Into<PathBuf>, target: impl Into<String>) -> Result<Self> {
        let drop_directory = drop_directory.into();
        if drop_directory.as_os_str().is_empty() {
            return Err(anyhow!("broadcast drop directory is empty"));
        }
        let target = validate_target_name(&target.into())?;
        let path = drop_directory.join(format!("{target}{DROP_FILE_SUFFIX}"));
        Ok(Self {
            drop_directory,
            target,
            path,
            last_content: None,
            last_readiness_reason: None,
        })
    }

    /// Validate and normalize a configured target name.
    ///
    /// # Errors
    ///
    /// Returns an error when the target is empty, hidden, temporary-looking,
    /// or contains a path separator.
    pub fn validate_target_name(target: &str) -> Result<String> {
        validate_target_name(target)
    }

    /// Return the final drop-file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the publisher target name carried by the drop file.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Return the latest readiness reason for a playing track.
    #[must_use]
    pub fn last_readiness_reason(&self) -> Option<&str> {
        self.last_readiness_reason.as_deref()
    }

    /// Return the shutdown warning required for this source.
    #[must_use]
    pub const fn shutdown_warning(&self) -> &'static str {
        SHUTDOWN_WARNING
    }

    /// Publish one playing update to the drop file.
    ///
    /// # Errors
    ///
    /// Returns an error when audio tags or embedded routes cannot be parsed, or
    /// when the drop file cannot be written atomically.
    pub fn publish(
        &mut self,
        update: &NowPlayingUpdate,
        audio_path: &Path,
    ) -> Result<DropFilePublication> {
        let tags = read_audio_tags(audio_path)?;
        let value_routes_json =
            musicindex_value(&tags, VALUE_ROUTES_KEY).filter(|value| !value.trim().is_empty());
        let value_routes = value_routes_json
            .map(serde_json::from_str::<Vec<PaymentRoute>>)
            .transpose()
            .with_context(|| {
                format!(
                    "parse embedded MusicIndex Value Routes from {}",
                    audio_path.display()
                )
            })?
            .unwrap_or_default();
        let value_routes_source = value_routes_json.map(|_| EMBEDDED_ID3_ROUTES_SOURCE);
        let readiness_reason = value_routes_source
            .is_none()
            .then(|| MISSING_VALUE_ROUTES_TAG_REASON.to_owned());
        let route_count = value_routes.len();

        let payload = DropFile {
            schema: DROP_FILE_SCHEMA,
            target: &self.target,
            artist: &update.artist,
            title: &update.title,
            duration_secs: update
                .duration_ms
                .map(|duration_ms| Duration::from_millis(duration_ms).as_secs_f64()),
            image: musicindex_value(&tags, IMAGE_KEY)
                .map(ToOwned::to_owned)
                .or_else(|| update.image.clone()),
            feed_guid: musicindex_value(&tags, FEED_GUID_KEY)
                .map(ToOwned::to_owned)
                .or_else(|| Some(update.feed_guid.clone())),
            track_guid: musicindex_value(&tags, TRACK_GUID_KEY)
                .map(ToOwned::to_owned)
                .or_else(|| Some(update.item_guid.clone())),
            value_routes: &value_routes,
            value_routes_source,
        };
        let content = serde_json::to_string_pretty(&payload)
            .context("serialize musicindex.nowplaying/1 drop file")?;
        let wrote_file = if self.last_content.as_deref() == Some(&content) && self.path.exists() {
            false
        } else {
            self.write_atomic(&content)?;
            self.last_content = Some(content);
            true
        };
        self.last_readiness_reason.clone_from(&readiness_reason);

        Ok(DropFilePublication {
            path: self.path.clone(),
            value_routes_count: route_count,
            readiness_reason,
            wrote_file,
        })
    }

    /// Remove the final drop file.
    ///
    /// # Errors
    ///
    /// Returns an error when removing the file fails for a reason other than
    /// absence.
    pub fn clear(&mut self) -> Result<bool> {
        let removed = remove_file_if_exists(&self.path)?;
        self.last_content = None;
        self.last_readiness_reason = None;
        Ok(removed)
    }

    fn write_atomic(&self, content: &str) -> Result<()> {
        fs::create_dir_all(&self.drop_directory).with_context(|| {
            format!(
                "create broadcast drop directory {}",
                self.drop_directory.display()
            )
        })?;
        let filename = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                anyhow!(
                    "broadcast drop path has no UTF-8 file name: {}",
                    self.path.display()
                )
            })?;
        let temp_path = self
            .drop_directory
            .join(format!(".{filename}.{}.tmp", std::process::id()));
        fs::write(&temp_path, content).with_context(|| {
            format!(
                "write temporary broadcast drop file {}",
                temp_path.display()
            )
        })?;
        fs::rename(&temp_path, &self.path).with_context(|| {
            format!(
                "rename temporary broadcast drop file {} to {}",
                temp_path.display(),
                self.path.display()
            )
        })?;
        Ok(())
    }
}

fn validate_target_name(target: &str) -> Result<String> {
    let target = target.trim();
    if target.is_empty() {
        return Err(anyhow!("broadcast drop-file target is empty"));
    }
    if target.starts_with('.') {
        return Err(anyhow!(
            "broadcast drop-file target {target:?} would create a hidden file"
        ));
    }
    if Path::new(target)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("tmp"))
    {
        return Err(anyhow!(
            "broadcast drop-file target {target:?} would create a temporary-looking file"
        ));
    }
    if target.contains('/') || target.contains('\\') {
        return Err(anyhow!(
            "broadcast drop-file target {target:?} must not contain a path separator"
        ));
    }
    Ok(target.to_owned())
}

fn musicindex_value<'a>(tags: &'a AudioTags, key: &str) -> Option<&'a str> {
    let from_fields = tags.fields.iter().find_map(|field| {
        (canonical_musicindex_key(&field.frame_id) == Some(key))
            .then_some(field.value.trim())
            .filter(|value| !value.is_empty())
    });
    if from_fields.is_some() {
        return from_fields;
    }

    let custom_key = format!("MusicIndex {key}");
    tags.custom
        .get(&custom_key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
}

fn canonical_musicindex_key(key: &str) -> Option<&'static str> {
    let normalized = normalize_key(frame_match_key(key));
    MUSICINDEX_VOCABULARY
        .iter()
        .find(|(_, frame_label)| normalize_key(frame_match_key(frame_label)) == normalized)
        .map(|(canonical, _)| *canonical)
}

fn frame_match_key(frame_label: &str) -> &str {
    frame_label
        .rsplit_once(':')
        .map_or(frame_label, |(_, key)| key)
}

fn normalize_key(key: &str) -> String {
    key.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}

fn remove_file_if_exists(path: &Path) -> Result<bool> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("remove {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use anyhow::Result;
    use chrono::Utc;
    use serde_json::Value;

    use super::*;
    use crate::audio_tags::{write_id3v24_edits, Id3v24Edit};

    fn update_with_value_block(value: Value) -> NowPlayingUpdate {
        NowPlayingUpdate {
            session_id: "default".to_owned(),
            sequence: 7,
            started_at: Utc::now(),
            position_ms: 12_000,
            duration_ms: Some(187_326),
            feed_guid: "rss-feed-guid".to_owned(),
            item_guid: "rss-track-guid".to_owned(),
            local_track_id: 42,
            title: "Track Title".to_owned(),
            artist: "Artist Name".to_owned(),
            album: Some("Album Title".to_owned()),
            image: Some("https://example.test/rss.png".to_owned()),
            value_block: value,
            raw_extra_json: serde_json::json!({"source": "rss"}),
        }
    }

    fn tagged_audio_file(temp: &tempfile::TempDir, value_routes: Option<&str>) -> Result<PathBuf> {
        let path = temp.path().join("track.mp3");
        fs::write(&path, b"not really an mp3")?;
        let mut edits = vec![
            Id3v24Edit {
                frame_label: "TXXX:MusicIndex Feed Guid".to_owned(),
                value: "tag-feed-guid".to_owned(),
            },
            Id3v24Edit {
                frame_label: "TXXX:MusicIndex Track Guid".to_owned(),
                value: "tag-track-guid".to_owned(),
            },
        ];
        if let Some(routes) = value_routes {
            edits.push(Id3v24Edit {
                frame_label: "TXXX:MusicIndex Value Routes".to_owned(),
                value: routes.to_owned(),
            });
        }
        write_id3v24_edits(&path, &edits)?;
        Ok(path)
    }

    fn read_drop_file(path: &Path) -> Result<Value> {
        Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
    }

    #[test]
    fn publishes_contract_field_list_and_embedded_routes() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let routes = r#"[{"recipient_name":"Tagged Artist","route_type":"node","address":"03ab","split":90.0,"fee":false,"custom_key":null,"custom_value":null}]"#;
        let audio_path = tagged_audio_file(&temp, Some(routes))?;
        let update = update_with_value_block(serde_json::json!({"payment": "rss"}));
        let mut producer = DropFileProducer::new(temp.path(), "default")?;

        let publication = producer.publish(&update, &audio_path)?;
        let value = read_drop_file(producer.path())?;
        let object = value.as_object().expect("drop file object");
        let mut keys = object.keys().map(String::as_str).collect::<Vec<_>>();
        keys.sort_unstable();

        assert!(publication.wrote_file);
        assert_eq!(
            publication.path,
            temp.path().join("default.nowplaying.json")
        );
        assert_eq!(
            keys,
            vec![
                "artist",
                "duration_secs",
                "feed_guid",
                "image",
                "schema",
                "target",
                "title",
                "track_guid",
                "value_routes",
                "value_routes_source",
            ]
        );
        assert_eq!(value["schema"], DROP_FILE_SCHEMA);
        assert_eq!(value["target"], "default");
        assert_eq!(value["artist"], "Artist Name");
        assert_eq!(value["title"], "Track Title");
        assert_eq!(value["duration_secs"], 187.326);
        assert_eq!(value["feed_guid"], "tag-feed-guid");
        assert_eq!(value["track_guid"], "tag-track-guid");
        assert_eq!(value["image"], "https://example.test/rss.png");
        assert_eq!(value["value_routes"][0]["recipient_name"], "Tagged Artist");
        assert_eq!(value["value_routes"][0]["split"], 90.0);
        assert_eq!(value["value_routes_source"], EMBEDDED_ID3_ROUTES_SOURCE);
        assert_eq!(publication.value_routes_count, 1);
        assert_eq!(publication.readiness_reason, None);
        Ok(())
    }

    #[test]
    fn payment_routes_come_from_file_tag_not_playback_value_block() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let routes = r#"[{"recipient_name":"Embedded Route","route_type":"node","split":77.0}]"#;
        let audio_path = tagged_audio_file(&temp, Some(routes))?;
        let update = update_with_value_block(serde_json::json!({
            "value": {
                "destinations": [
                    {"recipient_name": "Wrong Source", "split": 100.0}
                ]
            }
        }));
        let mut producer = DropFileProducer::new(temp.path(), "default")?;

        producer.publish(&update, &audio_path)?;
        let value = read_drop_file(producer.path())?;

        assert_eq!(
            value["value_routes"],
            serde_json::json!([
                {
                    "recipient_name": "Embedded Route",
                    "route_type": "node",
                    "split": 77.0,
                    "fee": null,
                    "address": null,
                    "custom_key": null,
                    "custom_value": null
                }
            ])
        );
        Ok(())
    }

    #[test]
    fn missing_route_tag_writes_empty_routes_and_records_reason() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let audio_path = tagged_audio_file(&temp, None)?;
        let update = update_with_value_block(serde_json::json!({
            "value": {
                "destinations": [
                    {"recipient_name": "RSS Route", "split": 100.0}
                ]
            }
        }));
        let mut producer = DropFileProducer::new(temp.path(), "default")?;

        let publication = producer.publish(&update, &audio_path)?;
        let value = read_drop_file(producer.path())?;

        assert_eq!(value["value_routes"], serde_json::json!([]));
        assert_eq!(value["value_routes_source"], Value::Null);
        assert_eq!(
            publication.readiness_reason.as_deref(),
            Some(MISSING_VALUE_ROUTES_TAG_REASON)
        );
        assert_eq!(
            producer.last_readiness_reason(),
            Some(MISSING_VALUE_ROUTES_TAG_REASON)
        );
        Ok(())
    }

    #[test]
    fn publish_skips_redundant_rewrite_and_clear_removes_file() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let routes = r#"[{"recipient_name":"Embedded Route","route_type":"node","split":77.0}]"#;
        let audio_path = tagged_audio_file(&temp, Some(routes))?;
        let update = update_with_value_block(serde_json::json!({}));
        let mut producer = DropFileProducer::new(temp.path(), "stream-a")?;

        let first = producer.publish(&update, &audio_path)?;
        let second = producer.publish(&update, &audio_path)?;
        let removed = producer.clear()?;

        assert!(first.wrote_file);
        assert!(!second.wrote_file);
        assert!(removed);
        assert!(!producer.path().exists());
        assert_eq!(producer.last_readiness_reason(), None);
        Ok(())
    }

    #[test]
    fn validates_visible_target_file_names() -> Result<()> {
        let temp = tempfile::tempdir()?;

        assert_eq!(
            DropFileProducer::new(temp.path(), " stream-a ")?.path(),
            temp.path().join("stream-a.nowplaying.json")
        );
        assert!(DropFileProducer::new(temp.path(), ".hidden").is_err());
        assert!(DropFileProducer::new(temp.path(), "nested/default").is_err());
        assert!(DropFileProducer::new(temp.path(), "default.tmp").is_err());
        Ok(())
    }
}
