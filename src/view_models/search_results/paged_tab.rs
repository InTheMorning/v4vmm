//! The windowed rows of one search-results tab.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet};

use crate::runtime::paged_list_vm::{PagedListVm, RowSlot};

use super::SearchResultItemId;

/// The identity that merges the Library row and the Index row of one item
/// (ADR 0084 Decision 4, packet 010 "One row for each item"). Two rows with
/// the same key are one row. A row with no key never merges.
pub(crate) trait SearchRowMergeKey {
    /// The type of the key.
    type Key: Ord + Clone;

    /// The key of this row, or `None` for a row that never merges.
    fn merge_key(&self) -> Option<Self::Key>;

    /// Fills a fact that this row carries as empty, from `other`, the row
    /// that this row's key drops (packet 010 "One row for each item"). The
    /// default keeps this row as it is. A Library row overrides this to
    /// take the image of the dropped Index row when it has none of its
    /// own.
    fn adopt_missing_thumbnail_href(&mut self, other: &Self) {
        let _ = other;
    }
}

/// The rows of one result tab in one ADR 0041 window (packet 010 Required
/// Change 2). The Library rows come first, in their present order. The
/// Index rows come after them, in rank order. An Index row whose key is
/// already in the window does not show, so a merged row keeps the place of
/// its Library row.
#[derive(Debug)]
pub(crate) struct SearchResultsPagedTab<Row> {
    window: PagedListVm<SearchResultItemId, Row>,
    library_rows: Vec<(SearchResultItemId, Row)>,
}

impl<Row> SearchResultsPagedTab<Row> {
    /// Creates an empty tab.
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self {
            window: PagedListVm::new(Vec::new()),
            library_rows: Vec::new(),
        }
    }

    /// Returns the one window of this tab.
    #[must_use]
    pub(crate) const fn window(&self) -> &PagedListVm<SearchResultItemId, Row> {
        &self.window
    }

    /// Returns whether the tab has no row.
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.window.total() == 0
    }
}

impl<Row: Clone + SearchRowMergeKey> SearchResultsPagedTab<Row> {
    /// Creates a tab whose local-library rows are already loaded.
    #[must_use]
    pub(crate) fn ready_library(rows: Vec<(SearchResultItemId, Row)>) -> Self {
        let mut tab = Self::empty();
        tab.library_rows = rows;
        tab.fill(Vec::new());
        tab
    }

    /// Replaces the remote-index rows and keeps the loaded library rows.
    pub(crate) fn replace_index_rows(&mut self, rows: Vec<(SearchResultItemId, Row)>) {
        self.fill(rows);
    }

    fn fill(&mut self, index_rows: Vec<(SearchResultItemId, Row)>) {
        let rows = merge_rows(self.library_rows.clone(), index_rows);
        self.window
            .replace_index(rows.iter().map(|(id, _row)| *id).collect());
        self.window.fulfill_page(0, rows);
    }

    /// Each loaded row, in the visible order.
    #[must_use]
    pub(crate) fn rows(&self) -> Vec<Row> {
        (0..self.window.total())
            .filter_map(|index| match self.window.peek_row(index) {
                RowSlot::Ready(row) => Some(row.as_ref().clone()),
                RowSlot::Pending(_) => None,
            })
            .collect()
    }

    pub(super) fn cached_row_matching(&self, predicate: impl Fn(&Row) -> bool) -> Option<Row> {
        (0..self.window.total()).find_map(|index| match self.window.peek_row(index) {
            RowSlot::Ready(row) if predicate(row.as_ref()) => Some(row.as_ref().clone()),
            RowSlot::Ready(_) | RowSlot::Pending(_) => None,
        })
    }
}

/// The merged rows: each Library row, then each Index row whose key is not
/// already in the list (packet 010 "One row for each item"). An Index row
/// also drops when an earlier Index row has its key. A row id that is
/// already in the list also drops, so the window holds each id one time.
/// A dropped row that carries a merge key still gives its image to the row
/// that stays, when that row has none of its own.
pub(crate) fn merge_rows<Row: SearchRowMergeKey>(
    library_rows: Vec<(SearchResultItemId, Row)>,
    index_rows: Vec<(SearchResultItemId, Row)>,
) -> Vec<(SearchResultItemId, Row)> {
    let mut kept_index_of_key: BTreeMap<Row::Key, usize> = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let mut merged: Vec<(SearchResultItemId, Row)> = Vec::new();
    for (id, row) in library_rows.into_iter().chain(index_rows) {
        let key = row.merge_key();
        if let Some(key) = key.clone() {
            if let Some(&kept_index) = kept_index_of_key.get(&key) {
                merged[kept_index].1.adopt_missing_thumbnail_href(&row);
                continue;
            }
        }
        if ids.insert(id) {
            if let Some(key) = key {
                kept_index_of_key.insert(key, merged.len());
            }
            merged.push((id, row));
        }
    }
    merged
}
