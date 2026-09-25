//! Playlist RSS check runs (ADR 0076 Decision 2, packet 001).
//!
//! Migration 15 creates `rss_check_runs` and `rss_check_feed_results`.
//! A run row records one check of the feeds of one playlist. A result row
//! records the fetch outcome of one feed of that run. Each result write also
//! updates the counts of its run in the same transaction, so the counts
//! always agree with the result rows.

#![warn(clippy::pedantic)]

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

pub(super) const COLUMNS: &[(&str, &[&str])] = &[
    (
        "rss_check_runs",
        &[
            "id",
            "playlist_id",
            "trigger",
            "started_at_us",
            "finished_at_us",
            "document_count",
            "not_modified_count",
            "failed_count",
            "not_checked_count",
        ],
    ),
    (
        "rss_check_feed_results",
        &[
            "run_id",
            "feed_id",
            "outcome",
            "http_status",
            "observation_id",
            "message",
        ],
    ),
];

const DDL: &str = r"
CREATE TABLE rss_check_runs (
    id INTEGER NOT NULL PRIMARY KEY,
    playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    trigger TEXT NOT NULL CHECK (trigger IN ('button', 'playback_start')),
    started_at_us INTEGER NOT NULL,
    finished_at_us INTEGER NULL,
    document_count INTEGER NOT NULL DEFAULT 0 CHECK (document_count >= 0),
    not_modified_count INTEGER NOT NULL DEFAULT 0 CHECK (not_modified_count >= 0),
    failed_count INTEGER NOT NULL DEFAULT 0 CHECK (failed_count >= 0),
    not_checked_count INTEGER NOT NULL DEFAULT 0 CHECK (not_checked_count >= 0),
    CHECK (finished_at_us IS NULL OR finished_at_us >= started_at_us)
);

CREATE INDEX rss_check_runs_playlist ON rss_check_runs(playlist_id, id);

CREATE TABLE rss_check_feed_results (
    run_id INTEGER NOT NULL REFERENCES rss_check_runs(id) ON DELETE CASCADE,
    feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
    outcome TEXT NOT NULL CHECK (outcome IN ('document', 'not_modified', 'failed', 'not_checked')),
    http_status INTEGER NULL CHECK (http_status IS NULL OR http_status BETWEEN 100 AND 599),
    observation_id INTEGER NULL REFERENCES metadata_observations(id) ON DELETE RESTRICT,
    message TEXT NULL,
    PRIMARY KEY (run_id, feed_id)
);
";

/// Migration 15 (ADR 0016 registry, ADR 0076 Decision 2).
pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(DDL)
        .context("Create playlist RSS check run tables")
}

/// What started a playlist RSS check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RssCheckTrigger {
    /// The operator selected "Check RSS" on the playlist page.
    Button,
    /// Playback started from the playlist (phase plan trigger mapping).
    PlaybackStart,
}

impl RssCheckTrigger {
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::PlaybackStart => "playback_start",
        }
    }

    fn from_token(token: &str) -> Option<Self> {
        match token {
            "button" => Some(Self::Button),
            "playback_start" => Some(Self::PlaybackStart),
            _ => None,
        }
    }
}

/// The fetch outcome of one feed in one check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RssFeedOutcome {
    /// The host sent the RSS document.
    Document,
    /// The host sent HTTP 304. The document did not change.
    NotModified,
    /// The request or the response failed.
    Failed,
    /// The check sent no request for this feed.
    NotChecked,
}

impl RssFeedOutcome {
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::NotModified => "not_modified",
            Self::Failed => "failed",
            Self::NotChecked => "not_checked",
        }
    }

    fn from_token(token: &str) -> Option<Self> {
        match token {
            "document" => Some(Self::Document),
            "not_modified" => Some(Self::NotModified),
            "failed" => Some(Self::Failed),
            "not_checked" => Some(Self::NotChecked),
            _ => None,
        }
    }

    fn count_column(self) -> &'static str {
        match self {
            Self::Document => "document_count",
            Self::NotModified => "not_modified_count",
            Self::Failed => "failed_count",
            Self::NotChecked => "not_checked_count",
        }
    }
}

/// One distinct feed of a playlist, in the order of its first track.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaylistCheckFeed {
    pub feed_id: i64,
    pub title: Option<String>,
    pub feed_url: String,
}

/// One stored feed result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredFeedResult {
    pub feed_id: i64,
    pub title: Option<String>,
    pub feed_url: Option<String>,
    pub outcome: RssFeedOutcome,
    pub http_status: Option<u16>,
    pub observation_id: Option<i64>,
    pub message: Option<String>,
}

/// One stored run with its results.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRun {
    pub id: i64,
    pub playlist_id: i64,
    pub trigger: RssCheckTrigger,
    pub started_at_us: i64,
    pub finished_at_us: Option<i64>,
    pub document_count: i64,
    pub not_modified_count: i64,
    pub failed_count: i64,
    pub not_checked_count: i64,
    pub results: Vec<StoredFeedResult>,
}

/// Read each distinct feed that has a track in the playlist.
///
/// # Errors
///
/// Returns an error when the database query fails.
pub fn playlist_check_feeds(conn: &Connection, playlist_id: i64) -> Result<Vec<PlaylistCheckFeed>> {
    let mut statement = conn
        .prepare(
            "SELECT f.id, f.title, f.feed_url
             FROM playlist_tracks pt
             JOIN tracks t ON t.id = pt.track_id
             JOIN feeds f ON f.id = t.feed_id
             WHERE pt.playlist_id = ?1
             GROUP BY f.id
             ORDER BY MIN(pt.position), f.id",
        )
        .context("prepare playlist_check_feeds")?;
    let rows = statement
        .query_map([playlist_id], |row| {
            Ok(PlaylistCheckFeed {
                feed_id: row.get(0)?,
                title: row.get(1)?,
                feed_url: row.get(2)?,
            })
        })
        .context("query playlist_check_feeds")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("collect playlist_check_feeds")?;
    Ok(rows)
}

/// Insert a run row with zero counts and no finish time.
///
/// # Errors
///
/// Returns an error when the database query fails.
pub fn insert_run(
    conn: &Connection,
    playlist_id: i64,
    trigger: RssCheckTrigger,
    started_at_us: i64,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO rss_check_runs(playlist_id, trigger, started_at_us) VALUES (?1, ?2, ?3)",
        params![playlist_id, trigger.token(), started_at_us],
    )
    .context("insert rss_check_runs")?;
    Ok(conn.last_insert_rowid())
}

/// Insert one feed result and add it to the counts of its run.
///
/// # Errors
///
/// Returns an error when the database query fails.
pub fn insert_feed_result(
    conn: &Connection,
    run_id: i64,
    feed_id: i64,
    outcome: RssFeedOutcome,
    http_status: Option<u16>,
    observation_id: Option<i64>,
    message: Option<&str>,
) -> Result<()> {
    let tx = conn
        .unchecked_transaction()
        .context("begin rss_check_feed_results")?;
    tx.execute(
        "INSERT INTO rss_check_feed_results(run_id, feed_id, outcome, http_status, observation_id, message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            run_id,
            feed_id,
            outcome.token(),
            http_status,
            observation_id,
            message
        ],
    )
    .context("insert rss_check_feed_results")?;
    let column = outcome.count_column();
    tx.execute(
        &format!("UPDATE rss_check_runs SET {column} = {column} + 1 WHERE id = ?1"),
        [run_id],
    )
    .context("update rss_check_runs counts")?;
    tx.commit().context("commit rss_check_feed_results")
}

/// Record the finish time of a run.
///
/// # Errors
///
/// Returns an error when the database query fails.
pub fn finish_run(conn: &Connection, run_id: i64, finished_at_us: i64) -> Result<()> {
    conn.execute(
        "UPDATE rss_check_runs SET finished_at_us = max(?2, started_at_us) WHERE id = ?1",
        params![run_id, finished_at_us],
    )
    .context("finish rss_check_runs")?;
    Ok(())
}

/// Read the latest run of a playlist with its results, in feed order.
///
/// # Errors
///
/// Returns an error when the database query fails.
pub fn latest_run(conn: &Connection, playlist_id: i64) -> Result<Option<StoredRun>> {
    let run = conn
        .query_row(
            "SELECT id, playlist_id, trigger, started_at_us, finished_at_us, document_count,
                    not_modified_count, failed_count, not_checked_count
             FROM rss_check_runs WHERE playlist_id = ?1 ORDER BY id DESC LIMIT 1",
            [playlist_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    [
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                    ],
                ))
            },
        )
        .optional()
        .context("query latest rss_check_runs")?;
    let Some((id, playlist_id, trigger, started_at_us, finished_at_us, counts)) = run else {
        return Ok(None);
    };
    let trigger = RssCheckTrigger::from_token(&trigger)
        .with_context(|| format!("unknown RSS check trigger {trigger:?}"))?;
    let mut statement = conn
        .prepare(
            "SELECT r.feed_id, f.title, f.feed_url, r.outcome, r.http_status, r.observation_id, r.message
             FROM rss_check_feed_results r
             LEFT JOIN feeds f ON f.id = r.feed_id
             WHERE r.run_id = ?1
             ORDER BY r.rowid",
        )
        .context("prepare rss_check_feed_results")?;
    let results = statement
        .query_map([id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        })
        .context("query rss_check_feed_results")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("collect rss_check_feed_results")?
        .into_iter()
        .map(
            |(feed_id, title, feed_url, outcome, http_status, observation_id, message)| {
                Ok(StoredFeedResult {
                    feed_id,
                    title,
                    feed_url,
                    outcome: RssFeedOutcome::from_token(&outcome)
                        .with_context(|| format!("unknown RSS check outcome {outcome:?}"))?,
                    http_status: http_status.and_then(|status| u16::try_from(status).ok()),
                    observation_id,
                    message,
                })
            },
        )
        .collect::<Result<Vec<_>>>()?;
    Ok(Some(StoredRun {
        id,
        playlist_id,
        trigger,
        started_at_us,
        finished_at_us,
        document_count: counts[0],
        not_modified_count: counts[1],
        failed_count: counts[2],
        not_checked_count: counts[3],
        results,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{inspect_schema, upgrades, SchemaCompatibility, CURRENT_VERSION, MIGRATIONS};

    fn table_names(conn: &Connection) -> Vec<String> {
        conn.prepare(
            "SELECT name FROM sqlite_schema WHERE type = 'table' AND name LIKE 'rss_check_%' ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
    }

    fn row_count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    /// R1-12: a version 14 database migrates to version 15 with two empty tables.
    #[test]
    fn adr_0076_playlist_check_version_14_migrates_to_15_with_empty_tables() {
        let conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, 14).unwrap();
        conn.execute_batch(
            "INSERT INTO feeds(id, feed_url, feed_guid, title) VALUES (1, 'https://example.test/feed.xml', 'f1', 'Album');
             INSERT INTO playlists(id, name) VALUES (1, 'Show');",
        )
        .unwrap();
        assert_eq!(
            inspect_schema(&conn).unwrap(),
            SchemaCompatibility::UpgradeRequired {
                applied: 14,
                current: MIGRATIONS.len()
            }
        );
        assert!(table_names(&conn).is_empty());

        crate::db::migrate_schema_to(&conn, 15).unwrap();

        assert!(CURRENT_VERSION >= 15);
        assert_eq!(
            table_names(&conn),
            ["rss_check_feed_results", "rss_check_runs"]
        );
        assert_eq!(row_count(&conn, "rss_check_runs"), 0);
        assert_eq!(row_count(&conn, "rss_check_feed_results"), 0);
        let name: String = conn
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 15",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "playlist_rss_check_runs");
        assert_eq!(
            inspect_schema(&conn).unwrap(),
            SchemaCompatibility::UpgradeRequired {
                applied: 15,
                current: MIGRATIONS.len()
            }
        );
        upgrades::verify_target(&conn, 15).unwrap();
        for (table, columns) in COLUMNS {
            let names = conn
                .prepare(&format!("SELECT * FROM {table}"))
                .unwrap()
                .column_names()
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert_eq!(names, *columns);
        }
    }

    /// R1-12: a fresh database reaches version 15.
    #[test]
    fn adr_0076_playlist_check_fresh_database_reaches_version_15() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_schema(&conn).unwrap();
        crate::db::migrate_schema(&conn).unwrap();
        let version: i64 = conn
            .query_row("SELECT max(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, CURRENT_VERSION);
        assert_eq!(
            table_names(&conn),
            [
                "rss_check_differences",
                "rss_check_feed_results",
                "rss_check_runs"
            ]
        );
        assert_eq!(inspect_schema(&conn).unwrap(), SchemaCompatibility::Current);
    }

    #[test]
    fn adr_0076_playlist_check_failed_migration_15_leaves_version_14() {
        let conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, 14).unwrap();
        let result = crate::db::migrate_schema_with(&conn, 15, |version, reached| {
            anyhow::ensure!(
                version != 15 || reached != crate::db::MigrationBoundary::AfterRecord,
                "fixture stop after migration 15"
            );
            Ok(())
        });
        assert!(result.is_err());
        upgrades::verify_target(&conn, 14).unwrap();
        assert_eq!(
            inspect_schema(&conn).unwrap(),
            SchemaCompatibility::UpgradeRequired {
                applied: 14,
                current: MIGRATIONS.len()
            }
        );
    }

    #[test]
    fn adr_0076_playlist_check_result_counts_follow_rows_and_constraints_hold() {
        let conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, 15).unwrap();
        conn.execute_batch(
            "INSERT INTO feeds(id, feed_url, title) VALUES (1, 'https://a.test/feed', 'A'), (2, 'https://b.test/feed', 'B');
             INSERT INTO playlists(id, name) VALUES (1, 'Show');",
        )
        .unwrap();
        let run = insert_run(&conn, 1, RssCheckTrigger::Button, 10).unwrap();
        insert_feed_result(
            &conn,
            run,
            1,
            RssFeedOutcome::Document,
            Some(200),
            None,
            None,
        )
        .unwrap();
        insert_feed_result(
            &conn,
            run,
            2,
            RssFeedOutcome::NotChecked,
            None,
            None,
            Some("no request"),
        )
        .unwrap();
        finish_run(&conn, run, 20).unwrap();
        let stored = latest_run(&conn, 1).unwrap().unwrap();
        assert_eq!(stored.trigger, RssCheckTrigger::Button);
        assert_eq!(stored.finished_at_us, Some(20));
        assert_eq!(
            (
                stored.document_count,
                stored.not_modified_count,
                stored.failed_count,
                stored.not_checked_count
            ),
            (1, 0, 0, 1)
        );
        assert_eq!(stored.results.len(), 2);
        assert_eq!(stored.results[1].title.as_deref(), Some("B"));
        for invalid in [
            "UPDATE rss_check_runs SET trigger = 'timer'",
            "UPDATE rss_check_runs SET failed_count = -1",
            "UPDATE rss_check_feed_results SET outcome = 'changed'",
            "UPDATE rss_check_feed_results SET http_status = 42",
            "INSERT INTO rss_check_feed_results(run_id, feed_id, outcome) VALUES (1, 1, 'failed')",
            "INSERT INTO rss_check_runs(playlist_id, trigger, started_at_us) VALUES (99, 'button', 1)",
        ] {
            assert!(
                conn.execute_batch(invalid).is_err(),
                "accepted invalid SQL: {invalid}"
            );
        }
    }
}
