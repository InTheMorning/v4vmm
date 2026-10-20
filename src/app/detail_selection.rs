//! The Music detail column selection of the top app (ADR 0084 Decision 10,
//! packet 018).
//!
//! `LibraryApp` holds the one detail selection. The top app gives the rows
//! of the open page, because it renders the Index release page and the
//! feed page itself. It also composes the detail column: the keyboard
//! focus, the header button and the action bar. Library actions go to
//! `LibraryApp`. Index downloads stay here.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, App, Context, Modifiers, SharedString, Window};

use super::publisher_dispatch::PublisherPageState;
use super::search_dispatch::feed_guid_from_view;
use super::TopApp;
use crate::application::library_removal::LibraryRemovalIntent;
use crate::library::{playlist_options, LibraryApp};
use crate::ui::composites::{SelectionActionBar, SelectionActionBarPlaylists};
use crate::view_models::detail_selection::{
    detail_header_action, feed_page_release_rows, index_release_track_rows, DetailListKind,
    DetailPageRows, DetailRowKey,
};
use crate::view_models::list_selection::FocusStep;
use crate::view_models::publisher_page::PublisherPageVm;
use crate::view_models::search_results::IndexDetailDisplay;
use crate::view_models::selection_action_bar::{SelectionActionBarVm, SelectionActionKind};
use crate::view_models::workspace::{
    FrameHeaderActionDisplay, FrameNavigationEntry, ReleaseKey, ReleasePageSource,
};
use crate::views::FeedView;

impl TopApp {
    /// The open Music page.
    pub(super) fn current_music_entry(&self) -> Option<FrameNavigationEntry> {
        let content_list_id = self.content_list_frame_id()?;
        self.workspace_layout
            .frame_nav(content_list_id)
            .map(|nav| nav.current().clone())
    }

    /// The display of the Index release page of `feed_guid`. While the
    /// release page's own fetch loads, the cached content list row shows
    /// at once, when the content list holds it.
    pub(super) fn index_release_page_detail(
        &self,
        feed_guid: &str,
        cx: &App,
    ) -> IndexDetailDisplay {
        let activation_id = format!("index-feed:{feed_guid}");
        let label = self
            .release_page_labels
            .get(feed_guid)
            .cloned()
            .unwrap_or_else(|| feed_guid.to_owned());
        self.index_feed_detail_is_pending(feed_guid)
            .then(|| {
                self.library.read(cx).recent_music_index_feed_detail(
                    &activation_id,
                    feed_guid,
                    &label,
                )
            })
            .flatten()
            .unwrap_or_else(|| self.index_feed_detail_display(feed_guid, &label))
    }

    /// The feed of the open Index release page, when it shows one.
    pub(super) fn index_release_page_feed(&self, cx: &App) -> Option<FeedView> {
        let Some(ReleasePageSource::Index { feed_guid }) = &self.release_page_source else {
            return None;
        };
        self.index_release_page_detail(feed_guid, cx).feed
    }

    /// The selectable rows of the open detail page (ADR 0084 packet 018):
    /// a release page, a singles group page or a feed page. `None` for
    /// each other page.
    pub(super) fn detail_page_rows(&self, cx: &App) -> Option<DetailPageRows> {
        let entry = self.current_music_entry()?;
        DetailListKind::for_entry(&entry)?;
        let library = self.library.read(cx);
        match entry {
            FrameNavigationEntry::Release(_) => match &self.release_page_source {
                Some(ReleasePageSource::Index { .. }) => {
                    let feed = self.index_release_page_feed(cx)?;
                    Some(DetailPageRows::Tracks(index_release_track_rows(
                        feed_guid_from_view(&feed).as_deref(),
                        &feed.tracks,
                    )))
                }
                Some(ReleasePageSource::Library { .. }) | None => {
                    library.library_detail_page_rows()
                }
            },
            FrameNavigationEntry::SinglesGroup(_) => library.library_detail_page_rows(),
            FrameNavigationEntry::PublisherDetail(_) => {
                let Some(PublisherPageState::Loaded { facts, .. }) = &self.publisher_page else {
                    return None;
                };
                let sections = PublisherPageVm::new(facts.as_ref().clone()).sections();
                Some(DetailPageRows::Releases(feed_page_release_rows(
                    &sections,
                    |key| library.library_release_facts(key),
                )))
            }
            _ => None,
        }
    }

    /// The "Select" and "Done" button of the detail column header.
    pub(super) fn detail_column_header_action(&self, cx: &App) -> Option<FrameHeaderActionDisplay> {
        let rows = self.detail_page_rows(cx);
        let mode = self.library.read(cx).detail_selection().mode();
        detail_header_action(rows.as_ref(), mode)
    }

    /// The click of the "Select" and "Done" button.
    pub(super) fn toggle_detail_selection_mode(&mut self, cx: &mut Context<Self>) {
        self.library
            .update(cx, LibraryApp::toggle_detail_selection_mode);
        cx.notify();
    }

    /// A click on the body of a row of a page that the top app renders.
    /// A plain click in `Normal` mode opens the page of the row.
    pub(super) fn click_detail_row(
        &mut self,
        key: &DetailRowKey,
        label: String,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) {
        let Some(rows) = self.detail_page_rows(cx) else {
            return;
        };
        let open = self.library.update(cx, |library, cx| {
            library.click_detail_row(key, modifiers, &rows, cx)
        });
        if open {
            self.open_detail_row(key, label, cx);
        }
        cx.notify();
    }

    /// A click on the checkbox of a row of a page that the top app
    /// renders.
    pub(super) fn toggle_detail_row(&mut self, key: &DetailRowKey, cx: &mut Context<Self>) {
        self.library
            .update(cx, |library, cx| library.toggle_detail_row(key, cx));
        cx.notify();
    }

    fn open_detail_row(&mut self, key: &DetailRowKey, label: String, cx: &mut Context<Self>) {
        let Some(content_frame_id) = self.content_list_frame_id() else {
            return;
        };
        match key {
            DetailRowKey::Track(key) => self.open_track_in_frame(content_frame_id, key, label, cx),
            DetailRowKey::Release(key) => {
                self.open_release_in_frame(content_frame_id, key, label, cx);
            }
        }
    }

    /// Up, Down, Page Up, Page Down, Home and End, for the detail column.
    /// `LibraryApp` acts only while the detail column carries the focus.
    pub(super) fn move_detail_selection_focus(
        &mut self,
        step: FocusStep,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rows = self.detail_page_rows(cx);
        self.library.update(cx, |library, cx| {
            library.move_detail_focus(step, rows.as_ref(), window, cx);
        });
    }

    /// Space, for the detail column.
    pub(super) fn toggle_focused_detail_row(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rows = self.detail_page_rows(cx);
        self.library.update(cx, |library, cx| {
            library.toggle_focused_detail_row(rows.as_ref(), window, cx);
        });
    }

    /// Ctrl+A, for the detail column.
    pub(super) fn select_all_detail_rows(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.detail_page_rows(cx);
        self.library.update(cx, |library, cx| {
            library.select_all_detail_rows(rows.as_ref(), window, cx);
        });
    }

    /// Enter, for the detail column: opens the page of the row with the
    /// keyboard focus.
    pub(super) fn open_focused_detail_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.library.read(cx).focused_detail_row(window, cx) else {
            return;
        };
        let label = self
            .detail_page_rows(cx)
            .and_then(|rows| rows.title_of(&key).map(str::to_owned))
            .unwrap_or_default();
        self.open_detail_row(&key, label, cx);
    }

    /// Escape, for the detail column. Gives `false` outside selection
    /// mode, so the caller keeps the Back action of Escape.
    pub(super) fn end_detail_selecting(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.library
            .update(cx, |library, cx| library.end_detail_selecting(window, cx))
    }

    /// The detail column content with its keyboard focus and its action
    /// bar (ADR 0084 packet 018). A click in the column, outside a text
    /// field, gives the column the keyboard focus. The bar shows at the
    /// bottom of the column, in selection mode only.
    pub(super) fn render_detail_column_frame(
        &mut self,
        content: AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let focus = self.library.read(cx).detail_column_focus().clone();
        let bar = self
            .detail_page_rows(cx)
            .and_then(|rows| rows.action_bar(self.library.read(cx).detail_selection()));
        let bar = bar.map(|bar| self.render_detail_action_bar(bar, cx));
        div()
            .id("music-detail-column")
            .track_focus(&focus)
            .size_full()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .child(content),
            )
            .children(bar)
            .into_any_element()
    }

    fn render_detail_action_bar(
        &mut self,
        bar: SelectionActionBarVm,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let playlists = SelectionActionBarPlaylists {
            playlists: playlist_options(self.library.read(cx).playlists()),
            new_playlist_a11y_label: SharedString::from("Create a new playlist"),
            back_a11y_label: SharedString::from("Back to playlist choices"),
            create_a11y_label: SharedString::from("Create playlist and add the selection"),
        };
        // `cx.listener` gives a handler that reads an event by reference.
        // Download, Remove and Clear act on no event, so each wraps the
        // listener with a throwaway `&()`.
        let download = cx.listener(|this, (): &(), window, cx| {
            this.run_detail_selection_action(SelectionActionKind::Download, window, cx);
        });
        let remove = cx.listener(|this, (): &(), window, cx| {
            this.run_detail_selection_action(SelectionActionKind::Remove, window, cx);
        });
        let clear = cx.listener(|this, (): &(), window, cx| {
            this.run_detail_selection_action(SelectionActionKind::Clear, window, cx);
        });
        SelectionActionBar::new(bar, playlists)
            .on_download(move |window, cx| download(&(), window, cx))
            .on_remove(move |window, cx| remove(&(), window, cx))
            .on_clear(move |window, cx| clear(&(), window, cx))
            .on_add_to_playlist(cx.listener(|this, playlist_id: &i64, _window, cx| {
                this.add_detail_selection_to_playlist(*playlist_id, cx);
            }))
            .on_create_playlist(cx.listener(|this, name: &String, _window, cx| {
                this.create_playlist_and_add_detail_selection(name, cx);
            }))
            .into_any_element()
    }

    /// Runs "Download", "Remove…" or "Clear" of the detail bar. The
    /// selection stays after each action. "Add to playlist" opens its own
    /// popover.
    fn run_detail_selection_action(
        &mut self,
        kind: SelectionActionKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(rows) = self.detail_page_rows(cx) else {
            return;
        };
        match kind {
            SelectionActionKind::Download => self.download_detail_selection(&rows, cx),
            SelectionActionKind::Remove => {
                let (items, item_noun) = {
                    let selection = self.library.read(cx).detail_selection();
                    detail_removal_items(&rows, selection)
                };
                self.library.update(cx, |library, cx| {
                    library.request_selection_removal(items, item_noun, window, cx);
                });
            }
            SelectionActionKind::Clear => {
                self.library.update(cx, LibraryApp::clear_detail_rows);
            }
            SelectionActionKind::AddToPlaylist => {}
        }
        cx.notify();
    }

    /// "Download n tracks" and "Download n releases" of the detail bar.
    /// Each item queues one job in the download queue of `LibraryApp`
    /// (ADR 0084 Decision 10).
    fn download_detail_selection(&mut self, rows: &DetailPageRows, cx: &mut Context<Self>) {
        let selection = self.library.read(cx).detail_selection().clone();
        match rows {
            DetailPageRows::Tracks(_) => {
                let selected = rows.selected_tracks(&selection);
                let library_ids: Vec<i64> = selected
                    .iter()
                    .filter(|row| !row.downloaded)
                    .filter_map(|row| row.library_track_id)
                    .collect();
                let index_keys: Vec<_> = selected
                    .iter()
                    .filter(|row| row.library_track_id.is_none() && !row.downloaded)
                    .filter_map(|row| row.key.clone())
                    .collect();
                self.library.update(cx, |library, cx| {
                    library.download_library_tracks(&library_ids, cx);
                });
                if index_keys.is_empty() {
                    return;
                }
                let Some(feed) = self.index_release_page_feed(cx) else {
                    return;
                };
                let feed_guid = feed_guid_from_view(&feed);
                let index_rows = index_release_track_rows(feed_guid.as_deref(), &feed.tracks);
                for (row, track) in index_rows.iter().zip(&feed.tracks) {
                    if row.key.as_ref().is_some_and(|key| index_keys.contains(key)) {
                        self.download_index_track(&feed, track, cx);
                    }
                }
            }
            DetailPageRows::Releases(_) => {
                let selected = rows.selected_releases(&selection);
                let library_feed_ids: Vec<i64> = selected
                    .iter()
                    .filter_map(|row| row.library)
                    .filter(|facts| !facts.fully_downloaded)
                    .map(|facts| facts.feed_id)
                    .collect();
                let index_releases: Vec<(String, String)> = selected
                    .iter()
                    .filter(|row| row.library.is_none())
                    .filter_map(|row| match &row.key {
                        ReleaseKey::FeedGuid(feed_guid) => {
                            Some((feed_guid.clone(), row.title.clone()))
                        }
                        ReleaseKey::LibraryFeed(_) => None,
                    })
                    .collect();
                self.library.update(cx, |library, cx| {
                    library.download_library_feeds(&library_feed_ids, cx);
                });
                for (feed_guid, title) in index_releases {
                    self.download_index_album_of_track(feed_guid, Some(title), cx);
                }
            }
        }
    }

    /// "Add n tracks to playlist" and "Add n releases to playlist" of the
    /// detail bar. Only a Library item goes into a playlist.
    fn add_detail_selection_to_playlist(&mut self, playlist_id: i64, cx: &mut Context<Self>) {
        let Some(rows) = self.detail_page_rows(cx) else {
            return;
        };
        let (track_ids, feed_ids) = {
            let selection = self.library.read(cx).detail_selection();
            detail_library_ids(&rows, selection)
        };
        self.library.update(cx, |library, cx| match rows {
            DetailPageRows::Tracks(_) => library.add_tracks_to_playlist(track_ids, playlist_id, cx),
            DetailPageRows::Releases(_) => {
                library.add_albums_to_playlist(&feed_ids, playlist_id, cx);
            }
        });
    }

    /// The "New Playlist" create flow of the detail bar's "Add to
    /// playlist" popover.
    fn create_playlist_and_add_detail_selection(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(rows) = self.detail_page_rows(cx) else {
            return;
        };
        let (track_ids, feed_ids) = {
            let selection = self.library.read(cx).detail_selection();
            detail_library_ids(&rows, selection)
        };
        self.library.update(cx, |library, cx| match rows {
            DetailPageRows::Tracks(_) => {
                library.create_playlist_and_add_tracks(name, track_ids, cx);
            }
            DetailPageRows::Releases(_) => {
                library.create_playlist_and_add_albums(name, feed_ids, cx);
            }
        });
    }
}

/// The Library track ids and the Library feed ids of the selected rows,
/// for "Add to playlist".
fn detail_library_ids(
    rows: &DetailPageRows,
    selection: &crate::view_models::detail_selection::DetailSelection,
) -> (Vec<i64>, Vec<i64>) {
    let track_ids = rows
        .selected_tracks(selection)
        .iter()
        .filter_map(|row| row.library_track_id)
        .collect();
    let feed_ids = rows
        .selected_releases(selection)
        .iter()
        .filter_map(|row| row.library.map(|facts| facts.feed_id))
        .collect();
    (track_ids, feed_ids)
}

/// The removal items of "Remove n tracks…" and "Remove n releases…": each
/// downloaded Library track, or each fully downloaded Library release.
fn detail_removal_items(
    rows: &DetailPageRows,
    selection: &crate::view_models::detail_selection::DetailSelection,
) -> (Vec<(LibraryRemovalIntent, String)>, &'static str) {
    match rows {
        DetailPageRows::Tracks(_) => (
            rows.selected_tracks(selection)
                .iter()
                .filter(|row| row.removable())
                .filter_map(|row| {
                    row.library_track_id
                        .map(|id| (LibraryRemovalIntent::TrackId(id), row.title.clone()))
                })
                .collect(),
            "track",
        ),
        DetailPageRows::Releases(_) => (
            rows.selected_releases(selection)
                .iter()
                .filter_map(|row| row.library.map(|facts| (facts, row.title.clone())))
                .filter(|(facts, _)| facts.removable())
                .map(|(facts, title)| (LibraryRemovalIntent::FeedId(facts.feed_id), title))
                .collect(),
            "release",
        ),
    }
}
