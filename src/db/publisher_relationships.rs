//! Publisher relationship storage for Library feeds (ADR 0077 Decision 5).
//!
//! Migration 14 creates `feed_publisher_relationships`. The table keeps one
//! row for each publisher relationship entry of a local feed, with each
//! field of the live `PublisherResponse` contract and the observation time.
//!
//! Packet 013 keeps `MusicIndex` collection replacement disabled. Thus a write
//! inserts or updates the rows of the entries in a response, and it deletes
//! no row. A later packet enables replacement through the packet 013
//! registry.

#![warn(clippy::pedantic)]

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::api::PublisherRelationship;

pub(super) const COLUMNS: &[(&str, &[&str])] = &[(
    "feed_publisher_relationships",
    &[
        "feed_id",
        "direction",
        "publisher_feed_guid",
        "remote_feed_guid",
        "music_feed_guid",
        "remote_feed_url",
        "remote_feed_medium",
        "publisher_feed_url",
        "music_feed_url",
        "music_names_publisher",
        "publisher_lists_music",
        "publisher_link_resolution",
        "publisher_link_observed_at",
        "reciprocal_declared",
        "reciprocal_medium",
        "two_way_validated",
        "publisher_rel",
        "music_rel",
        "role",
        "role_source",
        "publisher_feed_title",
        "observed_at",
    ],
)];

const DDL: &str = r"
CREATE TABLE feed_publisher_relationships (
    feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
    direction TEXT NOT NULL CHECK (direction != ''),
    publisher_feed_guid TEXT NOT NULL CHECK (publisher_feed_guid != ''),
    remote_feed_guid TEXT NOT NULL CHECK (remote_feed_guid != ''),
    music_feed_guid TEXT NULL,
    remote_feed_url TEXT NULL,
    remote_feed_medium TEXT NULL,
    publisher_feed_url TEXT NULL,
    music_feed_url TEXT NULL,
    music_names_publisher INTEGER NULL CHECK (music_names_publisher IN (0, 1)),
    publisher_lists_music INTEGER NULL CHECK (publisher_lists_music IN (0, 1)),
    publisher_link_resolution TEXT NULL,
    publisher_link_observed_at INTEGER NULL,
    reciprocal_declared INTEGER NULL CHECK (reciprocal_declared IN (0, 1)),
    reciprocal_medium TEXT NULL,
    two_way_validated INTEGER NULL CHECK (two_way_validated IN (0, 1)),
    publisher_rel TEXT NULL,
    music_rel TEXT NULL,
    role TEXT NULL,
    role_source TEXT NULL,
    publisher_feed_title TEXT NULL,
    observed_at INTEGER NOT NULL,
    PRIMARY KEY (feed_id, direction, publisher_feed_guid, remote_feed_guid)
);
";

/// Migration 14 (ADR 0016 registry, ADR 0077 Decision 5).
pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(DDL)
        .context("Create feed publisher relationship table")
}

/// Returns the value when it is present and not empty.
fn nonempty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.is_empty())
}

/// Insert or update one row for each keyed entry of one response.
///
/// `relationships` is the `publisher` value of the response. An absent or
/// null value (`None`) and an empty array change no row. An entry that the
/// response omits keeps its row. The key is `direction`,
/// `publisher_feed_guid` and `remote_feed_guid`. An entry without one of
/// these values has no key, so it writes no row.
///
/// On a `music_to_publisher` entry, `remote_feed_guid` is the publisher
/// feed GUID. On a `publisher_to_music` entry, it is the listed feed GUID.
/// Thus each entry of a publisher feed gets its own row.
///
/// `publisher_feed_title` comes from the same feed response. `observed_at`
/// is the Unix time in seconds of the observation that supplied the
/// response. The function returns the number of rows that it wrote.
pub(crate) fn upsert_feed_publisher_relationships(
    conn: &mut Connection,
    feed_id: i64,
    publisher_feed_title: Option<&str>,
    relationships: Option<&[PublisherRelationship]>,
    observed_at: i64,
) -> Result<usize> {
    let Some(relationships) = relationships.filter(|entries| !entries.is_empty()) else {
        return Ok(0);
    };
    let transaction = conn
        .savepoint()
        .context("Begin feed publisher relationship write")?;
    let mut written = 0;
    {
        let mut statement = transaction.prepare(
            "INSERT INTO feed_publisher_relationships (
                feed_id, direction, publisher_feed_guid, remote_feed_guid, music_feed_guid,
                remote_feed_url, remote_feed_medium, publisher_feed_url, music_feed_url,
                music_names_publisher, publisher_lists_music, publisher_link_resolution,
                publisher_link_observed_at, reciprocal_declared, reciprocal_medium,
                two_way_validated, publisher_rel, music_rel, role, role_source,
                publisher_feed_title, observed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                ?17, ?18, ?19, ?20, ?21, ?22)
            ON CONFLICT (feed_id, direction, publisher_feed_guid, remote_feed_guid) DO UPDATE SET
                music_feed_guid = excluded.music_feed_guid,
                remote_feed_url = excluded.remote_feed_url,
                remote_feed_medium = excluded.remote_feed_medium,
                publisher_feed_url = excluded.publisher_feed_url,
                music_feed_url = excluded.music_feed_url,
                music_names_publisher = excluded.music_names_publisher,
                publisher_lists_music = excluded.publisher_lists_music,
                publisher_link_resolution = excluded.publisher_link_resolution,
                publisher_link_observed_at = excluded.publisher_link_observed_at,
                reciprocal_declared = excluded.reciprocal_declared,
                reciprocal_medium = excluded.reciprocal_medium,
                two_way_validated = excluded.two_way_validated,
                publisher_rel = excluded.publisher_rel,
                music_rel = excluded.music_rel,
                role = excluded.role,
                role_source = excluded.role_source,
                publisher_feed_title = excluded.publisher_feed_title,
                observed_at = excluded.observed_at",
        )?;
        for entry in relationships {
            let (Some(direction), Some(publisher_feed_guid), Some(remote_feed_guid)) = (
                nonempty(entry.direction.as_deref()),
                nonempty(entry.publisher_feed_guid.as_deref()),
                nonempty(entry.remote_feed_guid.as_deref()),
            ) else {
                continue;
            };
            statement
                .execute(params![
                    feed_id,
                    direction,
                    publisher_feed_guid,
                    remote_feed_guid,
                    entry.music_feed_guid,
                    entry.remote_feed_url,
                    entry.remote_feed_medium,
                    entry.publisher_feed_url,
                    entry.music_feed_url,
                    entry.music_names_publisher,
                    entry.publisher_lists_music,
                    entry
                        .publisher_link_resolution
                        .as_ref()
                        .map(crate::api::PublisherLinkResolution::as_str),
                    entry.publisher_link_observed_at,
                    entry.reciprocal_declared,
                    entry.reciprocal_medium,
                    entry.two_way_validated,
                    entry.publisher_rel,
                    entry.music_rel,
                    entry.role,
                    entry
                        .role_source
                        .as_ref()
                        .map(crate::api::RoleSource::as_str),
                    publisher_feed_title,
                    observed_at,
                ])
                .context("Write feed publisher relationship")?;
            written += 1;
        }
    }
    transaction
        .commit()
        .context("Commit feed publisher relationship write")?;
    Ok(written)
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Test-only reads. Packets 003 and 004 add the production readers.

    use rusqlite::types::Value;
    use rusqlite::Connection;

    /// Each stored row of one feed as column name and value, in key order.
    pub(crate) fn rows(conn: &Connection, feed_id: i64) -> Vec<Vec<(String, Value)>> {
        let mut statement = conn
            .prepare(
                "SELECT * FROM feed_publisher_relationships WHERE feed_id = ?1
                 ORDER BY direction, publisher_feed_guid, remote_feed_guid",
            )
            .unwrap();
        let names = statement
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        statement
            .query_map([feed_id], |row| {
                names
                    .iter()
                    .enumerate()
                    .map(|(index, name)| Ok((name.clone(), row.get::<_, Value>(index)?)))
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
    }

    /// One column of one stored row.
    pub(crate) fn value(row: &[(String, Value)], column: &str) -> Value {
        row.iter()
            .find(|(name, _)| name == column)
            .map_or_else(|| panic!("no column {column}"), |(_, value)| value.clone())
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::db::{inspect_schema, upgrades, SchemaCompatibility, CURRENT_VERSION, MIGRATIONS};

    fn entry(publisher_feed_guid: &str, role: Option<&str>) -> PublisherRelationship {
        PublisherRelationship {
            direction: Some("music_to_publisher".to_owned()),
            publisher_feed_guid: Some(publisher_feed_guid.to_owned()),
            remote_feed_guid: Some(publisher_feed_guid.to_owned()),
            role: role.map(str::to_owned),
            ..PublisherRelationship::default()
        }
    }

    /// R2-06: a version 13 database migrates to version 14 and has an empty
    /// `feed_publisher_relationships` table.
    #[test]
    fn adr_0077_publisher_relationship_version_13_migrates_to_14_with_empty_table() {
        let conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, 13).unwrap();
        conn.execute(
            "INSERT INTO feeds(id, feed_url, feed_guid, title) VALUES (1, 'https://example.test/feed.xml', 'f1', 'Album')",
            [],
        )
        .unwrap();
        assert_eq!(
            inspect_schema(&conn).unwrap(),
            SchemaCompatibility::UpgradeRequired {
                applied: 13,
                current: MIGRATIONS.len()
            }
        );
        let table_count = |conn: &Connection| -> i64 {
            conn.query_row(
                "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name = 'feed_publisher_relationships'",
                [],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(table_count(&conn), 0);

        crate::db::migrate_schema_to(&conn, 14).unwrap();

        assert_eq!(CURRENT_VERSION, 14);
        assert_eq!(table_count(&conn), 1);
        let rows: i64 = conn
            .query_row(
                "SELECT count(*) FROM feed_publisher_relationships",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(rows, 0);
        let name: String = conn
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 14",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "feed_publisher_relationships");
        assert_eq!(inspect_schema(&conn).unwrap(), SchemaCompatibility::Current);
        upgrades::verify_target(&conn, 14).unwrap();
        let columns = conn
            .prepare("SELECT * FROM feed_publisher_relationships")
            .unwrap()
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert_eq!(columns, COLUMNS[0].1);
    }

    /// R2-06: a failure in migration 14 rolls back its table and its ledger record.
    #[test]
    fn adr_0077_publisher_relationship_failed_migration_14_leaves_version_13() {
        let conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, 13).unwrap();
        let result = crate::db::migrate_schema_with(&conn, 14, |version, reached| {
            anyhow::ensure!(
                version != 14 || reached != crate::db::MigrationBoundary::AfterRecord,
                "fixture stop after migration 14"
            );
            Ok(())
        });
        assert!(result.is_err());
        upgrades::verify_target(&conn, 13).unwrap();
        assert_eq!(
            inspect_schema(&conn).unwrap(),
            SchemaCompatibility::UpgradeRequired {
                applied: 13,
                current: MIGRATIONS.len()
            }
        );
    }

    /// R2-11: a feed removal deletes the rows of that feed and keeps the rows
    /// of another feed.
    #[test]
    fn adr_0077_publisher_relationship_feed_removal_deletes_its_rows() {
        let mut conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, CURRENT_VERSION).unwrap();
        conn.execute_batch(
            "INSERT INTO feeds(id, feed_url, feed_guid) VALUES (1, 'https://example.test/one.xml', 'f1');
             INSERT INTO feeds(id, feed_url, feed_guid) VALUES (2, 'https://example.test/two.xml', 'f2');",
        )
        .unwrap();
        let entries = [
            entry("publisher-a", Some("artist")),
            entry("publisher-b", None),
        ];
        assert_eq!(
            upsert_feed_publisher_relationships(&mut conn, 1, Some("A"), Some(&entries), 10)
                .unwrap(),
            2
        );
        assert_eq!(
            upsert_feed_publisher_relationships(&mut conn, 2, None, Some(&entries[..1]), 10)
                .unwrap(),
            1
        );

        conn.execute("DELETE FROM feeds WHERE id = 1", []).unwrap();

        assert!(test_support::rows(&conn, 1).is_empty());
        assert_eq!(test_support::rows(&conn, 2).len(), 1);
    }

    /// The recorded publisher feed response of R2-02 stores one row for each
    /// of its nine `publisher_to_music` entries. The entries share
    /// `direction` and `publisher_feed_guid`, and `remote_feed_guid` keeps
    /// them apart.
    #[test]
    fn adr_0077_publisher_relationship_recorded_publisher_feed_stores_one_row_per_entry() {
        use crate::api::tests::adr_0077_publisher_relationship::RECORDED_PUBLISHER_RESPONSE;
        let mut conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, CURRENT_VERSION).unwrap();
        conn.execute(
            "INSERT INTO feeds(id, feed_url, feed_guid) VALUES (1, 'https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979', 'bcbe7207-9338-474e-ba18-09e6b1b69979')",
            [],
        )
        .unwrap();
        let feed = serde_json::from_str::<crate::api::DetailResponse<crate::api::Feed>>(
            RECORDED_PUBLISHER_RESPONSE,
        )
        .unwrap()
        .data;
        let entries = feed.publisher.unwrap();
        assert_eq!(entries.len(), 9);

        let written = upsert_feed_publisher_relationships(
            &mut conn,
            1,
            feed.publisher_feed_title.as_deref(),
            Some(&entries),
            1_790_253_973,
        )
        .unwrap();

        assert_eq!(written, 9);
        let rows = test_support::rows(&conn, 1);
        assert_eq!(rows.len(), 9);
        let mut stored = rows
            .iter()
            .map(|row| {
                (
                    test_support::value(row, "remote_feed_guid"),
                    test_support::value(row, "music_feed_guid"),
                )
            })
            .collect::<Vec<_>>();
        let mut recorded = entries
            .iter()
            .map(|entry| {
                (
                    rusqlite::types::Value::Text(entry.remote_feed_guid.clone().unwrap()),
                    rusqlite::types::Value::Text(entry.music_feed_guid.clone().unwrap()),
                )
            })
            .collect::<Vec<_>>();
        stored.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        recorded.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        assert_eq!(stored, recorded);
        for row in &rows {
            assert_eq!(
                test_support::value(row, "direction"),
                rusqlite::types::Value::Text("publisher_to_music".into())
            );
            assert_eq!(
                test_support::value(row, "publisher_feed_guid"),
                rusqlite::types::Value::Text("bcbe7207-9338-474e-ba18-09e6b1b69979".into())
            );
        }
    }

    /// An entry without a key writes no row, and a missing feed is refused.
    #[test]
    fn adr_0077_publisher_relationship_unkeyed_entries_write_no_row() {
        let mut conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, CURRENT_VERSION).unwrap();
        conn.execute(
            "INSERT INTO feeds(id, feed_url, feed_guid) VALUES (1, 'https://example.test/one.xml', 'f1')",
            [],
        )
        .unwrap();
        let entries = [
            PublisherRelationship {
                direction: None,
                ..entry("publisher-a", None)
            },
            PublisherRelationship {
                publisher_feed_guid: Some(String::new()),
                ..entry("unused", None)
            },
            PublisherRelationship {
                remote_feed_guid: None,
                ..entry("publisher-c", None)
            },
        ];
        assert_eq!(
            upsert_feed_publisher_relationships(&mut conn, 1, None, Some(&entries), 10).unwrap(),
            0
        );
        assert!(test_support::rows(&conn, 1).is_empty());
        assert!(upsert_feed_publisher_relationships(
            &mut conn,
            99,
            None,
            Some(&[entry("publisher-a", None)]),
            10
        )
        .is_err());
        assert!(test_support::rows(&conn, 99).is_empty());
    }
}
