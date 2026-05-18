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

pub(crate) fn feed_facts(conn: &Connection, feed_id: i64) -> Result<FeedMetadataFacts> {
    let rows = db::local_metadata_facts(conn, LocalMetadataOwner::Feed(feed_id))?;
    Ok(feed_facts_from_rows(rows))
}

pub(crate) fn track_facts(conn: &Connection, track_id: i64) -> Result<TrackMetadataFacts> {
    let rows = db::local_metadata_facts(conn, LocalMetadataOwner::Track(track_id))?;
    Ok(track_facts_from_rows(rows))
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

        let facts = track_facts(&conn, 11)?;

        assert_eq!(facts.publisher_text.as_deref(), Some("Example Publisher"));
        assert_eq!(facts.description.as_deref(), Some("Track description"));
        assert_eq!(facts.pub_date, Some(1_700_000_000));
        assert_eq!(facts.explicit, Some(true));
        Ok(())
    }
}
