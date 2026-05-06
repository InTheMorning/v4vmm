//! Local identity source-fact hydration helpers.
//!
//! This module maps `SQLite` source-fact rows into the GPUI-free `views` fact
//! types. It is intentionally below screen code and above raw DB helpers so
//! Library and local metadata sources do not duplicate provenance mapping.

#![warn(clippy::pedantic)]

use anyhow::Result;
use rusqlite::Connection;

use crate::db::{self, LocalEntityOwner, LocalIdentityOwner};
use crate::views::{ContributorView, IdentityIdFact, IdentityLinkFact, LocalIdentityFacts};

pub(crate) fn facts_for_owner(
    conn: &Connection,
    identity_owner: LocalIdentityOwner,
    entity_owner: LocalEntityOwner,
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
    let contributors = db::local_contributors(conn, entity_owner)?
        .into_iter()
        .map(|row| ContributorView {
            name: row.name,
            role: row.role,
            group_name: row.group_name,
            href: row.href,
            image_url: row.image_url,
            nostr_npub: row.nostr_npub,
        })
        .collect();

    Ok(LocalIdentityFacts {
        source_links,
        source_ids,
        contributors,
    })
}

pub(crate) fn feed_facts(conn: &Connection, feed_id: i64) -> Result<LocalIdentityFacts> {
    facts_for_owner(
        conn,
        LocalIdentityOwner::Feed(feed_id),
        LocalEntityOwner::Feed(feed_id),
    )
}

pub(crate) fn track_facts(conn: &Connection, track_id: i64) -> Result<LocalIdentityFacts> {
    facts_for_owner(
        conn,
        LocalIdentityOwner::Track(track_id),
        LocalEntityOwner::Track(track_id),
    )
}
