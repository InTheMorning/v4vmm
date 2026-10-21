//! The saved searches of the Library (ADR 0084 Decision 3, packet 012).
//!
//! `LibraryApp` holds the saved searches for the Discover column. It reads
//! them at start, and again after each save and each delete. Only
//! `src/db/saved_searches.rs` writes the table.

#![warn(clippy::pedantic)]

use gpui::Context;

use super::LibraryApp;
use crate::db::saved_searches::{self, SavedSearchInsert, StoredStartTab};
use crate::view_models::search_results::{SavedSearch, SavedSearchId, SearchResultsTab};
use crate::view_models::workspace::NavigationPathStart;

impl LibraryApp {
    /// The saved searches, in `id` order.
    pub(crate) fn saved_searches(&self) -> &[SavedSearch] {
        &self.saved_searches
    }

    /// Reads the saved searches again. A read failure keeps the present
    /// list and shows the error in the Library status.
    pub(crate) fn reload_saved_searches(&mut self) {
        let read = {
            let conn = self.conn.lock().expect("lock db");
            saved_searches::saved_searches(&conn)
        };
        match read {
            Ok(stored) => {
                self.saved_searches = stored.into_iter().map(SavedSearch::from_stored).collect();
            }
            Err(error) => self.vm.set_error_status(format!(
                "The app could not read the saved searches: {error:#}"
            )),
        }
    }

    /// Saves `query` with the tab `start_tab`. `created_at` is the time of
    /// the click, in Unix seconds. A query that is already saved changes
    /// nothing. The error text names the database error.
    pub(crate) fn save_search(
        &mut self,
        query: &str,
        start_tab: SearchResultsTab,
        created_at: i64,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let inserted = {
            let conn = self.conn.lock().expect("lock db");
            saved_searches::insert_saved_search(
                &conn,
                query,
                StoredStartTab::from(start_tab),
                created_at,
            )
        };
        let result = match inserted {
            Ok(SavedSearchInsert::Inserted(_) | SavedSearchInsert::AlreadySaved) => Ok(()),
            Err(error) => Err(format!("{error:#}")),
        };
        self.reload_saved_searches();
        cx.notify();
        result
    }

    /// Deletes the saved search `id`. The Library and the downloads do not
    /// change. The error text names the database error.
    pub(crate) fn delete_saved_search(
        &mut self,
        id: SavedSearchId,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let deleted = {
            let conn = self.conn.lock().expect("lock db");
            saved_searches::delete_saved_search(&conn, id.0)
        };
        self.reload_saved_searches();
        cx.notify();
        deleted.map(|_| ()).map_err(|error| format!("{error:#}"))
    }

    /// Opens a saved-search row directly under Discover (ADR 0084
    /// Decisions 3 and 9), as `open_discover_view` does.
    pub(crate) fn open_saved_search_row(&mut self, search: &SavedSearch, cx: &mut Context<Self>) {
        self.open_page(search.entry(), NavigationPathStart::ListRow, cx);
    }

    /// Sets the saved search of the current location, for the open row of
    /// the Discover column.
    pub(crate) fn set_open_saved_search(&mut self, id: Option<SavedSearchId>) {
        self.open_saved_search = id;
    }
}
