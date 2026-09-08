//! Broadcast readiness read model for ADR 0059.
//!
//! The query is read-only: it checks local library rows and embedded audio tag
//! state so Show and CLI surfaces can report payment-route readiness without
//! mutating files.

#![warn(clippy::pedantic)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;

use crate::api::PaymentRoute;
use crate::application::application_query_service::ApplicationQueryService;
use crate::application::errors::command::CommandError;
use crate::audio_tags::{read_audio_tags, AudioTags};
use crate::db::TrackRow;
use crate::library_service;

const VALUE_ROUTES_KEY: &str = "Value Routes";
const VALUE_ROUTES_FRAME: &str = "TXXX:MusicIndex Value Routes";
const MUSICINDEX_VALUE_ROUTES_CUSTOM_KEY: &str = "MusicIndex Value Routes";

/// Readiness report for local library tracks before a show.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct BroadcastReadinessReport {
    /// Count summary by readiness state.
    pub(crate) summary: BroadcastReadinessSummary,
    /// Every scanned library track with its readiness state.
    pub(crate) tracks: Vec<BroadcastReadinessTrack>,
}

impl BroadcastReadinessReport {
    /// Returns tracks that are not ready for broadcast payment routing.
    #[must_use]
    pub(crate) fn problem_tracks(&self) -> Vec<&BroadcastReadinessTrack> {
        self.tracks
            .iter()
            .filter(|track| !matches!(track.state, BroadcastReadinessState::Ready))
            .collect()
    }

    /// Returns the number of tracks that need curator attention.
    #[must_use]
    pub(crate) const fn problem_count(&self) -> usize {
        self.summary.no_route_tag + self.summary.file_missing
    }
}

/// Count summary by readiness state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct BroadcastReadinessSummary {
    /// Tracks with a non-empty parsed payment-route array.
    pub(crate) ready: usize,
    /// Tracks whose local file lacks a usable value-routes tag.
    pub(crate) no_route_tag: usize,
    /// Tracks whose recorded local path cannot be read.
    pub(crate) file_missing: usize,
}

impl BroadcastReadinessSummary {
    fn record(&mut self, state: BroadcastReadinessState) {
        match state {
            BroadcastReadinessState::Ready => self.ready += 1,
            BroadcastReadinessState::NoRouteTag => self.no_route_tag += 1,
            BroadcastReadinessState::FileMissing => self.file_missing += 1,
        }
    }
}

/// Per-track broadcast payment-route readiness state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BroadcastReadinessState {
    /// Local file contains a usable embedded payment-route array.
    Ready,
    /// Local file has no usable embedded payment-route tag.
    NoRouteTag,
    /// Local path is missing or does not point to a readable file.
    FileMissing,
}

impl BroadcastReadinessState {
    /// Returns the compact row label for this state.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::NoRouteTag => "Missing routes",
            Self::FileMissing => "Missing file",
        }
    }
}

/// One local library track classified for broadcast readiness.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct BroadcastReadinessTrack {
    /// Local track database id.
    pub(crate) track_id: i64,
    /// Listener-facing track title fallback.
    pub(crate) title: String,
    /// Artist display fallback.
    pub(crate) artist: Option<String>,
    /// Album display fallback.
    pub(crate) album: Option<String>,
    /// Recorded local path, if the database has one.
    pub(crate) path: Option<String>,
    /// Readiness classification.
    pub(crate) state: BroadcastReadinessState,
    /// Curator-facing reason for this classification.
    pub(crate) reason: String,
}

impl ApplicationQueryService {
    /// Builds the broadcast readiness report from local library rows and files.
    ///
    /// # Errors
    ///
    /// Returns an error when local library rows cannot be read. Unreadable tag
    /// data is classified as not ready.
    #[expect(
        clippy::unused_self,
        reason = "ApplicationQueryService uses an instance receiver across query families"
    )]
    pub(crate) fn broadcast_readiness_report(
        &self,
        conn: &Connection,
    ) -> Result<BroadcastReadinessReport, CommandError> {
        broadcast_readiness_report(conn).map_err(|error| CommandError::Query(format!("{error:#}")))
    }
}

/// Builds the broadcast readiness report from local library rows and files.
///
/// # Errors
///
/// Returns an error when local library rows cannot be read. Unreadable tag data
/// is classified as not ready.
pub(crate) fn broadcast_readiness_report(conn: &Connection) -> Result<BroadcastReadinessReport> {
    let tracks = library_service::library_tracks(conn).context("load local library tracks")?;
    Ok(readiness_report_from_tracks(&tracks))
}

fn readiness_report_from_tracks(tracks: &[TrackRow]) -> BroadcastReadinessReport {
    let mut report = BroadcastReadinessReport::default();

    for track in tracks {
        let item = readiness_track(track);
        report.summary.record(item.state);
        report.tracks.push(item);
    }

    report
}

fn readiness_track(track: &TrackRow) -> BroadcastReadinessTrack {
    let title = track
        .track_title
        .clone()
        .or_else(|| track.feed_title.clone())
        .unwrap_or_else(|| "Untitled".to_string());
    let Some(path) = track.local_path.as_ref().map(PathBuf::from) else {
        return BroadcastReadinessTrack {
            track_id: track.id,
            title,
            artist: track.artist_name.clone(),
            album: track.album_title.clone(),
            path: None,
            state: BroadcastReadinessState::FileMissing,
            reason: "No local file path is recorded.".to_owned(),
        };
    };

    if !path.is_file() {
        return BroadcastReadinessTrack {
            track_id: track.id,
            title,
            artist: track.artist_name.clone(),
            album: track.album_title.clone(),
            path: Some(path.display().to_string()),
            state: BroadcastReadinessState::FileMissing,
            reason: "Recorded local file is missing.".to_owned(),
        };
    }

    match read_audio_tags(&path).context("read embedded audio tags") {
        Ok(tags) => readiness_track_from_tags(track, title, path.as_path(), &tags),
        Err(error) => BroadcastReadinessTrack {
            track_id: track.id,
            title,
            artist: track.artist_name.clone(),
            album: track.album_title.clone(),
            path: Some(path.display().to_string()),
            state: BroadcastReadinessState::NoRouteTag,
            reason: format!("Value routes tag could not be read: {error:#}"),
        },
    }
}

fn readiness_track_from_tags(
    track: &TrackRow,
    title: String,
    path: &Path,
    tags: &AudioTags,
) -> BroadcastReadinessTrack {
    let (state, reason) = match musicindex_value_routes(tags) {
        Some(value) if value_routes_are_ready(value) => (
            BroadcastReadinessState::Ready,
            "Embedded value routes are present.".to_owned(),
        ),
        Some(_) => (
            BroadcastReadinessState::NoRouteTag,
            "Embedded value routes are empty or invalid.".to_owned(),
        ),
        None => (
            BroadcastReadinessState::NoRouteTag,
            "Embedded MusicIndex Value Routes tag is missing.".to_owned(),
        ),
    };

    BroadcastReadinessTrack {
        track_id: track.id,
        title,
        artist: track.artist_name.clone(),
        album: track.album_title.clone(),
        path: Some(path.display().to_string()),
        state,
        reason,
    }
}

fn musicindex_value_routes(tags: &AudioTags) -> Option<&str> {
    let from_fields = tags.fields.iter().find_map(|field| {
        (canonical_musicindex_key(&field.frame_id) == Some(VALUE_ROUTES_KEY))
            .then_some(field.value.trim())
            .filter(|value| !value.is_empty())
    });
    if from_fields.is_some() {
        return from_fields;
    }

    tags.custom
        .get(MUSICINDEX_VALUE_ROUTES_CUSTOM_KEY)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
}

fn canonical_musicindex_key(key: &str) -> Option<&'static str> {
    let normalized = normalize_key(frame_match_key(key));
    (normalize_key(frame_match_key(VALUE_ROUTES_FRAME)) == normalized).then_some(VALUE_ROUTES_KEY)
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

fn value_routes_are_ready(value: &str) -> bool {
    serde_json::from_str::<Vec<PaymentRoute>>(value).is_ok_and(|routes| !routes.is_empty())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::Connection;

    use super::*;
    use crate::audio_tags::{write_id3v24_edits, Id3v24Edit};
    use crate::db;

    fn setup_test_db() -> anyhow::Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    fn create_feed(conn: &Connection) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["https://example.test/feed.xml", "feed-guid", "Feed Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_track(
        conn: &Connection,
        feed_id: i64,
        title: &str,
        path: &Path,
    ) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title, artist_name, album_title)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                feed_id,
                format!("item-guid-{title}"),
                title,
                "Track Artist",
                "Track Album"
            ],
        )?;
        let track_id = conn.last_insert_rowid();
        library_service::mark_track_downloaded(conn, track_id, path, None)?;
        Ok(track_id)
    }

    fn tagged_audio_file(
        temp: &tempfile::TempDir,
        name: &str,
        routes: &str,
    ) -> anyhow::Result<PathBuf> {
        let path = temp.path().join(name);
        fs::write(&path, b"not really an mp3")?;
        write_id3v24_edits(
            &path,
            &[Id3v24Edit {
                frame_label: VALUE_ROUTES_FRAME.to_owned(),
                value: routes.to_owned(),
            }],
        )?;
        Ok(path)
    }

    fn untagged_audio_file(temp: &tempfile::TempDir, name: &str) -> anyhow::Result<PathBuf> {
        let path = temp.path().join(name);
        fs::write(&path, b"not really an mp3")?;
        Ok(path)
    }

    #[test]
    fn broadcast_readiness_report_marks_non_empty_value_routes_ready() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let feed_id = create_feed(&conn)?;
        let path = tagged_audio_file(
            &temp,
            "ready.mp3",
            r#"[{"recipient_name":"Artist","route_type":"node","split":100.0}]"#,
        )?;
        let track_id = create_track(&conn, feed_id, "Ready Track", &path)?;

        let report = ApplicationQueryService::new().broadcast_readiness_report(&conn)?;

        assert_eq!(report.summary.ready, 1);
        assert_eq!(report.summary.no_route_tag, 0);
        assert_eq!(report.summary.file_missing, 0);
        assert_eq!(report.tracks[0].track_id, track_id);
        assert_eq!(report.tracks[0].state, BroadcastReadinessState::Ready);
        Ok(())
    }

    #[test]
    fn broadcast_readiness_report_marks_missing_value_routes_tag() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let feed_id = create_feed(&conn)?;
        let path = untagged_audio_file(&temp, "missing-tag.mp3")?;
        create_track(&conn, feed_id, "Missing Tag", &path)?;

        let report = ApplicationQueryService::new().broadcast_readiness_report(&conn)?;

        assert_eq!(report.summary.ready, 0);
        assert_eq!(report.summary.no_route_tag, 1);
        assert_eq!(report.summary.file_missing, 0);
        assert_eq!(report.tracks[0].state, BroadcastReadinessState::NoRouteTag);
        Ok(())
    }

    #[test]
    fn broadcast_readiness_report_marks_empty_route_array_not_ready() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let feed_id = create_feed(&conn)?;
        let path = tagged_audio_file(&temp, "empty-routes.mp3", "[]")?;
        create_track(&conn, feed_id, "Empty Routes", &path)?;

        let report = ApplicationQueryService::new().broadcast_readiness_report(&conn)?;

        assert_eq!(report.summary.ready, 0);
        assert_eq!(report.summary.no_route_tag, 1);
        assert_eq!(report.summary.file_missing, 0);
        assert_eq!(report.tracks[0].state, BroadcastReadinessState::NoRouteTag);
        Ok(())
    }

    #[test]
    fn broadcast_readiness_report_marks_recorded_missing_file() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let feed_id = create_feed(&conn)?;
        let path = temp.path().join("missing.mp3");
        create_track(&conn, feed_id, "Missing File", &path)?;

        let report = ApplicationQueryService::new().broadcast_readiness_report(&conn)?;

        assert_eq!(report.summary.ready, 0);
        assert_eq!(report.summary.no_route_tag, 0);
        assert_eq!(report.summary.file_missing, 1);
        assert_eq!(report.tracks[0].state, BroadcastReadinessState::FileMissing);
        Ok(())
    }

    #[test]
    fn broadcast_readiness_report_allows_empty_library() -> anyhow::Result<()> {
        let conn = setup_test_db()?;

        let report = ApplicationQueryService::new().broadcast_readiness_report(&conn)?;

        assert_eq!(report.summary, BroadcastReadinessSummary::default());
        assert!(report.tracks.is_empty());
        Ok(())
    }
}
