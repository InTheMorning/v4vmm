use anyhow::{Context, Result};
use rss::extension::{Extension, ExtensionMap};
use rss::Channel;
use rusqlite::Connection;
use std::io::Cursor;

use super::helpers::*;
use crate::api::Client as MusicIndexClient;
use crate::config::Config;
use crate::db;

pub fn subscribe_feed(
    _cfg: &Config,
    conn: &mut Connection,
    feed_url: &str,
    musicindex_endpoint: &str,
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

    let language = feed.language().map(|s| s.to_string());

    let desc = feed.description().trim();
    let description = if desc.is_empty() {
        None
    } else {
        Some(desc.to_string())
    };

    // Podcasting 2.0 extensions: rss crate stores keys without prefix (guid, medium, value, ...)
    let feed_guid = find_ext_text(feed.extensions(), "podcast", "guid");
    let podcast_medium = find_ext_text(feed.extensions(), "podcast", "medium");

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
    let album_image_mime: Option<String> = None;

    // People at feed level (podcast:person); ok if None
    let feed_people_json = collect_people_json(feed.extensions());

    // Full value block (including recipients) as JSON
    let podcast_value_json = value_block_json(feed.extensions(), "podcast", "value");

    // --- upsert feed row (always mark subscribed) ---
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
            is_subscribed,
            last_fetched_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1, datetime('now'))
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
            is_subscribed      = 1,
            last_fetched_at    = datetime('now')
        "#,
        rusqlite::params![
            feed_url,
            feed_guid,
            feed_title,
            feed_link,
            language,
            description,
            podcast_medium,
            album_image_href,
            album_image_mime,
            feed_people_json,
            podcast_value_json,
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
    persist_rss_feed_identity(
        conn,
        feed_id,
        feed_guid.as_deref(),
        feed_link.as_deref(),
        feed.extensions(),
    )?;

    // Best-effort: capture MusicIndex feed `updated_at` so freshly-subscribed
    // feeds aren't immediately marked stale by the auto-update checker.
    if let Some(guid) = feed_guid.as_deref() {
        let client = MusicIndexClient::new_with_base_url(musicindex_endpoint.to_string());
        match client.fetch_feed(guid, None) {
            Ok(api_feed) => {
                if let Some(updated_at) = api_feed.updated_at {
                    if let Err(err) = db::set_feed_musicindex_updated_at(conn, feed_id, updated_at)
                    {
                        eprintln!("set baseline musicindex_updated_at: {err:#}");
                    }
                }
                if !crate::metadata::source_text_missing(api_feed.description.as_deref()) {
                    db::set_feed_description(conn, feed_id, api_feed.description.as_deref())?;
                }
            }
            Err(err) => eprintln!("fetch MusicIndex feed for baseline updated_at: {err:#}"),
        }
    }

    // --- tracks: upsert all items in one transaction ---
    let tx = conn.transaction().context("begin transaction")?;
    let mut upserted = 0usize;
    let mut rss_track_facts = Vec::new();

    for item in feed.items() {
        // Stable identity: item <guid>. If missing, skip (we need stable IDs).
        let item_guid = match item.guid() {
            Some(g) => g.value().to_string(),
            None => continue,
        };

        let enclosure_url = item.enclosure().map(|e| e.url().to_string());
        let enclosure_type = item.enclosure().and_then(|e| {
            let mime = e.mime_type().trim();
            if mime.is_empty() {
                None
            } else {
                Some(mime.to_string())
            }
        });
        let item_link = item.link().map(|s| s.to_string());
        let pub_date = item.pub_date().map(|s| s.to_string());

        // Provisional music fields (ID3 will become canonical once downloaded)
        let track_title = item.title().map(|s| s.to_string());
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
        let album_title = Some(feed_title.clone());
        let album_artist_name = feed_artist.clone().or_else(|| artist_name.clone());
        let disc_number: Option<i64> = None;

        // Canonical ordering: podcast:episode
        let track_number: Option<i64> = find_ext_text(item.extensions(), "podcast", "episode")
            .and_then(|s| s.trim().parse::<i64>().ok());

        // iTunes item tags are NOT in extensions; rss crate exposes them via itunes_ext()
        let itunes = item.itunes_ext();
        let itunes_duration_raw = itunes.and_then(|it| it.duration()).map(|s| s.to_string());
        let duration_seconds: Option<i64> = itunes_duration_raw
            .as_deref()
            .and_then(parse_itunes_duration);
        let itunes_explicit = itunes.and_then(|it| it.explicit()).map(|s| s.to_string());
        let track_image_href = itunes.and_then(|it| it.image()).map(|s| s.to_string());
        let track_image_mime: Option<String> = None;

        // Item-level people/value/transcript (podcast:* extensions)
        let people_json = collect_people_json(item.extensions());
        let item_value_json = value_block_json(item.extensions(), "podcast", "value");
        let transcript_url = find_ext_attr(item.extensions(), "podcast", "transcript", "url");
        let transcript_type = find_ext_attr(item.extensions(), "podcast", "transcript", "type");
        let extra_json = track_extra_json(transcript_url.as_deref(), transcript_type.as_deref());
        rss_track_facts.push(RssTrackIdentityFacts {
            item_guid: item_guid.clone(),
            contributors: contributor_inputs_from_extensions(item.extensions()),
            links: rss_track_link_inputs(
                &item_guid,
                transcript_url.as_deref(),
                transcript_type.as_deref(),
            ),
        });

        let changed = tx.execute(
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
                extra_json
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
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
                extra_json          = excluded.extra_json
            "#,
            rusqlite::params![
                feed_id,
                item_guid,
                enclosure_url,
                enclosure_type,
                item_link,
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
            ],
        )?;

        if changed > 0 {
            upserted += 1;
        }
    }

    tx.commit().context("commit tracks")?;
    for facts in &rss_track_facts {
        persist_rss_track_identity(conn, feed_url, facts)?;
    }

    println!("Subscribed/updated feed: {feed_title} (tracks upserted: {upserted})");
    Ok(())
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
    extensions: &ExtensionMap,
) -> Result<()> {
    db::replace_local_contributors(
        conn,
        db::LocalEntityOwner::Feed(feed_id),
        "rss",
        &contributor_inputs_from_extensions(extensions),
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

fn rss_feed_link_inputs(
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
    transcript_url: Option<&str>,
    transcript_type: Option<&str>,
) -> Vec<db::LocalIdentityLinkInput> {
    clean_text(transcript_url)
        .map(|url| {
            vec![db::LocalIdentityLinkInput {
                entity_type: Some("track".to_owned()),
                entity_id: Some(item_guid.to_owned()),
                position: Some(0),
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
            }]
        })
        .unwrap_or_default()
}

fn clean_attr(ext: &Extension, name: &str) -> Option<String> {
    clean_text(ext.attrs.get(name).map(String::as_str))
}

#[cfg(test)]
mod tests {
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
            &podcast_person_extensions(),
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
    fn rss_track_identity_persistence_preserves_transcript_link() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (_, track_id) = create_feed_and_track(&conn)?;
        let facts = RssTrackIdentityFacts {
            item_guid: "item-guid".to_owned(),
            contributors: contributor_inputs_from_extensions(&podcast_person_extensions()),
            links: rss_track_link_inputs(
                "item-guid",
                Some("https://example.test/transcript.vtt"),
                Some("text/vtt"),
            ),
        };

        persist_rss_track_identity(&mut conn, "https://example.test/feed.xml", &facts)?;

        let contributors = db::local_contributors(&conn, db::LocalEntityOwner::Track(track_id))?;
        assert_eq!(contributors.len(), 1);
        assert_eq!(contributors[0].source, "rss");

        let links = db::local_identity_links(&conn, db::LocalIdentityOwner::Track(track_id))?;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].source, "rss");
        assert_eq!(links[0].link_type.as_deref(), Some("transcript"));
        assert_eq!(
            links[0].url.as_deref(),
            Some("https://example.test/transcript.vtt")
        );

        Ok(())
    }
}
