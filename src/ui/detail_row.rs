//! Legacy detail-row shape used by `library.rs` / `search.rs` /
//! `ui_feed.rs` while the inspectors are still hand-built. The value
//! is a pre-rendered `AnyElement` so callers can embed rich content
//! (links, multi-line truncations).
//!
//! Lifted out of the deprecated `ui_common` module unchanged. New
//! code should build [`crate::ui::composites::DetailRow`] directly;
//! this shim only exists so the existing call sites compile while
//! the screen-search / screen-library migrations land.

#![warn(clippy::pedantic)]

use gpui::{AnyElement, SharedString};

use crate::ui::composites::DetailRow as CompositeDetailRow;

pub struct DetailRow {
    pub key: String,
    pub value: AnyElement,
}

impl From<DetailRow> for CompositeDetailRow {
    fn from(row: DetailRow) -> Self {
        Self {
            key: SharedString::from(row.key),
            value: row.value,
        }
    }
}
