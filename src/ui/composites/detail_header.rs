//! Detail-view header — thumbnail + entity badge + title + optional
//! subtitle. Used by the artist / feed / track inspectors.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{
    div, App, FontWeight, Image, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window,
};

use crate::ui::layouts as layout;
use crate::ui::primitives::{HStack, MultilineText, VStack};
use crate::ui::tokens::{resolve_color, Appearance, FontSize, SemanticColor, Spacing};

use super::tag_badge::{EntityKind, TagBadge};
use super::thumbnail::{Thumbnail, ThumbnailSize};

#[derive(IntoElement)]
#[must_use]
pub struct DetailHeader {
    kind: EntityKind,
    title: SharedString,
    subtitle: Option<SharedString>,
    data_rows: Vec<DetailHeaderDataRow>,
    image: Option<Arc<Image>>,
    appearance: Option<Appearance>,
}

struct DetailHeaderDataRow {
    label: SharedString,
    value: SharedString,
    max_lines: usize,
}

impl DetailHeader {
    pub fn new(kind: EntityKind, title: impl Into<SharedString>) -> Self {
        Self {
            kind,
            title: title.into(),
            subtitle: None,
            data_rows: Vec::new(),
            image: None,
            appearance: None,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn data_row(
        mut self,
        label: impl Into<SharedString>,
        value: impl Into<SharedString>,
        max_lines: usize,
    ) -> Self {
        self.data_rows.push(DetailHeaderDataRow {
            label: label.into(),
            value: value.into(),
            max_lines: max_lines.max(1),
        });
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
        let metadata_label_color = resolve_color(cx, SemanticColor::TertiaryLabel, self.appearance);
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

        if !self.data_rows.is_empty() {
            let mut metadata = VStack::new().spacing(Spacing::XXS).leading();
            for row in self.data_rows {
                metadata = metadata.child(header_data_row(
                    row.label,
                    row.value,
                    row.max_lines,
                    metadata_label_color,
                    self.appearance,
                ));
            }
            text_block = text_block.child(metadata);
        }

        HStack::new()
            .spacing(Spacing::LG)
            .top()
            .child(Thumbnail::new(self.kind, ThumbnailSize::Lg).image(self.image))
            .child(div().flex_1().min_w_0().child(text_block))
    }
}

fn header_data_row(
    label: SharedString,
    value: SharedString,
    max_lines: usize,
    label_color: gpui::Rgba,
    appearance: Option<Appearance>,
) -> impl IntoElement {
    let mut value_text = MultilineText::new(value)
        .max_lines(max_lines)
        .size(FontSize::Micro)
        .color(SemanticColor::SecondaryLabel);
    if let Some(appearance) = appearance {
        value_text = value_text.appearance(appearance);
    }

    div()
        .flex()
        .flex_row()
        .gap(Spacing::SM.px())
        .min_w_0()
        .child(
            div()
                .w(layout::COMPACT_COLUMN_WIDTH)
                .flex_shrink_0()
                .text_size(FontSize::Micro.px())
                .font_weight(FontWeight::MEDIUM)
                .text_color(label_color)
                .child(label),
        )
        .child(div().min_w_0().flex_1().child(value_text))
}
