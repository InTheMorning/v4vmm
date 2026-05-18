use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use crate::api;
use crate::db;
use crate::library_service;
use crate::local_identity;
use crate::local_metadata;
use crate::views::{ArtistRef, ArtistView, FeedRef, FeedView, TrackRef, TrackView};

#[derive(Clone, Copy, Debug)]
pub enum FetchMode {
    Shallow,
    WithTracks,
}

pub trait MetadataSource {
    fn fetch_artist(&self, r: &ArtistRef) -> Result<ArtistView>;
    fn fetch_feed(&self, r: &FeedRef, mode: FetchMode) -> Result<FeedView>;
    fn fetch_track(&self, r: &TrackRef) -> Result<TrackView>;
    fn list_feeds_for_artist(&self, r: &ArtistRef) -> Result<Vec<FeedView>>;
}

pub struct ApiSource {
    client: api::Client,
}

impl ApiSource {
    pub fn new(client: api::Client) -> Self {
        Self { client }
    }
}

impl MetadataSource for ApiSource {
    fn fetch_artist(&self, r: &ArtistRef) -> Result<ArtistView> {
        let id = match r {
            ArtistRef::Musicindex(s) => s,
            _ => return Err(anyhow!("ApiSource only handles Musicindex refs")),
        };
        let detail = self.client.fetch_detail("artist", id)?;
        match detail {
            api::EntityDetail::Artist(a) => Ok(ArtistView::from_api(a)),
            _ => Err(anyhow!("expected artist")),
        }
    }

    fn fetch_feed(&self, r: &FeedRef, mode: FetchMode) -> Result<FeedView> {
        let id = match r {
            FeedRef::Musicindex(s) => s,
            _ => return Err(anyhow!("ApiSource only handles Musicindex refs")),
        };
        let include = match mode {
            FetchMode::Shallow => None,
            FetchMode::WithTracks => Some("tracks,source_contributors,payment_routes"),
        };
        let f = self.client.fetch_feed(id, include)?;
        Ok(FeedView::from_api(f))
    }

    fn fetch_track(&self, r: &TrackRef) -> Result<TrackView> {
        let id = match r {
            TrackRef::Musicindex(s) => s,
            _ => return Err(anyhow!("ApiSource only handles Musicindex refs")),
        };
        let t = self.client.fetch_track(
            id,
            Some("source_contributors,payment_routes,source_links,source_ids"),
        )?;
        Ok(TrackView::from_api(t))
    }

    fn list_feeds_for_artist(&self, r: &ArtistRef) -> Result<Vec<FeedView>> {
        let name = match r {
            ArtistRef::Musicindex(s) => s.clone(),
            ArtistRef::LocalArtistName(s) => s.clone(),
        };
        let resp = self.client.fetch_tracks_by_artist(&name, None, None)?;
        let mut by_feed: std::collections::BTreeMap<String, FeedView> = Default::default();
        for t in resp.data {
            let key = t.feed_guid.clone().unwrap_or_default();
            if key.is_empty() {
                continue;
            }
            by_feed.entry(key.clone()).or_insert_with(|| FeedView {
                id: Some(FeedRef::Musicindex(key.clone())),
                feed_guid: Some(key),
                title: t.feed_title.clone(),
                feed_url: t.feed_url.clone(),
                image_url: t.image_url.clone(),
                ..Default::default()
            });
        }
        Ok(by_feed.into_values().collect())
    }
}

pub struct LocalSource {
    conn: Arc<Mutex<Connection>>,
}

impl LocalSource {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

fn local_track_view(conn: &Connection, row: db::TrackRow) -> Result<TrackView> {
    let facts = local_identity::track_facts(conn, row.id)?;
    let metadata_facts = local_metadata::track_facts(conn, row.id)?;
    Ok(TrackView::from_local_with_facts(row, facts, metadata_facts))
}

fn local_feed_view(
    conn: &Connection,
    feed_row: db::FeedRow,
    tracks: Vec<db::TrackRow>,
) -> Result<FeedView> {
    let track_views = tracks
        .into_iter()
        .map(|row| local_track_view(conn, row))
        .collect::<Result<Vec<_>>>()?;
    let facts = local_identity::feed_facts(conn, feed_row.id)?;
    let metadata_facts = local_metadata::feed_facts(conn, feed_row.id)?;
    Ok(FeedView::from_local_with_facts(
        feed_row,
        track_views,
        facts,
        metadata_facts,
    ))
}

pub(crate) fn local_artist_view_from_tracks(
    conn: &Connection,
    name: &str,
    tracks: &[db::TrackRow],
) -> Result<ArtistView> {
    let source_facts = artist_source_facts_for_tracks(conn, tracks)?;
    Ok(ArtistView::from_local_rows_with_artist_source_facts(
        name,
        tracks,
        source_facts,
    ))
}

impl MetadataSource for LocalSource {
    fn fetch_artist(&self, r: &ArtistRef) -> Result<ArtistView> {
        let conn = self.conn.lock().map_err(|e| anyhow!("conn lock: {e}"))?;
        match r {
            ArtistRef::Musicindex(id) => {
                let row = db::artist_source_fact(&conn, "musicindex", id)?
                    .ok_or_else(|| anyhow!("artist source fact {id} not found"))?;
                Ok(ArtistView::from_artist_source_fact(row))
            }
            ArtistRef::LocalArtistName(name) => {
                let rows = library_service::library_tracks(&conn)?;
                let filtered: Vec<_> = rows
                    .into_iter()
                    .filter(|t| {
                        t.album_artist_name.as_deref() == Some(name.as_str())
                            || t.artist_name.as_deref() == Some(name.as_str())
                    })
                    .collect();
                local_artist_view_from_tracks(&conn, name, &filtered)
            }
        }
    }

    fn fetch_feed(&self, r: &FeedRef, _mode: FetchMode) -> Result<FeedView> {
        let id = match r {
            FeedRef::LocalFeedId(i) => *i,
            _ => return Err(anyhow!("LocalSource only handles Local refs")),
        };
        let conn = self.conn.lock().map_err(|e| anyhow!("conn lock: {e}"))?;
        let tracks = db::feed_tracks(&conn, id)?;
        let feed_row: db::FeedRow = conn
            .query_row(
                "SELECT id, feed_url, feed_guid, title, language, description, album_image_href, is_subscribed FROM feeds WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok(db::FeedRow {
                        id: row.get(0)?,
                        feed_url: row.get(1)?,
                        feed_guid: row.get(2)?,
                        title: row.get(3)?,
                        language: row.get(4)?,
                        description: row.get(5)?,
                        album_image_href: row.get(6)?,
                        is_subscribed: row.get::<_, i64>(7)? != 0,
                    })
                },
            )
            .map_err(|e| anyhow!("feed {id} not found: {e}"))?;
        local_feed_view(&conn, feed_row, tracks)
    }

    fn fetch_track(&self, r: &TrackRef) -> Result<TrackView> {
        let id = match r {
            TrackRef::LocalTrackId(i) => *i,
            _ => return Err(anyhow!("LocalSource only handles Local refs")),
        };
        let conn = self.conn.lock().map_err(|e| anyhow!("conn lock: {e}"))?;
        let row = db::track_row_by_id(&conn, id)?.ok_or_else(|| anyhow!("track {id} not found"))?;
        local_track_view(&conn, row)
    }

    fn list_feeds_for_artist(&self, r: &ArtistRef) -> Result<Vec<FeedView>> {
        let name = match r {
            ArtistRef::LocalArtistName(s) => s.clone(),
            _ => return Err(anyhow!("LocalSource only handles Local refs")),
        };
        let conn = self.conn.lock().map_err(|e| anyhow!("conn lock: {e}"))?;
        let rows = library_service::library_tracks(&conn)?;
        let mut by_feed: std::collections::BTreeMap<i64, Vec<db::TrackRow>> = Default::default();
        for t in rows {
            if t.album_artist_name.as_deref() == Some(&name)
                || t.artist_name.as_deref() == Some(&name)
            {
                by_feed.entry(t.feed_id).or_default().push(t);
            }
        }
        let mut out = Vec::with_capacity(by_feed.len());
        for (_feed_id, tracks) in by_feed {
            let feed_row: Option<db::FeedRow> = conn
                .query_row(
                    "SELECT id, feed_url, feed_guid, title, language, description, album_image_href, is_subscribed FROM feeds WHERE id = ?1",
                    rusqlite::params![tracks.first().map(|t| t.feed_id)],
                    |row| {
                        Ok(db::FeedRow {
                            id: row.get(0)?,
                            feed_url: row.get(1)?,
                            feed_guid: row.get(2)?,
                            title: row.get(3)?,
                            language: row.get(4)?,
                            description: row.get(5)?,
                            album_image_href: row.get(6)?,
                            is_subscribed: row.get::<_, i64>(7)? != 0,
                        })
                    },
                )
                .ok();
            if let Some(feed_row) = feed_row {
                out.push(local_feed_view(&conn, feed_row, tracks)?);
            }
        }
        Ok(out)
    }
}

fn artist_source_facts_for_tracks(
    conn: &Connection,
    tracks: &[db::TrackRow],
) -> Result<Vec<db::ArtistSourceFactRow>> {
    let mut seen = BTreeSet::new();
    let mut source_facts = Vec::new();
    for track in tracks {
        for binding in db::track_artist_source_bindings_for_track(conn, track.id)? {
            let key = (binding.source.clone(), binding.source_artist_id.clone());
            if !seen.insert(key) {
                continue;
            }
            if let Some(row) = db::artist_source_fact(
                conn,
                binding.source.as_str(),
                binding.source_artist_id.as_str(),
            )? {
                source_facts.push(row);
            }
        }
    }
    Ok(source_facts)
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

    fn create_feed_and_track(conn: &Connection) -> Result<(i64, i64)> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title, album_image_href)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                "https://example.test/feed.xml",
                "feed-guid",
                "Feed",
                "https://example.test/feed.jpg"
            ],
        )?;
        let feed_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO tracks (
                 feed_id, item_guid, track_title, artist_name, album_title,
                 track_image_href, is_in_library
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
            rusqlite::params![
                feed_id,
                "track-guid",
                "Track",
                "Alice",
                "Feed",
                "https://example.test/track.jpg"
            ],
        )?;
        Ok((feed_id, conn.last_insert_rowid()))
    }

    fn add_feed_identity(conn: &mut Connection, feed_id: i64) -> Result<()> {
        db::replace_local_identity_links(
            conn,
            db::LocalIdentityOwner::Feed(feed_id),
            "musicindex",
            &[db::LocalIdentityLinkInput {
                link_type: Some("website".into()),
                url: Some("https://example.test/feed".into()),
                ..db::LocalIdentityLinkInput::default()
            }],
        )?;
        db::replace_local_identity_ids(
            conn,
            db::LocalIdentityOwner::Feed(feed_id),
            "musicindex",
            &[db::LocalIdentityIdInput {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1feed".into()),
                ..db::LocalIdentityIdInput::default()
            }],
        )?;
        db::replace_local_contributors(
            conn,
            db::LocalEntityOwner::Feed(feed_id),
            "musicindex",
            &[db::LocalContributorInput {
                position: 0,
                name: Some("Feed Contributor".into()),
                href: Some("https://example.test/feed-contributor".into()),
                image_url: Some("https://example.test/feed-contributor.jpg".into()),
                nostr_npub: Some("npub1feedcontributor".into()),
                ..db::LocalContributorInput::default()
            }],
        )
    }

    fn add_feed_metadata(conn: &mut Connection, feed_id: i64) -> Result<()> {
        db::replace_local_metadata_facts(
            conn,
            db::LocalMetadataOwner::Feed(feed_id),
            "musicindex",
            &[
                db::LocalMetadataFactInput {
                    fact_key: "publisher_text".into(),
                    value: db::LocalMetadataValue::Text("Example Publisher".into()),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
                db::LocalMetadataFactInput {
                    fact_key: "musicindex_release_kind".into(),
                    value: db::LocalMetadataValue::Text("album".into()),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
                db::LocalMetadataFactInput {
                    fact_key: "language".into(),
                    value: db::LocalMetadataValue::Text("en".into()),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
            ],
        )
    }

    fn add_track_identity(conn: &mut Connection, track_id: i64) -> Result<()> {
        db::replace_local_identity_links(
            conn,
            db::LocalIdentityOwner::Track(track_id),
            "musicindex",
            &[db::LocalIdentityLinkInput {
                link_type: Some("website".into()),
                url: Some("https://example.test/track".into()),
                ..db::LocalIdentityLinkInput::default()
            }],
        )?;
        db::replace_local_identity_ids(
            conn,
            db::LocalIdentityOwner::Track(track_id),
            "musicindex",
            &[db::LocalIdentityIdInput {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1track".into()),
                ..db::LocalIdentityIdInput::default()
            }],
        )?;
        db::replace_local_contributors(
            conn,
            db::LocalEntityOwner::Track(track_id),
            "musicindex",
            &[db::LocalContributorInput {
                position: 0,
                name: Some("Track Contributor".into()),
                href: Some("https://example.test/track-contributor".into()),
                image_url: Some("https://example.test/track-contributor.jpg".into()),
                nostr_npub: Some("npub1trackcontributor".into()),
                ..db::LocalContributorInput::default()
            }],
        )
    }

    fn add_track_metadata(conn: &mut Connection, track_id: i64) -> Result<()> {
        db::replace_local_metadata_facts(
            conn,
            db::LocalMetadataOwner::Track(track_id),
            "musicindex",
            &[db::LocalMetadataFactInput {
                fact_key: "description".into(),
                value: db::LocalMetadataValue::Text("Track description".into()),
                extraction_path: None,
                observed_at: None,
                raw_json: None,
            }],
        )
    }

    #[test]
    fn local_source_fetch_track_hydrates_persisted_identity_facts() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (_, track_id) = create_feed_and_track(&conn)?;
        add_track_identity(&mut conn, track_id)?;
        let source = LocalSource::new(Arc::new(Mutex::new(conn)));

        let view = source.fetch_track(&TrackRef::LocalTrackId(track_id))?;

        assert_eq!(
            view.identity.website_url.as_deref(),
            Some("https://example.test/track")
        );
        assert_eq!(view.identity.nostr_npub.as_deref(), Some("npub1track"));
        assert_eq!(view.identity.source_links.len(), 1);
        assert_eq!(view.identity.source_ids.len(), 1);
        assert_eq!(view.contributors.len(), 1);
        assert_eq!(
            view.contributors[0].image_url.as_deref(),
            Some("https://example.test/track-contributor.jpg")
        );

        Ok(())
    }

    #[test]
    fn local_source_fetch_track_hydrates_persisted_metadata_facts() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (_, track_id) = create_feed_and_track(&conn)?;
        add_track_metadata(&mut conn, track_id)?;
        let source = LocalSource::new(Arc::new(Mutex::new(conn)));

        let view = source.fetch_track(&TrackRef::LocalTrackId(track_id))?;

        assert_eq!(view.description.as_deref(), Some("Track description"));
        Ok(())
    }

    #[test]
    fn local_source_fetch_feed_hydrates_feed_and_track_identity_facts() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (feed_id, track_id) = create_feed_and_track(&conn)?;
        add_feed_identity(&mut conn, feed_id)?;
        add_feed_metadata(&mut conn, feed_id)?;
        add_track_identity(&mut conn, track_id)?;
        let source = LocalSource::new(Arc::new(Mutex::new(conn)));

        let view = source.fetch_feed(&FeedRef::LocalFeedId(feed_id), FetchMode::WithTracks)?;

        assert_eq!(
            view.identity.website_url.as_deref(),
            Some("https://example.test/feed")
        );
        assert_eq!(view.identity.nostr_npub.as_deref(), Some("npub1feed"));
        assert_eq!(view.contributors.len(), 1);
        assert_eq!(
            view.contributors[0].href.as_deref(),
            Some("https://example.test/feed-contributor")
        );
        assert_eq!(view.publisher_text.as_deref(), Some("Example Publisher"));
        assert_eq!(view.release_kind.as_deref(), Some("album"));
        assert_eq!(view.language.as_deref(), Some("en"));
        assert_eq!(view.tracks.len(), 1);
        assert_eq!(
            view.tracks[0].identity.website_url.as_deref(),
            Some("https://example.test/track")
        );
        assert_eq!(view.tracks[0].contributors.len(), 1);

        Ok(())
    }

    #[test]
    fn local_source_fetch_musicindex_artist_hydrates_persisted_source_fact() -> Result<()> {
        let mut conn = setup_test_db()?;
        db::replace_artist_source_fact(
            &mut conn,
            "musicindex",
            "artist-123",
            &db::ArtistSourceFactInput {
                name: Some("Alice".into()),
                image_url: Some("https://example.test/alice.jpg".into()),
                website_url: Some("https://example.test/alice".into()),
                aliases: vec!["A. Example".into()],
                area: Some("Montreal".into()),
                begin_year: Some(2020),
                source_links: vec![db::LocalIdentityLinkInput {
                    link_type: Some("website".into()),
                    url: Some("https://example.test/source-link".into()),
                    ..db::LocalIdentityLinkInput::default()
                }],
                source_ids: vec![db::LocalIdentityIdInput {
                    scheme: Some("nostr_npub".into()),
                    value: Some("npub1artist".into()),
                    ..db::LocalIdentityIdInput::default()
                }],
                ..db::ArtistSourceFactInput::default()
            },
        )?;
        let source = LocalSource::new(Arc::new(Mutex::new(conn)));

        let view = source.fetch_artist(&ArtistRef::Musicindex("artist-123".into()))?;

        assert!(matches!(
            view.id,
            Some(ArtistRef::Musicindex(ref id)) if id == "artist-123"
        ));
        assert_eq!(view.name.as_deref(), Some("Alice"));
        assert_eq!(
            view.image_url.as_deref(),
            Some("https://example.test/alice.jpg")
        );
        assert_eq!(view.url.as_deref(), Some("https://example.test/alice"));
        assert_eq!(
            view.identity.website_url.as_deref(),
            Some("https://example.test/source-link")
        );
        assert_eq!(view.identity.nostr_npub.as_deref(), Some("npub1artist"));
        assert_eq!(view.aliases, vec!["A. Example"]);
        assert_eq!(view.area.as_deref(), Some("Montreal"));
        assert_eq!(view.begin_year, Some(2020));

        Ok(())
    }

    #[test]
    fn local_source_fetch_musicindex_artist_requires_persisted_fact() -> Result<()> {
        let conn = setup_test_db()?;
        let source = LocalSource::new(Arc::new(Mutex::new(conn)));

        let error = source
            .fetch_artist(&ArtistRef::Musicindex("missing-artist".into()))
            .expect_err("missing explicit artist fact should fail");

        assert!(
            error
                .to_string()
                .contains("artist source fact missing-artist not found"),
            "unexpected error: {error:#}"
        );

        Ok(())
    }

    #[test]
    fn local_source_fetch_local_artist_name_does_not_use_source_facts() -> Result<()> {
        let mut conn = setup_test_db()?;
        create_feed_and_track(&conn)?;
        db::replace_artist_source_fact(
            &mut conn,
            "musicindex",
            "artist-123",
            &db::ArtistSourceFactInput {
                name: Some("Alice".into()),
                image_url: Some("https://example.test/source-artist.jpg".into()),
                website_url: Some("https://example.test/alice".into()),
                ..db::ArtistSourceFactInput::default()
            },
        )?;
        let source = LocalSource::new(Arc::new(Mutex::new(conn)));

        let view = source.fetch_artist(&ArtistRef::LocalArtistName("Alice".into()))?;

        assert!(matches!(
            view.id,
            Some(ArtistRef::LocalArtistName(ref name)) if name == "Alice"
        ));
        assert_eq!(
            view.image_url.as_deref(),
            Some("https://example.test/feed.jpg")
        );
        assert_eq!(view.url, None);
        assert_eq!(view.identity.website_url, None);

        Ok(())
    }

    #[test]
    fn local_source_fetch_local_artist_name_enriches_single_bound_subject() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (_, track_id) = create_feed_and_track(&conn)?;
        db::replace_artist_source_fact(
            &mut conn,
            "musicindex",
            "artist-123",
            &db::ArtistSourceFactInput {
                name: Some("Remote Alice".into()),
                sort_name: Some("Alice, Remote".into()),
                image_url: Some("https://example.test/source-artist.jpg".into()),
                website_url: Some("https://example.test/alice".into()),
                aliases: vec!["A. Example".into()],
                area: Some("Montreal".into()),
                begin_year: Some(2020),
                source_links: vec![db::LocalIdentityLinkInput {
                    link_type: Some("website".into()),
                    url: Some("https://example.test/source-link".into()),
                    ..db::LocalIdentityLinkInput::default()
                }],
                ..db::ArtistSourceFactInput::default()
            },
        )?;
        db::replace_track_artist_source_bindings(
            &mut conn,
            track_id,
            &[db::TrackArtistSourceBindingInput {
                role: "artist".into(),
                source: "musicindex".into(),
                source_artist_id: "artist-123".into(),
                confidence: Some(1.0),
                provenance: Some("test".into()),
                observed_at: Some(1),
            }],
        )?;
        let source = LocalSource::new(Arc::new(Mutex::new(conn)));

        let view = source.fetch_artist(&ArtistRef::LocalArtistName("Alice".into()))?;

        assert!(matches!(
            view.id,
            Some(ArtistRef::LocalArtistName(ref name)) if name == "Alice"
        ));
        assert_eq!(view.name.as_deref(), Some("Alice"));
        assert_eq!(view.feed_count, Some(1));
        assert_eq!(view.track_count, Some(1));
        assert_eq!(
            view.image_url.as_deref(),
            Some("https://example.test/source-artist.jpg")
        );
        assert_eq!(view.sort_name.as_deref(), Some("Alice, Remote"));
        assert_eq!(view.url.as_deref(), Some("https://example.test/alice"));
        assert_eq!(
            view.identity.website_url.as_deref(),
            Some("https://example.test/source-link")
        );
        assert_eq!(view.aliases, vec!["A. Example"]);
        assert_eq!(view.area.as_deref(), Some("Montreal"));
        assert_eq!(view.begin_year, Some(2020));
        assert_eq!(view.source_subjects.len(), 1);

        Ok(())
    }

    #[test]
    fn local_source_fetch_local_artist_name_keeps_multi_subjects_conservative() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (_, track_id) = create_feed_and_track(&conn)?;
        for (source_artist_id, image_url) in [
            ("artist-123", "https://example.test/one.jpg"),
            ("artist-456", "https://example.test/two.jpg"),
        ] {
            db::replace_artist_source_fact(
                &mut conn,
                "musicindex",
                source_artist_id,
                &db::ArtistSourceFactInput {
                    name: Some(format!("Remote {source_artist_id}")),
                    image_url: Some(image_url.into()),
                    website_url: Some(format!("https://example.test/{source_artist_id}")),
                    ..db::ArtistSourceFactInput::default()
                },
            )?;
        }
        db::replace_track_artist_source_bindings(
            &mut conn,
            track_id,
            &[
                db::TrackArtistSourceBindingInput {
                    role: "artist".into(),
                    source: "musicindex".into(),
                    source_artist_id: "artist-123".into(),
                    confidence: Some(1.0),
                    provenance: Some("test.one".into()),
                    observed_at: Some(1),
                },
                db::TrackArtistSourceBindingInput {
                    role: "artist".into(),
                    source: "musicindex".into(),
                    source_artist_id: "artist-456".into(),
                    confidence: Some(1.0),
                    provenance: Some("test.two".into()),
                    observed_at: Some(1),
                },
            ],
        )?;
        let source = LocalSource::new(Arc::new(Mutex::new(conn)));

        let view = source.fetch_artist(&ArtistRef::LocalArtistName("Alice".into()))?;

        assert_eq!(
            view.image_url.as_deref(),
            Some("https://example.test/feed.jpg"),
            "multi-subject local artist view should keep local artwork"
        );
        assert_eq!(view.url, None);
        assert_eq!(view.identity.website_url, None);
        assert_eq!(view.source_subjects.len(), 2);

        Ok(())
    }
}
