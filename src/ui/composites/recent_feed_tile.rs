//! Recent feed tile composite.
//!
//! The display contract is [`crate::view_models::search::RecentFeedTileDisplay`].
//! The view model owns title/subtitle/image URL fallback policy; this
//! composite owns the HIG-style tile chrome: artwork, spacing, label hierarchy,
//! hover state, and click target.

#![warn(clippy::pedantic)]

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, prelude::*, App, ClickEvent, ElementId, FontWeight, Image, InteractiveElement,
    IntoElement, RenderOnce, Styled, Window,
};

use crate::ui::composites::EntityKind;
use crate::ui::layouts as layout;
use crate::ui::primitives::{Image as ImagePrimitive, Label};
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor, Spacing};
use crate::view_models::search::RecentFeedTileDisplay;

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Discovery recent-feed tile.
#[derive(IntoElement)]
#[must_use]
pub struct RecentFeedTile {
    id: ElementId,
    display: RecentFeedTileDisplay,
    thumbnail: Option<Arc<Image>>,
    on_click: Option<ClickHandler>,
}

impl RecentFeedTile {
    pub fn new(id: impl Into<ElementId>, display: RecentFeedTileDisplay) -> Self {
        Self {
            id: id.into(),
            display,
            thumbnail: None,
            on_click: None,
        }
    }

    pub fn thumbnail(mut self, thumbnail: Option<Arc<Image>>) -> Self {
        self.thumbnail = thumbnail;
        self
    }

    pub fn on_click<F>(mut self, handler: F) -> Self
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
        let hover_bg = color(cx, SemanticColor::SecondarySystemBackground);
        let fallback_bg = color(cx, SemanticColor::SystemFill);
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
