//! Detail-view header — thumbnail + entity badge + title + optional
//! subtitle. Used by the artist / feed / track inspectors.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{
    div, App, FontWeight, Image, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window,
};

use crate::ui::primitives::{HStack, VStack};
use crate::ui::tokens::{resolve_color, Appearance, FontSize, SemanticColor, Spacing};

use super::tag_badge::{EntityKind, TagBadge};
use super::thumbnail::{Thumbnail, ThumbnailSize};

#[derive(IntoElement)]
#[must_use]
pub struct DetailHeader {
    kind: EntityKind,
    title: SharedString,
    subtitle: Option<SharedString>,
    image: Option<Arc<Image>>,
    appearance: Option<Appearance>,
}

impl DetailHeader {
    pub fn new(kind: EntityKind, title: impl Into<SharedString>) -> Self {
        Self {
            kind,
            title: title.into(),
            subtitle: None,
            image: None,
            appearance: None,
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
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for DetailHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let title_color = resolve_color(cx, SemanticColor::Label, self.appearance);
        let subtitle_color = resolve_color(cx, SemanticColor::SecondaryLabel, self.appearance);
        let mut badge = TagBadge::new(self.kind);
        if let Some(appearance) = self.appearance {
            badge = badge.appearance(appearance);
        }

        let mut text_block = VStack::new()
            .spacing(Spacing::XS)
            .leading()
            .child(div().mb(Spacing::XS.scaled(cx)).child(badge))
            .child(
                div()
                    .text_size(FontSize::Title2.scaled(cx))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(title_color)
                    .child(self.title),
            );

        if let Some(sub) = self.subtitle {
            text_block = text_block.child(
                div()
                    .text_size(FontSize::Body.scaled(cx))
                    .text_color(subtitle_color)
                    .child(sub),
            );
        }

        HStack::new()
            .spacing(Spacing::LG)
            .top()
            .child(Thumbnail::new(self.kind, ThumbnailSize::Lg).image(self.image))
            .child(div().flex_1().min_w_0().child(text_block))
    }
}
