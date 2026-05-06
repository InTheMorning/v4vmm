//! Detail-view header — thumbnail + entity badge + title + optional
//! subtitle. Used by the artist / feed / track inspectors.
//!
//! ## Display contract: `DetailHeaderDisplay`

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{
    div, App, FontWeight, Image, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window,
};

use crate::ui::layouts as layout;
use crate::ui::primitives::{HStack, MultilineText, VStack};
use crate::ui::tokens::{resolve_color, Appearance, FontSize, SemanticColor, Spacing};

use super::tag_badge::{EntityKind, TagBadge, TagBadgeDisplay};
use super::thumbnail::{Thumbnail, ThumbnailSize};

#[derive(IntoElement)]
#[must_use]
pub struct DetailHeader {
    display: DetailHeaderDisplay,
    image: Option<Arc<Image>>,
    appearance: Option<Appearance>,
}

/// Display-ready header facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailHeaderDisplay {
    pub kind: EntityKind,
    pub title: SharedString,
    pub subtitle: Option<SharedString>,
    pub data_rows: Vec<DetailHeaderDataRow>,
}

/// Display-ready metadata row shown under a detail header subtitle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailHeaderDataRow {
    pub label: SharedString,
    pub value: SharedString,
    pub max_lines: usize,
}

impl DetailHeader {
    pub fn new(display: DetailHeaderDisplay) -> Self {
        Self {
            display,
            image: None,
            appearance: None,
        }
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
        let mut badge = TagBadge::new(TagBadgeDisplay {
            kind: self.display.kind,
            label: None,
        });
        if let Some(appearance) = self.appearance {
            badge = badge.appearance(appearance);
        }

        let mut text_block = VStack::new()
            .spacing(Spacing::XS)
            .leading()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_start()
                    .mb(Spacing::XS.scaled(cx))
                    .child(badge),
            )
            .child(
                div()
                    .text_size(FontSize::Title2.scaled(cx))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(title_color)
                    .child(self.display.title),
            );

        if let Some(sub) = self.display.subtitle {
            text_block = text_block.child(
                div()
                    .text_size(FontSize::Body.scaled(cx))
                    .text_color(subtitle_color)
                    .child(sub),
            );
        }

        if !self.display.data_rows.is_empty() {
            let mut metadata = VStack::new().spacing(Spacing::XXS).leading();
            for row in self.display.data_rows {
                metadata = metadata.child(header_data_row(
                    row.label,
                    row.value,
                    row.max_lines.max(1),
                    metadata_label_color,
                    self.appearance,
                    cx,
                ));
            }
            text_block = text_block.child(metadata);
        }

        HStack::new()
            .spacing(Spacing::LG)
            .top()
            .child(Thumbnail::new(self.display.kind, ThumbnailSize::Lg).image(self.image))
            .child(div().flex_1().min_w_0().child(text_block))
    }
}

fn header_data_row(
    label: SharedString,
    value: SharedString,
    max_lines: usize,
    label_color: gpui::Rgba,
    appearance: Option<Appearance>,
    cx: &App,
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
        .gap(Spacing::SM.scaled(cx))
        .min_w_0()
        .child(
            div()
                .w(layout::COMPACT_COLUMN_WIDTH)
                .flex_shrink_0()
                .text_size(FontSize::Micro.scaled(cx))
                .font_weight(FontWeight::MEDIUM)
                .text_color(label_color)
                .child(label),
        )
        .child(div().min_w_0().flex_1().child(value_text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_uses_display_contract() {
        let header = DetailHeader::new(DetailHeaderDisplay {
            kind: EntityKind::Artist,
            title: "Artist".into(),
            subtitle: Some("Subtitle".into()),
            data_rows: vec![DetailHeaderDataRow {
                label: "Publisher".into(),
                value: "Label".into(),
                max_lines: 0,
            }],
        });

        assert_eq!(header.display.kind, EntityKind::Artist);
        assert_eq!(header.display.title, SharedString::from("Artist"));
        assert_eq!(
            header.display.subtitle,
            Some(SharedString::from("Subtitle"))
        );
        assert_eq!(header.display.data_rows.len(), 1);
    }
}
