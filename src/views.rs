use crate::api;
use crate::db;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtistRef {
    Musicindex(String),
    LocalArtistName(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeedRef {
    Musicindex(String),
    LocalFeedId(i64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrackRef {
    Musicindex(String),
    LocalTrackId(i64),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IdentityLinkFact {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub position: Option<i64>,
    pub link_type: Option<String>,
    pub url: Option<String>,
    pub source: Option<String>,
    pub extraction_path: Option<String>,
    pub observed_at: Option<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IdentityIdFact {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub position: Option<i64>,
    pub scheme: Option<String>,
    pub value: Option<String>,
    pub source: Option<String>,
    pub extraction_path: Option<String>,
    pub observed_at: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtworkRef {
    Url(String),
    CacheKey(String),
    LocalPath(String),
    EmbeddedBytesKey(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ContributorView {
    pub name: Option<String>,
    pub role: Option<String>,
    pub group_name: Option<String>,
    pub href: Option<String>,
    pub image_url: Option<String>,
    pub nostr_npub: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EntityIdentityLinks {
    pub nostr_npub: Option<String>,
    pub website_url: Option<String>,
    pub image_url: Option<String>,
    pub source_links: Vec<IdentityLinkFact>,
    pub source_ids: Vec<IdentityIdFact>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LocalIdentityFacts {
    pub source_links: Vec<IdentityLinkFact>,
    pub source_ids: Vec<IdentityIdFact>,
    pub contributors: Vec<ContributorView>,
}

#[derive(Clone, Debug, Default)]
pub struct ArtistView {
    pub id: Option<ArtistRef>,
    pub name: Option<String>,
    pub sort_name: Option<String>,
    pub image_url: Option<String>,
    pub artwork: Option<ArtworkRef>,
    pub identity: EntityIdentityLinks,
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
    pub artwork: Option<ArtworkRef>,
    pub identity: EntityIdentityLinks,
    pub release_date: Option<i64>,
    pub language: Option<String>,
    pub explicit: Option<bool>,
    pub episode_count: Option<i32>,
    pub release_kind: Option<String>,
    pub publisher_text: Option<String>,
    pub description: Option<String>,
    pub payment_routes: Vec<api::PaymentRoute>,
    pub contributors: Vec<ContributorView>,
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
    pub artwork: Option<ArtworkRef>,
    pub identity: EntityIdentityLinks,
    pub audio_url: Option<String>,
    pub mime: Option<String>,
    pub bytes: Option<i64>,
    pub publisher_text: Option<String>,
    pub contributors: Vec<ContributorView>,
    pub payment_routes: Vec<api::PaymentRoute>,
    pub transcript_url: Option<String>,
}

impl From<api::SourceEntityLink> for IdentityLinkFact {
    fn from(link: api::SourceEntityLink) -> Self {
        Self {
            entity_type: link.entity_type,
            entity_id: link.entity_id,
            position: link.position,
            link_type: link.link_type,
            url: link.url,
            source: link.source,
            extraction_path: link.extraction_path,
            observed_at: link.observed_at,
        }
    }
}

impl From<api::SourceEntityId> for IdentityIdFact {
    fn from(id: api::SourceEntityId) -> Self {
        Self {
            entity_type: id.entity_type,
            entity_id: id.entity_id,
            position: id.position,
            scheme: id.scheme,
            value: id.value,
            source: id.source,
            extraction_path: id.extraction_path,
            observed_at: id.observed_at,
        }
    }
}

impl From<api::Contributor> for ContributorView {
    fn from(contributor: api::Contributor) -> Self {
        Self {
            name: contributor.name,
            role: contributor.role,
            group_name: contributor.group_name,
            href: contributor.href,
            image_url: contributor.img,
            nostr_npub: contributor.npub,
        }
    }
}

impl From<ContributorView> for api::Contributor {
    fn from(contributor: ContributorView) -> Self {
        Self {
            name: contributor.name,
            role: contributor.role,
            href: contributor.href,
            img: contributor.image_url,
            npub: contributor.nostr_npub,
            group_name: contributor.group_name,
        }
    }
}

impl EntityIdentityLinks {
    fn from_api_facts(
        image_url: Option<String>,
        source_links: Option<Vec<api::SourceEntityLink>>,
        source_ids: Option<Vec<api::SourceEntityId>>,
    ) -> Self {
        let source_links = source_links
            .unwrap_or_default()
            .into_iter()
            .map(IdentityLinkFact::from)
            .collect();
        let source_ids = source_ids
            .unwrap_or_default()
            .into_iter()
            .map(IdentityIdFact::from)
            .collect();
        Self::from_source_facts(image_url, source_links, source_ids)
    }

    #[must_use]
    pub fn from_source_facts(
        image_url: Option<String>,
        source_links: Vec<IdentityLinkFact>,
        source_ids: Vec<IdentityIdFact>,
    ) -> Self {
        Self {
            nostr_npub: nostr_npub_from_ids(&source_ids),
            website_url: website_url_from_links(&source_links),
            image_url,
            source_links,
            source_ids,
        }
    }

    #[must_use]
    pub fn from_image_url(image_url: Option<String>) -> Self {
        Self {
            image_url,
            ..Self::default()
        }
    }
}

#[must_use]
pub fn contributor_views_to_api(contributors: &[ContributorView]) -> Vec<api::Contributor> {
    contributors
        .iter()
        .cloned()
        .map(api::Contributor::from)
        .collect()
}

fn nostr_npub_from_ids(ids: &[IdentityIdFact]) -> Option<String> {
    ids.iter().find_map(|id| {
        if id.scheme.as_deref() == Some("nostr_npub") {
            id.value.clone()
        } else {
            None
        }
    })
}

fn website_url_from_links(links: &[IdentityLinkFact]) -> Option<String> {
    links.iter().find_map(|link| {
        if link.link_type.as_deref() == Some("website") {
            link.url.clone()
        } else {
            None
        }
    })
}

fn artwork_from_url(url: &Option<String>) -> Option<ArtworkRef> {
    url.clone().map(ArtworkRef::Url)
}

impl ArtistView {
    pub fn from_api(a: api::Artist) -> Self {
        let image_url = a.image_url;
        let mut identity = EntityIdentityLinks::from_image_url(image_url.clone());
        identity.website_url = a.url.clone();
        Self {
            id: a.artist_id.map(ArtistRef::Musicindex),
            name: a.name,
            sort_name: a.sort_name,
            image_url: image_url.clone(),
            artwork: artwork_from_url(&image_url),
            identity,
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
            image_url: image_url.clone(),
            artwork: artwork_from_url(&image_url),
            identity: EntityIdentityLinks::from_image_url(image_url),
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
        let image_url = f.image_url;
        let identity =
            EntityIdentityLinks::from_api_facts(image_url.clone(), f.source_links, f.source_ids);
        Self {
            id: f.feed_guid.clone().map(FeedRef::Musicindex),
            feed_guid: f.feed_guid,
            feed_url: f.feed_url,
            title: f.title.or(f.name),
            artist: f.release_artist,
            image_url: image_url.clone(),
            artwork: artwork_from_url(&image_url),
            identity,
            release_date: f.release_date,
            language: f.language,
            explicit: f.explicit,
            episode_count: f.episode_count,
            release_kind: f.release_kind,
            publisher_text: f.publisher_text,
            description: f.description,
            payment_routes: f.payment_routes.unwrap_or_default(),
            contributors: f
                .source_contributors
                .unwrap_or_default()
                .into_iter()
                .map(ContributorView::from)
                .collect(),
            tracks: f
                .tracks
                .unwrap_or_default()
                .into_iter()
                .map(TrackView::from_api)
                .collect(),
        }
    }

    pub fn from_local(f: db::FeedRow, tracks: Vec<db::TrackRow>) -> Self {
        let track_views = tracks.into_iter().map(TrackView::from_local).collect();
        Self::from_local_with_identity(f, track_views, LocalIdentityFacts::default())
    }

    pub fn from_local_with_identity(
        f: db::FeedRow,
        tracks: Vec<TrackView>,
        facts: LocalIdentityFacts,
    ) -> Self {
        let artist = tracks.first().and_then(|t| t.artist.clone());
        let image_url = f.album_image_href;
        let identity = EntityIdentityLinks::from_source_facts(
            image_url.clone(),
            facts.source_links,
            facts.source_ids,
        );

        Self {
            id: Some(FeedRef::LocalFeedId(f.id)),
            feed_guid: f.feed_guid,
            feed_url: Some(f.feed_url),
            title: f.title,
            artist,
            image_url: image_url.clone(),
            artwork: artwork_from_url(&image_url),
            identity,
            release_date: None,
            language: None,
            explicit: None,
            episode_count: Some(tracks.len() as i32),
            release_kind: None,
            publisher_text: None,
            description: f.description,
            payment_routes: Vec::new(),
            contributors: facts.contributors,
            tracks,
        }
    }
}

impl TrackView {
    pub fn from_api(t: api::Track) -> Self {
        let source_links = t.source_links;
        // Find transcript_url from source_links
        let transcript_url = source_links
            .as_ref()
            .and_then(|links| {
                links
                    .iter()
                    .find(|l| l.link_type.as_deref() == Some("transcript"))
            })
            .and_then(|link| link.url.clone());
        let image_url = t.image_url;
        let identity =
            EntityIdentityLinks::from_api_facts(image_url.clone(), source_links, t.source_ids);

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
            image_url: image_url.clone(),
            artwork: artwork_from_url(&image_url),
            identity,
            audio_url: t.enclosure_url,
            mime: t.enclosure_type,
            bytes: t.enclosure_bytes,
            publisher_text: t.publisher_text,
            contributors: t
                .source_contributors
                .unwrap_or_default()
                .into_iter()
                .map(ContributorView::from)
                .collect(),
            payment_routes: t.payment_routes.unwrap_or_default(),
            transcript_url,
        }
    }

    pub fn from_local(t: db::TrackRow) -> Self {
        Self::from_local_with_identity(t, LocalIdentityFacts::default())
    }

    pub fn from_local_with_identity(t: db::TrackRow, facts: LocalIdentityFacts) -> Self {
        let image_url = t.track_image_href.or(t.album_image_href);
        let identity = EntityIdentityLinks::from_source_facts(
            image_url.clone(),
            facts.source_links,
            facts.source_ids,
        );
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
            image_url: image_url.clone(),
            artwork: artwork_from_url(&image_url),
            identity,
            audio_url: t.enclosure_url,
            mime: t.enclosure_type,
            bytes: None,
            publisher_text: None,
            contributors: facts.contributors,
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
    fn from_api_feed_preserves_identity_facts() {
        let feed = api::Feed {
            image_url: Some("https://example.test/feed.jpg".into()),
            source_links: Some(vec![
                api::SourceEntityLink {
                    link_type: Some("website".into()),
                    url: Some("https://example.test/artist".into()),
                    extraction_path: Some("feed.link".into()),
                    ..Default::default()
                },
                api::SourceEntityLink {
                    link_type: Some("donate".into()),
                    url: Some("https://example.test/donate".into()),
                    ..Default::default()
                },
            ]),
            source_ids: Some(vec![api::SourceEntityId {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1feed".into()),
                source: Some("rss".into()),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let view = FeedView::from_api(feed);

        assert_eq!(
            view.identity.image_url.as_deref(),
            Some("https://example.test/feed.jpg")
        );
        assert_eq!(
            view.identity.website_url.as_deref(),
            Some("https://example.test/artist")
        );
        assert_eq!(view.identity.nostr_npub.as_deref(), Some("npub1feed"));
        assert_eq!(view.identity.source_links.len(), 2);
        assert_eq!(
            view.identity.source_links[0].extraction_path.as_deref(),
            Some("feed.link")
        );
        assert_eq!(view.identity.source_ids[0].source.as_deref(), Some("rss"));
    }

    #[test]
    fn from_api_track_converts_contributors_to_view_facts() {
        let track = api::Track {
            source_contributors: Some(vec![api::Contributor {
                name: Some("Alice".into()),
                role: Some("guitar".into()),
                group_name: Some("Band".into()),
                href: Some("https://example.test/alice".into()),
                img: Some("https://example.test/alice.jpg".into()),
                npub: Some("npub1alice".into()),
            }]),
            ..Default::default()
        };

        let view = TrackView::from_api(track);
        let contributor = view
            .contributors
            .first()
            .expect("track contributor should project");

        assert_eq!(contributor.name.as_deref(), Some("Alice"));
        assert_eq!(contributor.role.as_deref(), Some("guitar"));
        assert_eq!(contributor.group_name.as_deref(), Some("Band"));
        assert_eq!(
            contributor.href.as_deref(),
            Some("https://example.test/alice")
        );
        assert_eq!(
            contributor.image_url.as_deref(),
            Some("https://example.test/alice.jpg")
        );
        assert_eq!(contributor.nostr_npub.as_deref(), Some("npub1alice"));
    }

    #[test]
    fn contributor_view_round_trips_to_api_for_legacy_shims() {
        let contributors = vec![ContributorView {
            name: Some("Alice".into()),
            role: Some("vocals".into()),
            group_name: Some("Band".into()),
            href: Some("https://example.test/alice".into()),
            image_url: Some("https://example.test/alice.jpg".into()),
            nostr_npub: Some("npub1alice".into()),
        }];

        let api_contributors = contributor_views_to_api(&contributors);
        let contributor = api_contributors
            .first()
            .expect("contributor should convert to legacy API shape");

        assert_eq!(contributor.name.as_deref(), Some("Alice"));
        assert_eq!(contributor.role.as_deref(), Some("vocals"));
        assert_eq!(contributor.group_name.as_deref(), Some("Band"));
        assert_eq!(
            contributor.href.as_deref(),
            Some("https://example.test/alice")
        );
        assert_eq!(
            contributor.img.as_deref(),
            Some("https://example.test/alice.jpg")
        );
        assert_eq!(contributor.npub.as_deref(), Some("npub1alice"));
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
    fn from_local_track_hydrates_identity_facts_and_contributors() {
        let track = db::TrackRow {
            id: 42,
            item_guid: "g".into(),
            track_image_href: Some("https://example.test/track.jpg".into()),
            ..Default::default()
        };
        let facts = LocalIdentityFacts {
            source_links: vec![IdentityLinkFact {
                link_type: Some("website".into()),
                url: Some("https://example.test/track".into()),
                ..IdentityLinkFact::default()
            }],
            source_ids: vec![IdentityIdFact {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1track".into()),
                ..IdentityIdFact::default()
            }],
            contributors: vec![ContributorView {
                name: Some("Alice".into()),
                href: Some("https://example.test/alice".into()),
                image_url: Some("https://example.test/alice.jpg".into()),
                nostr_npub: Some("npub1alice".into()),
                ..ContributorView::default()
            }],
        };

        let view = TrackView::from_local_with_identity(track, facts);

        assert_eq!(
            view.identity.website_url.as_deref(),
            Some("https://example.test/track")
        );
        assert_eq!(view.identity.nostr_npub.as_deref(), Some("npub1track"));
        assert_eq!(
            view.identity.image_url.as_deref(),
            Some("https://example.test/track.jpg")
        );
        assert_eq!(view.identity.source_links.len(), 1);
        assert_eq!(view.identity.source_ids.len(), 1);
        assert_eq!(view.contributors.len(), 1);
        assert_eq!(view.contributors[0].name.as_deref(), Some("Alice"));
        assert_eq!(
            view.contributors[0].image_url.as_deref(),
            Some("https://example.test/alice.jpg")
        );
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
    fn from_local_feed_hydrates_identity_facts_and_contributors() {
        let feed = db::FeedRow {
            id: 1,
            feed_url: "http://example.com".into(),
            album_image_href: Some("https://example.test/feed.jpg".into()),
            ..Default::default()
        };
        let facts = LocalIdentityFacts {
            source_links: vec![IdentityLinkFact {
                link_type: Some("website".into()),
                url: Some("https://example.test/feed".into()),
                ..IdentityLinkFact::default()
            }],
            source_ids: vec![IdentityIdFact {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1feed".into()),
                ..IdentityIdFact::default()
            }],
            contributors: vec![ContributorView {
                name: Some("Bob".into()),
                href: Some("https://example.test/bob".into()),
                image_url: Some("https://example.test/bob.jpg".into()),
                nostr_npub: Some("npub1bob".into()),
                ..ContributorView::default()
            }],
        };

        let view = FeedView::from_local_with_identity(feed, Vec::new(), facts);

        assert_eq!(
            view.identity.website_url.as_deref(),
            Some("https://example.test/feed")
        );
        assert_eq!(view.identity.nostr_npub.as_deref(), Some("npub1feed"));
        assert_eq!(
            view.identity.image_url.as_deref(),
            Some("https://example.test/feed.jpg")
        );
        assert_eq!(view.identity.source_links.len(), 1);
        assert_eq!(view.identity.source_ids.len(), 1);
        assert_eq!(view.contributors.len(), 1);
        assert_eq!(view.contributors[0].name.as_deref(), Some("Bob"));
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
