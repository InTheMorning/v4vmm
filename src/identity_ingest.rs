use std::collections::BTreeMap;

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

use crate::api::{
    Artist, ArtistCredit, Contributor, Feed, SourceEntityId, SourceEntityLink, Track,
};
use crate::db::{
    self, ArtistSourceFactInput, LocalContributorInput, LocalEntityOwner, LocalIdentityIdInput,
    LocalIdentityLinkInput, LocalIdentityOwner, LocalMetadataFactInput, LocalMetadataOwner,
    LocalMetadataValue, TrackArtistSourceBindingInput,
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
    )?;
    persist_feed_metadata_facts(conn, feed_id, feed)
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
    )?;
    persist_track_artist_bindings(conn, track_id, track)?;
    persist_track_metadata_facts(conn, track_id, track)
}

pub(crate) fn persist_musicindex_artist(conn: &mut Connection, artist: &Artist) -> Result<()> {
    let Some(source_artist_id) = artist
        .artist_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    else {
        return Ok(());
    };

    db::replace_artist_source_fact(
        conn,
        MUSICINDEX_SOURCE,
        source_artist_id,
        &ArtistSourceFactInput {
            name: artist.name.clone(),
            sort_name: artist.sort_name.clone(),
            image_url: artist.image_url.clone(),
            website_url: artist.url.clone(),
            aliases: artist.aliases.clone().unwrap_or_default(),
            tags: artist.tags.clone().unwrap_or_default(),
            area: artist.area.clone(),
            begin_year: artist.begin_year.map(i64::from),
            end_year: artist.end_year.map(i64::from),
            observed_at: artist.updated_at,
            raw_json: raw_json(artist),
            source_links: Vec::new(),
            source_ids: Vec::new(),
        },
    )
}

fn persist_track_artist_bindings(
    conn: &mut Connection,
    track_id: i64,
    track: &Track,
) -> Result<()> {
    let Some(artist_credit) = track.artist_credit.as_ref() else {
        db::replace_track_artist_source_bindings_for_source(
            conn,
            track_id,
            MUSICINDEX_SOURCE,
            &[],
        )?;
        return Ok(());
    };
    let Some(source_artist_id) = explicit_artist_credit_id(artist_credit) else {
        db::replace_track_artist_source_bindings_for_source(
            conn,
            track_id,
            MUSICINDEX_SOURCE,
            &[],
        )?;
        return Ok(());
    };

    ensure_musicindex_artist_source_fact(conn, source_artist_id, artist_credit)?;
    db::replace_track_artist_source_bindings_for_source(
        conn,
        track_id,
        MUSICINDEX_SOURCE,
        &[TrackArtistSourceBindingInput {
            role: "artist".to_owned(),
            source: MUSICINDEX_SOURCE.to_owned(),
            source_artist_id: source_artist_id.to_owned(),
            confidence: Some(1.0),
            provenance: Some("musicindex.track.artist_credit.artist_id".to_owned()),
            observed_at: track.updated_at,
        }],
    )
}

fn ensure_musicindex_artist_source_fact(
    conn: &mut Connection,
    source_artist_id: &str,
    artist_credit: &ArtistCredit,
) -> Result<()> {
    if db::artist_source_fact(conn, MUSICINDEX_SOURCE, source_artist_id)?.is_some() {
        return Ok(());
    }

    db::replace_artist_source_fact(
        conn,
        MUSICINDEX_SOURCE,
        source_artist_id,
        &ArtistSourceFactInput {
            name: artist_credit.display_name.clone(),
            raw_json: raw_json(artist_credit),
            ..ArtistSourceFactInput::default()
        },
    )
}

fn explicit_artist_credit_id(artist_credit: &ArtistCredit) -> Option<&str> {
    artist_credit
        .artist_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
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

fn persist_feed_metadata_facts(conn: &mut Connection, feed_id: i64, feed: &Feed) -> Result<()> {
    let grouped = feed_metadata_facts_by_source(feed);
    let owner = LocalMetadataOwner::Feed(feed_id);

    for (source, mut facts) in grouped {
        if source != MUSICINDEX_SOURCE {
            facts = merge_existing_metadata_facts_for_partial_source(conn, owner, &source, facts)?;
        }
        db::replace_local_metadata_facts(conn, owner, &source, &facts)?;
    }

    Ok(())
}

fn persist_track_metadata_facts(conn: &mut Connection, track_id: i64, track: &Track) -> Result<()> {
    let facts = track_metadata_facts(track);
    db::replace_local_metadata_facts(
        conn,
        LocalMetadataOwner::Track(track_id),
        MUSICINDEX_SOURCE,
        &facts,
    )
}

fn merge_existing_metadata_facts_for_partial_source(
    conn: &Connection,
    owner: LocalMetadataOwner,
    source: &str,
    facts: Vec<LocalMetadataFactInput>,
) -> Result<Vec<LocalMetadataFactInput>> {
    let mut merged = db::local_metadata_facts(conn, owner)?
        .into_iter()
        .filter(|row| {
            row.source == source && !facts.iter().any(|fact| fact.fact_key == row.fact_key)
        })
        .map(local_metadata_fact_input_from_row)
        .collect::<Vec<_>>();
    merged.extend(facts);
    Ok(merged)
}

fn local_metadata_fact_input_from_row(row: db::LocalMetadataFactRow) -> LocalMetadataFactInput {
    LocalMetadataFactInput {
        fact_key: row.fact_key,
        value: row.value,
        extraction_path: row.extraction_path,
        observed_at: row.observed_at,
        raw_json: row.raw_json,
    }
}

fn feed_metadata_facts_by_source(feed: &Feed) -> BTreeMap<String, Vec<LocalMetadataFactInput>> {
    let mut grouped = BTreeMap::from([(MUSICINDEX_SOURCE.to_owned(), Vec::new())]);
    let feed_raw_json = raw_json(feed);

    push_grouped_text_metadata_fact(
        &mut grouped,
        MUSICINDEX_SOURCE,
        "publisher_text",
        feed.publisher_text.as_deref(),
        Some("$.publisher_text"),
        feed.updated_at,
        feed_raw_json.clone(),
    );
    push_grouped_text_metadata_fact(
        &mut grouped,
        MUSICINDEX_SOURCE,
        "musicindex_release_kind",
        feed.release_kind.as_deref(),
        Some("$.release_kind"),
        feed.updated_at,
        feed_raw_json.clone(),
    );
    if let Some(release_date) = feed.release_date {
        grouped
            .entry(MUSICINDEX_SOURCE.to_owned())
            .or_default()
            .push(LocalMetadataFactInput {
                fact_key: "release_date".to_owned(),
                value: LocalMetadataValue::Integer(release_date),
                extraction_path: Some("$.release_date".to_owned()),
                observed_at: feed.updated_at,
                raw_json: feed_raw_json.clone(),
            });
    }
    push_grouped_text_metadata_fact(
        &mut grouped,
        MUSICINDEX_SOURCE,
        "language",
        feed.language.as_deref(),
        Some("$.language"),
        feed.updated_at,
        feed_raw_json.clone(),
    );
    if let Some(explicit) = feed.explicit {
        grouped
            .entry(MUSICINDEX_SOURCE.to_owned())
            .or_default()
            .push(LocalMetadataFactInput {
                fact_key: "explicit".to_owned(),
                value: LocalMetadataValue::Boolean(explicit),
                extraction_path: Some("$.explicit".to_owned()),
                observed_at: feed.updated_at,
                raw_json: feed_raw_json.clone(),
            });
    }
    push_grouped_text_metadata_fact(
        &mut grouped,
        MUSICINDEX_SOURCE,
        "description",
        feed.description.as_deref(),
        Some("$.description"),
        feed.updated_at,
        feed_raw_json,
    );

    if let Some(claims) = feed.source_release_claims.as_deref() {
        for claim in claims {
            if claim.claim_type.as_deref() != Some("description") {
                continue;
            }
            push_grouped_text_metadata_fact(
                &mut grouped,
                &source_token(claim.source.as_deref()),
                "description",
                claim.claim_value.as_deref(),
                claim.extraction_path.as_deref(),
                claim.observed_at,
                raw_json(claim),
            );
        }
    }

    grouped
}

fn track_metadata_facts(track: &Track) -> Vec<LocalMetadataFactInput> {
    let mut facts = Vec::new();
    let track_raw_json = raw_json(track);

    push_text_metadata_fact(
        &mut facts,
        "publisher_text",
        track.publisher_text.as_deref(),
        Some("$.publisher_text"),
        track.updated_at,
        track_raw_json.clone(),
    );
    push_text_metadata_fact(
        &mut facts,
        "description",
        track.description.as_deref(),
        Some("$.description"),
        track.updated_at,
        track_raw_json.clone(),
    );
    if let Some(pub_date) = track.pub_date {
        facts.push(LocalMetadataFactInput {
            fact_key: "pub_date".to_owned(),
            value: LocalMetadataValue::Integer(pub_date),
            extraction_path: Some("$.pub_date".to_owned()),
            observed_at: track.updated_at,
            raw_json: track_raw_json.clone(),
        });
    }
    if let Some(explicit) = track.explicit {
        facts.push(LocalMetadataFactInput {
            fact_key: "explicit".to_owned(),
            value: LocalMetadataValue::Boolean(explicit),
            extraction_path: Some("$.explicit".to_owned()),
            observed_at: track.updated_at,
            raw_json: track_raw_json,
        });
    }

    facts
}

fn push_grouped_text_metadata_fact(
    grouped: &mut BTreeMap<String, Vec<LocalMetadataFactInput>>,
    source: &str,
    fact_key: &str,
    value: Option<&str>,
    extraction_path: Option<&str>,
    observed_at: Option<i64>,
    raw_json: Option<String>,
) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };

    grouped
        .entry(source.to_owned())
        .or_default()
        .push(LocalMetadataFactInput {
            fact_key: fact_key.to_owned(),
            value: LocalMetadataValue::Text(value.to_owned()),
            extraction_path: extraction_path.map(str::to_owned),
            observed_at,
            raw_json,
        });
}

fn push_text_metadata_fact(
    facts: &mut Vec<LocalMetadataFactInput>,
    fact_key: &str,
    value: Option<&str>,
    extraction_path: Option<&str>,
    observed_at: Option<i64>,
    raw_json: Option<String>,
) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };

    facts.push(LocalMetadataFactInput {
        fact_key: fact_key.to_owned(),
        value: LocalMetadataValue::Text(value.to_owned()),
        extraction_path: extraction_path.map(str::to_owned),
        observed_at,
        raw_json,
    });
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
    use crate::api::{SourceEntityId, SourceEntityLink, SourceReleaseClaim};
    use anyhow::Context;

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

    fn table_row_count(conn: &Connection, table: &str) -> Result<i64> {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .context("count table rows")
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
    fn musicindex_feed_metadata_persists_supported_top_level_fields() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (feed_id, track_id) = create_feed_and_track(&conn)?;
        let feed = Feed {
            publisher_text: Some("Example Publisher".into()),
            release_kind: Some("album".into()),
            release_date: Some(1_714_000_000),
            language: Some("en".into()),
            explicit: Some(true),
            description: Some("Feed description".into()),
            updated_at: Some(1_714_100_000),
            ..Feed::default()
        };

        persist_musicindex_feed(&mut conn, feed_id, &feed)?;

        let facts = db::local_metadata_facts(&conn, LocalMetadataOwner::Feed(feed_id))?;
        assert_eq!(facts.len(), 6, "all supported top-level facts persist");
        assert!(facts.iter().all(|fact| fact.source == "musicindex"));
        assert!(
            facts.iter().any(|fact| fact.fact_key == "publisher_text"
                && fact.value == LocalMetadataValue::Text("Example Publisher".to_owned())
                && fact.extraction_path.as_deref() == Some("$.publisher_text")
                && fact.observed_at == Some(1_714_100_000)
                && fact
                    .raw_json
                    .as_deref()
                    .is_some_and(|raw| raw.contains("Example Publisher"))),
            "publisher_text should retain top-level feed provenance"
        );
        assert!(facts
            .iter()
            .any(|fact| fact.fact_key == "musicindex_release_kind"
                && fact.value == LocalMetadataValue::Text("album".to_owned())));
        assert!(facts.iter().any(|fact| fact.fact_key == "release_date"
            && fact.value == LocalMetadataValue::Integer(1_714_000_000)));
        assert!(facts.iter().any(|fact| fact.fact_key == "language"
            && fact.value == LocalMetadataValue::Text("en".to_owned())));
        assert!(facts
            .iter()
            .any(|fact| fact.fact_key == "explicit"
                && fact.value == LocalMetadataValue::Boolean(true)));
        assert!(facts.iter().any(|fact| fact.fact_key == "description"
            && fact.value == LocalMetadataValue::Text("Feed description".to_owned())));
        assert!(
            db::local_metadata_facts(&conn, LocalMetadataOwner::Track(track_id))?.is_empty(),
            "feed metadata ingest must not write track metadata facts"
        );

        Ok(())
    }

    #[test]
    fn musicindex_feed_metadata_persists_description_claim_provenance() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (feed_id, _) = create_feed_and_track(&conn)?;
        let feed = Feed {
            source_release_claims: Some(vec![
                SourceReleaseClaim {
                    claim_type: Some("description".into()),
                    claim_value: Some("Claim description".into()),
                    source: Some("rss".into()),
                    extraction_path: Some("$.channel.description".into()),
                    observed_at: Some(1_714_200_000),
                    ..SourceReleaseClaim::default()
                },
                SourceReleaseClaim {
                    claim_type: Some("release_date".into()),
                    claim_value: Some("2024-04-01".into()),
                    source: Some("rss".into()),
                    ..SourceReleaseClaim::default()
                },
            ]),
            ..Feed::default()
        };

        persist_musicindex_feed(&mut conn, feed_id, &feed)?;

        let facts = db::local_metadata_facts(&conn, LocalMetadataOwner::Feed(feed_id))?;
        assert_eq!(
            facts
                .iter()
                .filter(|fact| fact.source == "rss" && fact.fact_key == "description")
                .count(),
            1,
            "description source-release claim should keep its own source token"
        );
        let claim_fact = facts
            .iter()
            .find(|fact| fact.source == "rss" && fact.fact_key == "description")
            .context("claim description fact should exist")?;
        assert_eq!(
            claim_fact.value,
            LocalMetadataValue::Text("Claim description".to_owned())
        );
        assert_eq!(
            claim_fact.extraction_path.as_deref(),
            Some("$.channel.description")
        );
        assert_eq!(claim_fact.observed_at, Some(1_714_200_000));
        assert!(
            claim_fact
                .raw_json
                .as_deref()
                .is_some_and(|raw| raw.contains("Claim description")),
            "claim raw JSON should be retained"
        );

        Ok(())
    }

    #[test]
    fn musicindex_feed_metadata_skips_empty_strings() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (feed_id, _) = create_feed_and_track(&conn)?;
        let feed = Feed {
            publisher_text: Some("   ".into()),
            release_kind: Some(String::new()),
            language: Some("\t".into()),
            explicit: Some(false),
            description: Some("  Description  ".into()),
            source_release_claims: Some(vec![SourceReleaseClaim {
                claim_type: Some("description".into()),
                claim_value: Some("  ".into()),
                source: Some("rss".into()),
                ..SourceReleaseClaim::default()
            }]),
            ..Feed::default()
        };

        persist_musicindex_feed(&mut conn, feed_id, &feed)?;

        let facts = db::local_metadata_facts(&conn, LocalMetadataOwner::Feed(feed_id))?;
        assert_eq!(facts.len(), 2);
        assert!(facts
            .iter()
            .any(|fact| fact.fact_key == "explicit"
                && fact.value == LocalMetadataValue::Boolean(false)));
        assert!(facts.iter().any(|fact| fact.fact_key == "description"
            && fact.value == LocalMetadataValue::Text("Description".to_owned())));
        assert!(
            facts.iter().all(|fact| fact.source == "musicindex"),
            "empty claim strings should not create source-specific rows"
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
    fn musicindex_track_metadata_persists_supported_top_level_fields() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (_, track_id) = create_feed_and_track(&conn)?;
        let track = Track {
            publisher_text: Some("Track Publisher".into()),
            description: Some("Track description".into()),
            pub_date: Some(1_714_300_000),
            explicit: Some(true),
            updated_at: Some(1_714_400_000),
            ..Track::default()
        };

        persist_musicindex_track(&mut conn, track_id, &track)?;

        let facts = db::local_metadata_facts(&conn, LocalMetadataOwner::Track(track_id))?;
        assert_eq!(facts.len(), 4, "all supported track metadata facts persist");
        assert!(facts.iter().all(|fact| fact.source == "musicindex"));
        assert!(
            facts.iter().any(|fact| fact.fact_key == "publisher_text"
                && fact.value == LocalMetadataValue::Text("Track Publisher".to_owned())
                && fact.extraction_path.as_deref() == Some("$.publisher_text")
                && fact.observed_at == Some(1_714_400_000)
                && fact
                    .raw_json
                    .as_deref()
                    .is_some_and(|raw| raw.contains("Track Publisher"))),
            "publisher_text should retain top-level track provenance"
        );
        assert!(facts.iter().any(|fact| fact.fact_key == "description"
            && fact.value == LocalMetadataValue::Text("Track description".to_owned())));
        assert!(facts.iter().any(|fact| fact.fact_key == "pub_date"
            && fact.value == LocalMetadataValue::Integer(1_714_300_000)));
        assert!(facts
            .iter()
            .any(|fact| fact.fact_key == "explicit"
                && fact.value == LocalMetadataValue::Boolean(true)));

        Ok(())
    }

    #[test]
    fn musicindex_track_metadata_skips_empty_strings_and_preserves_false_explicit() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (_, track_id) = create_feed_and_track(&conn)?;
        let track = Track {
            publisher_text: Some("   ".into()),
            description: Some("\t".into()),
            explicit: Some(false),
            ..Track::default()
        };

        persist_musicindex_track(&mut conn, track_id, &track)?;

        let facts = db::local_metadata_facts(&conn, LocalMetadataOwner::Track(track_id))?;
        assert_eq!(facts.len(), 1);
        assert!(facts
            .iter()
            .any(|fact| fact.fact_key == "explicit"
                && fact.value == LocalMetadataValue::Boolean(false)));

        Ok(())
    }

    #[test]
    fn musicindex_track_metadata_replacement_preserves_rss_source_rows() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (_, track_id) = create_feed_and_track(&conn)?;
        db::replace_local_metadata_facts(
            &mut conn,
            LocalMetadataOwner::Track(track_id),
            "rss",
            &[LocalMetadataFactInput {
                fact_key: "description".to_owned(),
                value: LocalMetadataValue::Text("RSS track description".to_owned()),
                extraction_path: Some("$.item.description".to_owned()),
                observed_at: Some(1),
                raw_json: Some(r#"{"description":"RSS track description"}"#.to_owned()),
            }],
        )?;

        persist_musicindex_track(
            &mut conn,
            track_id,
            &Track {
                description: Some("MusicIndex track description".into()),
                ..Track::default()
            },
        )?;

        let facts = db::local_metadata_facts(&conn, LocalMetadataOwner::Track(track_id))?;
        assert_eq!(facts.len(), 2);
        assert!(facts.iter().any(|fact| fact.source == "rss"
            && fact.fact_key == "description"
            && fact.value == LocalMetadataValue::Text("RSS track description".to_owned())));
        assert!(facts.iter().any(|fact| fact.source == "musicindex"
            && fact.fact_key == "description"
            && fact.value == LocalMetadataValue::Text("MusicIndex track description".to_owned())));

        Ok(())
    }

    #[test]
    fn musicindex_track_metadata_does_not_persist_feed_defaulted_text() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (feed_id, track_id) = create_feed_and_track(&conn)?;
        let feed = Feed {
            publisher_text: Some("Feed Publisher".into()),
            description: Some("Feed description".into()),
            ..Feed::default()
        };
        let track = Track {
            track_guid: Some("track-guid".into()),
            enclosure_url: Some("https://example.test/track.mp3".into()),
            ..Track::default()
        };

        persist_musicindex_context_by_feed_url(
            &mut conn,
            "https://example.test/feed.xml",
            Some(&feed),
            Some(&track),
        )?;

        let feed_facts = db::local_metadata_facts(&conn, LocalMetadataOwner::Feed(feed_id))?;
        assert!(feed_facts
            .iter()
            .any(|fact| fact.fact_key == "publisher_text"
                && fact.value == LocalMetadataValue::Text("Feed Publisher".to_owned())));
        assert!(feed_facts.iter().any(|fact| fact.fact_key == "description"
            && fact.value == LocalMetadataValue::Text("Feed description".to_owned())));
        let track_facts = db::local_metadata_facts(&conn, LocalMetadataOwner::Track(track_id))?;
        assert!(
            track_facts
                .iter()
                .all(|fact| fact.fact_key != "publisher_text" && fact.fact_key != "description"),
            "feed-default copied publisher/description must not become track facts"
        );

        Ok(())
    }

    #[test]
    fn musicindex_track_artist_binding_persists_explicit_artist_credit_id() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (_, track_id) = create_feed_and_track(&conn)?;
        db::replace_artist_source_fact(
            &mut conn,
            "musicindex",
            "artist-123",
            &ArtistSourceFactInput {
                name: Some("Richer Artist".into()),
                image_url: Some("https://example.test/richer.jpg".into()),
                ..ArtistSourceFactInput::default()
            },
        )?;
        let track = Track {
            artist_credit: Some(ArtistCredit {
                artist_id: Some("artist-123".into()),
                display_name: Some("Track Artist".into()),
            }),
            updated_at: Some(1_714_000_000),
            ..Track::default()
        };

        persist_musicindex_track(&mut conn, track_id, &track)?;

        let bindings = db::track_artist_source_bindings_for_track(&conn, track_id)?;
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].role, "artist");
        assert_eq!(bindings[0].source, "musicindex");
        assert_eq!(bindings[0].source_artist_id, "artist-123");
        assert_eq!(bindings[0].confidence, Some(1.0));
        assert_eq!(
            bindings[0].provenance.as_deref(),
            Some("musicindex.track.artist_credit.artist_id")
        );
        assert_eq!(bindings[0].observed_at, Some(1_714_000_000));

        let artist = db::artist_source_fact(&conn, "musicindex", "artist-123")?
            .context("artist source fact should still exist")?;
        assert_eq!(artist.name.as_deref(), Some("Richer Artist"));
        assert_eq!(
            artist.image_url.as_deref(),
            Some("https://example.test/richer.jpg")
        );

        Ok(())
    }

    #[test]
    fn musicindex_track_artist_binding_skips_name_only_artist_credit() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (_, track_id) = create_feed_and_track(&conn)?;
        db::replace_artist_source_fact(
            &mut conn,
            "musicindex",
            "artist-123",
            &ArtistSourceFactInput {
                name: Some("Existing".into()),
                ..ArtistSourceFactInput::default()
            },
        )?;
        db::replace_track_artist_source_bindings(
            &mut conn,
            track_id,
            &[TrackArtistSourceBindingInput {
                role: "artist".to_owned(),
                source: "musicindex".to_owned(),
                source_artist_id: "artist-123".to_owned(),
                confidence: Some(1.0),
                provenance: Some("test".to_owned()),
                observed_at: Some(1),
            }],
        )?;
        let track = Track {
            artist_credit: Some(ArtistCredit {
                artist_id: None,
                display_name: Some("Name Only".into()),
            }),
            ..Track::default()
        };

        persist_musicindex_track(&mut conn, track_id, &track)?;

        assert!(
            db::track_artist_source_bindings_for_track(&conn, track_id)?.is_empty(),
            "name-only artist credits should clear MusicIndex bindings instead of creating one"
        );
        assert_eq!(
            db::artist_source_fact(&conn, "musicindex", "Name Only")?,
            None,
            "name-only artist credits must not create artist source facts"
        );

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

    #[test]
    fn musicindex_feed_metadata_replacement_preserves_rss_source_rows() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (feed_id, _) = create_feed_and_track(&conn)?;
        db::replace_local_metadata_facts(
            &mut conn,
            LocalMetadataOwner::Feed(feed_id),
            "rss",
            &[LocalMetadataFactInput {
                fact_key: "rss_podcast_medium".to_owned(),
                value: LocalMetadataValue::Text("podcast".to_owned()),
                extraction_path: Some("$.channel.podcast:medium".to_owned()),
                observed_at: Some(1),
                raw_json: Some(r#"{"podcast:medium":"podcast"}"#.to_owned()),
            }],
        )?;

        persist_musicindex_feed(
            &mut conn,
            feed_id,
            &Feed {
                publisher_text: Some("MusicIndex Publisher".into()),
                ..Feed::default()
            },
        )?;

        let facts = db::local_metadata_facts(&conn, LocalMetadataOwner::Feed(feed_id))?;
        assert_eq!(facts.len(), 2);
        assert!(
            facts.iter().any(|fact| fact.source == "rss"
                && fact.fact_key == "rss_podcast_medium"
                && fact.value == LocalMetadataValue::Text("podcast".to_owned())),
            "rss metadata row should not be deleted by MusicIndex replacement"
        );
        assert!(facts.iter().any(|fact| fact.source == "musicindex"
            && fact.fact_key == "publisher_text"
            && fact.value == LocalMetadataValue::Text("MusicIndex Publisher".to_owned())));

        Ok(())
    }

    #[test]
    fn musicindex_feed_metadata_claim_source_preserves_other_source_keys() -> Result<()> {
        let mut conn = setup_test_db()?;
        let (feed_id, _) = create_feed_and_track(&conn)?;
        db::replace_local_metadata_facts(
            &mut conn,
            LocalMetadataOwner::Feed(feed_id),
            "rss",
            &[LocalMetadataFactInput {
                fact_key: "rss_podcast_medium".to_owned(),
                value: LocalMetadataValue::Text("podcast".to_owned()),
                extraction_path: Some("$.channel.podcast:medium".to_owned()),
                observed_at: Some(1),
                raw_json: Some(r#"{"podcast:medium":"podcast"}"#.to_owned()),
            }],
        )?;

        persist_musicindex_feed(
            &mut conn,
            feed_id,
            &Feed {
                source_release_claims: Some(vec![SourceReleaseClaim {
                    claim_type: Some("description".into()),
                    claim_value: Some("RSS description".into()),
                    source: Some("rss".into()),
                    extraction_path: Some("$.channel.description".into()),
                    observed_at: Some(2),
                    ..SourceReleaseClaim::default()
                }]),
                ..Feed::default()
            },
        )?;

        let facts = db::local_metadata_facts(&conn, LocalMetadataOwner::Feed(feed_id))?;
        assert!(facts.iter().any(|fact| fact.source == "rss"
            && fact.fact_key == "rss_podcast_medium"
            && fact.value == LocalMetadataValue::Text("podcast".to_owned())));
        assert!(facts.iter().any(|fact| fact.source == "rss"
            && fact.fact_key == "description"
            && fact.value == LocalMetadataValue::Text("RSS description".to_owned())));

        Ok(())
    }

    #[test]
    fn musicindex_artist_source_fact_persists_explicit_artist_id() -> Result<()> {
        let mut conn = setup_test_db()?;
        let artist = Artist {
            artist_id: Some("artist-123".into()),
            name: Some("Alice".into()),
            sort_name: Some("Alice, The".into()),
            image_url: Some("https://example.test/alice.jpg".into()),
            url: Some("https://example.test/alice".into()),
            aliases: Some(vec!["A. Example".into()]),
            tags: Some(vec!["podcast".into()]),
            area: Some("Montreal".into()),
            begin_year: Some(2020),
            end_year: Some(2024),
            updated_at: Some(1_714_000_000),
            ..Artist::default()
        };

        persist_musicindex_artist(&mut conn, &artist)?;

        let row = db::artist_source_fact(&conn, "musicindex", "artist-123")?
            .context("artist source fact should exist")?;
        assert_eq!(row.name.as_deref(), Some("Alice"));
        assert_eq!(row.sort_name.as_deref(), Some("Alice, The"));
        assert_eq!(
            row.image_url.as_deref(),
            Some("https://example.test/alice.jpg")
        );
        assert_eq!(
            row.website_url.as_deref(),
            Some("https://example.test/alice")
        );
        assert_eq!(row.aliases, vec!["A. Example"]);
        assert_eq!(row.tags, vec!["podcast"]);
        assert_eq!(row.area.as_deref(), Some("Montreal"));
        assert_eq!(row.begin_year, Some(2020));
        assert_eq!(row.end_year, Some(2024));
        assert_eq!(row.observed_at, Some(1_714_000_000));
        assert!(
            row.raw_json
                .as_deref()
                .is_some_and(|raw| raw.contains("artist-123")),
            "raw artist JSON should be retained"
        );
        assert!(
            row.source_links.is_empty(),
            "artist API has no source_links field yet"
        );
        assert!(
            row.source_ids.is_empty(),
            "artist API has no source_ids field yet"
        );

        Ok(())
    }

    #[test]
    fn musicindex_artist_source_fact_skips_missing_artist_id() -> Result<()> {
        let mut conn = setup_test_db()?;

        persist_musicindex_artist(
            &mut conn,
            &Artist {
                name: Some("Name Only".into()),
                ..Artist::default()
            },
        )?;
        persist_musicindex_artist(
            &mut conn,
            &Artist {
                artist_id: Some("   ".into()),
                name: Some("Blank Id".into()),
                ..Artist::default()
            },
        )?;

        assert_eq!(
            db::artist_source_fact(&conn, "musicindex", "Name Only")?,
            None
        );
        assert_eq!(table_row_count(&conn, "artist_source_facts")?, 0);

        Ok(())
    }

    #[test]
    fn musicindex_artist_source_fact_replaces_same_musicindex_key() -> Result<()> {
        let mut conn = setup_test_db()?;

        persist_musicindex_artist(
            &mut conn,
            &Artist {
                artist_id: Some("artist-123".into()),
                name: Some("Original".into()),
                aliases: Some(vec!["Old".into()]),
                updated_at: Some(1),
                ..Artist::default()
            },
        )?;
        persist_musicindex_artist(
            &mut conn,
            &Artist {
                artist_id: Some("artist-123".into()),
                name: Some("Updated".into()),
                aliases: Some(vec!["New".into()]),
                updated_at: Some(2),
                ..Artist::default()
            },
        )?;

        let row = db::artist_source_fact(&conn, "musicindex", "artist-123")?
            .context("artist source fact should exist")?;
        assert_eq!(row.name.as_deref(), Some("Updated"));
        assert_eq!(row.aliases, vec!["New"]);
        assert_eq!(row.observed_at, Some(2));
        assert_eq!(table_row_count(&conn, "artist_source_facts")?, 1);

        Ok(())
    }

    #[test]
    fn musicindex_artist_source_fact_preserves_other_source_rows() -> Result<()> {
        let mut conn = setup_test_db()?;
        db::replace_artist_source_fact(
            &mut conn,
            "rss",
            "artist-123",
            &ArtistSourceFactInput {
                name: Some("RSS Artist".into()),
                ..ArtistSourceFactInput::default()
            },
        )?;

        persist_musicindex_artist(
            &mut conn,
            &Artist {
                artist_id: Some("artist-123".into()),
                name: Some("MusicIndex Artist".into()),
                ..Artist::default()
            },
        )?;

        let rss_row = db::artist_source_fact(&conn, "rss", "artist-123")?
            .context("rss artist source fact should exist")?;
        let musicindex_row = db::artist_source_fact(&conn, "musicindex", "artist-123")?
            .context("musicindex artist source fact should exist")?;
        assert_eq!(rss_row.name.as_deref(), Some("RSS Artist"));
        assert_eq!(musicindex_row.name.as_deref(), Some("MusicIndex Artist"));
        assert_eq!(table_row_count(&conn, "artist_source_facts")?, 2);

        Ok(())
    }
}
