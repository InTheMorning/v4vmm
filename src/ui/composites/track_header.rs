//! Track inspector header composite.
//!
//! Library and Discovery both present track inspectors with the same primary
//! header: artwork, a track badge, title, artist, and optional screen-owned
//! supplementary controls. This composite owns the shared chrome while view
//! models own display fallbacks and screens keep command wiring.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{
    div, AnyElement, App, FontWeight, Image, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window,
};

use crate::ui::tokens::{resolve_color, Appearance, FontSize, SemanticColor, Spacing};
use crate::view_models::track::TrackHeaderVm;

use super::{EntityKind, TagBadge, Thumbnail, ThumbnailSize};

#[derive(IntoElement)]
#[must_use]
pub struct TrackHeader {
    vm: TrackHeaderVm,
    image: Option<Arc<Image>>,
    supplementary_row: Option<AnyElement>,
    appearance: Option<Appearance>,
}

impl TrackHeader {
    pub fn new(vm: TrackHeaderVm) -> Self {
        Self {
            vm,
            image: None,
            supplementary_row: None,
            appearance: None,
        }
    }

    pub fn image(mut self, image: Option<Arc<Image>>) -> Self {
        self.image = image;
        self
    }

    pub fn supplementary_row(mut self, row: impl IntoElement) -> Self {
        self.supplementary_row = Some(row.into_any_element());
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for TrackHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let title_color = resolve_color(cx, SemanticColor::Label, self.appearance);
        let artist_color = resolve_color(cx, SemanticColor::SecondaryLabel, self.appearance);
        let mut badge = TagBadge::new(EntityKind::Track);
        if let Some(appearance) = self.appearance {
            badge = badge.appearance(appearance);
        }

        let mut text_block = div()
            .flex_1()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_start()
                    .mb(Spacing::SM.scaled(cx))
                    .child(badge),
            )
            .child(
                div()
                    .text_size(FontSize::Title2.scaled(cx))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(title_color)
                    .child(SharedString::from(self.vm.title)),
            )
            .child(
                div()
                    .mt(Spacing::XS.scaled(cx))
                    .text_size(FontSize::Headline.scaled(cx))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(artist_color)
                    .child(SharedString::from(self.vm.artist)),
            );

        if let Some(row) = self.supplementary_row {
            text_block = text_block.child(row);
        }

        div()
            .flex()
            .flex_row()
            .items_start()
            .gap(Spacing::LG.scaled(cx))
            .child(Thumbnail::new(EntityKind::Track, ThumbnailSize::Lg).image(self.image))
            .child(text_block)
    }
}
