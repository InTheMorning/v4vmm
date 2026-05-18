//! Skeleton tile composite that mirrors the dimensions of the discover
//! recent-feeds tile so prefetch placeholders do not cause grid reflow.
//!
//! Sized to match the `RecentFeedTile` in
//! `src/ui/shells/discover/recent.rs`.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, App, ElementId, IntoElement, RenderOnce, Styled, Window};

use crate::ui::layouts as layout;
use crate::ui::primitives::Skeleton;
use crate::ui::tokens::{Radius, SkeletonBlock, Spacing};

/// Skeleton feed-tile sized to match the discover recent-feeds tile.
#[derive(IntoElement)]
#[must_use]
pub struct SkeletonFeedTile {
    id: ElementId,
    show_subtitle: bool,
}

impl SkeletonFeedTile {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            show_subtitle: true,
        }
    }

    pub const fn show_subtitle(mut self, show: bool) -> Self {
        self.show_subtitle = show;
        self
    }
}

impl RenderOnce for SkeletonFeedTile {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let gap = Spacing::SM.scaled(cx);
        let pad = Spacing::SM.scaled(cx);
        let radius_lg = Radius::LG.scaled(cx);
        let (subtitle_w, subtitle_h) = SkeletonBlock::FeedTileSubtitle.scaled(cx);

        let mut tile = div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(gap)
            .w(layout::SEARCH_TILE_WIDTH)
            .p(pad)
            .rounded(radius_lg)
            // Album-art block — same footprint as the real thumbnail.
            .child(div().flex_shrink_0().child(
                Skeleton::block(layout::THUMBNAIL_XL, layout::THUMBNAIL_XL).radius(Radius::MD),
            ))
            // Title block.
            .child(
                div()
                    .w(layout::THUMBNAIL_XL)
                    .child(Skeleton::row().full_width()),
            );

        if self.show_subtitle {
            tile = tile.child(
                div()
                    .w(layout::THUMBNAIL_XL)
                    .child(Skeleton::block(subtitle_w, subtitle_h)),
            );
        }

        tile
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_show_subtitle() {
        let tile = SkeletonFeedTile::new("st-1");
        assert!(tile.show_subtitle);
    }

    #[test]
    fn subtitle_can_be_hidden() {
        let tile = SkeletonFeedTile::new("st-2").show_subtitle(false);
        assert!(!tile.show_subtitle);
    }
}
