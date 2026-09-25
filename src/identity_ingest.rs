use std::collections::BTreeMap;

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

use crate::api::{Contributor, Feed, SourceEntityId, SourceEntityLink, Track};
use crate::db::rss_field_holds::{self, HoldOwner, MusicIndexClaim, RssField};
use crate::db::{
    self, LocalContributorInput, LocalEntityOwner, LocalIdentityIdInput, LocalIdentityLinkInput,
    LocalIdentityOwner, LocalMetadataFactInput, LocalMetadataOwner, LocalMetadataValue,
};

const MUSICINDEX_SOURCE: &str = "musicindex";
const RSS_SOURCE: &str = "rss";

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
        HoldOwner::Feed(feed_id),
        feed.source_ids.as_deref(),
        feed.updated_at,
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
    let feed_id: i64 = conn.query_row(
        "SELECT feed_id FROM tracks WHERE id = ?1",
        [track_id],
        |row| row.get(0),
    )?;
    let hold_owner = HoldOwner::Track { feed_id, track_id };
    persist_source_links(
        conn,
        LocalIdentityOwner::Track(track_id),
        track.source_links.as_deref(),
    )?;
    persist_source_ids(
        conn,
        LocalIdentityOwner::Track(track_id),
        hold_owner,
        track.source_ids.as_deref(),
        track.updated_at,
    )?;
    persist_contributors(
        conn,
        LocalEntityOwner::Track(track_id),
        track.source_contributors.as_deref(),
    )?;
    persist_track_metadata_facts(conn, track_id, hold_owner, track)
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
    hold_owner: HoldOwner,
    source_ids: Option<&[SourceEntityId]>,
    updated_at: Option<i64>,
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

    // ADR 0076 Decision 5: the RSS Nostr identities are a compared slot.
    // A held RSS value keeps its rows until `MusicIndex` agrees or supplies
    // a newer record. The other rows of the source are still written.
    let nostr_claim = grouped
        .get(RSS_SOURCE)
        .map(|ids| {
            ids.iter()
                .filter(|id| is_nostr_scheme(id.scheme.as_deref()))
                .filter_map(|id| {
                    Some(serde_json::json!({
                        "scheme": id.scheme.as_deref()?,
                        "value": id.value.as_deref()?,
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let nostr_gate = rss_field_holds::musicindex_gate(
        conn,
        MusicIndexClaim {
            owner: hold_owner,
            field: RssField::Nostr,
            value: (!nostr_claim.is_empty()).then(|| serde_json::Value::Array(nostr_claim)),
            updated_at,
        },
    )?;
    for (source, mut ids) in grouped {
        if source == RSS_SOURCE && !nostr_gate.writes() {
            let held = db::local_identity_ids(conn, owner)?
                .into_iter()
                .filter(|row| row.source == RSS_SOURCE && is_nostr_scheme(row.scheme.as_deref()))
                .map(|row| LocalIdentityIdInput {
                    entity_type: row.entity_type,
                    entity_id: row.entity_id,
                    position: row.position,
                    scheme: row.scheme,
                    value: row.value,
                    extraction_path: row.extraction_path,
                    observed_at: row.observed_at,
                    raw_json: row.raw_json,
                });
            ids.retain(|id| !is_nostr_scheme(id.scheme.as_deref()));
            ids.extend(held);
        }
        db::replace_local_identity_ids(conn, owner, &source, &ids)?;
    }

    Ok(())
}

fn is_nostr_scheme(scheme: Option<&str>) -> bool {
    matches!(scheme, Some("nostr_npub" | "nostr_nprofile"))
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
    let hold_owner = HoldOwner::Feed(feed_id);

    // ADR 0076 Decision 5: the gate of each compared fact-backed field. The
    // `musicindex` fact rows are evidence, so they are always written. A
    // held field keeps its `rss` fact row.
    let mut held = Vec::new();
    for (field, value) in [
        (
            RssField::Description,
            feed.description.clone().map(serde_json::Value::String),
        ),
        (
            RssField::Language,
            feed.language.clone().map(serde_json::Value::String),
        ),
        (
            RssField::Explicit,
            feed.explicit.map(serde_json::Value::Bool),
        ),
        (
            RssField::Owner,
            feed.publisher_text.clone().map(serde_json::Value::String),
        ),
    ] {
        let decision = rss_field_holds::musicindex_gate(
            conn,
            MusicIndexClaim {
                owner: hold_owner,
                field,
                value,
                updated_at: feed.updated_at,
            },
        )?;
        if !decision.writes() {
            held.push(field);
        }
    }

    for (source, mut facts) in grouped {
        if source == RSS_SOURCE && held.contains(&RssField::Description) {
            facts.retain(|fact| fact.fact_key != "description");
        }
        if source != MUSICINDEX_SOURCE {
            facts = merge_existing_metadata_facts_for_partial_source(conn, owner, &source, facts)?;
        }
        db::replace_local_metadata_facts(conn, owner, &source, &facts)?;
    }

    Ok(())
}

fn persist_track_metadata_facts(
    conn: &mut Connection,
    track_id: i64,
    hold_owner: HoldOwner,
    track: &Track,
) -> Result<()> {
    // ADR 0076 Decision 5: the gate of each compared fact-backed field. The
    // `musicindex` fact rows are evidence, so they are always written.
    for (field, value) in [
        (
            RssField::Description,
            track.description.clone().map(serde_json::Value::String),
        ),
        (
            RssField::Date,
            track
                .pub_date
                .map(|instant| serde_json::json!({ "instant": instant })),
        ),
        (
            RssField::Explicit,
            track.explicit.map(serde_json::Value::Bool),
        ),
    ] {
        rss_field_holds::musicindex_gate(
            conn,
            MusicIndexClaim {
                owner: hold_owner,
                field,
                value,
                updated_at: track.updated_at,
            },
        )?;
    }
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
}
