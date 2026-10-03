//! Publisher link facts of Stophammer 0.7.0 (ADR 0082 Decision 2).
//!
//! Migration 18 adds `album_names_as` and `role_agreement` to
//! `feed_publisher_relationships`, the table that migration 14 created. Each
//! column holds one more fact of the same stored row. An existing row gets
//! null in each new column.
//!
//! `publisher_relationships.rs` owns the write and the read of this table.
//! This module owns only migration 18: its DDL and its read contract.

#![warn(clippy::pedantic)]

use anyhow::{Context, Result};
use rusqlite::Connection;

/// The read contract of `feed_publisher_relationships` after migration 18:
/// the migration-14 columns of [`super::publisher_relationships::COLUMNS`],
/// then the two columns that migration 18 appends.
pub(super) const EXTENDED_COLUMNS: &[(&str, &[&str])] = &[(
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
        "album_names_as",
        "role_agreement",
    ],
)];

const DDL: &str = r"
ALTER TABLE feed_publisher_relationships ADD COLUMN album_names_as TEXT NULL;
ALTER TABLE feed_publisher_relationships ADD COLUMN role_agreement TEXT NULL;
";

/// Migration 18 (ADR 0016 registry, ADR 0082 Decision 2).
pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(DDL)
        .context("Add the publisher link fact columns")
}
