//! Local read-model query families.

/// The most feed detail reads that run at the same time: at most 4 (ADR
/// 0086 Decision 5, ADR 0084 packet 026). `feed.rs` and `name_lists.rs`
/// both send their feed detail reads through this one limit.
pub(crate) const PARALLEL_DETAIL_READS: usize = 4;

pub mod broadcast;
pub mod feed;
pub mod images;
pub mod library;
pub mod metadata;
pub mod name_lists;
pub mod playback;
pub mod playlist;
pub mod search;
pub mod stored_values;
pub mod tag_update;
