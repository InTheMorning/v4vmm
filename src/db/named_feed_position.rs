//! The RSS position of each named feed (ADR 0085 Decision 6).
//!
//! Migration 19 adds `named_feed_position` to `feed_publisher_relationships`,
//! the table that migration 14 created and that migration 18
//! (`publisher_link_facts`) extended. The new column holds the `position` of
//! the `remote_items` entry that named the feed, so the app can keep the RSS
//! sequence of each named feed. An existing row gets null.
//!
//! `publisher_relationships.rs` owns the write and the read of this table.
//! This module owns only migration 19: its DDL and its read contract.

#![warn(clippy::pedantic)]

use anyhow::{Context, Result};
use rusqlite::Connection;

/// The read contract of `feed_publisher_relationships` after migration 19:
/// the migration-18 columns of
/// [`super::publisher_link_facts::EXTENDED_COLUMNS`], then the one column
/// that migration 19 appends.
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
        "named_feed_position",
    ],
)];

const DDL: &str = r"
ALTER TABLE feed_publisher_relationships ADD COLUMN named_feed_position INTEGER NULL;
";

/// Migration 19 (ADR 0016 registry, ADR 0085 Decision 6).
pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(DDL)
        .context("Add the named feed position column")
}
