//! `ContentList` breadcrumb handlers and label helpers.

use gpui::Context;

use crate::view_models::workspace::{
    FrameNavigationEntry, FrameNavigationState, ReleaseKey, TrackKey, WorkspaceFrameId,
};

use super::TopApp;

impl TopApp {
    pub(super) fn content_list_breadcrumb_labels(
        &self,
        content_list_id: WorkspaceFrameId,
        cx: &mut Context<Self>,
    ) -> Vec<(FrameNavigationEntry, String)> {
        self.workspace_layout
            .frame_nav(content_list_id)
            .map(FrameNavigationState::path_entries)
            .unwrap_or_default()
            .into_iter()
            .map(|entry| {
                let label = self.content_list_breadcrumb_label(&entry, cx);
                (entry, label)
            })
            .collect()
    }

    fn content_list_breadcrumb_label(
        &self,
        entry: &FrameNavigationEntry,
        cx: &mut Context<Self>,
    ) -> String {
        match entry {
            // ADR 0084 Decision 9: the Library gives the names of its pages.
            // ADR 0084 Decision 6: a singles group page is one of them.
            FrameNavigationEntry::PlaylistDetail(_) | FrameNavigationEntry::SinglesGroup(_) => {
                self.library.read(cx).navigation_label(entry)
            }
            // ADR 0084 Decision 4: the Library album name, the loaded Index
            // title, or the label the opening row gave.
            FrameNavigationEntry::Release(key) => self.release_navigation_label(key, cx),
            // ADR 0084 Decision 4: the Library track title, the loaded
            // Index title, or the label the opening row gave.
            FrameNavigationEntry::Track(key) => self.track_navigation_label(key, cx),
            // ADR 0077 packet 004, R4-04: the breadcrumb text is the
            // publisher page view model title, when the page has loaded.
            FrameNavigationEntry::PublisherDetail(publisher_feed_guid) => self
                .publisher_page_breadcrumb_text()
                .unwrap_or_else(|| publisher_feed_guid.clone()),
            _ => entry.display_label(),
        }
    }

    /// The breadcrumb label of a release entry (ADR 0084 Decision 4): the
    /// Library album name, the loaded Index title, or the label the
    /// opening row gave. "Release" shows when none of these is known.
    ///
    /// Each segment uses its own key, never the source of the open page, so
    /// a path with two releases shows two titles.
    fn release_navigation_label(&self, key: &ReleaseKey, cx: &mut Context<Self>) -> String {
        if let Some(label) = self.library.read(cx).release_label(key) {
            return label;
        }
        match key {
            ReleaseKey::FeedGuid(feed_guid) => self
                .index_feed_loaded_title(feed_guid)
                .unwrap_or_else(|| self.release_opening_label(feed_guid)),
            ReleaseKey::LibraryFeed(_) => "Release".to_string(),
        }
    }

    /// The breadcrumb label of a track entry (ADR 0084 Decision 4): the
    /// Library track title, the loaded Index title, or the label the
    /// opening row gave. "Track" shows when none of these is known.
    ///
    /// Each segment uses its own key, never the source of the open page, so
    /// a path with two tracks shows two titles.
    fn track_navigation_label(&self, key: &TrackKey, cx: &mut Context<Self>) -> String {
        if let Some(label) = self.library.read(cx).track_label(key) {
            return label;
        }
        let target = key.index_target();
        self.index_track_loaded_title(&target)
            .unwrap_or_else(|| self.track_opening_label(&target))
    }

    /// Opens a breadcrumb segment (ADR 0084 Decision 9). The segment is a
    /// page of the current location, so the push cuts the path at that page
    /// and adds one history entry. The page restores through the same
    /// function that Back and Forward use.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "breadcrumb entries are command payloads cloned by GPUI listener dispatch"
    )]
    pub(super) fn handle_content_list_breadcrumb_select(
        &mut self,
        entry: FrameNavigationEntry,
        cx: &mut Context<Self>,
    ) {
        let Some(content_list_id) = self.content_list_frame_id() else {
            return;
        };
        if self
            .workspace_layout
            .push_nav(content_list_id, entry.clone())
            .is_ok()
        {
            // ADR 0084 task 030, decision D5 (review of 2026-10-21): a
            // breadcrumb segment is a history move, so it shows the query
            // of a restored search page in the toolbar field.
            self.pending_toolbar_search.record(&entry);
            self.restore_content_list_nav_entry(content_list_id, &entry, cx);
        }
    }
}
