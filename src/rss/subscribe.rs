//! RSS subscribe: fetch, parse and persist one feed.
//!
//! ADR 0076 packet 002 splits the command into a parse step and a persist
//! step. The playlist RSS check (`check_apply.rs`) uses the same parse step
//! and the same column mapping, so both write equal column values from one
//! document.

use anyhow::{Context, Result};
use rss::extension::{Extension, ExtensionMap};
use rss::Channel;
use rusqlite::Connection;
use std::io::Cursor;

use super::helpers::*;
use crate::api::Client as MusicIndexClient;
use crate::db;

/// One parsed RSS document: the channel values and one item for each RSS
/// item with a GUID (ADR 0076 packet 002).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ParsedFeedDocument {
    pub(crate) channel: ParsedChannel,
    pub(crate) items: Vec<ParsedItem>,
}

/// The channel values of one document. Column values have the form that
/// the `feeds` columns store.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ParsedChannel {
    pub(crate) feed_guid: Option<String>,
    pub(crate) title: String,
    pub(crate) link: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) podcast_medium: Option<String>,
    pub(crate) album_image_href: Option<String>,
    pub(crate) album_image_mime: Option<String>,
    pub(crate) people_json: Option<String>,
    pub(crate) podcast_value_json: Option<String>,
    /// Channel `itunes:author`, then a channel person with an artist role.
    pub(crate) album_artist: Option<String>,
    /// The `itunes:owner` name.
    pub(crate) owner_name: Option<String>,
    /// The channel `itunes:explicit` text.
    pub(crate) explicit: Option<String>,
    pub(crate) contributors: Vec<db::LocalContributorInput>,
    /// Valid Nostr identities of direct `podcast:txt purpose="npub"` elements.
    pub(crate) nostr_ids: Vec<ParsedNostrId>,
    /// The `podcast:publisher` remote item of the album.
    pub(crate) publisher: Option<ParsedPublisher>,
}

/// One item of a document. Column values have the form that the `tracks`
/// columns store.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ParsedItem {
    pub(crate) item_guid: String,
    pub(crate) enclosure_url: Option<String>,
    pub(crate) enclosure_type: Option<String>,
    pub(crate) link: Option<String>,
    pub(crate) pub_date: Option<String>,
    pub(crate) track_title: Option<String>,
    pub(crate) artist_name: Option<String>,
    pub(crate) album_title: Option<String>,
    pub(crate) album_artist_name: Option<String>,
    pub(crate) disc_number: Option<i64>,
    pub(crate) track_number: Option<i64>,
    pub(crate) duration_seconds: Option<i64>,
    pub(crate) itunes_duration_raw: Option<String>,
    pub(crate) itunes_explicit: Option<String>,
    pub(crate) track_image_href: Option<String>,
    pub(crate) track_image_mime: Option<String>,
    pub(crate) people_json: Option<String>,
    pub(crate) item_value_json: Option<String>,
    pub(crate) extra_json: String,
    pub(crate) description: Option<String>,
    pub(crate) contributors: Vec<db::LocalContributorInput>,
    pub(crate) links: Vec<db::LocalIdentityLinkInput>,
    pub(crate) nostr_ids: Vec<ParsedNostrId>,
}

/// One valid Nostr identity of a `podcast:txt purpose="npub"` element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedNostrId {
    /// `nostr_npub` or `nostr_nprofile`, as the identity validation names it.
    pub(crate) scheme: String,
    pub(crate) value: String,
    pub(crate) position: i64,
}

/// The `podcast:remoteItem` of the channel `podcast:publisher` element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedPublisher {
    pub(crate) feed_guid: Option<String>,
    pub(crate) feed_url: Option<String>,
}

/// Parse one RSS document. The subscribe command and the playlist RSS
/// check call this one function.
pub(crate) fn parse_feed_document(body: &[u8]) -> Result<ParsedFeedDocument> {
    let feed = Channel::read_from(Cursor::new(body)).context("parse RSS")?;

    // --- feed-level fields (RSS channel + podcast extensions) ---
    let feed_title = feed.title().to_string();
    let feed_artist = feed
        .itunes_ext()
        .and_then(|it| clean_text(it.author()))
        .or_else(|| first_person_by_role(feed.extensions(), &["artist", "creator", "composer"]));

    let feed_link = {
        let l = feed.link().trim();
        if l.is_empty() {
            None
        } else {
            Some(l.to_string())
        }
    };

    let desc = feed.description().trim();
    let description = if desc.is_empty() {
        None
    } else {
        Some(desc.to_string())
    };

    // Album image (prefer podcast:image/@href; fall back to itunes channel image; then <image><url>)
    let mut album_image_href = find_ext_attr(feed.extensions(), "podcast", "image", "href")
        .or_else(|| {
            feed.itunes_ext()
                .and_then(|it| it.image())
                .map(|s| s.to_string())
        });
    if album_image_href.is_none() {
        if let Some(img) = feed.image() {
            album_image_href = Some(img.url().to_string());
        }
    }

    let channel = ParsedChannel {
        // Podcasting 2.0 extensions: rss crate stores keys without prefix (guid, medium, value, ...)
        feed_guid: find_ext_text(feed.extensions(), "podcast", "guid"),
        title: feed_title.clone(),
        link: feed_link,
        language: feed.language().map(|s| s.to_string()),
        description,
        podcast_medium: find_ext_text(feed.extensions(), "podcast", "medium"),
        album_image_href,
        album_image_mime: None,
        // People at feed level (podcast:person); ok if None
        people_json: collect_people_json(feed.extensions()),
        // Full value block (including recipients) as JSON
        podcast_value_json: value_block_json(feed.extensions(), "podcast", "value"),
        album_artist: feed_artist.clone(),
        owner_name: feed
            .itunes_ext()
            .and_then(|it| it.owner())
            .and_then(|owner| clean_text(owner.name())),
        explicit: feed
            .itunes_ext()
            .and_then(|it| it.explicit())
            .map(ToOwned::to_owned),
        contributors: contributor_inputs_from_extensions(feed.extensions()),
        nostr_ids: nostr_ids_from_extensions(feed.extensions()),
        publisher: publisher_from_extensions(feed.extensions()),
    };

    let mut items = Vec::new();
    for item in feed.items() {
        // Stable identity: item <guid>. If missing, skip (we need stable IDs).
        let item_guid = match item.guid() {
            Some(g) => g.value().to_string(),
            None => continue,
        };

        let (enclosure_url, enclosure_type) = selected_enclosure(item);
        let item_link = item.link().map(|s| s.to_string());

        // Provisional music fields (ID3 will become canonical once downloaded)
        let artist_name = item
            .itunes_ext()
            .and_then(|it| clean_text(it.author()))
            .or_else(|| clean_text(item.author()))
            .or_else(|| {
                first_person_by_role(
                    item.extensions(),
                    &["artist", "creator", "composer", "performer"],
                )
            })
            .or_else(|| feed_artist.clone());

        // iTunes item tags are NOT in extensions; rss crate exposes them via itunes_ext()
        let itunes = item.itunes_ext();
        let itunes_duration_raw = itunes.and_then(|it| it.duration()).map(|s| s.to_string());

        // Item-level people/value/transcript (podcast:* extensions)
        let transcript_url = find_ext_attr(item.extensions(), "podcast", "transcript", "url");
        let transcript_type = find_ext_attr(item.extensions(), "podcast", "transcript", "type");

        items.push(ParsedItem {
            enclosure_url,
            enclosure_type,
            link: item_link.clone(),
            pub_date: item.pub_date().map(|s| s.to_string()),
            track_title: item.title().map(|s| s.to_string()),
            album_title: Some(feed_title.clone()),
            album_artist_name: feed_artist.clone().or_else(|| artist_name.clone()),
            artist_name,
            disc_number: None,
            // Canonical ordering: podcast:episode
            track_number: find_ext_text(item.extensions(), "podcast", "episode")
                .and_then(|s| s.trim().parse::<i64>().ok()),
            duration_seconds: itunes_duration_raw
                .as_deref()
                .and_then(parse_itunes_duration),
            itunes_duration_raw,
            itunes_explicit: itunes.and_then(|it| it.explicit()).map(|s| s.to_string()),
            track_image_href: itunes.and_then(|it| it.image()).map(|s| s.to_string()),
            track_image_mime: None,
            people_json: collect_people_json(item.extensions()),
            item_value_json: value_block_json(item.extensions(), "podcast", "value"),
            extra_json: track_extra_json(transcript_url.as_deref(), transcript_type.as_deref()),
            description: item.description().map(ToOwned::to_owned),
            contributors: contributor_inputs_from_extensions(item.extensions()),
            links: rss_track_link_inputs(
                &item_guid,
                item_link.as_deref(),
                transcript_url.as_deref(),
                transcript_type.as_deref(),
            ),
            nostr_ids: nostr_ids_from_extensions(item.extensions()),
            item_guid,
        });
    }

    Ok(ParsedFeedDocument { channel, items })
}

/// ADR 0075 Decision E: the first supported primary enclosure, then the
/// first supported enclosure. The direct `<enclosure>` and each
/// `podcast:alternateEnclosure` with `default="true"` are primary. When no
/// candidate is supported, the direct enclosure stays the stored value, as
/// before this packet.
fn selected_enclosure(item: &rss::Item) -> (Option<String>, Option<String>) {
    let direct = item.enclosure().map(|enclosure| {
        let mime = enclosure.mime_type().trim();
        (
            enclosure.url().to_string(),
            (!mime.is_empty()).then(|| mime.to_string()),
            true,
        )
    });
    let alternates = item
        .extensions()
        .get("podcast")
        .and_then(|podcast| podcast.get("alternateEnclosure"))
        .map(|alternates| {
            alternates
                .iter()
                .filter_map(|alternate| {
                    let url = alternate
                        .children
                        .get("source")?
                        .iter()
                        .find_map(|source| clean_attr(source, "uri"))?;
                    let primary = alternate
                        .attrs
                        .get("default")
                        .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"));
                    Some((url, clean_attr(alternate, "type"), primary))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let candidates = direct.iter().cloned().chain(alternates).collect::<Vec<_>>();
    let supported = |(url, mime, _): &&(String, Option<String>, bool)| {
        crate::track_compare::enclosure_supported(mime.as_deref(), url)
    };
    candidates
        .iter()
        .filter(|candidate| candidate.2)
        .find(supported)
        .or_else(|| candidates.iter().find(supported))
        .or(direct.as_ref())
        .map_or((None, None), |(url, mime, _)| {
            (Some(url.clone()), mime.clone())
        })
}

/// Direct `podcast:txt purpose="npub"` elements with a valid identity.
fn nostr_ids_from_extensions(exts: &ExtensionMap) -> Vec<ParsedNostrId> {
    let Some(texts) = exts.get("podcast").and_then(|podcast| podcast.get("txt")) else {
        return Vec::new();
    };
    texts
        .iter()
        .enumerate()
        .filter(|(_, text)| {
            text.attrs
                .get("purpose")
                .is_some_and(|purpose| purpose.trim() == "npub")
        })
        .filter_map(|(position, text)| {
            let value = clean_text(text.value.as_deref())?;
            match super::validate_nostr_identity(&value) {
                super::IdentityValidation::Valid(identity) => Some(ParsedNostrId {
                    scheme: identity.scheme().to_owned(),
                    value: identity.original().to_owned(),
                    position: i64::try_from(position).unwrap_or_default(),
                }),
                _ => None,
            }
        })
        .collect()
}

fn publisher_from_extensions(exts: &ExtensionMap) -> Option<ParsedPublisher> {
    let publisher = find_ext(exts, "podcast", "publisher")?;
    let remote = publisher.children.get("remoteItem")?.first()?;
    let parsed = ParsedPublisher {
        feed_guid: clean_attr(remote, "feedGuid"),
        feed_url: clean_attr(remote, "feedUrl"),
    };
    (parsed.feed_guid.is_some() || parsed.feed_url.is_some()).then_some(parsed)
}

pub fn subscribe_feed(
    conn: &mut Connection,
    feed_url: &str,
    musicindex_endpoint: &crate::config::MusicIndexEndpoint,
) -> Result<()> {
    // --- fetch ---
    let body = crate::http_client::document()
        .get(feed_url)
        .send()
        .with_context(|| format!("GET {feed_url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error for {feed_url}"))?
        .bytes()
        .with_context(|| format!("read body {feed_url}"))?;

    // --- parse ---
    let document = parse_feed_document(&body)?;

    // --- persist the channel (always mark subscribed), the items, and the
    // RSS values of the compared slots ---
    let (feed_id, upserted) =
        persist_subscribed_document(conn, feed_url, &document, unix_now_us())?;
    let feed_guid = document.channel.feed_guid.clone();

    // Best-effort: capture MusicIndex feed `updated_at` so freshly-subscribed
    // feeds aren't immediately marked stale by the auto-update checker.
    // ADR 0076 Decision 5 and ADR 0075 packet 020: a subscribe reads RSS, so
    // it writes no MusicIndex value into a compared slot. The feed
    // description stays the RSS value, and its hold decides later
    // MusicIndex writes.
    if let (Some(guid), Ok(endpoint)) = (feed_guid.as_deref(), musicindex_endpoint.require()) {
        let client = MusicIndexClient::new_with_base_url(endpoint);
        match client.fetch_feed(guid, None) {
            Ok(api_feed) => {
                if let Some(updated_at) = api_feed.updated_at {
                    if let Err(err) = db::set_feed_musicindex_updated_at(conn, feed_id, updated_at)
                    {
                        eprintln!("set baseline musicindex_updated_at: {err:#}");
                    }
                }
            }
            Err(err) => eprintln!("fetch MusicIndex feed for baseline updated_at: {err:#}"),
        }
    }

    eprintln!(
        "App subscribed or updated RSS feed {}; stored {upserted} track updates.",
        document.channel.title
    );
    Ok(())
}

/// The persist step of one parsed document: the channel, the items, and the
/// `rss` fact rows and holds of the compared slots (ADR 0076 Decision 5,
/// ADR 0075 packet 020). Returns the feed id and the number of track updates.
pub(crate) fn persist_subscribed_document(
    conn: &mut Connection,
    feed_url: &str,
    document: &ParsedFeedDocument,
    subscribed_at_us: i64,
) -> Result<(i64, usize)> {
    let feed_id = persist_channel(conn, feed_url, &document.channel)?;
    let upserted = persist_items(conn, feed_url, feed_id, &document.items)?;
    super::check_apply::record_subscribed_document(conn, feed_id, document, subscribed_at_us)?;
    Ok((feed_id, upserted))
}

fn unix_now_us() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_micros()).unwrap_or(i64::MAX)
        })
}

/// The persist step of the channel: the `feeds` row and the RSS
/// contributors and links of the feed. It marks the feed subscribed.
pub(crate) fn persist_channel(
    conn: &mut Connection,
    feed_url: &str,
    channel: &ParsedChannel,
) -> Result<i64> {
    conn.execute(
        r#"
        INSERT INTO feeds (
            feed_url,
            feed_guid,
            title,
            link,
            language,
            description,
            podcast_medium,
            album_image_href,
            album_image_mime,
            people_json,
            podcast_value_json,
            album_artist,
            is_subscribed,
            last_fetched_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1, datetime('now'))
        ON CONFLICT(feed_url) DO UPDATE SET
            feed_guid          = excluded.feed_guid,
            title              = excluded.title,
            link               = excluded.link,
            language           = excluded.language,
            description        = excluded.description,
            podcast_medium     = excluded.podcast_medium,
            album_image_href   = excluded.album_image_href,
            album_image_mime   = excluded.album_image_mime,
            people_json        = excluded.people_json,
            podcast_value_json = excluded.podcast_value_json,
            album_artist       = excluded.album_artist,
            is_subscribed      = 1,
            last_fetched_at    = datetime('now')
        "#,
        rusqlite::params![
            feed_url,
            channel.feed_guid,
            channel.title,
            channel.link,
            channel.language,
            channel.description,
            channel.podcast_medium,
            channel.album_image_href,
            channel.album_image_mime,
            channel.people_json,
            channel.podcast_value_json,
            channel.album_artist,
        ],
    )
    .context("upsert feed")?;

    let feed_id: i64 = conn
        .query_row(
            "SELECT id FROM feeds WHERE feed_url = ?1",
            rusqlite::params![feed_url],
            |row| row.get(0),
        )
        .context("lookup feed_id")?;
    // ADR 0076 Decision 9: the canonical route follows each raw block write.
    crate::db::payment_routes::refresh_feed_route(
        conn,
        feed_id,
        channel.podcast_value_json.as_deref(),
    )?;
    persist_rss_feed_identity(
        conn,
        feed_id,
        channel.feed_guid.as_deref(),
        channel.link.as_deref(),
        &channel.contributors,
    )?;
    Ok(feed_id)
}

/// The persist step of the items: one `tracks` row for each item, in one
/// transaction, and the RSS contributors and links of each track.
pub(crate) fn persist_items(
    conn: &mut Connection,
    feed_url: &str,
    feed_id: i64,
    items: &[ParsedItem],
) -> Result<usize> {
    let tx = conn.transaction().context("begin transaction")?;
    let mut upserted = 0usize;
    for item in items {
        if upsert_item_columns(&tx, feed_id, item)? > 0 {
            upserted += 1;
        }
    }
    tx.commit().context("commit tracks")?;
    for item in items {
        persist_rss_track_identity(
            conn,
            feed_url,
            &RssTrackIdentityFacts {
                item_guid: item.item_guid.clone(),
                contributors: item.contributors.clone(),
                links: item.links.clone(),
            },
        )?;
    }
    Ok(upserted)
}

/// The column mapping of one item. The subscribe command and the playlist
/// RSS check insert a track with this one statement.
///
/// The statement also writes the canonical route of the raw block (ADR 0076
/// Decision 9). A row with no raw block before and after the write keeps its
/// canonical column, because that column can hold a `MusicIndex` route.
pub(crate) fn upsert_item_columns(
    conn: &Connection,
    feed_id: i64,
    item: &ParsedItem,
) -> Result<usize> {
    conn.execute(
        r#"
        INSERT INTO tracks (
            feed_id,
            item_guid,
            enclosure_url,
            enclosure_type,
            link,
            pub_date,
            track_title,
            artist_name,
            album_title,
            album_artist_name,
            disc_number,
            track_number,
            duration_seconds,
            itunes_duration_raw,
            itunes_explicit,
            track_image_href,
            track_image_mime,
            people_json,
            item_value_json,
            extra_json,
            payment_routes_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
        ON CONFLICT(feed_id, item_guid) DO UPDATE SET
            enclosure_url       = excluded.enclosure_url,
            enclosure_type      = excluded.enclosure_type,
            link                = excluded.link,
            pub_date            = excluded.pub_date,
            track_title         = excluded.track_title,
            artist_name         = excluded.artist_name,
            album_title         = excluded.album_title,
            album_artist_name   = excluded.album_artist_name,
            disc_number         = excluded.disc_number,
            track_number        = excluded.track_number,
            duration_seconds    = excluded.duration_seconds,
            itunes_duration_raw = excluded.itunes_duration_raw,
            itunes_explicit     = excluded.itunes_explicit,
            track_image_href    = excluded.track_image_href,
            track_image_mime    = excluded.track_image_mime,
            people_json         = excluded.people_json,
            item_value_json     = excluded.item_value_json,
            extra_json          = excluded.extra_json,
            payment_routes_json = CASE
                WHEN excluded.item_value_json IS NULL AND tracks.item_value_json IS NULL
                THEN tracks.payment_routes_json
                ELSE excluded.payment_routes_json
            END
        "#,
        rusqlite::params![
            feed_id,
            item.item_guid,
            item.enclosure_url,
            item.enclosure_type,
            item.link,
            item.pub_date,
            item.track_title,
            item.artist_name,
            item.album_title,
            item.album_artist_name,
            item.disc_number,
            item.track_number,
            item.duration_seconds,
            item.itunes_duration_raw,
            item.itunes_explicit,
            item.track_image_href,
            item.track_image_mime,
            item.people_json,
            item.item_value_json,
            item.extra_json,
            crate::rss::value_routes::canonical_routes_json(item.item_value_json.as_deref()),
        ],
    )
    .context("upsert track")
}

fn track_extra_json(transcript_url: Option<&str>, transcript_type: Option<&str>) -> String {
    let mut object = serde_json::Map::new();
    if let Some(transcript_url) = transcript_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        object.insert(
            "transcript_url".into(),
            serde_json::Value::String(transcript_url.to_string()),
        );
    }
    if let Some(transcript_type) = transcript_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        object.insert(
            "transcript_type".into(),
            serde_json::Value::String(transcript_type.to_string()),
        );
    }
    serde_json::Value::Object(object).to_string()
}

#[derive(Debug)]
struct RssTrackIdentityFacts {
    item_guid: String,
    contributors: Vec<db::LocalContributorInput>,
    links: Vec<db::LocalIdentityLinkInput>,
}

fn persist_rss_feed_identity(
    conn: &mut Connection,
    feed_id: i64,
    feed_guid: Option<&str>,
    feed_link: Option<&str>,
    contributors: &[db::LocalContributorInput],
) -> Result<()> {
    db::replace_local_contributors(
        conn,
        db::LocalEntityOwner::Feed(feed_id),
        "rss",
        contributors,
    )?;
    db::replace_local_identity_links(
        conn,
        db::LocalIdentityOwner::Feed(feed_id),
        "rss",
        &rss_feed_link_inputs(feed_guid, feed_link),
    )
}

fn persist_rss_track_identity(
    conn: &mut Connection,
    feed_url: &str,
    facts: &RssTrackIdentityFacts,
) -> Result<()> {
    let Some(track_id) = db::find_track_id(conn, Some(feed_url), Some(&facts.item_guid), None)?
    else {
        return Ok(());
    };
    db::replace_local_contributors(
        conn,
        db::LocalEntityOwner::Track(track_id),
        "rss",
        &facts.contributors,
    )?;
    db::replace_local_identity_links(
        conn,
        db::LocalIdentityOwner::Track(track_id),
        "rss",
        &facts.links,
    )
}

fn contributor_inputs_from_extensions(exts: &ExtensionMap) -> Vec<db::LocalContributorInput> {
    let Some(persons) = exts
        .get("podcast")
        .and_then(|podcast| podcast.get("person"))
    else {
        return Vec::new();
    };

    persons
        .iter()
        .enumerate()
        .map(|(position, person)| db::LocalContributorInput {
            position: i64::try_from(position).unwrap_or_default(),
            name: clean_text(person.value.as_deref()),
            role: clean_attr(person, "role"),
            group_name: clean_attr(person, "group"),
            href: clean_attr(person, "href"),
            image_url: clean_attr(person, "img"),
            nostr_npub: clean_attr(person, "npub"),
            raw_json: serde_json::to_string(&ext_to_json(person)).ok(),
            observed_at: None,
        })
        .collect()
}

/// The feed `rss` website row of the channel link. The subscribe and the
/// RSS check both write it (ADR 0076 Decision 10).
pub(crate) fn rss_feed_link_inputs(
    feed_guid: Option<&str>,
    feed_link: Option<&str>,
) -> Vec<db::LocalIdentityLinkInput> {
    clean_text(feed_link)
        .map(|url| {
            vec![db::LocalIdentityLinkInput {
                entity_type: Some("feed".to_owned()),
                entity_id: clean_text(feed_guid),
                position: Some(0),
                link_type: Some("website".to_owned()),
                url: Some(url.clone()),
                extraction_path: Some("channel/link".to_owned()),
                observed_at: None,
                raw_json: Some(serde_json::json!({ "link": url }).to_string()),
            }]
        })
        .unwrap_or_default()
}

fn rss_track_link_inputs(
    item_guid: &str,
    item_link: Option<&str>,
    transcript_url: Option<&str>,
    transcript_type: Option<&str>,
) -> Vec<db::LocalIdentityLinkInput> {
    let mut links = Vec::new();

    if let Some(url) = clean_text(item_link) {
        links.push(db::LocalIdentityLinkInput {
            entity_type: Some("track".to_owned()),
            entity_id: Some(item_guid.to_owned()),
            position: Some(0),
            link_type: Some("web_page".to_owned()),
            url: Some(url),
            extraction_path: Some("entity.link".to_owned()),
            observed_at: None,
            raw_json: Some(serde_json::json!({ "link": item_link }).to_string()),
        });
    }

    if let Some(url) = clean_text(transcript_url) {
        links.push(db::LocalIdentityLinkInput {
            entity_type: Some("track".to_owned()),
            entity_id: Some(item_guid.to_owned()),
            position: Some(1),
            link_type: Some("transcript".to_owned()),
            url: Some(url.clone()),
            extraction_path: Some("podcast:transcript@url".to_owned()),
            observed_at: None,
            raw_json: Some(
                serde_json::json!({
                    "url": url,
                    "type": clean_text(transcript_type),
                })
                .to_string(),
            ),
        });
    }

    links
}

fn clean_attr(ext: &Extension, name: &str) -> Option<String> {
    clean_text(ext.attrs.get(name).map(String::as_str))
}

#[cfg(test)]
mod tests {

    #[test]
    fn adr_0066_known_rss_import_skips_invalid_index_and_retains_local_identity() -> Result<()> {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://{}/feed.xml", listener.local_addr()?);
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = [0; 2048];
            stream.read(&mut request).unwrap();
            let body = r#"<rss version="2.0"><channel><title>Known RSS</title><link>https://feed.test</link><description>Fixture</description><item><guid>known-track</guid><title>Local needle</title><enclosure url="https://feed.test/audio.mp3" type="audio/mpeg" length="123"/></item></channel></rss>"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let mut conn = setup_test_db()?;
        subscribe_feed(&mut conn, &url, &"invalid endpoint".into())?;
        server.join().unwrap();
        let id = db::find_track_id(&conn, Some(&url), Some("known-track"), None)?.unwrap();
        conn.execute("UPDATE tracks SET is_in_library = 1 WHERE id = ?1", [id])?;
        let rows = crate::application::ApplicationQueryService::new()
            .search_local_library_tracks(&conn, "needle", None)?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        let feed_id: i64 =
            conn.query_row("SELECT feed_id FROM tracks WHERE id = ?1", [id], |r| {
                r.get(0)
            })?;
        let links = db::local_identity_links(&conn, db::LocalIdentityOwner::Feed(feed_id))?;
        assert!(links.iter().any(|link| link.source == "rss"));
        Ok(())
    }

    use super::*;
    use std::collections::BTreeMap;

    fn setup_test_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    fn create_feed_and_track(conn: &Connection) -> Result<(i64, i64)> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title) VALUES (?1, ?2, ?3)",
            rusqlite::params!["https://example.test/feed.xml", "feed-guid", "Feed"],
        )?;
        let feed_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![feed_id, "item-guid", "Track"],
        )?;
        Ok((feed_id, conn.last_insert_rowid()))
    }

    fn podcast_person_extensions() -> ExtensionMap {
        let mut attrs = BTreeMap::new();
        attrs.insert("role".to_owned(), "host".to_owned());
        attrs.insert("group".to_owned(), "hosts".to_owned());
        attrs.insert("href".to_owned(), "https://example.test/alice".to_owned());
        attrs.insert(
            "img".to_owned(),
            "https://example.test/alice.jpg".to_owned(),
        );
        attrs.insert("npub".to_owned(), "npub1alice".to_owned());
        let person = Extension {
            name: "podcast:person".to_owned(),
            value: Some("Alice".to_owned()),
            attrs,
            children: BTreeMap::new(),
        };

        BTreeMap::from([(
            "podcast".to_owned(),
            BTreeMap::from([("person".to_owned(), vec![person])]),
        )])
    }

    #[test]
    fn rss_person_extensions_map_to_contributor_inputs() {
        let contributors = contributor_inputs_from_extensions(&podcast_person_extensions());

        assert_eq!(contributors.len(), 1);
        assert_eq!(contributors[0].name.as_deref(), Some("Alice"));
        assert_eq!(contributors[0].role.as_deref(), Some("host"));
        assert_eq!(contributors[0].group_name.as_deref(), Some("hosts"));
        assert_eq!(
            contributors[0].href.as_deref(),
            Some("https://example.test/alice")
        );
        assert_eq!(
            contributors[0].image_url.as_deref(),
            Some("https://example.test/alice.jpg")
        );
        assert_eq!(contributors[0].nostr_npub.as_deref(), Some("npub1alice"));
        assert!(
            contributors[0]
                .raw_json
                .as_deref()
                .is_some_and(|raw| raw.contains("Alice")),
            "raw RSS person JSON should be retained"
        );
    }

    #[test]
    fn rss_feed_identity_persistence_preserves_rss_source() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (feed_id, _) = create_feed_and_track(&conn)?;

        persist_rss_feed_identity(
            &mut conn,
            feed_id,
            Some("feed-guid"),
            Some("https://example.test"),
            &contributor_inputs_from_extensions(&podcast_person_extensions()),
        )?;

        let contributors = db::local_contributors(&conn, db::LocalEntityOwner::Feed(feed_id))?;
        assert_eq!(contributors.len(), 1);
        assert_eq!(contributors[0].source, "rss");
        assert_eq!(contributors[0].name.as_deref(), Some("Alice"));

        let links = db::local_identity_links(&conn, db::LocalIdentityOwner::Feed(feed_id))?;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].source, "rss");
        assert_eq!(links[0].link_type.as_deref(), Some("website"));
        assert_eq!(links[0].entity_id.as_deref(), Some("feed-guid"));

        Ok(())
    }

    #[test]
    fn adr_0075_item_page_persistence_preserves_page_and_transcript_links() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (_, track_id) = create_feed_and_track(&conn)?;
        let facts = RssTrackIdentityFacts {
            item_guid: "item-guid".to_owned(),
            contributors: contributor_inputs_from_extensions(&podcast_person_extensions()),
            links: rss_track_link_inputs(
                "item-guid",
                Some(" https://example.test/item "),
                Some("https://example.test/transcript.vtt"),
                Some("text/vtt"),
            ),
        };

        persist_rss_track_identity(&mut conn, "https://example.test/feed.xml", &facts)?;

        let contributors = db::local_contributors(&conn, db::LocalEntityOwner::Track(track_id))?;
        assert_eq!(contributors.len(), 1);
        assert_eq!(contributors[0].source, "rss");

        let links = db::local_identity_links(&conn, db::LocalIdentityOwner::Track(track_id))?;
        assert_eq!(links.len(), 2);
        let page = &links[0];
        assert_eq!(page.source, "rss");
        assert_eq!(page.entity_type.as_deref(), Some("track"));
        assert_eq!(page.entity_id.as_deref(), Some("item-guid"));
        assert_eq!(page.position, Some(0));
        assert_eq!(page.link_type.as_deref(), Some("web_page"));
        assert_eq!(page.url.as_deref(), Some("https://example.test/item"));
        assert_eq!(page.extraction_path.as_deref(), Some("entity.link"));
        assert_eq!(page.observed_at, None);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(page.raw_json.as_deref().unwrap())?,
            serde_json::json!({ "link": " https://example.test/item " })
        );

        let transcript = &links[1];
        assert_eq!(transcript.source, "rss");
        assert_eq!(transcript.position, Some(1));
        assert_eq!(transcript.link_type.as_deref(), Some("transcript"));
        assert_eq!(
            transcript.url.as_deref(),
            Some("https://example.test/transcript.vtt")
        );

        Ok(())
    }

    #[test]
    fn adr_0075_item_page_omits_missing_or_blank_pages_without_feed_inheritance() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (feed_id, track_id) = create_feed_and_track(&conn)?;
        persist_rss_feed_identity(
            &mut conn,
            feed_id,
            Some("feed-guid"),
            Some("https://example.test/feed"),
            &[],
        )?;

        for item_link in [None, Some(" \t ")] {
            let facts = RssTrackIdentityFacts {
                item_guid: "item-guid".to_owned(),
                contributors: Vec::new(),
                links: rss_track_link_inputs("item-guid", item_link, None, None),
            };
            persist_rss_track_identity(&mut conn, "https://example.test/feed.xml", &facts)?;

            assert!(
                db::local_identity_links(&conn, db::LocalIdentityOwner::Track(track_id))?
                    .is_empty()
            );
        }

        let feed_links = db::local_identity_links(&conn, db::LocalIdentityOwner::Feed(feed_id))?;
        assert_eq!(feed_links.len(), 1);
        assert_eq!(
            feed_links[0].url.as_deref(),
            Some("https://example.test/feed")
        );

        Ok(())
    }

    #[test]
    fn adr_0075_item_page_reimport_replaces_rss_rows_and_keeps_musicindex_rows() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (_, track_id) = create_feed_and_track(&conn)?;
        db::replace_local_identity_links(
            &mut conn,
            db::LocalIdentityOwner::Track(track_id),
            "musicindex",
            &[db::LocalIdentityLinkInput {
                entity_type: Some("track".to_owned()),
                entity_id: Some("item-guid".to_owned()),
                position: Some(0),
                link_type: Some("web_page".to_owned()),
                url: Some("https://musicindex.example/item".to_owned()),
                extraction_path: Some("entity.link".to_owned()),
                observed_at: Some(1_725_000_000),
                raw_json: Some(r#"{"link":"https://musicindex.example/item"}"#.to_owned()),
            }],
        )?;

        let first = RssTrackIdentityFacts {
            item_guid: "item-guid".to_owned(),
            contributors: Vec::new(),
            links: rss_track_link_inputs(
                "item-guid",
                Some("https://rss.example/old"),
                Some("https://rss.example/old.vtt"),
                Some("text/vtt"),
            ),
        };
        let second = RssTrackIdentityFacts {
            item_guid: "item-guid".to_owned(),
            contributors: Vec::new(),
            links: rss_track_link_inputs(
                "item-guid",
                Some("https://rss.example/current"),
                Some("https://rss.example/current.vtt"),
                Some("text/vtt"),
            ),
        };
        persist_rss_track_identity(&mut conn, "https://example.test/feed.xml", &first)?;
        persist_rss_track_identity(&mut conn, "https://example.test/feed.xml", &second)?;

        let links = db::local_identity_links(&conn, db::LocalIdentityOwner::Track(track_id))?;
        assert_eq!(links.len(), 3);
        assert_eq!(links.iter().filter(|link| link.source == "rss").count(), 2);
        assert!(links.iter().any(|link| {
            link.source == "rss" && link.url.as_deref() == Some("https://rss.example/current")
        }));
        assert!(links.iter().any(|link| {
            link.source == "rss" && link.url.as_deref() == Some("https://rss.example/current.vtt")
        }));
        assert!(!links.iter().any(|link| {
            link.source == "rss" && link.url.as_deref() == Some("https://rss.example/old")
        }));
        assert!(links.iter().any(|link| {
            link.source == "musicindex"
                && link.url.as_deref() == Some("https://musicindex.example/item")
                && link.observed_at == Some(1_725_000_000)
        }));

        Ok(())
    }
}
