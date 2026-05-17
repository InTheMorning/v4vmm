//! Windowed search-results tabs.

#![warn(clippy::pedantic)]

use crate::runtime::paged_list_vm::{PagedListVm, RowSlot};
use crate::view_models::workspace::ContentFilter;

use super::SearchResultItemId;

/// Paged result windows for one result category.
///
/// Each content filter owns an independent ADR 0041 window. This lets a
/// frame keep tab state and filter state independent while avoiding an eager
/// filtered `Vec<Row>` projection in the view-model.
#[derive(Debug)]
pub(crate) struct SearchResultsPagedTab<Row> {
    all: PagedListVm<SearchResultItemId, Row>,
    library: PagedListVm<SearchResultItemId, Row>,
    index: PagedListVm<SearchResultItemId, Row>,
    library_ids: Vec<SearchResultItemId>,
    index_ids: Vec<SearchResultItemId>,
}

impl<Row> SearchResultsPagedTab<Row> {
    /// Creates empty paged windows for all filters.
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self::new(Vec::new(), Vec::new(), Vec::new())
    }

    /// Creates paged windows from precomputed identity indexes.
    #[must_use]
    pub(crate) fn new(
        all: Vec<SearchResultItemId>,
        library: Vec<SearchResultItemId>,
        index: Vec<SearchResultItemId>,
    ) -> Self {
        Self {
            all: PagedListVm::new(all),
            library: PagedListVm::new(library.clone()),
            index: PagedListVm::new(index.clone()),
            library_ids: library,
            index_ids: index,
        }
    }

    /// Returns the paged window for a filter.
    #[must_use]
    pub(crate) const fn window(
        &self,
        filter: ContentFilter,
    ) -> &PagedListVm<SearchResultItemId, Row> {
        match filter {
            ContentFilter::All => &self.all,
            ContentFilter::Library => &self.library,
            ContentFilter::Index => &self.index,
        }
    }

    /// Returns the mutable paged window for a filter.
    pub(crate) const fn window_mut(
        &mut self,
        filter: ContentFilter,
    ) -> &mut PagedListVm<SearchResultItemId, Row> {
        match filter {
            ContentFilter::All => &mut self.all,
            ContentFilter::Library => &mut self.library,
            ContentFilter::Index => &mut self.index,
        }
    }

    /// Returns whether the filtered window is empty.
    #[must_use]
    pub(crate) fn is_empty(&self, filter: ContentFilter) -> bool {
        self.window(filter).total() == 0
    }
}

impl<Row: Clone> SearchResultsPagedTab<Row> {
    /// Creates a tab whose local-library rows are already loaded.
    #[must_use]
    pub(crate) fn ready_library(rows: Vec<(SearchResultItemId, Row)>) -> Self {
        let ids = rows.iter().map(|(id, _row)| *id).collect::<Vec<_>>();
        let mut tab = Self::new(ids.clone(), ids, Vec::new());
        tab.all.fulfill_page(0, rows.clone());
        tab.library.fulfill_page(0, rows);
        tab
    }

    /// Replaces the remote-index rows while preserving loaded library rows.
    pub(crate) fn replace_index_rows(&mut self, rows: Vec<(SearchResultItemId, Row)>) {
        let library_rows = self.cached_library_rows();
        self.index_ids = rows.iter().map(|(id, _row)| *id).collect();
        self.index.replace_index(self.index_ids.clone());
        self.index.fulfill_page(0, rows.clone());

        self.all.replace_index(self.all_ids());
        self.all
            .fulfill_page(0, library_rows.into_iter().chain(rows));
    }

    fn all_ids(&self) -> Vec<SearchResultItemId> {
        self.library_ids
            .iter()
            .chain(&self.index_ids)
            .copied()
            .collect()
    }

    fn cached_library_rows(&self) -> Vec<(SearchResultItemId, Row)> {
        self.library_ids
            .iter()
            .enumerate()
            .filter_map(|(index, id)| match self.library.peek_row(index) {
                RowSlot::Ready(row) => Some((*id, row.as_ref().clone())),
                RowSlot::Pending(_) => None,
            })
            .collect()
    }

    pub(super) fn cached_row_matching(
        &self,
        filter: ContentFilter,
        predicate: impl Fn(&Row) -> bool,
    ) -> Option<Row> {
        let window = self.window(filter);
        (0..window.total()).find_map(|index| match window.peek_row(index) {
            RowSlot::Ready(row) if predicate(row.as_ref()) => Some(row.as_ref().clone()),
            RowSlot::Ready(_) | RowSlot::Pending(_) => None,
        })
    }
}
