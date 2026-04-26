use crate::api;
use crate::db;

#[derive(Clone, Debug)]
pub enum ArtistRef {
    Musicindex(String),
    LocalArtistName(String),
}

#[derive(Clone, Debug)]
pub enum FeedRef {
    Musicindex(String),
    LocalFeedId(i64),
}

#[derive(Clone, Debug)]
pub enum TrackRef {
    Musicindex(String),
    LocalTrackId(i64),
}

#[derive(Clone, Debug, Default)]
pub struct ArtistView {
    pub id: Option<ArtistRef>,
    pub name: Option<String>,
    pub sort_name: Option<String>,
    pub image_url: Option<String>,
    pub area: Option<String>,
    pub begin_year: Option<i32>,
    pub end_year: Option<i32>,
    pub feed_count: Option<i32>,
    pub track_count: Option<i32>,
    pub url: Option<String>,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct FeedView {
    pub id: Option<FeedRef>,
    pub feed_guid: Option<String>,
    pub feed_url: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub image_url: Option<String>,
    pub release_date: Option<i64>,
    pub language: Option<String>,
    pub explicit: Option<bool>,
    pub episode_count: Option<i32>,
    pub release_kind: Option<String>,
    pub publisher_text: Option<String>,
    pub description: Option<String>,
    pub payment_routes: Vec<api::PaymentRoute>,
    pub contributors: Vec<api::Contributor>,
    pub tracks: Vec<TrackView>,
}

#[derive(Clone, Debug, Default)]
pub struct TrackView {
    pub id: Option<TrackRef>,
    pub track_guid: Option<String>,
    pub feed_guid: Option<String>,
    pub feed_title: Option<String>,
    pub feed_url: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub duration_secs: Option<i32>,
    pub pub_date: Option<i64>,
    pub explicit: Option<bool>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub audio_url: Option<String>,
    pub mime: Option<String>,
    pub bytes: Option<i64>,
    pub publisher_text: Option<String>,
    pub contributors: Vec<api::Contributor>,
    pub payment_routes: Vec<api::PaymentRoute>,
    pub transcript_url: Option<String>,
}

impl ArtistView {
    pub fn from_api(a: api::Artist) -> Self {
        Self {
            id: a.artist_id.map(ArtistRef::Musicindex),
            name: a.name,
            sort_name: a.sort_name,
            image_url: a.image_url,
            area: a.area,
            begin_year: a.begin_year,
            end_year: a.end_year,
            feed_count: a.feed_count,
            track_count: a.track_count,
            url: a.url,
            aliases: a.aliases.unwrap_or_default(),
            tags: a.tags.unwrap_or_default(),
        }
    }

    pub fn from_local_rows(name: &str, rows: &[db::TrackRow]) -> Self {
        // Count unique feed_ids
        let mut feed_ids = std::collections::HashSet::new();
        let mut image_url = None;

        for row in rows {
            feed_ids.insert(row.feed_id);
            if image_url.is_none() {
                image_url = row.album_image_href.clone();
            }
        }

        Self {
            id: Some(ArtistRef::LocalArtistName(name.into())),
            name: Some(name.into()),
            sort_name: None,
            image_url,
            area: None,
            begin_year: None,
            end_year: None,
            feed_count: Some(feed_ids.len() as i32),
            track_count: Some(rows.len() as i32),
            url: None,
            aliases: Vec::new(),
            tags: Vec::new(),
        }
    }
}

impl FeedView {
    pub fn from_api(f: api::Feed) -> Self {
        Self {
            id: f.feed_guid.clone().map(FeedRef::Musicindex),
            feed_guid: f.feed_guid,
            feed_url: f.feed_url,
            title: f.title.or(f.name),
            artist: f.release_artist,
            image_url: f.image_url,
            release_date: f.release_date,
            language: f.language,
            explicit: f.explicit,
            episode_count: f.episode_count,
            release_kind: f.release_kind,
            publisher_text: f.publisher_text,
            description: f.description,
            payment_routes: f.payment_routes.unwrap_or_default(),
            contributors: f.source_contributors.unwrap_or_default(),
            tracks: f
                .tracks
                .unwrap_or_default()
                .into_iter()
                .map(TrackView::from_api)
                .collect(),
        }
    }

    pub fn from_local(f: db::FeedRow, tracks: Vec<db::TrackRow>) -> Self {
        let artist = tracks
            .first()
            .and_then(|t| t.album_artist_name.clone().or(t.artist_name.clone()));

        Self {
            id: Some(FeedRef::LocalFeedId(f.id)),
            feed_guid: f.feed_guid,
            feed_url: Some(f.feed_url),
            title: f.title,
            artist,
            image_url: f.album_image_href,
            release_date: None,
            language: None,
            explicit: None,
            episode_count: Some(tracks.len() as i32),
            release_kind: None,
            publisher_text: None,
            description: f.description,
            payment_routes: Vec::new(),
            contributors: Vec::new(),
            tracks: tracks.into_iter().map(TrackView::from_local).collect(),
        }
    }
}

impl TrackView {
    pub fn from_api(t: api::Track) -> Self {
        // Find transcript_url from source_links
        let transcript_url = t
            .source_links
            .as_ref()
            .and_then(|links| {
                links
                    .iter()
                    .find(|l| l.link_type.as_deref() == Some("transcript"))
            })
            .and_then(|link| link.url.clone());

        Self {
            id: t.track_guid.clone().map(TrackRef::Musicindex),
            track_guid: t.track_guid,
            feed_guid: t.feed_guid,
            feed_title: t.feed_title.clone(),
            feed_url: t.feed_url,
            title: t.title.or(t.name),
            artist: t.track_artist.or(t.release_artist),
            album: t.feed_title,
            track_number: t.track_number,
            disc_number: None,
            duration_secs: t.duration_secs,
            pub_date: t.pub_date,
            explicit: t.explicit,
            description: t.description,
            image_url: t.image_url,
            audio_url: t.enclosure_url,
            mime: t.enclosure_type,
            bytes: t.enclosure_bytes,
            publisher_text: t.publisher_text,
            contributors: t.source_contributors.unwrap_or_default(),
            payment_routes: t.payment_routes.unwrap_or_default(),
            transcript_url,
        }
    }

    pub fn from_local(t: db::TrackRow) -> Self {
        Self {
            id: Some(TrackRef::LocalTrackId(t.id)),
            track_guid: Some(t.item_guid),
            feed_guid: t.feed_guid,
            feed_title: t.feed_title.clone(),
            feed_url: None,
            title: t.track_title,
            artist: t.artist_name.or(t.album_artist_name),
            album: t.album_title.or(t.feed_title),
            track_number: t.track_number.and_then(|v| v.try_into().ok()),
            disc_number: t.disc_number.and_then(|v| v.try_into().ok()),
            duration_secs: t.duration_seconds.and_then(|v| v.try_into().ok()),
            pub_date: None,
            explicit: None,
            description: None,
            image_url: t.track_image_href.or(t.album_image_href),
            audio_url: t.enclosure_url,
            mime: t.enclosure_type,
            bytes: None,
            publisher_text: None,
            contributors: Vec::new(),
            payment_routes: Vec::new(),
            transcript_url: t.transcript_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_api_track_roundtrip() {
        let track = api::Track {
            track_guid: Some("abc".into()),
            title: Some("T".into()),
            enclosure_url: Some("http://a/b.mp3".into()),
            enclosure_type: Some("audio/mpeg".into()),
            source_links: Some(vec![api::SourceEntityLink {
                link_type: Some("transcript".into()),
                url: Some("http://t/x.srt".into()),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let view = TrackView::from_api(track);

        assert_eq!(view.title, Some("T".into()));
        assert_eq!(view.audio_url, Some("http://a/b.mp3".into()));
        assert_eq!(view.mime, Some("audio/mpeg".into()));
        assert_eq!(view.transcript_url, Some("http://t/x.srt".into()));
        assert!(matches!(
            view.id,
            Some(TrackRef::Musicindex(ref s)) if s == "abc"
        ));
    }

    #[test]
    fn from_local_track_basic() {
        let track = db::TrackRow {
            id: 42,
            item_guid: "g".into(),
            enclosure_url: Some("http://a/b.mp3".into()),
            duration_seconds: Some(180),
            ..Default::default()
        };

        let view = TrackView::from_local(track);

        assert!(matches!(view.id, Some(TrackRef::LocalTrackId(42))));
        assert_eq!(view.duration_secs, Some(180));
        assert_eq!(view.audio_url, Some("http://a/b.mp3".into()));
    }

    #[test]
    fn from_local_feed_aggregates_artist() {
        let feed = db::FeedRow {
            id: 1,
            feed_url: "http://example.com".into(),
            ..Default::default()
        };

        let tracks = vec![
            db::TrackRow {
                id: 1,
                feed_id: 1,
                album_artist_name: Some("Mike Pietro".into()),
                ..Default::default()
            },
            db::TrackRow {
                id: 2,
                feed_id: 1,
                album_artist_name: Some("Mike Pietro".into()),
                ..Default::default()
            },
        ];

        let view = FeedView::from_local(feed, tracks);

        assert_eq!(view.artist, Some("Mike Pietro".into()));
        assert_eq!(view.tracks.len(), 2);
    }

    #[test]
    fn from_local_rows_artist_counts_feeds() {
        let rows = vec![
            db::TrackRow {
                feed_id: 1,
                ..Default::default()
            },
            db::TrackRow {
                feed_id: 1,
                ..Default::default()
            },
            db::TrackRow {
                feed_id: 2,
                ..Default::default()
            },
        ];

        let view = ArtistView::from_local_rows("Mike", &rows);

        assert_eq!(view.feed_count, Some(2));
        assert_eq!(view.track_count, Some(3));
    }
}
