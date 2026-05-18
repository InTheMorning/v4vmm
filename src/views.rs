use crate::api;
use crate::db;
use crate::metadata::drop_placeholder_source_text;

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
    pub source_subjects: Vec<ArtistSourceSubjectView>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ArtistSourceSubjectView {
    pub source: String,
    pub source_artist_id: String,
    pub name: Option<String>,
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

impl From<db::LocalIdentityLinkRow> for IdentityLinkFact {
    fn from(link: db::LocalIdentityLinkRow) -> Self {
        Self {
            entity_type: link.entity_type,
            entity_id: link.entity_id,
            position: link.position,
            link_type: link.link_type,
            url: link.url,
            source: Some(link.source),
            extraction_path: link.extraction_path,
            observed_at: link.observed_at,
        }
    }
}

impl From<db::LocalIdentityIdRow> for IdentityIdFact {
    fn from(id: db::LocalIdentityIdRow) -> Self {
        Self {
            entity_type: id.entity_type,
            entity_id: id.entity_id,
            position: id.position,
            scheme: id.scheme,
            value: id.value,
            source: Some(id.source),
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

fn checked_year(year: Option<i64>) -> Option<i32> {
    year.and_then(|year| i32::try_from(year).ok())
}

fn nonempty_owned(value: Option<String>) -> Option<String> {
    let value = value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    drop_placeholder_source_text(value)
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
            source_subjects: Vec::new(),
        }
    }

    pub fn from_artist_source_fact(row: db::ArtistSourceFactRow) -> Self {
        let image_url = row.image_url;
        let source_links = row
            .source_links
            .into_iter()
            .map(IdentityLinkFact::from)
            .collect();
        let source_ids = row
            .source_ids
            .into_iter()
            .map(IdentityIdFact::from)
            .collect();
        let mut identity =
            EntityIdentityLinks::from_source_facts(image_url.clone(), source_links, source_ids);
        if identity.website_url.is_none() {
            identity.website_url.clone_from(&row.website_url);
        }

        Self {
            id: if row.source == "musicindex" {
                Some(ArtistRef::Musicindex(row.source_artist_id))
            } else {
                None
            },
            name: row.name,
            sort_name: row.sort_name,
            image_url: image_url.clone(),
            artwork: artwork_from_url(&image_url),
            identity,
            area: row.area,
            begin_year: checked_year(row.begin_year),
            end_year: checked_year(row.end_year),
            feed_count: None,
            track_count: None,
            url: row.website_url,
            aliases: row.aliases,
            tags: row.tags,
            source_subjects: Vec::new(),
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
            source_subjects: Vec::new(),
        }
    }

    pub fn from_local_rows_with_artist_source_facts(
        name: &str,
        rows: &[db::TrackRow],
        mut source_facts: Vec<db::ArtistSourceFactRow>,
    ) -> Self {
        source_facts.sort_by(|a, b| {
            a.source
                .cmp(&b.source)
                .then(a.source_artist_id.cmp(&b.source_artist_id))
        });
        source_facts
            .dedup_by(|a, b| a.source == b.source && a.source_artist_id == b.source_artist_id);

        let mut view = Self::from_local_rows(name, rows);
        view.source_subjects = source_facts
            .iter()
            .map(|row| ArtistSourceSubjectView {
                source: row.source.clone(),
                source_artist_id: row.source_artist_id.clone(),
                name: row.name.clone(),
            })
            .collect();

        if source_facts.len() == 1 {
            apply_single_artist_source_fact(&mut view, &source_facts[0]);
        }

        view
    }
}

fn apply_single_artist_source_fact(view: &mut ArtistView, row: &db::ArtistSourceFactRow) {
    if row.image_url.is_some() {
        view.image_url.clone_from(&row.image_url);
        view.artwork = artwork_from_url(&row.image_url);
    }
    view.sort_name.clone_from(&row.sort_name);
    view.area.clone_from(&row.area);
    view.begin_year = checked_year(row.begin_year);
    view.end_year = checked_year(row.end_year);
    view.url.clone_from(&row.website_url);
    view.aliases.clone_from(&row.aliases);
    view.tags.clone_from(&row.tags);

    let source_links = row
        .source_links
        .iter()
        .cloned()
        .map(IdentityLinkFact::from)
        .collect();
    let source_ids = row
        .source_ids
        .iter()
        .cloned()
        .map(IdentityIdFact::from)
        .collect();
    view.identity =
        EntityIdentityLinks::from_source_facts(view.image_url.clone(), source_links, source_ids);
    if view.identity.website_url.is_none() {
        view.identity.website_url.clone_from(&row.website_url);
    }
}

impl FeedView {
    pub fn from_api(f: api::Feed) -> Self {
        let image_url = nonempty_owned(f.image_url);
        let source_description =
            description_from_release_claims(f.source_release_claims.as_deref());
        let identity =
            EntityIdentityLinks::from_api_facts(image_url.clone(), f.source_links, f.source_ids);
        Self {
            id: f.feed_guid.clone().map(FeedRef::Musicindex),
            feed_guid: f.feed_guid,
            feed_url: nonempty_owned(f.feed_url),
            title: nonempty_owned(f.title).or_else(|| nonempty_owned(f.name)),
            artist: nonempty_owned(f.release_artist),
            image_url: image_url.clone(),
            artwork: artwork_from_url(&image_url),
            identity,
            release_date: f.release_date,
            language: nonempty_owned(f.language),
            explicit: f.explicit,
            episode_count: f.episode_count,
            release_kind: nonempty_owned(f.release_kind),
            publisher_text: nonempty_owned(f.publisher_text),
            description: source_description.or_else(|| nonempty_owned(f.description)),
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
            language: nonempty_owned(f.language),
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

fn description_from_release_claims(claims: Option<&[api::SourceReleaseClaim]>) -> Option<String> {
    claims?
        .iter()
        .filter(|claim| claim.claim_type.as_deref() == Some("description"))
        .filter_map(|claim| nonempty_owned(claim.claim_value.clone()))
        .next()
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
        let image_url = nonempty_owned(t.image_url);
        let identity =
            EntityIdentityLinks::from_api_facts(image_url.clone(), source_links, t.source_ids);

        Self {
            id: t.track_guid.clone().map(TrackRef::Musicindex),
            track_guid: t.track_guid,
            feed_guid: t.feed_guid,
            feed_title: nonempty_owned(t.feed_title.clone()),
            feed_url: nonempty_owned(t.feed_url),
            title: nonempty_owned(t.title).or_else(|| nonempty_owned(t.name)),
            artist: nonempty_owned(t.track_artist).or_else(|| nonempty_owned(t.release_artist)),
            album: nonempty_owned(t.feed_title),
            track_number: t.track_number,
            disc_number: None,
            duration_secs: t.duration_secs,
            pub_date: t.pub_date,
            explicit: t.explicit,
            description: nonempty_owned(t.description),
            image_url: image_url.clone(),
            artwork: artwork_from_url(&image_url),
            identity,
            audio_url: nonempty_owned(t.enclosure_url),
            mime: nonempty_owned(t.enclosure_type),
            bytes: t.enclosure_bytes,
            publisher_text: nonempty_owned(t.publisher_text),
            contributors: t
                .source_contributors
                .unwrap_or_default()
                .into_iter()
                .map(ContributorView::from)
                .collect(),
            payment_routes: t.payment_routes.unwrap_or_default(),
            transcript_url: nonempty_owned(transcript_url),
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
    fn from_api_feed_prefers_explicit_source_description_claim() {
        let feed = api::Feed {
            description: Some("Top-level description".into()),
            source_release_claims: Some(vec![api::SourceReleaseClaim {
                claim_type: Some("description".into()),
                claim_value: Some("Source fact description".into()),
                source: Some("rss_metadata".into()),
                extraction_path: Some("feed.description".into()),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let view = FeedView::from_api(feed);

        assert_eq!(view.description.as_deref(), Some("Source fact description"));
    }

    #[test]
    fn from_api_feed_preserves_top_level_feed_description() {
        let description = "All music by Emily Whitehurst and Jaycen McKissick. All lyrics by Emily Whitehurst.\nProduced by Emily Whitehurst, Jaycen McKissick and Paul Haile.";
        let feed = api::Feed {
            description: Some(description.into()),
            ..Default::default()
        };

        let view = FeedView::from_api(feed);

        assert_eq!(view.description.as_deref(), Some(description));
    }

    #[test]
    fn from_api_projection_drops_placeholder_source_text() {
        let feed = api::Feed {
            title: Some("<p>...</p><p>...</p>".into()),
            name: Some("Real feed".into()),
            feed_url: Some("&hellip;".into()),
            description: Some("<p>...</p>".into()),
            source_release_claims: Some(vec![
                api::SourceReleaseClaim {
                    claim_type: Some("description".into()),
                    claim_value: Some("&hellip;".into()),
                    ..Default::default()
                },
                api::SourceReleaseClaim {
                    claim_type: Some("description".into()),
                    claim_value: Some("Real source description".into()),
                    ..Default::default()
                },
            ]),
            tracks: Some(vec![api::Track {
                title: Some("<p>...</p>".into()),
                name: Some("Real track".into()),
                description: Some("&#8230;".into()),
                enclosure_url: Some("&nbsp;<br />...".into()),
                source_links: Some(vec![api::SourceEntityLink {
                    link_type: Some("transcript".into()),
                    url: Some("&hellip;".into()),
                    ..Default::default()
                }]),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let view = FeedView::from_api(feed);

        assert_eq!(view.title.as_deref(), Some("Real feed"));
        assert_eq!(view.feed_url, None);
        assert_eq!(view.description.as_deref(), Some("Real source description"));
        let track = view.tracks.first().expect("track projects");
        assert_eq!(track.title.as_deref(), Some("Real track"));
        assert_eq!(track.description, None);
        assert_eq!(track.audio_url, None);
        assert_eq!(track.transcript_url, None);
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
    fn from_local_feed_hydrates_language_for_release_summary_facts() {
        use crate::view_models::entity_detail::{EntitySurfaceContext, ReleaseDetailVm};

        let feed = db::FeedRow {
            id: 1,
            feed_url: "http://example.com".into(),
            language: Some(" en ".into()),
            ..Default::default()
        };

        let view = FeedView::from_local(feed, Vec::new());
        let facts = ReleaseDetailVm::new(&view, EntitySurfaceContext::Library).summary_facts();

        assert_eq!(view.language.as_deref(), Some("en"));
        assert!(facts
            .iter()
            .any(|fact| fact.key == "Language" && fact.value == "en"));
    }

    #[test]
    fn from_local_feed_filters_empty_language() {
        let feed = db::FeedRow {
            id: 1,
            feed_url: "http://example.com".into(),
            language: Some("   ".into()),
            ..Default::default()
        };

        let view = FeedView::from_local(feed, Vec::new());

        assert_eq!(view.language, None);
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

    #[test]
    fn artist_source_fact_maps_explicit_musicindex_artist() {
        let view = ArtistView::from_artist_source_fact(db::ArtistSourceFactRow {
            source: "musicindex".into(),
            source_artist_id: "artist-123".into(),
            name: Some("Alice".into()),
            sort_name: Some("Alice, The".into()),
            image_url: Some("https://example.test/alice.jpg".into()),
            website_url: Some("https://example.test/alice".into()),
            aliases: vec!["A. Example".into()],
            tags: vec!["podcast".into()],
            area: Some("Montreal".into()),
            begin_year: Some(2020),
            end_year: Some(2024),
            observed_at: Some(1_714_000_000),
            raw_json: Some("{\"artist_id\":\"artist-123\"}".into()),
            source_links: vec![db::LocalIdentityLinkRow {
                entity_type: Some("artist".into()),
                entity_id: Some("artist-123".into()),
                position: Some(0),
                link_type: Some("website".into()),
                url: Some("https://example.test/source-link".into()),
                source: "musicindex".into(),
                extraction_path: Some("$.source_links[0]".into()),
                observed_at: Some(1_714_000_001),
                raw_json: None,
            }],
            source_ids: vec![db::LocalIdentityIdRow {
                entity_type: Some("artist".into()),
                entity_id: Some("artist-123".into()),
                position: Some(0),
                scheme: Some("nostr_npub".into()),
                value: Some("npub1artist".into()),
                source: "musicindex".into(),
                extraction_path: Some("$.source_ids[0]".into()),
                observed_at: Some(1_714_000_002),
                raw_json: None,
            }],
        });

        assert!(matches!(
            view.id,
            Some(ArtistRef::Musicindex(ref id)) if id == "artist-123"
        ));
        assert_eq!(view.name.as_deref(), Some("Alice"));
        assert_eq!(view.sort_name.as_deref(), Some("Alice, The"));
        assert_eq!(
            view.image_url.as_deref(),
            Some("https://example.test/alice.jpg")
        );
        assert_eq!(
            view.artwork,
            Some(ArtworkRef::Url("https://example.test/alice.jpg".into()))
        );
        assert_eq!(view.url.as_deref(), Some("https://example.test/alice"));
        assert_eq!(
            view.identity.website_url.as_deref(),
            Some("https://example.test/source-link")
        );
        assert_eq!(view.identity.nostr_npub.as_deref(), Some("npub1artist"));
        assert_eq!(
            view.identity.image_url.as_deref(),
            Some("https://example.test/alice.jpg")
        );
        assert_eq!(view.identity.source_links.len(), 1);
        assert_eq!(view.identity.source_ids.len(), 1);
        assert_eq!(view.area.as_deref(), Some("Montreal"));
        assert_eq!(view.begin_year, Some(2020));
        assert_eq!(view.end_year, Some(2024));
        assert_eq!(view.feed_count, None);
        assert_eq!(view.track_count, None);
        assert_eq!(view.aliases, vec!["A. Example"]);
        assert_eq!(view.tags, vec!["podcast"]);
    }

    #[test]
    fn artist_source_fact_drops_out_of_range_years() {
        let view = ArtistView::from_artist_source_fact(db::ArtistSourceFactRow {
            source: "musicindex".into(),
            source_artist_id: "artist-123".into(),
            begin_year: Some(i64::from(i32::MAX) + 1),
            end_year: Some(i64::from(i32::MIN) - 1),
            name: Some("Alice".into()),
            sort_name: None,
            image_url: None,
            website_url: None,
            aliases: Vec::new(),
            tags: Vec::new(),
            area: None,
            observed_at: None,
            raw_json: None,
            source_links: Vec::new(),
            source_ids: Vec::new(),
        });

        assert_eq!(view.begin_year, None);
        assert_eq!(view.end_year, None);
    }
}
