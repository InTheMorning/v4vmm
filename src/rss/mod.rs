pub(crate) mod check_apply;
pub(crate) mod compare;
mod enrich;
mod helpers;
mod identity;
mod subscribe;
pub(crate) mod value_routes;

pub use enrich::{
    enrich_track_from_feed_rss, enrich_track_from_feed_rss_observed, fetch_feed_podroll,
    fetch_track_enrichment_from_feed, PodrollEntry, RssFetchResult, RssObservation,
    RssTrackEnrichment,
};
// Packet 018 P18-7: `feed_service::apply_feed_updates` clears the retained
// RSS document of an explicitly refreshed feed.
pub(crate) use enrich::invalidate_feed_document;
// ADR 0076 packet 001: the playlist RSS check checks and retains its documents.
#[cfg(test)]
pub(crate) use enrich::retained_document_for_test;
pub(crate) use enrich::{check_rss_document, retain_checked_document};
pub(crate) use identity::validate_nostr_identity;
pub use identity::{IdentityValidation, NostrIdentity, NostrTlv};
pub use subscribe::subscribe_feed;
