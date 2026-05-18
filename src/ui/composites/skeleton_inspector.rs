//! Skeleton composite for the discover inspector body.
//!
//! Rendered while an `InspectorDetail::Loading` frame is on top of the
//! inspector stack. The shape mirrors the layout of a populated inspector
//! (large hero thumbnail, title block, a couple of subtitle bars, body
//! rows) so the placeholder occupies the same footprint as the real
//! content and the surrounding UI does not jump when data arrives.
//!
//! Apple HIG note: matches `.redacted(reason: .placeholder)` styling —
//! muted blocks, no shimmer animation.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, App, IntoElement, RenderOnce, Window};

use crate::ui::layouts as layout;
use crate::ui::primitives::Skeleton;
use crate::ui::tokens::{Radius, SkeletonBlock, Spacing};

/// Skeleton placeholder for the inspector detail pane.
///
/// `body_rows` controls how many text-row skeletons are rendered below
/// the header block. Tune per entity kind so the footprint roughly
/// matches the populated layout.
#[derive(IntoElement)]
#[must_use]
pub struct SkeletonInspector {
    body_rows: usize,
    show_subtitle: bool,
}

impl SkeletonInspector {
    pub const fn new() -> Self {
        Self {
            body_rows: 4,
            show_subtitle: true,
        }
    }

    pub const fn body_rows(mut self, rows: usize) -> Self {
        self.body_rows = rows;
        self
    }

    pub const fn show_subtitle(mut self, show: bool) -> Self {
        self.show_subtitle = show;
        self
    }
}

impl Default for SkeletonInspector {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for SkeletonInspector {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let gap = Spacing::MD.scaled(cx);
        let pad = Spacing::LG.scaled(cx);

        let mut header =
            div().flex().flex_row().gap(gap).child(
                Skeleton::block(layout::THUMBNAIL_XL, layout::THUMBNAIL_XL).radius(Radius::MD),
            );

        let (title_w, title_h) = SkeletonBlock::InspectorTitle.scaled(cx);
        let (subtitle_w, subtitle_h) = SkeletonBlock::InspectorSubtitle.scaled(cx);
        let (caption_w, caption_h) = SkeletonBlock::InspectorCaption.scaled(cx);

        let mut title_block = div()
            .flex()
            .flex_col()
            .flex_1()
            .gap(gap)
            .child(Skeleton::block(title_w, title_h).radius(Radius::SM));
        if self.show_subtitle {
            title_block = title_block
                .child(Skeleton::block(subtitle_w, subtitle_h).radius(Radius::SM))
                .child(Skeleton::block(caption_w, caption_h).radius(Radius::SM));
        }
        header = header.child(title_block);

        let mut body = div().flex().flex_col().gap(gap);
        for _ in 0..self.body_rows {
            body = body.child(Skeleton::row().full_width());
        }

        div()
            .flex()
            .flex_col()
            .gap(gap)
            .p(pad)
            .child(header)
            .child(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_show_subtitle_and_four_rows() {
        let s = SkeletonInspector::new();
        assert!(s.show_subtitle);
        assert_eq!(s.body_rows, 4);
    }

    #[test]
    fn body_rows_override() {
        let s = SkeletonInspector::new().body_rows(8);
        assert_eq!(s.body_rows, 8);
    }

    #[test]
    fn subtitle_can_be_hidden() {
        let s = SkeletonInspector::new().show_subtitle(false);
        assert!(!s.show_subtitle);
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(
            SkeletonInspector::default().body_rows,
            SkeletonInspector::new().body_rows
        );
    }
}
