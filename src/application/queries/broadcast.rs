//! Broadcast readiness read model for ADR 0059.
//!
//! The query is read-only: it checks local library rows and embedded audio tag
//! state so Show and CLI surfaces can report payment-route readiness without
//! mutating files.

#![warn(clippy::pedantic)]

use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;

use crate::application::application_query_service::ApplicationQueryService;
use crate::application::errors::command::CommandError;
use crate::audio_tags::{read_audio_tags, AudioTags};
use crate::db::{self, LocalMetadataOwner, LocalMetadataValue, TrackRow};
use crate::metadata::{
    audio_tags_have_ready_value_routes, audio_tags_value_routes, MUSICINDEX_METADATA_SOURCE,
    MUSICINDEX_PAYMENT_ROUTES_ABSENT_FACT_KEY,
};
use crate::{config, library_service};

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
        self.summary.no_route_tag
            + self.summary.no_routes_upstream
            + self.summary.file_missing
            + self.summary.not_downloaded
    }
}

/// Count summary by readiness state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct BroadcastReadinessSummary {
    /// Tracks with a non-empty parsed payment-route array.
    pub(crate) ready: usize,
    /// Tracks whose local file lacks a usable value-routes tag.
    pub(crate) no_route_tag: usize,
    /// Tracks whose upstream `MusicIndex` data has no payment routes.
    pub(crate) no_routes_upstream: usize,
    /// Tracks whose recorded local path cannot be read.
    pub(crate) file_missing: usize,
    /// Tracks in the library with no downloaded file.
    pub(crate) not_downloaded: usize,
}

impl BroadcastReadinessSummary {
    fn record(&mut self, state: BroadcastReadinessState) {
        match state {
            BroadcastReadinessState::Ready => self.ready += 1,
            BroadcastReadinessState::NoRouteTag => self.no_route_tag += 1,
            BroadcastReadinessState::NoRoutesUpstream => self.no_routes_upstream += 1,
            BroadcastReadinessState::FileMissing => self.file_missing += 1,
            BroadcastReadinessState::NotDownloaded => self.not_downloaded += 1,
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
    /// `MusicIndex` has no payment routes for this track or its feed.
    NoRoutesUpstream,
    /// The library row has no downloaded file at all.
    NotDownloaded,
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
            Self::NoRoutesUpstream => "No upstream routes",
            Self::FileMissing => "Missing file",
            Self::NotDownloaded => "Not downloaded",
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
        music_dir: &Path,
    ) -> Result<BroadcastReadinessReport, CommandError> {
        broadcast_readiness_report_for_music_dir(conn, music_dir)
            .map_err(|error| CommandError::Query(format!("{error:#}")))
    }
}

/// Builds the broadcast readiness report from local library rows and files.
///
/// # Errors
///
/// Returns an error when local library rows cannot be read. Unreadable tag data
/// is classified as not ready.
pub(crate) fn broadcast_readiness_report(conn: &Connection) -> Result<BroadcastReadinessReport> {
    let cfg_path = config::config_path()?;
    let music_dir = config::ConfigSnapshot::read_existing(&cfg_path)?.music_dir?;
    broadcast_readiness_report_for_music_dir(conn, &music_dir)
}

/// Builds the broadcast readiness report with a configured music directory.
///
/// # Errors
///
/// Returns an error when local library rows cannot be read. Unreadable tag data
/// is classified as not ready.
pub(crate) fn broadcast_readiness_report_for_music_dir(
    conn: &Connection,
    music_dir: &Path,
) -> Result<BroadcastReadinessReport> {
    let tracks = library_service::library_tracks(conn).context("load local library tracks")?;
    readiness_report_from_tracks(conn, &tracks, music_dir)
}

fn readiness_report_from_tracks(
    conn: &Connection,
    tracks: &[TrackRow],
    music_dir: &Path,
) -> Result<BroadcastReadinessReport> {
    let mut report = BroadcastReadinessReport::default();

    for track in tracks {
        let item = readiness_track(conn, track, music_dir)?;
        report.summary.record(item.state);
        report.tracks.push(item);
    }

    Ok(report)
}

fn readiness_track(
    conn: &Connection,
    track: &TrackRow,
    music_dir: &Path,
) -> Result<BroadcastReadinessTrack> {
    let title = track
        .track_title
        .clone()
        .or_else(|| track.feed_title.clone())
        .unwrap_or_else(|| "Untitled".to_string());
    let Some(path) = track
        .local_path
        .as_ref()
        .map(|path| path.resolve(music_dir))
    else {
        return Ok(BroadcastReadinessTrack {
            track_id: track.id,
            title,
            artist: track.artist_name.clone(),
            album: track.album_title.clone(),
            path: None,
            // `library_tracks` uses a LEFT JOIN, so a library row with no
            // download reaches here. The file is not missing. It was never
            // downloaded, and the operator fixes it with a download.
            state: BroadcastReadinessState::NotDownloaded,
            reason: "Track is in the library and has no downloaded file.".to_owned(),
        });
    };

    if !path.is_file() {
        return Ok(BroadcastReadinessTrack {
            track_id: track.id,
            title,
            artist: track.artist_name.clone(),
            album: track.album_title.clone(),
            path: Some(path.display().to_string()),
            state: BroadcastReadinessState::FileMissing,
            reason: "Recorded local file is missing.".to_owned(),
        });
    }

    let no_routes_upstream = track_has_no_upstream_payment_routes(conn, track)?;
    match read_audio_tags(&path).context("read embedded audio tags") {
        Ok(tags) if audio_tags_have_ready_value_routes(&tags) => {
            Ok(readiness_track_from_tags(track, title, path.as_path()))
        }
        Ok(tags) => Ok(readiness_track_from_missing_tag(
            track,
            title,
            path.as_path(),
            no_routes_upstream,
            &tags,
        )),
        Err(error) => Ok(BroadcastReadinessTrack {
            track_id: track.id,
            title,
            artist: track.artist_name.clone(),
            album: track.album_title.clone(),
            path: Some(path.display().to_string()),
            state: if no_routes_upstream {
                BroadcastReadinessState::NoRoutesUpstream
            } else {
                BroadcastReadinessState::NoRouteTag
            },
            reason: if no_routes_upstream {
                "MusicIndex has no payment routes for this track or feed.".to_owned()
            } else {
                format!("Value routes tag could not be read: {error:#}")
            },
        }),
    }
}

fn readiness_track_from_tags(
    track: &TrackRow,
    title: String,
    path: &Path,
) -> BroadcastReadinessTrack {
    BroadcastReadinessTrack {
        track_id: track.id,
        title,
        artist: track.artist_name.clone(),
        album: track.album_title.clone(),
        path: Some(path.display().to_string()),
        state: BroadcastReadinessState::Ready,
        reason: "Embedded value routes are present.".to_owned(),
    }
}

fn readiness_track_from_missing_tag(
    track: &TrackRow,
    title: String,
    path: &Path,
    no_routes_upstream: bool,
    tags: &AudioTags,
) -> BroadcastReadinessTrack {
    let (state, reason) = if no_routes_upstream {
        (
            BroadcastReadinessState::NoRoutesUpstream,
            "MusicIndex has no payment routes for this track or feed.".to_owned(),
        )
    } else if audio_tags_value_routes(tags).is_some() {
        (
            BroadcastReadinessState::NoRouteTag,
            "Embedded value routes are empty or invalid.".to_owned(),
        )
    } else {
        (
            BroadcastReadinessState::NoRouteTag,
            "Embedded MusicIndex Value Routes tag is missing.".to_owned(),
        )
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

pub(crate) fn track_has_no_upstream_payment_routes(
    conn: &Connection,
    track: &TrackRow,
) -> Result<bool> {
    Ok(
        local_payment_routes_absent(conn, LocalMetadataOwner::Track(track.id))?
            || local_payment_routes_absent(conn, LocalMetadataOwner::Feed(track.feed_id))?,
    )
}

fn local_payment_routes_absent(conn: &Connection, owner: LocalMetadataOwner) -> Result<bool> {
    let fact = db::local_metadata_fact(
        conn,
        owner,
        MUSICINDEX_METADATA_SOURCE,
        MUSICINDEX_PAYMENT_ROUTES_ABSENT_FACT_KEY,
    )?;
    Ok(fact.is_some_and(|fact| matches!(fact.value, LocalMetadataValue::Boolean(true))))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

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
        music_dir: &Path,
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
        let relative_path =
            crate::library_path::LibraryRelativePath::from_absolute(music_dir, path)?;
        library_service::mark_track_downloaded(conn, track_id, &relative_path, None)?;
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
                frame_label: crate::metadata::MUSICINDEX_VALUE_ROUTES_FRAME.to_owned(),
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
        let track_id = create_track(&conn, temp.path(), feed_id, "Ready Track", &path)?;

        let report =
            ApplicationQueryService::new().broadcast_readiness_report(&conn, temp.path())?;

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
        create_track(&conn, temp.path(), feed_id, "Missing Tag", &path)?;

        let report =
            ApplicationQueryService::new().broadcast_readiness_report(&conn, temp.path())?;

        assert_eq!(report.summary.ready, 0);
        assert_eq!(report.summary.no_route_tag, 1);
        assert_eq!(report.summary.no_routes_upstream, 0);
        assert_eq!(report.summary.file_missing, 0);
        assert_eq!(report.tracks[0].state, BroadcastReadinessState::NoRouteTag);
        Ok(())
    }

    #[test]
    fn broadcast_readiness_report_counts_no_routes_upstream_separately() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let feed_id = create_feed(&conn)?;
        let path = untagged_audio_file(&temp, "no-upstream-routes.mp3")?;
        let track_id = create_track(&conn, temp.path(), feed_id, "No Upstream Routes", &path)?;
        db::replace_local_metadata_fact(
            &conn,
            LocalMetadataOwner::Track(track_id),
            MUSICINDEX_METADATA_SOURCE,
            &db::LocalMetadataFactInput {
                fact_key: MUSICINDEX_PAYMENT_ROUTES_ABSENT_FACT_KEY.to_owned(),
                value: LocalMetadataValue::Boolean(true),
                extraction_path: Some("$.payment_routes".to_owned()),
                observed_at: None,
                raw_json: None,
            },
        )?;

        let report =
            ApplicationQueryService::new().broadcast_readiness_report(&conn, temp.path())?;

        assert_eq!(report.summary.ready, 0);
        assert_eq!(report.summary.no_route_tag, 0);
        assert_eq!(report.summary.no_routes_upstream, 1);
        assert_eq!(report.summary.file_missing, 0);
        assert_eq!(report.problem_count(), 1);
        assert_eq!(
            report.tracks[0].state,
            BroadcastReadinessState::NoRoutesUpstream
        );
        Ok(())
    }

    #[test]
    fn broadcast_readiness_report_marks_empty_route_array_not_ready() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let feed_id = create_feed(&conn)?;
        let path = tagged_audio_file(&temp, "empty-routes.mp3", "[]")?;
        create_track(&conn, temp.path(), feed_id, "Empty Routes", &path)?;

        let report =
            ApplicationQueryService::new().broadcast_readiness_report(&conn, temp.path())?;

        assert_eq!(report.summary.ready, 0);
        assert_eq!(report.summary.no_route_tag, 1);
        assert_eq!(report.summary.file_missing, 0);
        assert_eq!(report.tracks[0].state, BroadcastReadinessState::NoRouteTag);
        Ok(())
    }

    /// A library row with no download is not a missing file. An operator saw
    /// "54 missing files" on 2026-09-08 while every file was present, because
    /// `library_tracks` LEFT JOINs `local_files` and both cases shared one state.
    #[test]
    fn broadcast_readiness_report_separates_not_downloaded_from_missing_file() -> anyhow::Result<()>
    {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title, artist_name, album_title, is_in_library)
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            rusqlite::params![
                feed_id,
                "item-guid-never-downloaded",
                "Never Downloaded",
                "Track Artist",
                "Track Album"
            ],
        )?;

        let report = ApplicationQueryService::new()
            .broadcast_readiness_report(&conn, std::path::Path::new("/tmp"))?;

        assert_eq!(report.summary.not_downloaded, 1);
        assert_eq!(
            report.summary.file_missing, 0,
            "a library row with no download is not a missing file"
        );
        assert_eq!(
            report.tracks[0].state,
            BroadcastReadinessState::NotDownloaded
        );
        Ok(())
    }

    #[test]
    fn broadcast_readiness_report_marks_recorded_missing_file() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let feed_id = create_feed(&conn)?;
        let path = temp.path().join("missing.mp3");
        create_track(&conn, temp.path(), feed_id, "Missing File", &path)?;

        let report =
            ApplicationQueryService::new().broadcast_readiness_report(&conn, temp.path())?;

        assert_eq!(report.summary.ready, 0);
        assert_eq!(report.summary.no_route_tag, 0);
        assert_eq!(report.summary.file_missing, 1);
        assert_eq!(report.tracks[0].state, BroadcastReadinessState::FileMissing);
        Ok(())
    }

    #[test]
    fn broadcast_readiness_report_allows_empty_library() -> anyhow::Result<()> {
        let conn = setup_test_db()?;

        let report = ApplicationQueryService::new()
            .broadcast_readiness_report(&conn, std::path::Path::new("/tmp"))?;

        assert_eq!(report.summary, BroadcastReadinessSummary::default());
        assert!(report.tracks.is_empty());
        Ok(())
    }
}
