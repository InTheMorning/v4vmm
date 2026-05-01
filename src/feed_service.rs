use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use reqwest::blocking::Client as ReqwestClient;
use rusqlite::Connection;

use crate::api::{
    Client as MusicIndexClient, Contributor, Feed, SourceEntityId, SourceEntityLink, Track,
};
use crate::audio_tags::{read_audio_tags, write_id3v24_edits, AudioTags, Id3v24Edit};
use crate::db::{self, TrackRow};
use crate::identity_ingest;
use crate::library_service;
use crate::metadata::{MusicBrainzLookupResult, TrackContext};
use crate::metadata_service::{id3_edits_for_track_context, musicbrainz_lookup_metadata};
use crate::musicbrainz::{lookup_recordings, MusicBrainzCandidate, MusicBrainzLookup};

#[derive(Clone, Debug)]
pub struct StaleFeed {
    pub feed_id: i64,
    pub feed_guid: String,
    pub title: Option<String>,
    pub new_updated_at: i64,
}

#[derive(Default, Debug, Clone)]
pub struct FeedApplyOutcome {
    pub tracks_updated: usize,
    pub edits_written: usize,
    pub id3_errors: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct StagedMusicBrainzLookup {
    pub lookup: MusicBrainzLookupResult,
    pub edit_count: usize,
}

pub fn fetch_library_track_context(
    track: &TrackRow,
    musicindex_endpoint: &str,
) -> Result<TrackContext> {
    let (fetched_track, fetched_feed) = fetch_library_track_detail(track, musicindex_endpoint)?;
    Ok(merge_track_context_from_detail(
        track,
        fetched_track,
        fetched_feed,
    ))
}

fn fetch_library_track_detail(
    track: &TrackRow,
    musicindex_endpoint: &str,
) -> Result<(Option<Track>, Option<Feed>)> {
    let client = MusicIndexClient::new_with_base_url(musicindex_endpoint.to_string());
    let include =
        Some("source_links,source_ids,source_release_claims,source_contributors,payment_routes");
    let fetched_track = client.fetch_track(&track.item_guid, include).ok();
    let feed_guid = fetched_track
        .as_ref()
        .and_then(|track| track.feed_guid.as_deref())
        .or(track.feed_guid.as_deref());
    let fetched_feed = feed_guid.and_then(|feed_guid| client.fetch_feed(feed_guid, include).ok());
    if fetched_track.is_none() && fetched_feed.is_none() {
        return Err(anyhow!("MusicIndex metadata unavailable"));
    }
    Ok((fetched_track, fetched_feed))
}

fn merge_track_context_from_detail(
    track_row: &TrackRow,
    fetched_track: Option<Track>,
    fetched_feed: Option<Feed>,
) -> TrackContext {
    let local_track = crate::subscribe_service::track_row_to_api_track(track_row);
    let local_feed = track_row_to_feed(track_row);
    let feed = feed_defaults(
        fetched_feed.unwrap_or_else(|| local_feed.clone()),
        &local_feed,
    );
    let track = crate::api::track_with_feed_defaults(
        track_defaults(
            fetched_track.unwrap_or_else(|| local_track.clone()),
            &local_track,
        ),
        Some(&feed),
    );
    TrackContext {
        track,
        feed: Some(feed),
    }
}

pub fn track_row_to_feed(track: &TrackRow) -> Feed {
    Feed {
        feed_guid: track.feed_guid.clone(),
        title: track.feed_title.clone(),
        image_url: track.album_image_href.clone(),
        ..Feed::default()
    }
}

fn track_defaults(mut track: Track, defaults: &Track) -> Track {
    if track.track_guid.is_none() {
        track.track_guid = defaults.track_guid.clone();
    }
    if track.feed_guid.is_none() {
        track.feed_guid = defaults.feed_guid.clone();
    }
    if track.feed_title.is_none() {
        track.feed_title = defaults.feed_title.clone();
    }
    if track.title.is_none() {
        track.title = defaults.title.clone();
    }
    if track.duration_secs.is_none() {
        track.duration_secs = defaults.duration_secs;
    }
    if track.track_number.is_none() {
        track.track_number = defaults.track_number;
    }
    if track.enclosure_url.is_none() {
        track.enclosure_url = defaults.enclosure_url.clone();
    }
    if track.image_url.is_none() {
        track.image_url = defaults.image_url.clone();
    }
    if track.track_artist.is_none() {
        track.track_artist = defaults.track_artist.clone();
    }
    if track.release_artist.is_none() {
        track.release_artist = defaults.release_artist.clone();
    }
    if track.description.is_none() {
        track.description = defaults.description.clone();
    }
    if track.publisher_text.is_none() {
        track.publisher_text = defaults.publisher_text.clone();
    }
    if track.source_contributors.is_none() {
        track.source_contributors = defaults.source_contributors.clone();
    }
    if track.source_links.is_none() {
        track.source_links = defaults.source_links.clone();
    }
    if track.source_ids.is_none() {
        track.source_ids = defaults.source_ids.clone();
    }
    if track.source_release_claims.is_none() {
        track.source_release_claims = defaults.source_release_claims.clone();
    }
    if track.payment_routes.is_none() {
        track.payment_routes = defaults.payment_routes.clone();
    }
    track
}

fn feed_defaults(mut feed: Feed, defaults: &Feed) -> Feed {
    if feed.feed_guid.is_none() {
        feed.feed_guid = defaults.feed_guid.clone();
    }
    if feed.title.is_none() {
        feed.title = defaults.title.clone();
    }
    if feed.name.is_none() {
        feed.name = defaults.name.clone();
    }
    if feed.feed_url.is_none() {
        feed.feed_url = defaults.feed_url.clone();
    }
    if feed.image_url.is_none() {
        feed.image_url = defaults.image_url.clone();
    }
    feed
}

pub fn ensure_feed_in_db(
    conn: &Arc<Mutex<Connection>>,
    feed_guid: &str,
    feed_url: Option<&str>,
    musicindex_endpoint: &str,
) -> Result<i64> {
    {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        if let Some(id) = db::find_feed_id_by_guid(&db, feed_guid)? {
            return Ok(id);
        }
    }
    let url = feed_url.ok_or_else(|| anyhow!("feed URL unknown; cannot auto-subscribe"))?;
    let cfg_path = crate::config::config_path()?;
    let cfg = crate::config::load_config(&cfg_path)?;
    crate::config::ensure_dirs(&cfg)?;
    {
        let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        crate::rss::subscribe_feed(&cfg, &mut db, url, musicindex_endpoint)?;
    }
    let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
    db::find_feed_id_by_guid(&db, feed_guid)?
        .ok_or_else(|| anyhow!("subscribe completed but feed not found"))
}

pub fn check_feed_staleness(
    conn: &Arc<Mutex<Connection>>,
    musicindex_endpoint: &str,
    feed_id: i64,
) -> Result<Option<StaleFeed>> {
    let stored = {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        db::feed_stale_check_row(&db, feed_id)?
    };
    let Some(stored) = stored else {
        return Ok(None);
    };
    let client = MusicIndexClient::new_with_base_url(musicindex_endpoint.to_string());
    let api_feed = client.fetch_feed(&stored.feed_guid, None)?;
    let Some(api_updated_at) = api_feed.updated_at else {
        return Ok(None);
    };
    if stored
        .musicindex_updated_at
        .is_some_and(|stored_at| stored_at >= api_updated_at)
    {
        return Ok(None);
    }
    Ok(Some(StaleFeed {
        feed_id,
        feed_guid: stored.feed_guid,
        title: stored.title,
        new_updated_at: api_updated_at,
    }))
}

pub fn apply_feed_updates(
    conn: &Arc<Mutex<Connection>>,
    musicindex_endpoint: &str,
    stale: &StaleFeed,
) -> Result<FeedApplyOutcome> {
    let tracks = {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        library_service::tracks_for_feed(&db, stale.feed_id)?
    };
    let mut outcome = FeedApplyOutcome {
        tracks_updated: 0,
        edits_written: 0,
        id3_errors: Vec::new(),
    };
    for track in &tracks {
        let Some(local_path) = track.local_path.clone() else {
            continue;
        };
        let Ok((fetched_track, fetched_feed)) =
            fetch_library_track_detail(track, musicindex_endpoint)
        else {
            continue;
        };
        let context =
            merge_track_context_from_detail(track, fetched_track.clone(), fetched_feed.clone());
        {
            let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
            if let Some(feed) = fetched_feed.as_ref() {
                identity_ingest::persist_musicindex_feed(&mut db, stale.feed_id, feed)?;
            }
            if let Some(fetched_track) = fetched_track.as_ref() {
                identity_ingest::persist_musicindex_track(&mut db, track.id, fetched_track)?;
            }
        }
        let edits = id3_edits_for_track_context(&context);
        if edits.is_empty() {
            continue;
        }
        match write_id3v24_edits(Path::new(&local_path), &edits) {
            Ok(written) => {
                if written > 0 {
                    outcome.tracks_updated += 1;
                    outcome.edits_written += written;
                }
            }
            Err(error) => {
                let label = track
                    .track_title
                    .clone()
                    .unwrap_or_else(|| local_path.clone());
                outcome.id3_errors.push(format!("{label}: {error:#}"));
            }
        }
    }
    {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        db::set_feed_musicindex_updated_at(&db, stale.feed_id, stale.new_updated_at)?;
    }
    Ok(outcome)
}

pub fn track_row_to_track_context(track: &TrackRow) -> TrackContext {
    let feed = track_row_to_feed(track);
    let api_track = crate::api::track_with_feed_defaults(
        crate::subscribe_service::track_row_to_api_track(track),
        Some(&feed),
    );
    TrackContext {
        track: api_track,
        feed: Some(feed),
    }
}

pub fn track_row_to_track_context_with_local_identity(
    conn: &Connection,
    track: &TrackRow,
) -> Result<TrackContext> {
    let mut context = track_row_to_track_context(track);
    context.feed = Some(hydrate_feed_identity(
        conn,
        track.feed_id,
        context.feed.take(),
    )?);
    context.track = hydrate_track_identity(conn, track.id, context.track)?;
    Ok(context)
}

fn hydrate_feed_identity(conn: &Connection, feed_id: i64, feed: Option<Feed>) -> Result<Feed> {
    let mut feed = feed.unwrap_or_default();
    feed.source_links = Some(
        db::local_identity_links(conn, db::LocalIdentityOwner::Feed(feed_id))?
            .into_iter()
            .map(source_link_from_local)
            .collect(),
    );
    feed.source_ids = Some(
        db::local_identity_ids(conn, db::LocalIdentityOwner::Feed(feed_id))?
            .into_iter()
            .map(source_id_from_local)
            .collect(),
    );
    feed.source_contributors = Some(
        db::local_contributors(conn, db::LocalEntityOwner::Feed(feed_id))?
            .into_iter()
            .map(contributor_from_local)
            .collect(),
    );
    Ok(feed)
}

fn hydrate_track_identity(conn: &Connection, track_id: i64, mut track: Track) -> Result<Track> {
    track.source_links = Some(
        db::local_identity_links(conn, db::LocalIdentityOwner::Track(track_id))?
            .into_iter()
            .map(source_link_from_local)
            .collect(),
    );
    track.source_ids = Some(
        db::local_identity_ids(conn, db::LocalIdentityOwner::Track(track_id))?
            .into_iter()
            .map(source_id_from_local)
            .collect(),
    );
    track.source_contributors = Some(
        db::local_contributors(conn, db::LocalEntityOwner::Track(track_id))?
            .into_iter()
            .map(contributor_from_local)
            .collect(),
    );
    Ok(track)
}

fn source_link_from_local(row: db::LocalIdentityLinkRow) -> SourceEntityLink {
    SourceEntityLink {
        entity_type: row.entity_type,
        entity_id: row.entity_id,
        position: row.position,
        link_type: row.link_type,
        url: row.url,
        source: Some(row.source),
        extraction_path: row.extraction_path,
        observed_at: row.observed_at,
    }
}

fn source_id_from_local(row: db::LocalIdentityIdRow) -> SourceEntityId {
    SourceEntityId {
        entity_type: row.entity_type,
        entity_id: row.entity_id,
        position: row.position,
        scheme: row.scheme,
        value: row.value,
        source: Some(row.source),
        extraction_path: row.extraction_path,
        observed_at: row.observed_at,
    }
}

fn contributor_from_local(row: db::LocalContributorRow) -> Contributor {
    Contributor {
        name: row.name,
        role: row.role,
        href: row.href,
        img: row.image_url,
        npub: row.nostr_npub,
        group_name: row.group_name,
    }
}

pub fn lookup_musicbrainz_library_track(track: &TrackRow) -> Result<MusicBrainzLookupResult> {
    let path = track
        .local_path
        .as_deref()
        .ok_or_else(|| anyhow!("library track has no local file"))?;
    let tags = read_audio_tags(Path::new(path))?;
    let context = track_row_to_track_context(track);
    let metadata = musicbrainz_lookup_metadata(&context.track, &tags);
    let musicbrainz_client = ReqwestClient::builder()
        .user_agent(format!(
            "v4vmm/{} (MusicBrainz metadata lookup)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;
    let lookup = lookup_recordings(&musicbrainz_client, &metadata, 5)?;
    let image = lookup
        .candidates
        .first()
        .and_then(|candidate| candidate.release_id.as_deref())
        .and_then(|release_id| {
            let url = format!("https://coverartarchive.org/release/{release_id}/front-250");
            crate::subscribe_service::download_image(&musicbrainz_client, &url)
        });
    Ok(MusicBrainzLookupResult { lookup, image })
}

pub fn lookup_musicbrainz_stage_for_track(track: &TrackRow) -> Result<StagedMusicBrainzLookup> {
    let path = track
        .local_path
        .as_deref()
        .ok_or_else(|| anyhow!("no local file"))?;
    let tags = read_audio_tags(Path::new(path))?;
    let api_track = crate::subscribe_service::track_row_to_api_track(track);
    let metadata = musicbrainz_lookup_metadata(&api_track, &tags);
    let musicbrainz_client = ReqwestClient::builder()
        .user_agent(format!(
            "v4vmm/{} (MusicBrainz metadata lookup)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;
    let lookup = lookup_recordings(&musicbrainz_client, &metadata, 3)?;
    let candidate = lookup
        .candidates
        .first()
        .ok_or_else(|| anyhow!("no MusicBrainz results"))?;
    Ok(StagedMusicBrainzLookup {
        edit_count: mb_edits_for_missing_fields(&tags, candidate).len(),
        lookup: MusicBrainzLookupResult {
            lookup,
            image: None,
        },
    })
}

pub fn stage_candidate_for_track(
    track: &TrackRow,
    candidate: &MusicBrainzCandidate,
) -> Result<StagedMusicBrainzLookup> {
    let path = track
        .local_path
        .as_deref()
        .ok_or_else(|| anyhow!("no local file"))?;
    let tags = read_audio_tags(Path::new(path))?;
    Ok(StagedMusicBrainzLookup {
        edit_count: mb_edits_for_missing_fields(&tags, candidate).len(),
        lookup: MusicBrainzLookupResult {
            lookup: MusicBrainzLookup {
                query: "batch release lookup".into(),
                candidates: vec![candidate.clone()],
            },
            image: None,
        },
    })
}

fn mb_edits_for_missing_fields(
    tags: &AudioTags,
    candidate: &MusicBrainzCandidate,
) -> Vec<Id3v24Edit> {
    let mut edits = Vec::new();
    let trck_value = match (candidate.track_position, candidate.total_tracks) {
        (Some(pos), Some(total)) => Some(format!("{pos}/{total}")),
        _ => candidate.track_number.clone(),
    };
    let checks: Vec<(&str, bool, Option<String>)> = vec![
        ("TIT2", tags.title.is_some(), Some(candidate.title.clone())),
        ("TPE1", tags.artist.is_some(), candidate.artist.clone()),
        (
            "TALB",
            tags.album.is_some(),
            candidate.release_title.clone(),
        ),
        ("TRCK", tags.track_number.is_some(), trck_value),
        ("TDRC", tags.date.is_some(), candidate.release_date.clone()),
        (
            "TPUB",
            tag_has_frame(tags, "TPUB"),
            candidate.labels.first().cloned(),
        ),
        (
            "TSRC",
            tag_has_frame(tags, "TSRC"),
            candidate.isrcs.first().cloned(),
        ),
        (
            "TMED",
            tag_has_frame(tags, "TMED"),
            candidate.format.clone(),
        ),
        (
            "TPOS",
            tag_has_frame(tags, "TPOS"),
            candidate.medium_position.map(|p| p.to_string()),
        ),
        (
            "TSST",
            tag_has_frame(tags, "TSST"),
            candidate.medium_title.clone(),
        ),
        (
            "TLEN",
            tag_has_frame(tags, "TLEN"),
            candidate.track_length_ms.map(|ms| ms.to_string()),
        ),
        (
            "TXXX:MusicBrainz Album Id",
            tags.custom.contains_key("MusicBrainz Album Id"),
            candidate.release_id.clone(),
        ),
        (
            "TXXX:MusicBrainz Release Group Id",
            tags.custom.contains_key("MusicBrainz Release Group Id"),
            candidate.release_group_id.clone(),
        ),
        (
            "TXXX:BARCODE",
            tags.custom.contains_key("BARCODE"),
            candidate.release_barcode.clone(),
        ),
        (
            "UFID:http://musicbrainz.org",
            tag_has_frame(tags, "UFID"),
            if candidate.recording_id.is_empty() {
                None
            } else {
                Some(candidate.recording_id.clone())
            },
        ),
    ];

    for (frame_label, has_existing, mb_value) in checks {
        if has_existing {
            continue;
        }
        if let Some(value) = mb_value {
            if !value.is_empty() {
                edits.push(Id3v24Edit {
                    frame_label: frame_label.to_string(),
                    value,
                });
            }
        }
    }
    edits
}

fn tag_has_frame(tags: &AudioTags, frame_id: &str) -> bool {
    tags.fields.iter().any(|field| field.frame_id == frame_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Contributor, Feed, PaymentRoute, SourceEntityId, Track};
    use crate::metadata_service::id3_edits_for_track_context;

    fn setup_test_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    fn insert_track(conn: &Connection) -> Result<TrackRow> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["https://example.test/feed.xml", "feed-guid", "Feed"],
        )?;
        let feed_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title, artist_name)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![feed_id, "track-guid", "Track", "Artist"],
        )?;
        let track_id = conn.last_insert_rowid();
        Ok(TrackRow {
            id: track_id,
            feed_id,
            feed_guid: Some("feed-guid".into()),
            item_guid: "track-guid".into(),
            track_title: Some("Track".into()),
            artist_name: Some("Artist".into()),
            feed_title: Some("Feed".into()),
            ..TrackRow::default()
        })
    }

    #[test]
    fn library_track_context_preserves_feed_guid_for_id3_provenance() {
        let track = TrackRow {
            id: 1,
            feed_id: 2,
            feed_guid: Some("feed-guid".into()),
            item_guid: "track-guid".into(),
            track_title: Some("Song".into()),
            artist_name: None,
            album_title: None,
            album_artist_name: None,
            track_number: None,
            disc_number: None,
            duration_seconds: None,
            enclosure_url: None,
            enclosure_type: None,
            track_image_href: None,
            is_in_library: true,
            feed_title: Some("Feed".into()),
            album_image_href: None,
            local_path: None,
            transcript_url: None,
        };

        let context = track_row_to_track_context(&track);
        let edits = id3_edits_for_track_context(&context);

        assert!(edits.iter().any(|edit| {
            edit.frame_label == "TXXX:MusicIndex Feed Guid" && edit.value == "feed-guid"
        }));
    }

    #[test]
    fn library_track_context_inherits_feed_level_musicindex_metadata() {
        let track_row = TrackRow {
            id: 1,
            feed_id: 2,
            feed_guid: Some("feed-guid".into()),
            item_guid: "track-guid".into(),
            track_title: Some("Song".into()),
            artist_name: Some("Artist".into()),
            album_title: None,
            album_artist_name: None,
            track_number: Some(4),
            disc_number: None,
            duration_seconds: Some(223),
            enclosure_url: Some("https://example.test/track.mp3".into()),
            enclosure_type: None,
            track_image_href: None,
            is_in_library: true,
            feed_title: Some("Feed".into()),
            album_image_href: None,
            local_path: None,
            transcript_url: None,
        };
        let track = Track {
            track_guid: Some("track-guid".into()),
            feed_guid: Some("feed-guid".into()),
            title: Some("Song".into()),
            ..Default::default()
        };
        let feed = Feed {
            feed_guid: Some("feed-guid".into()),
            title: Some("Feed".into()),
            publisher_text: Some("HeyCitizen".into()),
            description: Some("Feed description".into()),
            source_ids: Some(vec![SourceEntityId {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1heycitizen".into()),
                ..Default::default()
            }]),
            source_contributors: Some(vec![Contributor {
                name: Some("HeyCitizen".into()),
                role: Some("musician".into()),
                ..Default::default()
            }]),
            payment_routes: Some(vec![PaymentRoute {
                recipient_name: Some("HeyCitizen".into()),
                split: Some(100.0),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let context = merge_track_context_from_detail(&track_row, Some(track), Some(feed));
        assert_eq!(context.track.publisher_text.as_deref(), Some("HeyCitizen"));
        assert_eq!(
            context.track.source_contributors.as_ref().map(Vec::len),
            Some(1)
        );
        assert_eq!(context.track.source_ids.as_ref().map(Vec::len), Some(1));
        assert_eq!(context.track.payment_routes.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            context
                .feed
                .as_ref()
                .and_then(|feed| feed.description.as_deref()),
            Some("Feed description")
        );
    }

    #[test]
    fn local_track_context_hydrates_persisted_source_facts() -> Result<()> {
        let mut conn = setup_test_db()?;
        let track = insert_track(&conn)?;
        db::replace_local_identity_ids(
            &mut conn,
            db::LocalIdentityOwner::Feed(track.feed_id),
            "musicindex",
            &[db::LocalIdentityIdInput {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1feed".into()),
                ..db::LocalIdentityIdInput::default()
            }],
        )?;
        db::replace_local_identity_links(
            &mut conn,
            db::LocalIdentityOwner::Track(track.id),
            "musicindex",
            &[db::LocalIdentityLinkInput {
                link_type: Some("website".into()),
                url: Some("https://example.test/track".into()),
                ..db::LocalIdentityLinkInput::default()
            }],
        )?;
        db::replace_local_contributors(
            &mut conn,
            db::LocalEntityOwner::Track(track.id),
            "musicindex",
            &[db::LocalContributorInput {
                position: 0,
                name: Some("Track Contributor".into()),
                image_url: Some("https://example.test/contributor.jpg".into()),
                nostr_npub: Some("npub1contributor".into()),
                ..db::LocalContributorInput::default()
            }],
        )?;

        let context = track_row_to_track_context_with_local_identity(&conn, &track)?;

        assert_eq!(
            context
                .feed
                .as_ref()
                .and_then(|feed| feed.source_ids.as_ref())
                .and_then(|ids| ids.first())
                .and_then(|id| id.value.as_deref()),
            Some("npub1feed")
        );
        assert_eq!(
            context
                .track
                .source_links
                .as_ref()
                .and_then(|links| links.first())
                .and_then(|link| link.url.as_deref()),
            Some("https://example.test/track")
        );
        assert_eq!(
            context
                .track
                .source_contributors
                .as_ref()
                .and_then(|contributors| contributors.first())
                .and_then(|contributor| contributor.img.as_deref()),
            Some("https://example.test/contributor.jpg")
        );

        Ok(())
    }
}
