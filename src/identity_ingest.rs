use std::collections::BTreeMap;

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

use crate::api::{Contributor, Feed, SourceEntityId, SourceEntityLink, Track};
use crate::db::{
    self, LocalContributorInput, LocalEntityOwner, LocalIdentityIdInput, LocalIdentityLinkInput,
    LocalIdentityOwner,
};

const MUSICINDEX_SOURCE: &str = "musicindex";

pub(crate) fn persist_musicindex_context_by_feed_url(
    conn: &mut Connection,
    feed_url: &str,
    feed: Option<&Feed>,
    track: Option<&Track>,
) -> Result<()> {
    if let (Some(feed_id), Some(feed)) = (db::feed_id_by_url(conn, feed_url)?, feed) {
        persist_musicindex_feed(conn, feed_id, feed)?;
    }

    if let Some(track) = track {
        if let Some(track_id) = db::find_track_id(
            conn,
            Some(feed_url),
            track.track_guid.as_deref(),
            track.enclosure_url.as_deref(),
        )? {
            persist_musicindex_track(conn, track_id, track)?;
        }
    }

    Ok(())
}

pub(crate) fn persist_musicindex_feed(
    conn: &mut Connection,
    feed_id: i64,
    feed: &Feed,
) -> Result<()> {
    persist_source_links(
        conn,
        LocalIdentityOwner::Feed(feed_id),
        feed.source_links.as_deref(),
    )?;
    persist_source_ids(
        conn,
        LocalIdentityOwner::Feed(feed_id),
        feed.source_ids.as_deref(),
    )?;
    persist_contributors(
        conn,
        LocalEntityOwner::Feed(feed_id),
        feed.source_contributors.as_deref(),
    )
}

pub(crate) fn persist_musicindex_track(
    conn: &mut Connection,
    track_id: i64,
    track: &Track,
) -> Result<()> {
    persist_source_links(
        conn,
        LocalIdentityOwner::Track(track_id),
        track.source_links.as_deref(),
    )?;
    persist_source_ids(
        conn,
        LocalIdentityOwner::Track(track_id),
        track.source_ids.as_deref(),
    )?;
    persist_contributors(
        conn,
        LocalEntityOwner::Track(track_id),
        track.source_contributors.as_deref(),
    )
}

fn persist_source_links(
    conn: &mut Connection,
    owner: LocalIdentityOwner,
    source_links: Option<&[SourceEntityLink]>,
) -> Result<()> {
    let Some(source_links) = source_links else {
        return Ok(());
    };

    let mut grouped = BTreeMap::from([(MUSICINDEX_SOURCE.to_owned(), Vec::new())]);
    for link in source_links {
        let source = source_token(link.source.as_deref());
        grouped
            .entry(source)
            .or_default()
            .push(LocalIdentityLinkInput {
                entity_type: link.entity_type.clone(),
                entity_id: link.entity_id.clone(),
                position: link.position,
                link_type: link.link_type.clone(),
                url: link.url.clone(),
                extraction_path: link.extraction_path.clone(),
                observed_at: link.observed_at,
                raw_json: raw_json(link),
            });
    }

    for (source, links) in grouped {
        db::replace_local_identity_links(conn, owner, &source, &links)?;
    }

    Ok(())
}

fn persist_source_ids(
    conn: &mut Connection,
    owner: LocalIdentityOwner,
    source_ids: Option<&[SourceEntityId]>,
) -> Result<()> {
    let Some(source_ids) = source_ids else {
        return Ok(());
    };

    let mut grouped = BTreeMap::from([(MUSICINDEX_SOURCE.to_owned(), Vec::new())]);
    for id in source_ids {
        let source = source_token(id.source.as_deref());
        grouped
            .entry(source)
            .or_default()
            .push(LocalIdentityIdInput {
                entity_type: id.entity_type.clone(),
                entity_id: id.entity_id.clone(),
                position: id.position,
                scheme: id.scheme.clone(),
                value: id.value.clone(),
                extraction_path: id.extraction_path.clone(),
                observed_at: id.observed_at,
                raw_json: raw_json(id),
            });
    }

    for (source, ids) in grouped {
        db::replace_local_identity_ids(conn, owner, &source, &ids)?;
    }

    Ok(())
}

fn persist_contributors(
    conn: &mut Connection,
    owner: LocalEntityOwner,
    contributors: Option<&[Contributor]>,
) -> Result<()> {
    let Some(contributors) = contributors else {
        return Ok(());
    };

    let rows = contributors
        .iter()
        .enumerate()
        .map(|(position, contributor)| LocalContributorInput {
            position: i64::try_from(position).unwrap_or_default(),
            name: contributor.name.clone(),
            role: contributor.role.clone(),
            group_name: contributor.group_name.clone(),
            href: contributor.href.clone(),
            image_url: contributor.img.clone(),
            nostr_npub: contributor.npub.clone(),
            raw_json: raw_json(contributor),
            observed_at: None,
        })
        .collect::<Vec<_>>();

    db::replace_local_contributors(conn, owner, MUSICINDEX_SOURCE, &rows)
}

fn source_token(source: Option<&str>) -> String {
    source
        .map(str::trim)
        .filter(|source| !source.is_empty())
        .unwrap_or(MUSICINDEX_SOURCE)
        .to_owned()
}

fn raw_json<T: Serialize>(value: &T) -> Option<String> {
    serde_json::to_string(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{SourceEntityId, SourceEntityLink};

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
            "INSERT INTO tracks (feed_id, item_guid, enclosure_url, track_title)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                feed_id,
                "track-guid",
                "https://example.test/track.mp3",
                "Track"
            ],
        )?;
        Ok((feed_id, conn.last_insert_rowid()))
    }

    #[test]
    fn musicindex_feed_source_facts_persist_under_local_owner() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (feed_id, _) = create_feed_and_track(&conn)?;
        let feed = Feed {
            source_links: Some(vec![SourceEntityLink {
                link_type: Some("website".into()),
                url: Some("https://example.test".into()),
                source: Some("musicindex".into()),
                extraction_path: Some("$.source_links[0]".into()),
                ..SourceEntityLink::default()
            }]),
            source_ids: Some(vec![SourceEntityId {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1feed".into()),
                source: Some("musicindex".into()),
                ..SourceEntityId::default()
            }]),
            source_contributors: Some(vec![Contributor {
                name: Some("Alice".into()),
                role: Some("host".into()),
                href: Some("https://example.test/alice".into()),
                img: Some("https://example.test/alice.jpg".into()),
                npub: Some("npub1alice".into()),
                ..Contributor::default()
            }]),
            ..Feed::default()
        };

        persist_musicindex_feed(&mut conn, feed_id, &feed)?;

        let links = db::local_identity_links(&conn, LocalIdentityOwner::Feed(feed_id))?;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].link_type.as_deref(), Some("website"));
        assert_eq!(links[0].source, "musicindex");
        assert!(
            links[0]
                .raw_json
                .as_deref()
                .is_some_and(|raw| raw.contains("source_links"))
                || links[0]
                    .raw_json
                    .as_deref()
                    .is_some_and(|raw| raw.contains("website")),
            "raw link JSON should be retained"
        );

        let ids = db::local_identity_ids(&conn, LocalIdentityOwner::Feed(feed_id))?;
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].scheme.as_deref(), Some("nostr_npub"));

        let contributors = db::local_contributors(&conn, LocalEntityOwner::Feed(feed_id))?;
        assert_eq!(contributors.len(), 1);
        assert_eq!(contributors[0].name.as_deref(), Some("Alice"));
        assert_eq!(
            contributors[0].image_url.as_deref(),
            Some("https://example.test/alice.jpg")
        );

        Ok(())
    }

    #[test]
    fn musicindex_track_source_facts_persist_by_feed_url_context() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (_, track_id) = create_feed_and_track(&conn)?;
        let track = Track {
            track_guid: Some("track-guid".into()),
            enclosure_url: Some("https://example.test/track.mp3".into()),
            source_links: Some(vec![SourceEntityLink {
                link_type: Some("transcript".into()),
                url: Some("https://example.test/transcript.vtt".into()),
                source: Some("musicindex".into()),
                ..SourceEntityLink::default()
            }]),
            source_ids: Some(vec![SourceEntityId {
                scheme: Some("isrc".into()),
                value: Some("US-AAA-24-00001".into()),
                source: Some("musicindex".into()),
                ..SourceEntityId::default()
            }]),
            source_contributors: Some(vec![Contributor {
                name: Some("Bob".into()),
                role: Some("guest".into()),
                ..Contributor::default()
            }]),
            ..Track::default()
        };

        persist_musicindex_context_by_feed_url(
            &mut conn,
            "https://example.test/feed.xml",
            None,
            Some(&track),
        )?;

        let links = db::local_identity_links(&conn, LocalIdentityOwner::Track(track_id))?;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].link_type.as_deref(), Some("transcript"));

        let ids = db::local_identity_ids(&conn, LocalIdentityOwner::Track(track_id))?;
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].value.as_deref(), Some("US-AAA-24-00001"));

        let contributors = db::local_contributors(&conn, LocalEntityOwner::Track(track_id))?;
        assert_eq!(contributors.len(), 1);
        assert_eq!(contributors[0].name.as_deref(), Some("Bob"));

        Ok(())
    }

    #[test]
    fn musicindex_replacement_preserves_rss_source_rows() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (feed_id, _) = create_feed_and_track(&conn)?;
        db::replace_local_identity_links(
            &mut conn,
            LocalIdentityOwner::Feed(feed_id),
            "rss",
            &[LocalIdentityLinkInput {
                url: Some("https://rss.example".into()),
                ..LocalIdentityLinkInput::default()
            }],
        )?;

        persist_musicindex_feed(
            &mut conn,
            feed_id,
            &Feed {
                source_links: Some(vec![SourceEntityLink {
                    url: Some("https://musicindex.example".into()),
                    ..SourceEntityLink::default()
                }]),
                ..Feed::default()
            },
        )?;

        let links = db::local_identity_links(&conn, LocalIdentityOwner::Feed(feed_id))?;
        assert_eq!(links.len(), 2);
        assert!(
            links
                .iter()
                .any(|link| link.source == "rss"
                    && link.url.as_deref() == Some("https://rss.example")),
            "rss source row should not be deleted by MusicIndex persistence"
        );

        Ok(())
    }
}
