use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use crate::api;
use crate::db;
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

impl MetadataSource for LocalSource {
    fn fetch_artist(&self, r: &ArtistRef) -> Result<ArtistView> {
        let name = match r {
            ArtistRef::LocalArtistName(s) => s.clone(),
            _ => return Err(anyhow!("LocalSource only handles Local refs")),
        };
        let conn = self.conn.lock().map_err(|e| anyhow!("conn lock: {e}"))?;
        let rows = db::library_tracks(&conn)?;
        let filtered: Vec<_> = rows
            .into_iter()
            .filter(|t| {
                t.album_artist_name.as_deref() == Some(&name)
                    || t.artist_name.as_deref() == Some(&name)
            })
            .collect();
        Ok(ArtistView::from_local_rows(&name, &filtered))
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
                "SELECT id, feed_url, feed_guid, title, description, album_image_href, is_subscribed FROM feeds WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok(db::FeedRow {
                        id: row.get(0)?,
                        feed_url: row.get(1)?,
                        feed_guid: row.get(2)?,
                        title: row.get(3)?,
                        description: row.get(4)?,
                        album_image_href: row.get(5)?,
                        is_subscribed: row.get::<_, i64>(6)? != 0,
                    })
                },
            )
            .map_err(|e| anyhow!("feed {id} not found: {e}"))?;
        Ok(FeedView::from_local(feed_row, tracks))
    }

    fn fetch_track(&self, r: &TrackRef) -> Result<TrackView> {
        let id = match r {
            TrackRef::LocalTrackId(i) => *i,
            _ => return Err(anyhow!("LocalSource only handles Local refs")),
        };
        let conn = self.conn.lock().map_err(|e| anyhow!("conn lock: {e}"))?;
        let row: db::TrackRow = conn
            .query_row(
                "SELECT t.id, t.feed_id, f.feed_guid, t.item_guid, t.track_title, t.artist_name, t.album_title,
                        t.album_artist_name, t.track_number, t.disc_number, t.duration_seconds,
                        t.enclosure_url, t.enclosure_type, t.track_image_href,
                        t.is_in_library, f.title, f.album_image_href, lf.path, t.extra_json
                 FROM tracks t
                 JOIN feeds f ON f.id = t.feed_id
                 LEFT JOIN local_files lf ON lf.track_id = t.id
                 WHERE t.id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok(db::TrackRow {
                        id: row.get(0)?,
                        feed_id: row.get(1)?,
                        feed_guid: row.get(2)?,
                        item_guid: row.get(3)?,
                        track_title: row.get(4)?,
                        artist_name: row.get(5)?,
                        album_title: row.get(6)?,
                        album_artist_name: row.get(7)?,
                        track_number: row.get(8)?,
                        disc_number: row.get(9)?,
                        duration_seconds: row.get(10)?,
                        enclosure_url: row.get(11)?,
                        enclosure_type: row.get(12)?,
                        track_image_href: row.get(13)?,
                        is_in_library: row.get::<_, i64>(14)? != 0,
                        feed_title: row.get(15)?,
                        album_image_href: row.get(16)?,
                        local_path: row.get(17)?,
                        transcript_url: db::transcript_url_from_extra_json(
                            row.get::<_, Option<String>>(18)?.as_deref(),
                        ),
                    })
                },
            )
            .map_err(|e| anyhow!("track {id} not found: {e}"))?;
        Ok(TrackView::from_local(row))
    }

    fn list_feeds_for_artist(&self, r: &ArtistRef) -> Result<Vec<FeedView>> {
        let name = match r {
            ArtistRef::LocalArtistName(s) => s.clone(),
            _ => return Err(anyhow!("LocalSource only handles Local refs")),
        };
        let conn = self.conn.lock().map_err(|e| anyhow!("conn lock: {e}"))?;
        let rows = db::library_tracks(&conn)?;
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
                    "SELECT id, feed_url, feed_guid, title, description, album_image_href, is_subscribed FROM feeds WHERE id = ?1",
                    rusqlite::params![tracks.first().map(|t| t.feed_id)],
                    |row| {
                        Ok(db::FeedRow {
                            id: row.get(0)?,
                            feed_url: row.get(1)?,
                            feed_guid: row.get(2)?,
                            title: row.get(3)?,
                            description: row.get(4)?,
                            album_image_href: row.get(5)?,
                            is_subscribed: row.get::<_, i64>(6)? != 0,
                        })
                    },
                )
                .ok();
            if let Some(feed_row) = feed_row {
                out.push(FeedView::from_local(feed_row, tracks));
            }
        }
        Ok(out)
    }
}
