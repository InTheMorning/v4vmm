//! The channel image of each named feed (ADR 0084 Decision 12, amended on
//! 2026-10-20, packet 024).
//!
//! Migration 20 adds `publisher_feed_image_url` to
//! `feed_publisher_relationships`, the table that migration 14 created and
//! that migrations 18 and 19 extended. The new column holds the
//! `remote_feed_image_url` of the `MusicIndex` album response. An existing
//! row gets null. The next album hydration of each album writes the value.
//!
//! `publisher_relationships.rs` owns the write and the read of this table.
//! This module owns only migration 20: its DDL and its read contract.

#![warn(clippy::pedantic)]

use anyhow::{Context, Result};
use rusqlite::Connection;

/// The read contract of `feed_publisher_relationships` after migration 20:
/// the migration-19 columns of
/// [`super::named_feed_position::EXTENDED_COLUMNS`], then the one column
/// that migration 20 appends.
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
        "publisher_feed_image_url",
    ],
)];

const DDL: &str = r"
ALTER TABLE feed_publisher_relationships ADD COLUMN publisher_feed_image_url TEXT NULL;
";

/// Migration 20 (ADR 0016 registry, ADR 0084 Decision 12).
pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(DDL)
        .context("Add the publisher feed image column")
}
