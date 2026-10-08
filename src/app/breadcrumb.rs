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
            // ADR 0084 Decision 9: the Library gives the names of its pages.
            FrameNavigationEntry::AlbumDetail(_)
            | FrameNavigationEntry::TrackDetail(_)
            | FrameNavigationEntry::PlaylistDetail(_) => {
                self.library.read(cx).navigation_label(entry)
            }
            // ADR 0077 packet 004, R4-04: the breadcrumb text is the
            // publisher page view model title, when the page has loaded.
            FrameNavigationEntry::PublisherDetail(publisher_feed_guid) => self
                .publisher_page_breadcrumb_text()
                .unwrap_or_else(|| publisher_feed_guid.clone()),
            _ => entry.display_label(),
        }
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
            self.restore_content_list_nav_entry(content_list_id, &entry, cx);
        }
    }
}
