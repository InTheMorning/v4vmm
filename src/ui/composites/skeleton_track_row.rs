//! Skeleton row composite that mirrors the dimensions of
//! [`super::TrackRow`] so paged surfaces can render placeholders without
//! the surrounding flex container reflowing when the real row hydrates.
//!
//! Apple HIG: redacted-style placeholder — no shimmer, low-contrast filled
//! shapes that read as "still loading" without competing for attention.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, px, App, ElementId, IntoElement, RenderOnce, SharedString, Window};

use crate::ui::composites::{ListRow, ListRowA11yLabel, ThumbnailSize};
use crate::ui::primitives::Skeleton;
use crate::ui::tokens::{Radius, Spacing};

/// Skeleton track row sized to match [`super::TrackRow`].
///
/// Use when a paged list returned `RowSlot::Pending` so the parent flex
/// container does not jump when the real row arrives.
#[derive(IntoElement)]
#[must_use]
pub struct SkeletonTrackRow {
    id: ElementId,
    show_duration: bool,
    show_thumbnail: bool,
}

impl SkeletonTrackRow {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            show_duration: true,
            show_thumbnail: true,
        }
    }

    /// Drop the trailing duration block when the real row will not show one.
    pub const fn show_duration(mut self, show: bool) -> Self {
        self.show_duration = show;
        self
    }

    /// Drop the leading thumbnail block when the real row will not show one.
    pub const fn show_thumbnail(mut self, show: bool) -> Self {
        self.show_thumbnail = show;
        self
    }
}

impl RenderOnce for SkeletonTrackRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let gap = Spacing::SM.scaled(cx);
        let thumb_size = ThumbnailSize::Sm.scaled(cx);
        // Match the body-text micro line-height TrackRow uses.
        let label_h = px(16.0);

        let mut body = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_row()
            .items_center()
            .gap(gap)
            // Track-number gutter — same width as the real number column.
            .child(
                div()
                    .w(crate::ui::layouts::TRACK_NUMBER_WIDTH)
                    .flex_shrink_0()
                    .child(Skeleton::block(px(12.0), label_h)),
            );

        if self.show_thumbnail {
            body = body.child(
                div()
                    .flex_shrink_0()
                    .child(Skeleton::block(thumb_size, thumb_size).radius(Radius::SM)),
            );
        }

        body = body.child(
            div()
                .flex_1()
                .min_w_0()
                .child(Skeleton::row().full_width().radius(Radius::SM)),
        );

        if self.show_duration {
            body = body.child(
                div()
                    .flex_shrink_0()
                    .child(Skeleton::block(px(32.0), label_h)),
            );
        }

        let row = ListRow::compact(self.id)
            .a11y_label(ListRowA11yLabel {
                label: SharedString::from("Loading track"),
            })
            .child(body);
        // The real TrackRow guarantees MIN_HIT_TARGET; mirror that so a
        // mostly-pending list does not produce a shorter overall column.
        div().min_h(crate::ui::layouts::MIN_HIT_TARGET).child(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_show_thumbnail_and_duration() {
        let row = SkeletonTrackRow::new("sk-1");
        assert!(row.show_thumbnail);
        assert!(row.show_duration);
    }

    #[test]
    fn duration_can_be_hidden() {
        let row = SkeletonTrackRow::new("sk-2").show_duration(false);
        assert!(!row.show_duration);
    }

    #[test]
    fn thumbnail_can_be_hidden() {
        let row = SkeletonTrackRow::new("sk-3").show_thumbnail(false);
        assert!(!row.show_thumbnail);
    }
}
