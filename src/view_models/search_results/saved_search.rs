//! Saved searches (ADR 0084 Decision 3, packet 012).
//!
//! A saved search is a view of the Discover column, after "New releases".
//! Its page runs its query against the Library and the Index, and shows
//! the results in the detail column with the view model of packet 010.
//! This module gives the types, the labels, the typed actions, the delete
//! confirmation and the stale-result rule of the saved-search page.

#![warn(clippy::pedantic)]

use crate::db::saved_searches::{StoredSavedSearch, StoredStartTab};
use crate::view_models::workspace::{FrameNavigationEntry, MusicDestination};

use super::{EmptyStateDisplay, SearchResultsInspectorPageVm, SearchResultsTab};

/// The identity of one saved search: its `saved_searches` row id.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct SavedSearchId(pub(crate) i64);

/// One saved search, as Discover shows it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SavedSearch {
    pub(crate) id: SavedSearchId,
    /// The trimmed query text.
    pub(crate) query: String,
    /// The tab that was open at the save. The page opens on this tab.
    pub(crate) start_tab: SearchResultsTab,
}

impl SavedSearch {
    /// The frame title of the saved-search page.
    pub(crate) const FRAME_TITLE: &'static str = "Saved search";

    /// The subtitle of a saved-search row of Discover. Each saved search
    /// runs against the Library and the Index (ADR 0084 Decision 3).
    pub(crate) const SUBTITLE: &'static str = "Library and Index";

    /// A saved search from its stored row.
    #[must_use]
    pub(crate) fn from_stored(stored: StoredSavedSearch) -> Self {
        Self {
            id: SavedSearchId(stored.id),
            query: stored.query,
            start_tab: SearchResultsTab::from(stored.start_tab),
        }
    }

    /// The navigation entry of this saved search.
    #[must_use]
    pub(crate) fn entry(&self) -> FrameNavigationEntry {
        FrameNavigationEntry::SavedSearch {
            id: self.id,
            query: self.query.clone(),
        }
    }
}

/// The label of a saved search, in Discover and in the breadcrumb, for
/// example "Saved: bluegrass".
#[must_use]
pub(crate) fn saved_search_label(query: &str) -> String {
    format!("Saved: {query}")
}

/// The accessibility label of a saved-search row of Discover.
#[must_use]
pub(crate) fn saved_search_row_a11y_label(query: &str) -> String {
    format!("Saved search: {query}, {}", SavedSearch::SUBTITLE)
}

impl From<StoredStartTab> for SearchResultsTab {
    fn from(tab: StoredStartTab) -> Self {
        match tab {
            StoredStartTab::Releases => Self::Releases,
            StoredStartTab::Artists => Self::Artists,
            StoredStartTab::Tracks => Self::Tracks,
        }
    }
}

impl From<SearchResultsTab> for StoredStartTab {
    fn from(tab: SearchResultsTab) -> Self {
        match tab {
            SearchResultsTab::Releases => Self::Releases,
            SearchResultsTab::Artists => Self::Artists,
            SearchResultsTab::Tracks => Self::Tracks,
        }
    }
}

/// Tells if `query` is saved. The compare ignores the case of ASCII
/// letters, as the `NOCASE` index of `saved_searches` does.
#[must_use]
pub(crate) fn is_query_saved(saved: &[SavedSearch], query: &str) -> bool {
    let query = query.trim();
    saved
        .iter()
        .any(|search| search.query.eq_ignore_ascii_case(query))
}

/// The control style of a search page action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchPageActionStyle {
    /// A plain button: "Save search".
    Plain,
    /// A destructive button: "Delete…".
    Destructive,
}

/// One typed action of the header of a search page (packet 012). The
/// view model decides its text and its availability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchPageActionDisplay {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: String,
    pub(crate) available: bool,
    pub(crate) style: SearchPageActionStyle,
}

/// "Save search" of the toolbar search, or "Saved" when the query is
/// already saved.
#[must_use]
pub(crate) fn save_search_action(query: &str, saved: &[SavedSearch]) -> SearchPageActionDisplay {
    if is_query_saved(saved, query) {
        SearchPageActionDisplay {
            id: "music-search-list-save-search",
            label: "Saved",
            a11y_label: "This search is saved in Discover".to_owned(),
            available: false,
            style: SearchPageActionStyle::Plain,
        }
    } else {
        SearchPageActionDisplay {
            id: "music-search-list-save-search",
            label: "Save search",
            a11y_label: format!("Save the search \"{}\" in Discover", query.trim()),
            available: true,
            style: SearchPageActionStyle::Plain,
        }
    }
}

/// "Delete…" of the saved-search page.
#[must_use]
pub(crate) fn delete_saved_search_action(query: &str) -> SearchPageActionDisplay {
    SearchPageActionDisplay {
        id: "music-saved-search-delete",
        label: "Delete\u{2026}",
        a11y_label: format!("Delete the saved search \"{query}\""),
        available: true,
        style: SearchPageActionStyle::Destructive,
    }
}

/// The text of the delete confirmation of a saved search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SavedSearchDeleteConfirmation {
    pub(crate) title: String,
    pub(crate) message: String,
    pub(crate) cancel_button_id: &'static str,
    pub(crate) cancel_label: &'static str,
    pub(crate) cancel_a11y_label: &'static str,
    pub(crate) confirm_button_id: &'static str,
    pub(crate) confirm_label: &'static str,
    pub(crate) confirm_a11y_label: String,
}

/// The delete confirmation of a saved search with the query `query`.
#[must_use]
pub(crate) fn delete_saved_search_confirmation(query: &str) -> SavedSearchDeleteConfirmation {
    SavedSearchDeleteConfirmation {
        title: "Delete saved search?".to_owned(),
        message: format!(
            "Delete the saved search \"{query}\"? The Library and the downloads do not change."
        ),
        cancel_button_id: "music-saved-search-delete-cancel",
        cancel_label: "Cancel",
        cancel_a11y_label: "Keep the saved search",
        confirm_button_id: "music-saved-search-delete-confirm",
        confirm_label: "Delete",
        confirm_a11y_label: format!("Delete the saved search \"{query}\""),
    }
}

/// The location after the delete of a saved search: Discover, as a
/// sidebar click opens it, with "New releases" (ADR 0084 Decision 9).
#[must_use]
pub(crate) const fn location_after_saved_search_delete() -> FrameNavigationEntry {
    FrameNavigationEntry::Destination(MusicDestination::Discover)
}

/// The state of the mounted saved-search page.
#[derive(Debug)]
pub(crate) enum SavedSearchPageState {
    /// The saved search exists. The page holds its results.
    Open(Box<SearchResultsInspectorPageVm>),
    /// The saved search was deleted. The page shows an empty state and
    /// runs no query.
    Deleted,
}

/// The mounted saved-search page of the detail column.
#[derive(Debug)]
pub(crate) struct SavedSearchPage {
    pub(crate) id: SavedSearchId,
    pub(crate) query: String,
    pub(crate) state: SavedSearchPageState,
}

impl SavedSearchPage {
    /// The title of the empty state of a deleted saved search.
    pub(crate) const DELETED_TITLE: &'static str = "Saved search deleted";

    /// The text of the empty state of a deleted saved search.
    pub(crate) const DELETED_TEXT: &'static str = "This saved search was deleted.";

    /// A page with the results of `search`: the Library rows of `results`
    /// and the tab of the save.
    #[must_use]
    pub(crate) fn open(search: &SavedSearch, results: SearchResultsInspectorPageVm) -> Self {
        Self {
            id: search.id,
            query: search.query.clone(),
            state: SavedSearchPageState::Open(Box::new(results.into_saved_search(search))),
        }
    }

    /// The page of a deleted saved search.
    #[must_use]
    pub(crate) fn deleted(id: SavedSearchId, query: impl Into<String>) -> Self {
        Self {
            id,
            query: query.into(),
            state: SavedSearchPageState::Deleted,
        }
    }

    /// The empty state of a deleted saved search.
    #[must_use]
    pub(crate) fn deleted_empty_state() -> EmptyStateDisplay {
        EmptyStateDisplay::new(Self::DELETED_TITLE, Self::DELETED_TEXT)
    }

    /// The empty state while the page of the current saved search is not
    /// mounted yet.
    #[must_use]
    pub(crate) fn loading_empty_state() -> EmptyStateDisplay {
        EmptyStateDisplay::new(
            SavedSearch::FRAME_TITLE,
            "The app is loading the results of the saved search.",
        )
    }

    /// The results of the page, or `None` for a deleted saved search.
    #[must_use]
    pub(crate) fn results(&self) -> Option<&SearchResultsInspectorPageVm> {
        match &self.state {
            SavedSearchPageState::Open(results) => Some(results),
            SavedSearchPageState::Deleted => None,
        }
    }

    /// The mutable results of the page.
    pub(crate) fn results_mut(&mut self) -> Option<&mut SearchResultsInspectorPageVm> {
        match &mut self.state {
            SavedSearchPageState::Open(results) => Some(results),
            SavedSearchPageState::Deleted => None,
        }
    }

    /// The header action of the page: "Delete…", or `None` for a deleted
    /// saved search.
    #[must_use]
    pub(crate) fn delete_action(&self) -> Option<SearchPageActionDisplay> {
        self.results()
            .map(|_| delete_saved_search_action(&self.query))
    }
}

/// What the restore of a saved-search entry does.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SavedSearchRestore {
    /// The mounted page is the page of this entry. It keeps its rows and
    /// its selection, and no query runs.
    Keep,
    /// A new page runs the query of this saved search: the Library rows at
    /// once, then the Index request.
    RunQuery(SavedSearch),
    /// The saved search was deleted. The page shows the empty state, and
    /// no query runs.
    Deleted,
}

/// Decides the restore of the entry `SavedSearch { id, query }`. A mounted
/// page with the same id and the same query keeps its rows. Any other case
/// runs the query again: the Library rows at once, then the Index request.
#[must_use]
pub(crate) fn saved_search_restore(
    mounted: Option<&SavedSearchPage>,
    id: SavedSearchId,
    query: &str,
    saved: &[SavedSearch],
) -> SavedSearchRestore {
    let Some(search) = saved
        .iter()
        .find(|search| search.id == id && search.query == query)
    else {
        return SavedSearchRestore::Deleted;
    };
    let same_page = mounted
        .is_some_and(|page| page.id == id && page.query == query && page.results().is_some());
    if same_page {
        SavedSearchRestore::Keep
    } else {
        SavedSearchRestore::RunQuery(search.clone())
    }
}

/// Tells if an Index result for the saved search `id` with the query
/// `query` can write the mounted page. The mounted page must hold that
/// same id and that same query. A move to saved search B replaces the
/// mounted page with the page of B, so a stale result for A then does not
/// match. A move away to "New releases" or to a toolbar search does not
/// touch the mounted page, so a result that arrives after that move still
/// writes the page of the saved search it belongs to.
#[must_use]
pub(crate) fn saved_search_result_is_current(
    mounted: Option<&SavedSearchPage>,
    id: SavedSearchId,
    query: &str,
) -> bool {
    mounted.is_some_and(|page| page.id == id && page.query == query && page.results().is_some())
}

#[cfg(test)]
mod tests {
    use super::{
        delete_saved_search_confirmation, location_after_saved_search_delete, save_search_action,
        saved_search_restore, saved_search_result_is_current, SavedSearch, SavedSearchId,
        SavedSearchPage, SavedSearchRestore,
    };
    use crate::view_models::search_results::tests::{index_release, index_track, RecordedLibrary};
    use crate::view_models::search_results::{
        ArtistSearchFacts, IndexSearchResultRows, SearchResultsInspectorPageVm, SearchResultsTab,
    };
    use crate::view_models::selection_action_bar::SelectionActionKind;
    use crate::view_models::workspace::{
        DiscoverView, FrameNavigationEntry, FrameNavigationState, MusicDestination,
    };

    fn saved(id: i64, query: &str, start_tab: SearchResultsTab) -> SavedSearch {
        SavedSearch {
            id: SavedSearchId(id),
            query: query.to_owned(),
            start_tab,
        }
    }

    fn open_page(search: &SavedSearch) -> SavedSearchPage {
        SavedSearchPage::open(
            search,
            SearchResultsInspectorPageVm::from_local_library_tracks(
                &search.query,
                &[],
                &ArtistSearchFacts::default(),
            ),
        )
    }

    /// R12-03 (ADR 0084 Decision 3): "Save search" is available for a new
    /// query. A saved query, also with another case, gives "Saved", not
    /// available, with its accessibility label.
    #[test]
    fn adr_0084_saved_search_r12_03_save_action_follows_the_saved_list() {
        let list = [saved(1, "bluegrass", SearchResultsTab::Releases)];

        let new = save_search_action("ambient", &list);
        assert_eq!(new.label, "Save search");
        assert!(new.available);
        assert_eq!(new.a11y_label, "Save the search \"ambient\" in Discover");

        let present = save_search_action("Bluegrass", &list);
        assert_eq!(present.label, "Saved");
        assert!(!present.available);
        assert_eq!(present.a11y_label, "This search is saved in Discover");

        let vm = SearchResultsInspectorPageVm::new("bluegrass");
        assert_eq!(vm.save_action(&list).label, "Saved");
        assert_eq!(vm.save_action(&[]).label, "Save search");
    }

    /// R12-06 (ADR 0084 Decision 3): the page opens on its start tab, also
    /// when the Index rows of another tab arrive first.
    #[test]
    fn adr_0084_saved_search_r12_06_page_opens_on_its_start_tab() {
        let search = saved(4, "bluegrass", SearchResultsTab::Tracks);
        let mut page = open_page(&search);
        let results = page.results_mut().expect("an open page has results");
        assert_eq!(results.tab(), SearchResultsTab::Tracks);
        results.mark_index_loading();

        results.replace_index_results(IndexSearchResultRows {
            feeds: vec![index_release(2_000_000_000, "g-index", "Index Release")],
            ..IndexSearchResultRows::default()
        });

        assert_eq!(
            results.tab(),
            SearchResultsTab::Tracks,
            "the Releases rows do not change the start tab"
        );
        assert_eq!(
            results.selection().entry(),
            Some(&search.entry()),
            "the selection belongs to the saved-search entry"
        );
    }

    /// R12-07 (ADR 0084 Decision 3): after a move from saved search A to
    /// saved search B, an Index result for A does not change the page of
    /// B. The move runs the query of B, which replaces the mounted page of
    /// A with the page of B, so the stale result for A no longer matches.
    #[test]
    fn adr_0084_saved_search_r12_07_stale_result_changes_nothing() {
        let a = saved(1, "bluegrass", SearchResultsTab::Releases);
        let b = saved(2, "ambient", SearchResultsTab::Releases);

        let page_a = open_page(&a);
        assert!(saved_search_result_is_current(
            Some(&page_a),
            a.id,
            &a.query
        ));

        // A move from A to B replaces the mounted page.
        let page_b = open_page(&b);
        assert!(
            !saved_search_result_is_current(Some(&page_b), a.id, &a.query),
            "the mounted page is now B, so a late reply for A does not match"
        );
        assert!(saved_search_result_is_current(
            Some(&page_b),
            b.id,
            &b.query
        ));
    }

    /// Review correction (2026-10-21): before this fix, a late Index reply
    /// for the open saved search A was dropped once the current location
    /// path no longer held the entry of A, for example after a move to
    /// "New releases". The page then stayed "loading" for good, because
    /// Back gave `Keep` with no rows to show.
    ///
    /// A reply for A now matches as long as the mounted page still holds
    /// the id and the query of A. A move away to "New releases" or to a
    /// toolbar search does not touch the mounted saved-search page, so the
    /// reply still writes it, and Back shows the rows, not a stuck loading
    /// state.
    #[test]
    fn adr_0084_saved_search_late_reply_after_leaving_the_path_is_not_stuck_loading() {
        let a = saved(1, "bluegrass", SearchResultsTab::Releases);
        let mut page = open_page(&a);
        page.results_mut()
            .expect("an open page has results")
            .mark_index_loading();

        // Leave to "New releases", a path that holds no entry for A,
        // before the reply arrives. The production code calls no
        // saved-search function for this move, so the mounted page stays
        // the page of A.
        let mut nav = FrameNavigationState::new_with_page(
            FrameNavigationEntry::Destination(MusicDestination::Discover),
            a.entry(),
        );
        nav.start_path(
            FrameNavigationEntry::Destination(MusicDestination::Discover),
            Some(FrameNavigationEntry::DiscoverView(
                DiscoverView::NewReleases,
            )),
        );
        assert!(
            !nav.path_entries().contains(&a.entry()),
            "the path no longer holds the entry of A"
        );

        // The late reply for A still matches, because the mounted page is
        // still the page of A.
        assert!(saved_search_result_is_current(Some(&page), a.id, &a.query));
        let wrote = page
            .results_mut()
            .expect("an open page has results")
            .replace_index_results_for_query(
                &a.query,
                IndexSearchResultRows {
                    feeds: vec![index_release(2_000_000_000, "g-index", "Index Release")],
                    ..IndexSearchResultRows::default()
                },
            );
        assert!(wrote, "the reply writes the page of A");

        // Back restores A: Keep, because the mounted page already holds
        // the rows of A.
        assert_eq!(
            saved_search_restore(Some(&page), a.id, &a.query, std::slice::from_ref(&a)),
            SavedSearchRestore::Keep
        );
        let results = page.results().expect("an open page has results");
        assert!(!results.is_index_loading(), "the page is not stuck loading");
    }

    /// R12-08 (ADR 0084 Decision 3): the confirmation names the query and
    /// gives "Delete". After the delete, the location is Discover with
    /// "New releases".
    #[test]
    fn adr_0084_saved_search_r12_08_delete_confirmation_and_location() {
        let confirmation = delete_saved_search_confirmation("bluegrass");
        assert_eq!(
            confirmation.message,
            "Delete the saved search \"bluegrass\"? The Library and the downloads do not change."
        );
        assert_eq!(confirmation.confirm_label, "Delete");

        let search = saved(3, "bluegrass", SearchResultsTab::Releases);
        let action = open_page(&search)
            .delete_action()
            .expect("an open page gives Delete");
        assert_eq!(action.label, "Delete\u{2026}");
        assert!(action.available);

        let entry = location_after_saved_search_delete();
        let FrameNavigationEntry::Destination(destination) = &entry else {
            panic!("the delete opens a destination");
        };
        let mut nav = FrameNavigationState::new_with_page(
            FrameNavigationEntry::Destination(MusicDestination::Discover),
            search.entry(),
        );
        nav.start_path(entry.clone(), destination.default_child_page());
        assert_eq!(
            nav.path_entries(),
            vec![
                FrameNavigationEntry::Destination(MusicDestination::Discover),
                FrameNavigationEntry::DiscoverView(DiscoverView::NewReleases),
            ]
        );
    }

    /// R12-09 (ADR 0084 Decision 3): the entry of a deleted saved search
    /// gives the empty state, and the restore starts no query.
    #[test]
    fn adr_0084_saved_search_r12_09_deleted_entry_runs_no_query() {
        let a = saved(1, "bluegrass", SearchResultsTab::Releases);
        let mounted = open_page(&a);

        assert_eq!(
            saved_search_restore(Some(&mounted), a.id, &a.query, &[]),
            SavedSearchRestore::Deleted
        );
        let deleted = SavedSearchPage::deleted(a.id, &a.query);
        assert!(deleted.results().is_none());
        assert!(deleted.delete_action().is_none());
        assert_eq!(
            SavedSearchPage::DELETED_TEXT,
            "This saved search was deleted."
        );

        // A mounted page of the same id and the same query keeps its
        // rows. With no mounted page, the restore runs the query.
        let list = [a.clone()];
        assert_eq!(
            saved_search_restore(Some(&mounted), a.id, &a.query, &list),
            SavedSearchRestore::Keep
        );
        assert_eq!(
            saved_search_restore(None, a.id, &a.query, &list),
            SavedSearchRestore::RunQuery(a)
        );
    }

    /// R12-10 (ADR 0084 Decision 3): the saved-search page gives the
    /// header action and the bar of packet 010 on its Releases tab and its
    /// Tracks tab. The header action has the id of the detail column.
    #[test]
    fn adr_0084_saved_search_r12_10_header_action_and_bar() {
        let search = saved(5, "q", SearchResultsTab::Releases);
        let mut page = open_page(&search);
        let results = page.results_mut().unwrap();
        results.replace_index_results(IndexSearchResultRows {
            feeds: vec![index_release(2_000_000_000, "g-index", "Index Release")],
            tracks: vec![index_track(
                3_000_000_000,
                Some("g-index"),
                "t1",
                "Index Track",
            )],
            ..IndexSearchResultRows::default()
        });
        let lookup = RecordedLibrary::default();

        let action = results.header_action(&lookup).expect("Releases has Select");
        assert_eq!(action.label, "Select");
        assert_eq!(action.id, "music-detail-column-select-releases");
        results.toggle_mode();
        results.select_all(&lookup);
        let bar = results.action_bar(&lookup).expect("Selecting shows a bar");
        assert_eq!(
            bar.action(SelectionActionKind::Download)
                .map(|action| action.label.clone())
                .as_deref(),
            Some("Download 1 release")
        );

        results.set_tab(SearchResultsTab::Tracks);
        let action = results.header_action(&lookup).expect("Tracks has Select");
        assert_eq!(action.id, "music-detail-column-select-tracks");
        results.toggle_mode();
        results.select_all(&lookup);
        let bar = results.action_bar(&lookup).expect("Selecting shows a bar");
        assert_eq!(
            bar.action(SelectionActionKind::Download)
                .map(|action| action.label.clone())
                .as_deref(),
            Some("Download 1 track")
        );
        assert_eq!(
            results.selection().entry(),
            Some(&search.entry()),
            "a tab change keeps the selection on the saved-search entry"
        );
    }
}
