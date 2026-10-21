//! Saved searches (ADR 0084 Decision 3, packet 012).
//!
//! Migration 21 creates `saved_searches`. Each row is one toolbar search
//! that the operator saved: the trimmed query text, the tab that was open
//! at the save, and the time of the save. Discover shows each row as a
//! view after "New releases".
//!
//! This module is the only writer of the table. The caller gives each
//! time, so no function here reads the clock.

#![warn(clippy::pedantic)]

use anyhow::{Context, Result};
use rusqlite::{params, Connection, ErrorCode};

/// The read contract of `saved_searches` after migration 21.
pub(super) const COLUMNS: &[(&str, &[&str])] = &[(
    "saved_searches",
    &["id", "query", "start_tab", "created_at"],
)];

const DDL: &str = r"
CREATE TABLE saved_searches (
    id INTEGER PRIMARY KEY,
    query TEXT NOT NULL,
    start_tab TEXT NOT NULL CHECK (start_tab IN ('releases', 'artists', 'tracks')),
    created_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX saved_searches_query ON saved_searches (query COLLATE NOCASE);
";

/// Migration 21 (ADR 0016 registry, ADR 0084 Decision 3).
pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(DDL)
        .context("Create the saved search table")
}

/// The tab that was open when the operator saved a search. It is the
/// first tab that the saved search shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StoredStartTab {
    Releases,
    Artists,
    Tracks,
}

impl StoredStartTab {
    /// The stored text of this tab.
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Releases => "releases",
            Self::Artists => "artists",
            Self::Tracks => "tracks",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "releases" => Some(Self::Releases),
            "artists" => Some(Self::Artists),
            "tracks" => Some(Self::Tracks),
            _ => None,
        }
    }
}

/// One stored saved search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StoredSavedSearch {
    pub(crate) id: i64,
    pub(crate) query: String,
    pub(crate) start_tab: StoredStartTab,
    pub(crate) created_at: i64,
}

/// The result of [`insert_saved_search`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SavedSearchInsert {
    /// The search is saved with this id.
    Inserted(i64),
    /// A saved search with the same query, without case, is already
    /// stored. The table does not change.
    AlreadySaved,
}

/// Saves the query `query` with its start tab. `created_at` is the time of
/// the save in Unix seconds, from the caller. A query that is already
/// saved, also with another case, gives [`SavedSearchInsert::AlreadySaved`].
pub(crate) fn insert_saved_search(
    conn: &Connection,
    query: &str,
    start_tab: StoredStartTab,
    created_at: i64,
) -> Result<SavedSearchInsert> {
    let query = query.trim();
    anyhow::ensure!(!query.is_empty(), "A saved search needs a query");
    match conn.execute(
        "INSERT INTO saved_searches (query, start_tab, created_at) VALUES (?1, ?2, ?3)",
        params![query, start_tab.as_str(), created_at],
    ) {
        Ok(_) => Ok(SavedSearchInsert::Inserted(conn.last_insert_rowid())),
        Err(rusqlite::Error::SqliteFailure(error, _))
            if error.code == ErrorCode::ConstraintViolation =>
        {
            Ok(SavedSearchInsert::AlreadySaved)
        }
        Err(error) => Err(error).context("Save the search"),
    }
}

/// Each saved search, in `id` order.
pub(crate) fn saved_searches(conn: &Connection) -> Result<Vec<StoredSavedSearch>> {
    let mut statement = conn
        .prepare("SELECT id, query, start_tab, created_at FROM saved_searches ORDER BY id")
        .context("Read the saved searches")?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .context("Read the saved searches")?;
    let mut searches = Vec::new();
    for row in rows {
        let (id, query, start_tab, created_at) = row.context("Read a saved search")?;
        let start_tab = StoredStartTab::parse(&start_tab)
            .with_context(|| format!("Saved search {id} has the unknown tab {start_tab}"))?;
        searches.push(StoredSavedSearch {
            id,
            query,
            start_tab,
            created_at,
        });
    }
    Ok(searches)
}

/// Deletes the saved search `id`. Gives `true` when a row was deleted.
pub(crate) fn delete_saved_search(conn: &Connection, id: i64) -> Result<bool> {
    let deleted = conn
        .execute("DELETE FROM saved_searches WHERE id = ?1", params![id])
        .context("Delete the saved search")?;
    Ok(deleted > 0)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{
        delete_saved_search, insert_saved_search, saved_searches, SavedSearchInsert, StoredStartTab,
    };
    use crate::db::{inspect_schema, upgrades, SchemaCompatibility, CURRENT_VERSION, MIGRATIONS};

    /// R12-01 (ADR 0084 Decision 3): migration 21 adds `saved_searches` to
    /// a schema 20 database. Each retained record stays, the new table is
    /// empty, and `CURRENT_VERSION` is 21.
    #[test]
    fn adr_0084_saved_search_r12_01_version_20_migrates_to_21() {
        let conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, 20).unwrap();
        conn.execute(
            "INSERT INTO feeds(id, feed_url, feed_guid, title) VALUES (1, 'https://example.test/one.xml', 'f1', 'One')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO playlists(id, name, description) VALUES (3, 'Friday Show', NULL)",
            [],
        )
        .unwrap();
        assert_eq!(
            inspect_schema(&conn).unwrap(),
            SchemaCompatibility::UpgradeRequired {
                applied: 20,
                current: MIGRATIONS.len()
            }
        );

        crate::db::migrate_schema_to(&conn, 21).unwrap();

        assert_eq!(CURRENT_VERSION, 21);
        assert_eq!(inspect_schema(&conn).unwrap(), SchemaCompatibility::Current);
        upgrades::verify_target(&conn, 21).unwrap();
        let name: String = conn
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 21",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "saved_searches");
        let feed: (String, String) = conn
            .query_row(
                "SELECT feed_guid, title FROM feeds WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(feed, ("f1".to_owned(), "One".to_owned()));
        let playlist: String = conn
            .query_row("SELECT name FROM playlists WHERE id = 3", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(playlist, "Friday Show");
        assert!(saved_searches(&conn).unwrap().is_empty());
    }

    /// R12-02 (ADR 0084 Decision 3): the three functions keep the `id`
    /// order. A second insert of "Bluegrass" after "bluegrass" gives
    /// `AlreadySaved` and changes no row.
    #[test]
    fn adr_0084_saved_search_r12_02_insert_list_delete_keep_id_order() {
        let conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, CURRENT_VERSION).unwrap();

        let first = insert_saved_search(&conn, " bluegrass ", StoredStartTab::Tracks, 100).unwrap();
        let second = insert_saved_search(&conn, "ambient", StoredStartTab::Releases, 200).unwrap();
        let third = insert_saved_search(&conn, "jazz", StoredStartTab::Artists, 300).unwrap();
        let (
            SavedSearchInsert::Inserted(first),
            SavedSearchInsert::Inserted(second),
            SavedSearchInsert::Inserted(third),
        ) = (first, second, third)
        else {
            panic!("each new query is inserted");
        };
        assert!(first < second && second < third);

        assert_eq!(
            insert_saved_search(&conn, "Bluegrass", StoredStartTab::Releases, 400).unwrap(),
            SavedSearchInsert::AlreadySaved
        );

        let stored = saved_searches(&conn).unwrap();
        assert_eq!(
            stored
                .iter()
                .map(|search| (search.id, search.query.as_str(), search.start_tab))
                .collect::<Vec<_>>(),
            vec![
                (first, "bluegrass", StoredStartTab::Tracks),
                (second, "ambient", StoredStartTab::Releases),
                (third, "jazz", StoredStartTab::Artists),
            ],
            "the query is trimmed, and the rows come in id order"
        );
        assert_eq!(stored[0].created_at, 100);

        assert!(delete_saved_search(&conn, second).unwrap());
        assert!(!delete_saved_search(&conn, second).unwrap());
        assert_eq!(
            saved_searches(&conn)
                .unwrap()
                .iter()
                .map(|search| search.id)
                .collect::<Vec<_>>(),
            vec![first, third]
        );
    }

    /// The table refuses an empty query and a tab outside the three tabs.
    #[test]
    fn adr_0084_saved_search_empty_query_and_unknown_tab_are_refused() {
        let conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, CURRENT_VERSION).unwrap();
        assert!(insert_saved_search(&conn, "   ", StoredStartTab::Releases, 1).is_err());
        assert!(conn
            .execute(
                "INSERT INTO saved_searches (query, start_tab, created_at) VALUES ('x', 'feeds', 1)",
                [],
            )
            .is_err());
    }
}
