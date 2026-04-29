//! Detail-view header — thumbnail + entity badge + title + optional
//! subtitle. Used by the artist / feed / track inspectors.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{
    div, prelude::*, App, FontWeight, Image, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window,
};

use crate::ui::tokens::{Appearance, FontSize, SemanticColor, Spacing};

use super::tag_badge::{EntityKind, TagBadge};
use super::thumbnail::{Thumbnail, ThumbnailSize};

#[derive(IntoElement)]
#[must_use]
pub struct DetailHeader {
    kind: EntityKind,
    title: SharedString,
    subtitle: Option<SharedString>,
    image: Option<Arc<Image>>,
    appearance: Appearance,
}

impl DetailHeader {
    pub fn new(kind: EntityKind, title: impl Into<SharedString>) -> Self {
        Self {
            kind,
            title: title.into(),
            subtitle: None,
            image: None,
            appearance: Appearance::Dark,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn image(mut self, image: Option<Arc<Image>>) -> Self {
        self.image = image;
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = appearance;
        self
    }
}

impl RenderOnce for DetailHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let appearance = self.appearance;
        let title_color = SemanticColor::Label.resolve(appearance);
        let subtitle_color = SemanticColor::SecondaryLabel.resolve(appearance);

        div()
            .flex()
            .flex_row()
            .items_start()
            .gap(Spacing::LG.scaled(cx))
            .child(Thumbnail::new(self.kind, ThumbnailSize::Lg).image(self.image))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .mb(Spacing::SM.scaled(cx))
                            .child(TagBadge::new(self.kind).appearance(appearance)),
                    )
                    .child(
                        div()
                            .text_size(FontSize::Title2.scaled(cx))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(title_color)
                            .child(self.title),
                    )
                    .when_some(self.subtitle, |el, sub| {
                        el.child(
                            div()
                                .mt(Spacing::XS.scaled(cx))
                                .text_size(FontSize::Body.scaled(cx))
                                .text_color(subtitle_color)
                                .child(sub),
                        )
                    }),
            )
    }
}
