//! Saved-search dispatch (ADR 0084 Decision 3, packet 012).
//!
//! `TopApp` owns the saved-search page of the detail column: its query
//! run, its stale-result check, its selection and its delete. The page uses
//! the view model and the renderer of the toolbar search (packet 010).
//! `LibraryApp` holds the saved searches and writes them through
//! `src/db/saved_searches.rs`.

#![warn(clippy::pedantic)]

use gpui::{AnyElement, App, Context, Window};

use crate::application::queries::search::FetchIndexSearchResults;
use crate::application::CommandContext;
use crate::presentation::present_command;
use crate::ui::shells::saved_search_delete_confirmation::open_saved_search_delete_dialog;
use crate::ui::shells::search_results_inspector::{
    render_search_empty_state, render_search_results_inspector, SearchResultsHeaderMode,
    SearchResultsInspectorSlots,
};
use crate::view_models::search_results::{
    delete_saved_search_confirmation, location_after_saved_search_delete, saved_search_restore,
    saved_search_result_is_current, SavedSearch, SavedSearchId, SavedSearchPage,
    SavedSearchRestore, SearchResultsTab,
};
use crate::view_models::workspace::{FrameNavigationEntry, NavigationPathStart};

use super::search_selection::{SearchPageSlot, SearchRowClick};
use super::TopApp;

impl TopApp {
    /// "Save search" of the toolbar search (packet 012). It saves the query
    /// and the open tab, with the time of the click. The action changes to
    /// "Saved" at once, with no navigation. A failure shows in the status
    /// text of the search view.
    pub(super) fn save_toolbar_search(&mut self, cx: &mut Context<Self>) {
        let Some((query, tab)) = self
            .search_results_detail
            .as_ref()
            .map(|detail| (detail.query().to_owned(), detail.tab()))
        else {
            return;
        };
        let created_at = chrono::Utc::now().timestamp();
        let saved = self.library.update(cx, |library, cx| {
            library.save_search(&query, tab, created_at, cx)
        });
        if let Some(detail) = self.search_results_detail.as_mut() {
            match saved {
                Ok(()) => detail.clear_status(),
                Err(error) => detail.set_save_failure(&error),
            }
        }
        cx.notify();
    }

    /// Restores the saved-search page of `entry`, when `entry` is a saved
    /// search. A mounted page of the same saved search keeps its rows. A
    /// deleted saved search gives the empty state and runs no query.
    pub(super) fn restore_saved_search_page_for_nav(
        &mut self,
        entry: &FrameNavigationEntry,
        cx: &mut Context<Self>,
    ) {
        let FrameNavigationEntry::SavedSearch { id, query } = entry else {
            return;
        };
        let restore = saved_search_restore(
            self.saved_search_page.as_ref(),
            *id,
            query,
            self.library.read(cx).saved_searches(),
        );
        match restore {
            SavedSearchRestore::Keep => {}
            SavedSearchRestore::Deleted => {
                self.saved_search_page = Some(SavedSearchPage::deleted(*id, query.clone()));
            }
            SavedSearchRestore::RunQuery(search) => self.run_saved_search(&search, cx),
        }
        cx.notify();
    }

    /// Runs the query of `search`: the Library rows at once, then the Index
    /// request on the ADR 0040 runtime.
    fn run_saved_search(&mut self, search: &SavedSearch, cx: &mut Context<Self>) {
        let results = self.search_results_detail_for_query(&search.query, cx);
        let mut page = SavedSearchPage::open(search, results);
        let endpoint = self
            .musicindex_endpoint
            .require()
            .unwrap_or("Invalid musicindex_endpoint setting")
            .to_owned();
        if let Err(reason) =
            crate::view_models::app_toolbar::AppToolbarVm::index_search_availability(
                self.feature_availability(),
            )
        {
            if let Some(results) = page.results_mut() {
                results.set_index_error(
                    &crate::application::errors::command::CommandError::Unavailable(reason),
                    &endpoint,
                    std::time::SystemTime::now(),
                );
            }
            self.saved_search_page = Some(page);
            return;
        }
        if let Some(results) = page.results_mut() {
            results.mark_index_loading();
        }
        self.saved_search_page = Some(page);

        let command = FetchIndexSearchResults::new(
            self.musicindex_endpoint.clone(),
            search.query.clone(),
            self.library.read(cx).name_feed_matches(),
        );
        let (success_id, success_query) = (search.id, search.query.clone());
        let (error_id, error_query) = (search.id, search.query.clone());
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, rows, cx| {
                if !this.saved_search_result_is_current(success_id, &success_query) {
                    return;
                }
                // Packet 010 "The stale results": the page also checks
                // its own query.
                if this
                    .search_page_mut(SearchPageSlot::SavedSearch)
                    .is_some_and(|page| page.replace_index_results_for_query(&success_query, rows))
                {
                    cx.notify();
                }
            },
            move |this, error, cx| {
                if !this.saved_search_result_is_current(error_id, &error_query) {
                    return;
                }
                if let Some(page) = this
                    .search_page_mut(SearchPageSlot::SavedSearch)
                    .filter(|page| page.query() == error_query)
                {
                    page.set_index_error(&error, &endpoint, std::time::SystemTime::now());
                    cx.notify();
                }
            },
        );
    }

    /// Tells if an Index result for the saved search `id` can write the
    /// mounted page: the mounted page holds that same id and that same
    /// query.
    fn saved_search_result_is_current(&self, id: SavedSearchId, query: &str) -> bool {
        saved_search_result_is_current(self.saved_search_page.as_ref(), id, query)
    }

    /// Selects a tab of the saved-search page.
    fn set_saved_search_tab(&mut self, tab: SearchResultsTab, cx: &mut Context<Self>) {
        if let Some(page) = self.search_page_mut(SearchPageSlot::SavedSearch) {
            page.set_tab(tab);
            cx.notify();
        }
    }

    /// "Delete…" of the saved-search page: opens the confirmation.
    fn request_saved_search_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.saved_search_page.as_ref() else {
            return;
        };
        if page.results().is_none() {
            return;
        }
        let display = delete_saved_search_confirmation(&page.query);
        let id = page.id;
        open_saved_search_delete_dialog(window, cx, display, move |this, cx| {
            this.delete_saved_search(id, cx);
        });
    }

    /// Deletes the saved search `id`. After the delete, the location
    /// changes to Discover with "New releases", as a sidebar click does
    /// (ADR 0084 Decision 9). A failure shows in the status text of the
    /// page, and the location does not change.
    fn delete_saved_search(&mut self, id: SavedSearchId, cx: &mut Context<Self>) {
        let deleted = self
            .library
            .update(cx, |library, cx| library.delete_saved_search(id, cx));
        match deleted {
            Ok(()) => {
                if let Some(page) = self.saved_search_page.as_mut().filter(|page| page.id == id) {
                    *page = SavedSearchPage::deleted(id, page.query.clone());
                }
                self.open_music_page(
                    &location_after_saved_search_delete(),
                    NavigationPathStart::NewPath,
                    cx,
                );
            }
            Err(error) => {
                if let Some(page) = self.search_page_mut(SearchPageSlot::SavedSearch) {
                    page.set_delete_failure(&error);
                }
                cx.notify();
            }
        }
    }

    /// Renders the saved-search page of the detail column: the view model
    /// and the renderer of packet 010, with "Delete…" in its header and the
    /// bar at the bottom of the column.
    pub(super) fn render_saved_search_page_content(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let current = match self.current_music_entry() {
            Some(FrameNavigationEntry::SavedSearch { id, query }) => Some((id, query)),
            _ => None,
        };
        let mounted = self.saved_search_page.as_ref().filter(|page| {
            current
                .as_ref()
                .is_some_and(|(id, query)| page.id == *id && page.query == *query)
        });
        let Some(page) = mounted else {
            return render_search_empty_state(&SavedSearchPage::loading_empty_state(), cx);
        };
        let Some(delete_action) = page.delete_action() else {
            return render_search_empty_state(&SavedSearchPage::deleted_empty_state(), cx);
        };
        let (rows, bar, thumbnail_hrefs) = {
            let library = self.library.read(cx);
            let lookup = library.search_lookup();
            let Some(results) = page.results() else {
                return render_search_empty_state(&SavedSearchPage::deleted_empty_state(), cx);
            };
            (
                results.visible_rows(&lookup),
                results
                    .action_bar(&lookup)
                    .zip(results.selection_rows(&lookup)),
                results.thumbnail_hrefs(),
            )
        };
        let thumbnails = self.resolve_search_result_thumbnails(thumbnail_hrefs, cx);
        let entity = cx.entity();
        let tab_entity = entity.clone();
        let select_entity = entity.clone();
        let toggle_entity = entity.clone();
        let failure_entity = entity.clone();
        let delete_entity = entity;
        let slots = SearchResultsInspectorSlots::new()
            .on_failure_action(move |action, _window, cx: &mut App| {
                failure_entity.update(cx, |this, cx| {
                    this.handle_search_failure_action(SearchPageSlot::SavedSearch, action, cx);
                });
            })
            .on_tab_select(move |tab, _window, cx: &mut App| {
                tab_entity.update(cx, |this, cx| this.set_saved_search_tab(tab, cx));
            })
            .on_result_select(
                move |tab, result_id, key, modifiers, window, cx: &mut App| {
                    select_entity.update(cx, |this, cx| {
                        this.click_search_result_row(
                            SearchPageSlot::SavedSearch,
                            SearchRowClick {
                                tab,
                                result_id,
                                key,
                                modifiers,
                            },
                            window,
                            cx,
                        );
                    });
                },
            )
            .on_row_toggle(move |key, window, cx: &mut App| {
                toggle_entity.update(cx, |this, cx| {
                    this.toggle_search_result_row(SearchPageSlot::SavedSearch, &key, window, cx);
                });
            })
            .header_action(delete_action, move |window, cx: &mut App| {
                delete_entity.update(cx, |this, cx| this.request_saved_search_delete(window, cx));
            })
            .with_thumbnails(thumbnails)
            .with_rows(rows);
        let Some(results) = self.search_page(SearchPageSlot::SavedSearch) else {
            return render_search_empty_state(&SavedSearchPage::deleted_empty_state(), cx);
        };
        let content =
            render_search_results_inspector(results, &slots, SearchResultsHeaderMode::Tabbed, cx);
        let bar = bar.map(|(bar, rows)| {
            self.render_search_action_bar(SearchPageSlot::SavedSearch, bar, &rows, cx)
        });
        self.render_search_list_frame(SearchPageSlot::SavedSearch, content, bar)
    }
}
