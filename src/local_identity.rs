//! Local identity source-fact hydration helpers.
//!
//! This module maps `SQLite` source-fact rows into the GPUI-free `views` fact
//! types. It is intentionally below screen code and above raw DB helpers so
//! Library and local metadata sources do not duplicate provenance mapping.

#![warn(clippy::pedantic)]

use anyhow::Result;
use rusqlite::Connection;

use crate::db::{self, LocalIdentityOwner};
use crate::views::{IdentityIdFact, IdentityLinkFact, LocalIdentityFacts};

/// The identity links and identifiers of one owner, with the rows of each
/// source. The credit list is not here: the stored value projection owns it
/// (ADR 0076 packet 006).
pub(crate) fn facts_for_owner(
    conn: &Connection,
    identity_owner: LocalIdentityOwner,
) -> Result<LocalIdentityFacts> {
    let source_links = db::local_identity_links(conn, identity_owner)?
        .into_iter()
        .map(|row| IdentityLinkFact {
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            position: row.position,
            link_type: row.link_type,
            url: row.url,
            source: Some(row.source),
            extraction_path: row.extraction_path,
            observed_at: row.observed_at,
        })
        .collect();
    let source_ids = db::local_identity_ids(conn, identity_owner)?
        .into_iter()
        .map(|row| IdentityIdFact {
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            position: row.position,
            scheme: row.scheme,
            value: row.value,
            source: Some(row.source),
            extraction_path: row.extraction_path,
            observed_at: row.observed_at,
        })
        .collect();
    Ok(LocalIdentityFacts {
        source_links,
        source_ids,
    })
}

pub(crate) fn feed_facts(conn: &Connection, feed_id: i64) -> Result<LocalIdentityFacts> {
    facts_for_owner(conn, LocalIdentityOwner::Feed(feed_id))
}

pub(crate) fn track_facts(conn: &Connection, track_id: i64) -> Result<LocalIdentityFacts> {
    facts_for_owner(conn, LocalIdentityOwner::Track(track_id))
}
