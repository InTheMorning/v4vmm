//! `ContentList` breadcrumb handlers and label helpers.

use gpui::Context;

use crate::view_models::workspace::{FrameNavigationEntry, FrameNavigationState, WorkspaceFrameId};

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
            FrameNavigationEntry::AlbumDetail(feed_id) => self
                .library
                .read(cx)
                .album_for_detail_by_feed_id(*feed_id)
                .map_or_else(|| "Album".to_string(), |album| album.name),
            _ => entry.display_label(),
        }
    }

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
            .pop_nav_until(content_list_id, &entry)
            .is_ok()
        {
            // Sync search_results_detail: if nav top is no longer Search, clear it
            self.sync_search_results_detail_with_nav(content_list_id);

            // Hydrate LibraryApp detail to match the new nav top
            self.library.update(cx, |library, cx| {
                library.hydrate_detail_from_nav(&entry, cx);
            });
            if let FrameNavigationEntry::Search(query) = &entry {
                self.start_index_search_for_query(query, cx);
            }
            if matches!(entry, FrameNavigationEntry::RecentFeeds)
                && self.recent_feeds_detail.is_none()
            {
                self.recent_feeds_detail =
                    Some(crate::view_models::recent_feeds::RecentFeedsPageVm::loading());
                self.start_recent_feeds_load(false, cx);
            }

            cx.notify();
        }
    }
}
