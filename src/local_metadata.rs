//! Local metadata source-fact hydration helpers.
//!
//! This module maps persisted metadata source-fact rows into GPUI-free view
//! projections. UI shells and view models receive the projected facts instead
//! of querying metadata storage.

#![warn(clippy::pedantic)]

use anyhow::Result;
use rusqlite::Connection;

use crate::db::{self, LocalMetadataOwner, LocalMetadataValue};
use crate::metadata::drop_placeholder_source_text;
use crate::views::{FeedMetadataFacts, TrackMetadataFacts};

/// The source token of the facts that a `MusicIndex` response supplies.
const MUSICINDEX_SOURCE: &str = "musicindex";
/// The source token of the facts that the RSS parse and the playlist RSS
/// check write.
const RSS_SOURCE: &str = "rss";

/// The decoded fact rows of one owner, one set for each source that the
/// stored value projection reads (ADR 0075 packet 020).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SourcedFacts<T> {
    /// The `musicindex` rows.
    pub(crate) musicindex: T,
    /// The `rss` rows.
    pub(crate) rss: T,
}

/// The `MusicIndex` facts of a feed. The Library uses them to find out if
/// an album has had its `MusicIndex` hydration.
pub(crate) fn feed_facts(conn: &Connection, feed_id: i64) -> Result<FeedMetadataFacts> {
    Ok(sourced_feed_facts(conn, feed_id)?.musicindex)
}

pub(crate) fn sourced_feed_facts(
    conn: &Connection,
    feed_id: i64,
) -> Result<SourcedFacts<FeedMetadataFacts>> {
    let rows = db::local_metadata_facts(conn, LocalMetadataOwner::Feed(feed_id))?;
    let (musicindex, rss) = split_by_source(rows);
    Ok(SourcedFacts {
        musicindex: feed_facts_from_rows(musicindex),
        rss: feed_facts_from_rows(rss),
    })
}

pub(crate) fn sourced_track_facts(
    conn: &Connection,
    track_id: i64,
) -> Result<SourcedFacts<TrackMetadataFacts>> {
    let rows = db::local_metadata_facts(conn, LocalMetadataOwner::Track(track_id))?;
    let (musicindex, rss) = split_by_source(rows);
    Ok(SourcedFacts {
        musicindex: track_facts_from_rows(musicindex),
        rss: track_facts_from_rows(rss),
    })
}

/// Keep the `musicindex` rows and the `rss` rows apart. A row of another
/// source token is evidence only, and the projection does not read it.
fn split_by_source(
    rows: Vec<db::LocalMetadataFactRow>,
) -> (Vec<db::LocalMetadataFactRow>, Vec<db::LocalMetadataFactRow>) {
    let mut musicindex = Vec::new();
    let mut rss = Vec::new();
    for row in rows {
        match row.source.as_str() {
            MUSICINDEX_SOURCE => musicindex.push(row),
            RSS_SOURCE => rss.push(row),
            _ => {}
        }
    }
    (musicindex, rss)
}

fn feed_facts_from_rows(rows: Vec<db::LocalMetadataFactRow>) -> FeedMetadataFacts {
    let mut facts = FeedMetadataFacts::default();
    let mut top_level_description = None;
    for row in rows {
        match row.fact_key.as_str() {
            "publisher_text" if facts.publisher_text.is_none() => {
                facts.publisher_text = text_value(row.value);
            }
            "musicindex_release_kind" if facts.release_kind.is_none() => {
                facts.release_kind = text_value(row.value);
            }
            "release_date" if facts.release_date.is_none() => {
                facts.release_date = integer_value(&row.value);
            }
            "channel_pub_date" if facts.channel_pub_date.is_none() => {
                facts.channel_pub_date = integer_value(&row.value);
            }
            "feed_pub_date_claim" if facts.pub_date_claim.is_none() => {
                facts.pub_date_claim = text_value(row.value);
            }
            "language" if facts.language.is_none() => {
                facts.language = text_value(row.value);
            }
            "explicit" if facts.explicit.is_none() => {
                facts.explicit = boolean_value(&row.value);
            }
            "description" => {
                let description = text_value(row.value);
                if row.source == "musicindex"
                    && row.extraction_path.as_deref() == Some("$.description")
                {
                    top_level_description = top_level_description.or(description);
                } else {
                    facts.description = facts.description.or(description);
                }
            }
            _ => {}
        }
    }
    facts.description = facts.description.or(top_level_description);
    facts
}

fn track_facts_from_rows(rows: Vec<db::LocalMetadataFactRow>) -> TrackMetadataFacts {
    let mut facts = TrackMetadataFacts::default();
    for row in rows {
        match row.fact_key.as_str() {
            "publisher_text" if facts.publisher_text.is_none() => {
                facts.publisher_text = text_value(row.value);
            }
            "description" if facts.description.is_none() => {
                facts.description = text_value(row.value);
            }
            "pub_date" if facts.pub_date.is_none() => {
                facts.pub_date = integer_value(&row.value);
            }
            "explicit" if facts.explicit.is_none() => {
                facts.explicit = boolean_value(&row.value);
            }
            _ => {}
        }
    }
    facts
}

fn text_value(value: LocalMetadataValue) -> Option<String> {
    match value {
        LocalMetadataValue::Text(value) => {
            drop_placeholder_source_text(Some(value.trim().to_string()))
        }
        LocalMetadataValue::Integer(_) | LocalMetadataValue::Boolean(_) => None,
    }
}

fn integer_value(value: &LocalMetadataValue) -> Option<i64> {
    match value {
        LocalMetadataValue::Integer(value) => Some(*value),
        LocalMetadataValue::Text(_) | LocalMetadataValue::Boolean(_) => None,
    }
}

fn boolean_value(value: &LocalMetadataValue) -> Option<bool> {
    match value {
        LocalMetadataValue::Boolean(value) => Some(*value),
        LocalMetadataValue::Text(_) | LocalMetadataValue::Integer(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_facts_projects_supported_feed_metadata_rows() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::init_schema(&conn)?;
        conn.execute(
            "INSERT INTO feeds (id, feed_url, title) VALUES (7, ?1, ?2)",
            rusqlite::params!["https://example.test/feed.xml", "Example Feed"],
        )?;
        db::replace_local_metadata_facts(
            &mut conn,
            LocalMetadataOwner::Feed(7),
            "musicindex",
            &[
                db::LocalMetadataFactInput {
                    fact_key: "publisher_text".into(),
                    value: LocalMetadataValue::Text("Example Publisher".into()),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
                db::LocalMetadataFactInput {
                    fact_key: "musicindex_release_kind".into(),
                    value: LocalMetadataValue::Text("album".into()),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
                db::LocalMetadataFactInput {
                    fact_key: "release_date".into(),
                    value: LocalMetadataValue::Integer(1_700_000_000),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
                db::LocalMetadataFactInput {
                    fact_key: "language".into(),
                    value: LocalMetadataValue::Text("en".into()),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
                db::LocalMetadataFactInput {
                    fact_key: "explicit".into(),
                    value: LocalMetadataValue::Boolean(true),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
                db::LocalMetadataFactInput {
                    fact_key: "description".into(),
                    value: LocalMetadataValue::Text("MusicIndex description".into()),
                    extraction_path: Some("$.description".into()),
                    observed_at: None,
                    raw_json: None,
                },
            ],
        )?;

        let facts = feed_facts(&conn, 7)?;

        assert_eq!(facts.publisher_text.as_deref(), Some("Example Publisher"));
        assert_eq!(facts.release_kind.as_deref(), Some("album"));
        assert_eq!(facts.release_date, Some(1_700_000_000));
        assert_eq!(facts.language.as_deref(), Some("en"));
        assert_eq!(facts.explicit, Some(true));
        assert_eq!(facts.description.as_deref(), Some("MusicIndex description"));
        Ok(())
    }

    #[test]
    fn feed_facts_prefers_description_claim_over_musicindex_top_level() {
        let facts = feed_facts_from_rows(vec![
            db::LocalMetadataFactRow {
                fact_key: "description".into(),
                value: LocalMetadataValue::Text("MusicIndex description".into()),
                source: "musicindex".into(),
                extraction_path: Some("$.description".into()),
                observed_at: None,
                raw_json: None,
            },
            db::LocalMetadataFactRow {
                fact_key: "description".into(),
                value: LocalMetadataValue::Text("RSS description".into()),
                source: "rss".into(),
                extraction_path: Some("$.channel.description".into()),
                observed_at: None,
                raw_json: None,
            },
        ]);

        assert_eq!(facts.description.as_deref(), Some("RSS description"));
    }

    /// ADR 0075 packet 050: `channel_pub_date` and `feed_pub_date_claim`
    /// project onto their own fields, apart from `release_date`'s
    /// oldest-item meaning.
    #[test]
    fn feed_facts_project_channel_pub_date_and_pub_date_claim() {
        let facts = feed_facts_from_rows(vec![
            db::LocalMetadataFactRow {
                fact_key: "channel_pub_date".into(),
                value: LocalMetadataValue::Integer(1_789_905_600),
                source: "rss".into(),
                extraction_path: Some("channel.pub_date".into()),
                observed_at: None,
                raw_json: None,
            },
            db::LocalMetadataFactRow {
                fact_key: "feed_pub_date_claim".into(),
                value: LocalMetadataValue::Text("1704067200".into()),
                source: "musicindex".into(),
                extraction_path: Some("feed.pub_date".into()),
                observed_at: None,
                raw_json: None,
            },
        ]);

        assert_eq!(facts.channel_pub_date, Some(1_789_905_600));
        assert_eq!(facts.pub_date_claim.as_deref(), Some("1704067200"));
    }

    #[test]
    fn track_facts_projects_supported_track_metadata_rows() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::init_schema(&conn)?;
        conn.execute(
            "INSERT INTO feeds (id, feed_url, title) VALUES (7, ?1, ?2)",
            rusqlite::params!["https://example.test/feed.xml", "Example Feed"],
        )?;
        conn.execute(
            "INSERT INTO tracks (id, feed_id, item_guid) VALUES (11, 7, ?1)",
            rusqlite::params!["track-guid"],
        )?;
        db::replace_local_metadata_facts(
            &mut conn,
            LocalMetadataOwner::Track(11),
            "musicindex",
            &[
                db::LocalMetadataFactInput {
                    fact_key: "publisher_text".into(),
                    value: LocalMetadataValue::Text("Example Publisher".into()),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
                db::LocalMetadataFactInput {
                    fact_key: "description".into(),
                    value: LocalMetadataValue::Text("Track description".into()),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
                db::LocalMetadataFactInput {
                    fact_key: "pub_date".into(),
                    value: LocalMetadataValue::Integer(1_700_000_000),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
                db::LocalMetadataFactInput {
                    fact_key: "explicit".into(),
                    value: LocalMetadataValue::Boolean(true),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
            ],
        )?;

        let facts = sourced_track_facts(&conn, 11)?.musicindex;

        assert_eq!(facts.publisher_text.as_deref(), Some("Example Publisher"));
        assert_eq!(facts.description.as_deref(), Some("Track description"));
        assert_eq!(facts.pub_date, Some(1_700_000_000));
        assert_eq!(facts.explicit, Some(true));
        Ok(())
    }
}
