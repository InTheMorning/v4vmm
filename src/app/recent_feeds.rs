//! Recent Feeds workspace integration.

use gpui::{Context, Entity, IntoElement};

use crate::ui::shells::recent_feeds::{render_recent_feeds_page, RecentFeedsPageSlots};
use crate::ui::shells::workspace::WorkspaceSlots;
use crate::view_models::recent_feeds::{RecentFeedsPageVm, RecentFeedsViewMode};
use crate::view_models::workspace::{FrameNavigationEntry, WorkspaceFrameId};

use super::TopApp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IndexFeedDetailOrigin {
    Search,
    RecentFeeds,
}

impl TopApp {
    pub(super) fn set_recent_feeds_view_mode(
        &mut self,
        view_mode: RecentFeedsViewMode,
        cx: &mut Context<Self>,
    ) {
        if let Some(detail) = &mut self.recent_feeds_detail {
            detail.set_view_mode(view_mode);
            cx.notify();
        }
    }

    pub(super) fn content_list_index_feed_detail_origin(
        &self,
        content_list_id: WorkspaceFrameId,
    ) -> Option<IndexFeedDetailOrigin> {
        let nav = self.workspace_layout.frame_nav(content_list_id)?;
        if !matches!(nav.current(), FrameNavigationEntry::IndexFeedDetail { .. }) {
            return None;
        }

        nav.path_entries()
            .into_iter()
            .rev()
            .skip(1)
            .find_map(|entry| match entry {
                FrameNavigationEntry::RecentFeeds => Some(IndexFeedDetailOrigin::RecentFeeds),
                FrameNavigationEntry::Search(_) => Some(IndexFeedDetailOrigin::Search),
                _ => None,
            })
    }

    pub(super) fn render_recent_feeds_content(
        &mut self,
        entity: &Entity<Self>,
        queue_frame: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> WorkspaceSlots {
        if self.recent_feeds_detail.is_none() {
            self.recent_feeds_detail = Some(RecentFeedsPageVm::loading());
            self.start_recent_feeds_load(false, cx);
        }

        let recent_thumbnails = self
            .recent_feeds_detail
            .as_ref()
            .map(RecentFeedsPageVm::feed_thumbnail_sources)
            .unwrap_or_default()
            .into_iter()
            .map(|(id, url)| {
                let image = self.index_remote_detail_hero_image(&url, cx);
                (id, image)
            })
            .collect();
        let recent_feeds = self.recent_feeds_detail.as_ref().unwrap();
        let select_entity = entity.clone();
        let mode_entity = entity.clone();
        let load_more_entity = entity.clone();
        let recent_slots = RecentFeedsPageSlots::new()
            .with_scroll_handle(self.recent_feeds_scroll.clone())
            .with_thumbnails(recent_thumbnails)
            .on_view_mode_select(move |view_mode, _window, cx| {
                mode_entity.update(cx, |this, cx| {
                    this.set_recent_feeds_view_mode(view_mode, cx);
                });
            })
            .on_load_more(move |_window, cx| {
                load_more_entity.update(cx, |this, cx| {
                    this.start_recent_feeds_load(true, cx);
                });
            })
            .on_result_select(move |result_id, _window, cx| {
                select_entity.update(cx, |this, cx| {
                    this.handle_recent_feed_selected(&result_id, cx);
                });
            });
        let recent_content = render_recent_feeds_page(recent_feeds, &recent_slots, cx);
        WorkspaceSlots::new()
            .content_list(recent_content)
            .queue_now_playing(queue_frame)
    }
}
