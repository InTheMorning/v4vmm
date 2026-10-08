use crate::api;
use crate::application::queries::stored_values::{self, FeedStoredValues, TrackStoredValues};
use crate::db;
use crate::metadata::drop_placeholder_source_text;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtistRef {
    LocalArtistName(String),
    /// A publisher feed GUID that identifies an artist or label page (ADR
    /// 0077 Decision 1). No code builds this value from name text or from
    /// `publisher_text`.
    PublisherFeed(String),
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
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FeedMetadataFacts {
    pub publisher_text: Option<String>,
    pub release_kind: Option<String>,
    pub release_date: Option<i64>,
    pub language: Option<String>,
    pub explicit: Option<bool>,
    pub description: Option<String>,
    /// The RSS channel `pubDate`, read from the `rss` source bucket (ADR
    /// 0075 packet 050). `None` for the `musicindex` bucket: no MusicIndex
    /// response states this value.
    pub channel_pub_date: Option<i64>,
    /// The raw value of a `release_date` claim with the path
    /// `feed.pub_date`, read from the `musicindex` source bucket (ADR 0075
    /// packet 050, operator decision D50-1). `None` for the `rss` bucket.
    pub pub_date_claim: Option<String>,
}

impl FeedMetadataFacts {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.publisher_text.is_none()
            && self.release_kind.is_none()
            && self.release_date.is_none()
            && self.language.is_none()
            && self.explicit.is_none()
            && self.description.is_none()
    }
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
    /// The oldest-item date that MusicIndex computes (ADR 0075 Decision I).
    /// The album page shows this as the derived fact "First track
    /// published", named with MusicIndex as its source; it is never a
    /// release date (ADR 0075 packet 050, operator decision D50-2).
    pub release_date: Option<i64>,
    /// The feed's own publication date, already formatted (ADR 0075 packet
    /// 050, operator decision D50-1): the fresh RSS channel `pubDate`, or a
    /// MusicIndex `release_date` claim with the path `feed.pub_date`.
    /// `None` when neither source has a value.
    pub published: Option<String>,
    /// The provider that supplied [`Self::published`]: `"RSS"` or
    /// `"MusicIndex"`. `None` when `published` is `None`.
    pub published_source: Option<&'static str>,
    pub language: Option<String>,
    pub explicit: Option<bool>,
    pub episode_count: Option<i32>,
    pub release_kind: Option<String>,
    pub publisher_text: Option<String>,
    pub description: Option<String>,
    pub payment_routes: Vec<api::PaymentRoute>,
    pub contributors: Vec<ContributorView>,
    pub tracks: Vec<TrackView>,
    /// The publisher feed GUID that this album names, when a stored or
    /// received relationship states `music_names_publisher = true` (ADR
    /// 0077 Decision 2, packet 004). `None` when the album names no
    /// publisher. Never built from name text or `publisher_text`.
    pub publisher_feed_guid: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TrackView {
    pub id: Option<TrackRef>,
    pub track_guid: Option<String>,
    pub feed_guid: Option<String>,
    pub feed_title: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// The album artist. A local track reads `feeds.album_artist`, a channel
    /// value (ADR 0075 packet 020).
    pub album_artist: Option<String>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub duration_secs: Option<i32>,
    pub pub_date: Option<i64>,
    pub explicit: Option<bool>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    /// The track's own image, when the source states an owner (ADR 0075
    /// Decision C, packet 048). This value is `None` when the track has
    /// no image of its own. It is also `None` when the source states no
    /// owner for either image field. An Index track reads this value
    /// from `Track::track_image_url`. A local track leaves this value
    /// `None`. The Library route picks one image for `image_url` (ADR
    /// 0076 Decision 1).
    pub track_image_url: Option<String>,
    /// The feed's image that the source states with this track (ADR
    /// 0075 Decision C, packet 048). An Index track reads this value
    /// from `Track::feed_image_url`. The track response carries this
    /// value. A separate feed read does not supply it.
    pub feed_image_url: Option<String>,
    pub artwork: Option<ArtworkRef>,
    pub identity: EntityIdentityLinks,
    pub audio_url: Option<String>,
    pub mime: Option<String>,
    pub bytes: Option<i64>,
    pub publisher_text: Option<String>,
    /// The publisher feed GUID that the track response names, when a
    /// `music_to_publisher` relationship states `music_names_publisher =
    /// true` (ADR 0077 Decision 2). Never built from name text. A Library
    /// track leaves this value `None`, and its page reads the album feed.
    pub publisher_feed_guid: Option<String>,
    pub contributors: Vec<ContributorView>,
    pub payment_routes: Vec<api::PaymentRoute>,
    pub transcript_url: Option<String>,
}

impl TrackView {
    /// The image to show for this track (ADR 0075 Decision C). This
    /// method shows the track's own image first. It shows the feed
    /// image second. When the source states no owner for either field,
    /// it shows `image_url`. That value has unknown ownership (the
    /// accepted legacy-artwork rule). This method never reports it as a
    /// track-owned image.
    #[must_use]
    pub fn display_artwork_url(&self) -> Option<&str> {
        trimmed(self.track_image_url.as_deref())
            .or_else(|| trimmed(self.feed_image_url.as_deref()))
            .or_else(|| trimmed(self.image_url.as_deref()))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TrackMetadataFacts {
    pub publisher_text: Option<String>,
    pub description: Option<String>,
    pub pub_date: Option<i64>,
    pub explicit: Option<bool>,
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
        // ADR 0075: a view model carries no provenance, so these fields stay unknown.
        Self {
            name: contributor.name,
            role: contributor.role,
            href: contributor.href,
            img: contributor.image_url,
            npub: contributor.nostr_npub,
            group_name: contributor.group_name,
            entity_type: None,
            entity_id: None,
            position: None,
            role_norm: None,
            source: None,
            extraction_path: None,
            observed_at: None,
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

fn nonempty_owned(value: Option<String>) -> Option<String> {
    let value = value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    drop_placeholder_source_text(value)
}

/// A trimmed, non-empty borrow of `value`, or `None`.
/// `TrackView::display_artwork_url` uses this. A blank value must not
/// hide a real fallback value.
fn trimmed(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

impl ArtistView {
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
        // ADR 0075 packet 050, operator decision D50-1: the feed's own
        // publication date, read before the fields below move `f`.
        let published = crate::metadata::feed_publication_pubdate(&f);
        let image_url = nonempty_owned(f.image_url);
        let source_description =
            description_from_release_claims(f.source_release_claims.as_deref());
        let identity =
            EntityIdentityLinks::from_api_facts(image_url.clone(), f.source_links, f.source_ids);
        let publisher_feed_guid =
            owned_publisher_feed_guid_from_relationships(f.publisher.as_deref());
        Self {
            id: f.feed_guid.clone().map(FeedRef::Musicindex),
            feed_guid: f.feed_guid,
            feed_url: nonempty_owned(f.feed_url),
            title: nonempty_owned(f.title),
            artist: nonempty_owned(f.release_artist),
            image_url: image_url.clone(),
            artwork: artwork_from_url(&image_url),
            identity,
            release_date: f.release_date,
            published: published.as_ref().map(|(date, _)| date.clone()),
            published_source: published.as_ref().map(|(_, source)| *source),
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
            publisher_feed_guid,
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
        let values = stored_values::feed_values_from_columns(&f);
        Self::from_local_with_facts(f, tracks, facts, values)
    }

    /// A local feed view. Each metadata value comes from the stored value
    /// projection, which owns the order of hold, `MusicIndex` fact and column
    /// (ADR 0076 Decision 1). This view selects no source. The credit list
    /// is the projected list of the channel (ADR 0076 packet 006).
    pub fn from_local_with_facts(
        f: db::FeedRow,
        tracks: Vec<TrackView>,
        facts: LocalIdentityFacts,
        values: FeedStoredValues,
    ) -> Self {
        // The album artist is a channel value. A feed without one shows the
        // artist of its first track.
        let artist = values
            .album_artist
            .value
            .or_else(|| tracks.first().and_then(|t| t.artist.clone()));
        let image_url = values.artwork.value;
        let identity = EntityIdentityLinks::from_source_facts(
            image_url.clone(),
            facts.source_links,
            facts.source_ids,
        );
        // ADR 0075 packet 050, operator decision D50-1: the feed's own
        // publication date, built from its two stored facts.
        let published = crate::metadata::feed_publication_date_from_parts(
            values.channel_pub_date.value,
            values.publication_claim.value.as_deref(),
        );

        Self {
            id: Some(FeedRef::LocalFeedId(f.id)),
            feed_guid: f.feed_guid,
            feed_url: Some(f.feed_url),
            title: values.title.value,
            artist,
            image_url: image_url.clone(),
            artwork: artwork_from_url(&image_url),
            identity,
            release_date: values.release_date.value,
            published: published.as_ref().map(|(date, _)| date.clone()),
            published_source: published.as_ref().map(|(_, source)| *source),
            language: values.language.value,
            explicit: values.explicit.value,
            episode_count: Some(tracks.len() as i32),
            release_kind: nonempty_owned(values.release_kind.value),
            publisher_text: nonempty_owned(values.owner_name.value),
            description: values.description.value,
            payment_routes: Vec::new(),
            contributors: values.credits,
            tracks,
            // A local read has no request in hand. The caller sets this
            // from its own stored relationship read when the screen needs
            // the "open publisher" action (ADR 0077 packet 004).
            publisher_feed_guid: None,
        }
    }
}

/// The publisher feed GUID of the received `music_to_publisher` entry that
/// states `music_names_publisher = true` (ADR 0077 Decision 2, packet 004).
/// `None` when the response names no relationship entries, or when no entry
/// names this album as owned by a publisher. Never reads `publisher_text`.
fn owned_publisher_feed_guid_from_relationships(
    relationships: Option<&[api::PublisherRelationship]>,
) -> Option<String> {
    relationships?
        .iter()
        .find(|entry| {
            entry.direction.as_deref() == Some("music_to_publisher")
                && entry.music_names_publisher == Some(true)
        })
        .and_then(|entry| {
            entry
                .publisher_feed_guid
                .clone()
                .or_else(|| entry.remote_feed_guid.clone())
        })
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
        let track_image_url = nonempty_owned(t.track_image_url);
        let feed_image_url = nonempty_owned(t.feed_image_url);
        let identity =
            EntityIdentityLinks::from_api_facts(image_url.clone(), source_links, t.source_ids);

        Self {
            id: t.track_guid.clone().map(TrackRef::Musicindex),
            track_guid: t.track_guid,
            feed_guid: t.feed_guid,
            feed_title: nonempty_owned(t.feed_title.clone()),
            title: nonempty_owned(t.title),
            artist: nonempty_owned(t.track_artist)
                .or_else(|| nonempty_owned(t.release_artist.clone())),
            album: nonempty_owned(t.feed_title),
            album_artist: nonempty_owned(t.release_artist),
            track_number: t.track_number,
            disc_number: None,
            duration_secs: t.duration_secs,
            pub_date: t.pub_date,
            explicit: t.explicit,
            description: nonempty_owned(t.description),
            image_url: image_url.clone(),
            track_image_url,
            feed_image_url,
            artwork: artwork_from_url(&image_url),
            identity,
            audio_url: nonempty_owned(t.enclosure_url),
            mime: nonempty_owned(t.enclosure_type),
            bytes: t.enclosure_bytes,
            publisher_feed_guid: owned_publisher_feed_guid_from_relationships(
                t.publisher.as_deref(),
            ),
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
        let values = stored_values::track_values_from_columns(&t);
        Self::from_local_with_facts(t, facts, values)
    }

    /// A local track view. Each metadata value comes from the stored value
    /// projection, which owns the order of hold, `MusicIndex` fact and column
    /// (ADR 0076 Decision 1). This view selects no source. Audio URL, type,
    /// duration and numbers have no fact, and they come from the row. The
    /// credit list is the projected list of the item (ADR 0076 packet 006).
    pub fn from_local_with_facts(
        t: db::TrackRow,
        facts: LocalIdentityFacts,
        values: TrackStoredValues,
    ) -> Self {
        let image_url = values.artwork.value;
        let identity = EntityIdentityLinks::from_source_facts(
            image_url.clone(),
            facts.source_links,
            facts.source_ids,
        );
        let album_artist = values.album_artist.value;
        Self {
            id: Some(TrackRef::LocalTrackId(t.id)),
            track_guid: Some(t.item_guid),
            feed_guid: t.feed_guid,
            feed_title: t.feed_title,
            title: values.title.value,
            // A track without its own artist shows the album artist.
            artist: values.artist.value.or_else(|| album_artist.clone()),
            album: values.album_title.value,
            album_artist,
            track_number: t.track_number.and_then(|v| v.try_into().ok()),
            disc_number: t.disc_number.and_then(|v| v.try_into().ok()),
            duration_secs: t.duration_seconds.and_then(|v| v.try_into().ok()),
            pub_date: values.pub_date.value,
            explicit: values.explicit.value,
            description: nonempty_owned(values.description.value),
            image_url: image_url.clone(),
            // The Library route picks one image for `image_url` and
            // keeps no separate track/feed owner facts (ADR 0076
            // Decision 1). This packet does not change the Library
            // route.
            track_image_url: None,
            feed_image_url: None,
            artwork: artwork_from_url(&image_url),
            identity,
            audio_url: t.enclosure_url,
            mime: t.enclosure_type,
            bytes: None,
            // A Library track page reads the publisher of its album feed.
            publisher_feed_guid: None,
            publisher_text: nonempty_owned(values.publisher_text.value),
            contributors: values.credits,
            payment_routes: Vec::new(),
            transcript_url: t.transcript_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R52-04 (ADR 0075 packet 052): a full track response gives the
    /// payment routes, the website, the credits and the publisher feed GUID
    /// of an Index track.
    #[test]
    fn adr_0075_r52_04_full_track_response_fills_the_index_track_view() {
        let view = TrackView::from_api(api::Track {
            payment_routes: Some(vec![api::PaymentRoute {
                recipient_name: Some("Music Side Project".into()),
                split: Some(1.0),
                ..Default::default()
            }]),
            source_links: Some(vec![api::SourceEntityLink {
                link_type: Some("website".into()),
                url: Some("https://example.test/track".into()),
                ..Default::default()
            }]),
            source_contributors: Some(vec![api::Contributor {
                name: Some("Amy".into()),
                role: Some("Guitar".into()),
                ..Default::default()
            }]),
            publisher: Some(vec![api::PublisherRelationship {
                direction: Some("music_to_publisher".into()),
                publisher_feed_guid: Some("publisher-guid".into()),
                music_names_publisher: Some(true),
                ..Default::default()
            }]),
            ..Default::default()
        });

        assert_eq!(view.payment_routes.len(), 1);
        assert_eq!(
            view.identity.website_url.as_deref(),
            Some("https://example.test/track")
        );
        assert_eq!(view.contributors[0].name.as_deref(), Some("Amy"));
        assert_eq!(view.publisher_feed_guid.as_deref(), Some("publisher-guid"));
    }

    /// ADR 0083 task 004: an Index track keeps the publisher feed GUID that
    /// its response names, so its page can link the publisher. A
    /// relationship that does not name the publisher gives no GUID.
    #[test]
    fn from_api_track_keeps_the_named_publisher_feed_guid() {
        let relationship = |names: bool| api::PublisherRelationship {
            direction: Some("music_to_publisher".into()),
            publisher_feed_guid: Some("publisher-guid".into()),
            music_names_publisher: Some(names),
            ..Default::default()
        };
        let named = TrackView::from_api(api::Track {
            publisher: Some(vec![relationship(true)]),
            ..Default::default()
        });
        assert_eq!(named.publisher_feed_guid.as_deref(), Some("publisher-guid"));

        let unnamed = TrackView::from_api(api::Track {
            publisher: Some(vec![relationship(false)]),
            ..Default::default()
        });
        assert_eq!(unnamed.publisher_feed_guid, None);
    }

    /// ADR 0075 packet 050, operator decision D50-1: the Index route
    /// resolves the feed's own publication date directly from the decoded
    /// `Feed`, preferring the RSS channel `pubDate` over the MusicIndex
    /// claim.
    #[test]
    fn from_api_resolves_published_date_prefers_channel_pub_date() {
        let feed = api::Feed {
            channel_pub_date: Some(1_789_905_600),
            source_release_claims: Some(vec![api::SourceReleaseClaim {
                claim_type: Some("release_date".into()),
                claim_value: Some("1704067200".into()),
                extraction_path: Some("feed.pub_date".into()),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let view = FeedView::from_api(feed);

        assert_eq!(view.published.as_deref(), Some("Sep 20, 2026"));
        assert_eq!(view.published_source, Some("RSS"));
    }

    /// R48-02 (ADR 0075 packet 048): a recorded track with its own image
    /// exposes it as the track image. `display_artwork_url` shows it.
    #[test]
    fn adr_0075_track_artwork_r48_02_track_image_field_is_shown() {
        let track: api::Track = serde_json::from_str(
            r#"{
                "track_guid": "track-1",
                "image_url": "https://example.test/track.jpg",
                "track_image_url": "https://example.test/track.jpg",
                "feed_image_url": "https://example.test/feed.jpg"
            }"#,
        )
        .expect("a recorded track with a track image should decode");

        let view = TrackView::from_api(track);

        assert_eq!(
            view.track_image_url.as_deref(),
            Some("https://example.test/track.jpg")
        );
        assert_eq!(
            view.display_artwork_url(),
            Some("https://example.test/track.jpg")
        );
    }

    /// R48-03 (ADR 0075 packet 048): a recorded track with a null track
    /// image and a feed image exposes no track image.
    /// `display_artwork_url` shows the feed image (ADR 0075 Decision C).
    #[test]
    fn adr_0075_track_artwork_r48_03_null_track_image_shows_feed_image() {
        let track: api::Track = serde_json::from_str(
            r#"{
                "track_guid": "track-1",
                "image_url": "https://example.test/feed.jpg",
                "track_image_url": null,
                "feed_image_url": "https://example.test/feed.jpg"
            }"#,
        )
        .expect("a recorded track with a null track image should decode");

        let view = TrackView::from_api(track);

        assert_eq!(view.track_image_url, None);
        assert_eq!(
            view.display_artwork_url(),
            Some("https://example.test/feed.jpg")
        );
    }

    /// R48-04 (ADR 0075 packet 048): a recorded track with only
    /// `image_url` states no owner. The view keeps that image for
    /// display, and it never reports it as the track's own image (the
    /// accepted legacy-artwork rule).
    #[test]
    fn adr_0075_track_artwork_r48_04_legacy_image_has_unknown_ownership() {
        let track: api::Track = serde_json::from_str(
            r#"{
                "track_guid": "track-1",
                "image_url": "https://example.test/legacy.jpg"
            }"#,
        )
        .expect("a recorded track with only image_url should decode");

        let view = TrackView::from_api(track);

        assert_eq!(view.track_image_url, None);
        assert_eq!(view.feed_image_url, None);
        assert_eq!(
            view.display_artwork_url(),
            Some("https://example.test/legacy.jpg")
        );
    }

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

        assert_eq!(view.title, None);
        assert_eq!(view.feed_url, None);
        assert_eq!(view.description.as_deref(), Some("Real source description"));
        let track = view.tracks.first().expect("track projects");
        assert_eq!(track.title, None);
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
                ..Default::default()
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
            pub_date: Some(1_712_275_200),
            explicit: Some(true),
            ..Default::default()
        };

        let view = TrackView::from_local(track);

        assert!(matches!(view.id, Some(TrackRef::LocalTrackId(42))));
        assert_eq!(view.duration_secs, Some(180));
        assert_eq!(view.pub_date, Some(1_712_275_200));
        assert_eq!(view.explicit, Some(true));
        assert_eq!(view.audio_url, Some("http://a/b.mp3".into()));
    }

    #[test]
    fn from_local_track_shows_projected_stored_values() {
        use crate::application::queries::stored_values::{Owned, ValueOwner};
        let item = |value: &str| Owned {
            value: Some(value.to_owned()),
            owner: ValueOwner::Item,
        };
        let track = db::TrackRow {
            id: 42,
            item_guid: "g".into(),
            track_title: Some("Column title".into()),
            pub_date: Some(1),
            explicit: Some(false),
            ..Default::default()
        };
        let values = TrackStoredValues {
            title: item("Projected title"),
            album_artist: Owned {
                value: Some("Channel artist".into()),
                owner: ValueOwner::Channel,
            },
            publisher_text: item("Example Publisher"),
            description: item("Track description"),
            pub_date: Owned {
                value: Some(1_712_275_200),
                owner: ValueOwner::Item,
            },
            explicit: Owned {
                value: Some(true),
                owner: ValueOwner::Item,
            },
            ..TrackStoredValues::default()
        };

        let view = TrackView::from_local_with_facts(track, LocalIdentityFacts::default(), values);

        assert_eq!(view.title.as_deref(), Some("Projected title"));
        assert_eq!(view.album_artist.as_deref(), Some("Channel artist"));
        assert_eq!(view.artist.as_deref(), Some("Channel artist"));
        assert_eq!(view.publisher_text.as_deref(), Some("Example Publisher"));
        assert_eq!(view.description.as_deref(), Some("Track description"));
        assert_eq!(view.pub_date, Some(1_712_275_200));
        assert_eq!(view.explicit, Some(true));
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
        };
        // ADR 0076 packet 006: the credit list comes from the projection.
        let values = TrackStoredValues {
            artwork: stored_values::Owned {
                value: track.track_image_href.clone(),
                owner: stored_values::ValueOwner::Item,
            },
            credits: vec![ContributorView {
                name: Some("Alice".into()),
                href: Some("https://example.test/alice".into()),
                image_url: Some("https://example.test/alice.jpg".into()),
                nostr_npub: Some("npub1alice".into()),
                ..ContributorView::default()
            }],
            ..TrackStoredValues::default()
        };

        let view = TrackView::from_local_with_facts(track, facts, values);

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
    fn from_local_feed_shows_projected_stored_values() {
        use crate::application::queries::stored_values::{Owned, ValueOwner};
        let channel = |value: &str| Owned {
            value: Some(value.to_owned()),
            owner: ValueOwner::Channel,
        };
        let feed = db::FeedRow {
            id: 1,
            feed_url: "http://example.com".into(),
            language: Some("scalar-language".into()),
            description: Some("Scalar description".into()),
            ..Default::default()
        };
        let values = FeedStoredValues {
            owner_name: channel("Example Publisher"),
            release_kind: channel("album"),
            release_date: Owned {
                value: Some(1_700_000_000),
                owner: ValueOwner::Channel,
            },
            language: channel("en"),
            explicit: Owned {
                value: Some(true),
                owner: ValueOwner::Channel,
            },
            description: channel("Fact description"),
            album_artist: channel("Channel artist"),
            ..FeedStoredValues::default()
        };

        let view = FeedView::from_local_with_facts(
            feed,
            Vec::new(),
            LocalIdentityFacts::default(),
            values,
        );

        assert_eq!(view.publisher_text.as_deref(), Some("Example Publisher"));
        assert_eq!(view.release_kind.as_deref(), Some("album"));
        assert_eq!(view.release_date, Some(1_700_000_000));
        assert_eq!(view.language.as_deref(), Some("en"));
        assert_eq!(view.explicit, Some(true));
        assert_eq!(view.description.as_deref(), Some("Fact description"));
        assert_eq!(view.artist.as_deref(), Some("Channel artist"));
    }

    /// ADR 0075 packet 050, operator decision D50-1: the Library route
    /// resolves the feed's own publication date from its two stored facts,
    /// preferring the RSS channel `pubDate` over the MusicIndex claim.
    #[test]
    fn from_local_feed_resolves_published_date_from_stored_facts() {
        use crate::application::queries::stored_values::{Owned, ValueOwner};
        let feed = db::FeedRow {
            id: 1,
            feed_url: "http://example.com".into(),
            ..Default::default()
        };
        let values = FeedStoredValues {
            channel_pub_date: Owned {
                value: Some(1_789_905_600),
                owner: ValueOwner::Channel,
            },
            publication_claim: Owned {
                value: Some("1704067200".to_owned()),
                owner: ValueOwner::Channel,
            },
            ..FeedStoredValues::default()
        };

        let view = FeedView::from_local_with_facts(
            feed.clone(),
            Vec::new(),
            LocalIdentityFacts::default(),
            values,
        );
        assert_eq!(view.published.as_deref(), Some("Sep 20, 2026"));
        assert_eq!(view.published_source, Some("RSS"));

        let claim_only = FeedStoredValues {
            publication_claim: Owned {
                value: Some("1704067200".to_owned()),
                owner: ValueOwner::Channel,
            },
            ..FeedStoredValues::default()
        };
        let view = FeedView::from_local_with_facts(
            feed,
            Vec::new(),
            LocalIdentityFacts::default(),
            claim_only,
        );
        assert_eq!(view.published.as_deref(), Some("Jan 1, 2024"));
        assert_eq!(view.published_source, Some("MusicIndex"));
    }

    #[test]
    fn from_local_feed_preserves_scalar_language_and_description_fallbacks() {
        let feed = db::FeedRow {
            id: 1,
            feed_url: "http://example.com".into(),
            language: Some(" en ".into()),
            description: Some("Scalar description".into()),
            ..Default::default()
        };

        let view =
            FeedView::from_local_with_identity(feed, Vec::new(), LocalIdentityFacts::default());

        assert_eq!(view.language.as_deref(), Some("en"));
        assert_eq!(view.description.as_deref(), Some("Scalar description"));
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
        };
        // ADR 0076 packet 006: the credit list comes from the projection.
        let values = FeedStoredValues {
            credits: vec![ContributorView {
                name: Some("Bob".into()),
                href: Some("https://example.test/bob".into()),
                image_url: Some("https://example.test/bob.jpg".into()),
                nostr_npub: Some("npub1bob".into()),
                ..ContributorView::default()
            }],
            ..stored_values::feed_values_from_columns(&feed)
        };

        let view = FeedView::from_local_with_facts(feed, Vec::new(), facts, values);

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
