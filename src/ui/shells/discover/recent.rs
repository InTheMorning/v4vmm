//! Discover recent-feeds surface.
//!
//! Renders the recent-feed tile grid shown at the Discover root when no search
//! term is active. `SearchApp` keeps loading and inspector-selection state.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Image, SharedString, Styled};

use crate::search::SearchApp;
use crate::ui::composites::RecentFeedTile;
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button as UiButton;
use crate::ui::style::{color, spacing, typography};
use crate::view_models::search::{RecentFeedTileDisplay, RecentFeedsDisplay};

pub(crate) struct DiscoverRecentTile {
    display: RecentFeedTileDisplay,
    thumbnail: Option<Arc<Image>>,
}

impl DiscoverRecentTile {
    #[must_use]
    pub(crate) fn new(display: RecentFeedTileDisplay, thumbnail: Option<Arc<Image>>) -> Self {
        Self { display, thumbnail }
    }
}

pub(crate) struct DiscoverRecentParams {
    pub(crate) tiles: Vec<DiscoverRecentTile>,
    pub(crate) display: RecentFeedsDisplay,
    pub(crate) status: String,
    pub(crate) has_more: bool,
    pub(crate) loading: bool,
    pub(crate) is_empty: bool,
}

pub(crate) fn render_discover_recent(
    params: DiscoverRecentParams,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let tiles = params
        .tiles
        .into_iter()
        .map(|tile| {
            let target = tile.display.open_target();
            RecentFeedTile::new(tile.display)
                .thumbnail(tile.thumbnail)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.open_recent_feed(target.guid.clone(), target.title.clone(), cx);
                }))
                .into_any_element()
        })
        .collect::<Vec<_>>();

    div()
        .flex()
        .flex_col()
        .gap(spacing::MD)
        .child(
            div()
                .text_size(typography::SIZE_HEADLINE)
                .font_weight(FontWeight::SEMIBOLD)
                .child(params.display.heading),
        )
        .when(!params.status.is_empty(), |el| {
            el.child(
                div()
                    .text_xs()
                    .text_color(color::text_muted())
                    .child(SharedString::from(params.status)),
            )
        })
        .when(params.is_empty && !params.loading, |el| {
            el.child(
                div()
                    .text_center()
                    .p(spacing::XXL)
                    .text_color(color::text_muted())
                    .child(params.display.empty_label),
            )
        })
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(spacing::MD)
                .children(tiles),
        )
        .when(params.has_more && !params.loading, |el| {
            el.child(
                div().pt(spacing::SM).child(
                    UiButton::styled(params.display.load_more_button_id, ControlStyle::Ghost)
                        .label(params.display.load_more_label)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.load_recent_feeds(true, cx);
                        })),
                ),
            )
        })
        .into_any_element()
}
