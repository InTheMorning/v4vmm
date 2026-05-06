//! Discover recent-feeds surface.
//!
//! Renders the recent-feed tile grid shown at the Discover root when no search
//! term is active. `SearchApp` keeps loading and inspector-selection state.

#![warn(clippy::pedantic)]

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, Context, ElementId, FontWeight, Image,
    InteractiveElement, IntoElement, RenderOnce, SharedString, Styled, Window,
};

use crate::search::SearchApp;
use crate::ui::composites::{EntityKind, SkeletonFeedTile};
use crate::ui::control_styles::ControlStyle;
use crate::ui::layouts as layout;
use crate::ui::primitives::{Button as UiButton, Image as ImagePrimitive, Label};
use crate::ui::style::{color, spacing, typography};
use crate::ui::tokens::{color as token_color, FontSize, Radius, SemanticColor, Spacing};
use crate::view_models::search::{
    pending_skeleton_count, RecentFeedTileDisplay, RecentFeedsDisplay,
};

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
#[must_use]
struct RecentFeedTile {
    id: ElementId,
    display: RecentFeedTileDisplay,
    thumbnail: Option<Arc<Image>>,
    on_click: Option<ClickHandler>,
}

impl RecentFeedTile {
    fn new(mut display: RecentFeedTileDisplay) -> Self {
        let id = SharedString::from(display.take_recent_tile_id());
        Self {
            id: id.into(),
            display,
            thumbnail: None,
            on_click: None,
        }
    }

    fn thumbnail(mut self, thumbnail: Option<Arc<Image>>) -> Self {
        self.thumbnail = thumbnail;
        self
    }

    fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for RecentFeedTile {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let gap = Spacing::SM.scaled(cx);
        let pad = Spacing::SM.scaled(cx);
        let radius_lg = Radius::LG.scaled(cx);
        let radius_md = Radius::MD.scaled(cx);
        let fallback_size = FontSize::Title2.scaled(cx);
        let hover_bg = token_color(cx, SemanticColor::SecondarySystemBackground);
        let fallback_bg = token_color(cx, SemanticColor::SystemFill);
        let has_thumbnail = self.thumbnail.is_some();

        let mut tile = div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(gap)
            .w(layout::SEARCH_TILE_WIDTH)
            .p(pad)
            .rounded(radius_lg);

        if let Some(handler) = self.on_click {
            tile = tile
                .cursor_pointer()
                .hover(move |el| el.bg(hover_bg))
                .on_click(move |event, window, cx| handler(event, window, cx));
        }

        tile.child(
            div()
                .w(layout::THUMBNAIL_XL)
                .h(layout::THUMBNAIL_XL)
                .rounded(radius_md)
                .overflow_hidden()
                .flex_shrink_0()
                .when_some(self.thumbnail, |el, image| {
                    el.child(
                        ImagePrimitive::new(image)
                            .dimension(layout::THUMBNAIL_XL)
                            .radius(Radius::MD),
                    )
                })
                .when(!has_thumbnail, |el| {
                    el.bg(fallback_bg)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(fallback_size)
                        .child(EntityKind::Feed.emoji())
                }),
        )
        .child(
            div().w(layout::THUMBNAIL_XL).min_w_0().child(
                Label::new(self.display.title)
                    .size(FontSize::Caption)
                    .weight(FontWeight::MEDIUM)
                    .truncated(),
            ),
        )
        .when_some(self.display.subtitle, |el, subtitle| {
            el.child(
                div().w(layout::THUMBNAIL_XL).min_w_0().child(
                    Label::new(subtitle)
                        .size(FontSize::Micro)
                        .color(SemanticColor::TertiaryLabel)
                        .truncated(),
                ),
            )
        })
    }
}

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
                .children(tiles)
                .when(params.loading && params.is_empty, |el| {
                    let count = pending_skeleton_count(true, false);
                    el.children((0..count).map(|i| {
                        SkeletonFeedTile::new(("discover-recent-skeleton", i))
                            .into_any_element()
                    }))
                })
                .when(params.loading && !params.is_empty, |el| {
                    let count = pending_skeleton_count(true, true);
                    el.children((0..count).map(|i| {
                        SkeletonFeedTile::new(("discover-recent-skeleton-tail", i))
                            .into_any_element()
                    }))
                }),
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
