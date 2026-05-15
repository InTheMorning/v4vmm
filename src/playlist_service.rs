//! Non-UI playlist service boundary.

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::{db, track_identity};

#[derive(Clone, Debug)]
pub struct PlaylistTrackSelection {
    pub playlist_id: i64,
    pub position: i64,
    pub track_id: i64,
    pub identity: track_identity::TrackIdentity,
}

pub fn list(conn: &Connection) -> Result<Vec<db::Playlist>> {
    db::playlists_list(conn)
}

pub fn tracks(conn: &Connection, playlist_id: i64) -> Result<Vec<db::TrackRow>> {
    db::playlist_tracks(conn, playlist_id)
}

pub fn create(conn: &Connection, name: &str) -> Result<i64> {
    db::playlist_create(conn, name)
}

pub fn rename(conn: &Connection, playlist_id: i64, new_name: &str) -> Result<()> {
    db::playlist_rename(conn, playlist_id, new_name)
}

pub fn set_description(conn: &Connection, playlist_id: i64, desc: Option<&str>) -> Result<()> {
    db::playlist_set_description(conn, playlist_id, desc)
}

pub fn delete(conn: &Connection, playlist_id: i64) -> Result<()> {
    db::playlist_delete(conn, playlist_id)
}

pub fn append_track(conn: &Connection, playlist_id: i64, track_id: i64) -> Result<()> {
    db::playlist_append(conn, playlist_id, track_id)
}

pub fn remove_track_at(conn: &mut Connection, playlist_id: i64, position: i64) -> Result<()> {
    ensure_non_negative_position(position)?;
    db::playlist_remove_at(conn, playlist_id, position)
}

pub fn reorder(conn: &mut Connection, playlist_id: i64, from: i64, to: i64) -> Result<()> {
    ensure_non_negative_position(from)?;
    ensure_non_negative_position(to)?;
    db::playlist_reorder(conn, playlist_id, from, to)
}

pub fn track_at(conn: &Connection, playlist_id: i64, position: i64) -> Result<Option<(i64, i64)>> {
    ensure_non_negative_position(position)?;
    db::playlist_track_at(conn, playlist_id, position)
}

pub fn select_track_at(
    conn: &Connection,
    playlist_id: i64,
    position: i64,
) -> Result<PlaylistTrackSelection> {
    let (track_id, position) = track_at(conn, playlist_id, position)?
        .with_context(|| format!("playlist {playlist_id} has no track at position {position}"))?;
    let track = db::track_row_by_id(conn, track_id)?
        .with_context(|| format!("playlist track {track_id} no longer exists"))?;
    ensure_track_can_play(&track)?;
    let identity = track_identity::local_track_identity(conn, track_id)?;
    Ok(PlaylistTrackSelection {
        playlist_id,
        position,
        track_id,
        identity,
    })
}

pub fn select_playable_track_after(
    conn: &Connection,
    playlist_id: i64,
    position: i64,
    delta: i64,
) -> Result<PlaylistTrackSelection> {
    anyhow::ensure!(delta != 0, "playlist skip delta cannot be zero");
    let mut next_position = position
        .checked_add(delta)
        .context("playlist position overflow")?;

    loop {
        if next_position < 0 {
            anyhow::bail!("already at the beginning of playlist {playlist_id}");
        }
        let Some((track_id, actual_position)) = track_at(conn, playlist_id, next_position)? else {
            let boundary = if delta > 0 { "end" } else { "beginning" };
            anyhow::bail!("already at the {boundary} of playlist {playlist_id}");
        };
        let track = db::track_row_by_id(conn, track_id)?
            .with_context(|| format!("playlist track {track_id} no longer exists"))?;
        if track_can_play(&track) {
            let identity = track_identity::local_track_identity(conn, track_id)?;
            return Ok(PlaylistTrackSelection {
                playlist_id,
                position: actual_position,
                track_id,
                identity,
            });
        }
        next_position = actual_position
            .checked_add(delta)
            .context("playlist position overflow")?;
    }
}

fn ensure_non_negative_position(position: i64) -> Result<()> {
    anyhow::ensure!(position >= 0, "playlist position cannot be negative");
    Ok(())
}

fn ensure_track_can_play(track: &db::TrackRow) -> Result<()> {
    anyhow::ensure!(
        track.is_in_library,
        "playlist track {} is no longer in the library",
        track.id
    );
    anyhow::ensure!(
        track.local_path.is_some(),
        "playlist track {} has no local file available for playback",
        track.id
    );
    Ok(())
}

fn track_can_play(track: &db::TrackRow) -> bool {
    track.is_in_library && track.local_path.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    fn create_feed(conn: &Connection) -> Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["https://example.test/feed.xml", "feed-guid", "Feed Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_track(conn: &Connection, feed_id: i64, item_guid: &str, path: &str) -> Result<i64> {
        conn.execute(
            "INSERT INTO tracks (
                 feed_id, item_guid, track_title, artist_name, is_in_library
             )
             VALUES (?1, ?2, ?3, ?4, 1)",
            rusqlite::params![feed_id, item_guid, format!("Track {item_guid}"), "Artist"],
        )?;
        let track_id = conn.last_insert_rowid();
        db::mark_track_downloaded(conn, track_id, std::path::Path::new(path), None)?;
        Ok(track_id)
    }

    #[test]
    fn select_track_at_returns_identity_for_position() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let first_track_id = create_track(&conn, feed_id, "first-guid", "/tmp/first.mp3")?;
        let second_track_id = create_track(&conn, feed_id, "second-guid", "/tmp/second.mp3")?;
        let playlist_id = create(&conn, "Service")?;
        append_track(&conn, playlist_id, first_track_id)?;
        append_track(&conn, playlist_id, second_track_id)?;

        let selection = select_track_at(&conn, playlist_id, 1)?;

        assert_eq!(selection.playlist_id, playlist_id);
        assert_eq!(selection.position, 1);
        assert_eq!(selection.track_id, second_track_id);
        assert_eq!(selection.identity.item_guid, "second-guid");

        Ok(())
    }

    #[test]
    fn select_track_at_rejects_tracks_removed_from_library() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id, "removed-guid", "/tmp/removed.mp3")?;
        let playlist_id = create(&conn, "Service")?;
        append_track(&conn, playlist_id, track_id)?;
        db::set_track_in_library(&conn, track_id, false)?;

        let result = select_track_at(&conn, playlist_id, 0);

        assert!(
            result
                .expect_err("out-of-library playlist tracks should not play")
                .to_string()
                .contains("no longer in the library"),
            "error should explain unavailable library membership"
        );

        Ok(())
    }

    #[test]
    fn select_playable_track_after_skips_removed_library_rows() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let first_track_id = create_track(&conn, feed_id, "first-guid", "/tmp/first.mp3")?;
        let removed_track_id = create_track(&conn, feed_id, "removed-guid", "/tmp/removed.mp3")?;
        let third_track_id = create_track(&conn, feed_id, "third-guid", "/tmp/third.mp3")?;
        let playlist_id = create(&conn, "Service")?;
        append_track(&conn, playlist_id, first_track_id)?;
        append_track(&conn, playlist_id, removed_track_id)?;
        append_track(&conn, playlist_id, third_track_id)?;
        db::set_track_in_library(&conn, removed_track_id, false)?;

        let forward = select_playable_track_after(&conn, playlist_id, 0, 1)?;
        let backward = select_playable_track_after(&conn, playlist_id, 2, -1)?;

        assert_eq!(forward.position, 2);
        assert_eq!(forward.track_id, third_track_id);
        assert_eq!(backward.position, 0);
        assert_eq!(backward.track_id, first_track_id);

        Ok(())
    }

    #[test]
    fn select_playable_track_after_skips_rows_without_local_files() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let first_track_id = create_track(&conn, feed_id, "first-guid", "/tmp/first.mp3")?;
        conn.execute(
            "INSERT INTO tracks (
                 feed_id, item_guid, track_title, artist_name, is_in_library
             )
             VALUES (?1, ?2, ?3, ?4, 1)",
            rusqlite::params![feed_id, "missing-guid", "Missing Local File", "Artist"],
        )?;
        let missing_file_track_id = conn.last_insert_rowid();
        let third_track_id = create_track(&conn, feed_id, "third-guid", "/tmp/third.mp3")?;
        let playlist_id = create(&conn, "Service")?;
        append_track(&conn, playlist_id, first_track_id)?;
        append_track(&conn, playlist_id, missing_file_track_id)?;
        append_track(&conn, playlist_id, third_track_id)?;

        let forward = select_playable_track_after(&conn, playlist_id, 0, 1)?;
        let backward = select_playable_track_after(&conn, playlist_id, 2, -1)?;

        assert_eq!(forward.position, 2);
        assert_eq!(forward.track_id, third_track_id);
        assert_eq!(backward.position, 0);
        assert_eq!(backward.track_id, first_track_id);

        let result = select_track_at(&conn, playlist_id, 1);
        assert!(
            result
                .expect_err("playlist tracks without local files should not play")
                .to_string()
                .contains("no local file"),
            "error should explain local-file unavailability"
        );

        Ok(())
    }

    #[test]
    fn negative_position_is_rejected_before_db_lookup() -> Result<()> {
        let conn = setup_test_db()?;

        let result = select_track_at(&conn, 1, -1);

        assert!(result.is_err(), "negative playlist position should fail");

        Ok(())
    }

    #[test]
    fn list_and_tracks_delegate_to_playlist_storage() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id, "item-guid", "/tmp/track.mp3")?;
        let playlist_id = create(&conn, "Service")?;
        append_track(&conn, playlist_id, track_id)?;

        let playlists = list(&conn)?;
        let tracks = tracks(&conn, playlist_id)?;

        assert_eq!(playlists.len(), 1);
        assert_eq!(playlists[0].track_count, 1);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].id, track_id);

        Ok(())
    }

    #[test]
    fn create_rename_describe_and_delete_playlist() -> Result<()> {
        let conn = setup_test_db()?;
        let playlist_id = create(&conn, "Service")?;

        rename(&conn, playlist_id, "Renamed")?;
        set_description(&conn, playlist_id, Some("Description"))?;

        let playlists = list(&conn)?;
        assert_eq!(playlists[0].name, "Renamed");
        assert_eq!(playlists[0].description.as_deref(), Some("Description"));

        delete(&conn, playlist_id)?;
        assert!(list(&conn)?.is_empty(), "playlist should be deleted");

        Ok(())
    }
}
