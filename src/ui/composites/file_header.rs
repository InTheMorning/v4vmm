//! Embedded-file header composite.
//!
//! Library and Discovery both show the same embedded-tag comparison header:
//! artwork, a tag badge, file actions, title, and source path. This composite
//! owns that shared chrome while screens keep command callbacks and image
//! resolution at the wiring layer.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{
    div, AnyElement, App, FontWeight, Image, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window,
};

use crate::ui::primitives::MultilineText;
use crate::ui::tokens::{resolve_color, Appearance, FontSize, SemanticColor, Spacing};
use crate::view_models::metadata::FileHeaderVm;

use super::{EntityKind, TagBadge, TagBadgeDisplay, Thumbnail, ThumbnailSize};

#[derive(IntoElement)]
#[must_use]
pub struct FileHeader {
    vm: FileHeaderVm,
    image: Option<Arc<Image>>,
    actions: Vec<AnyElement>,
    appearance: Option<Appearance>,
}

impl FileHeader {
    pub fn new(vm: FileHeaderVm) -> Self {
        Self {
            vm,
            image: None,
            actions: Vec::new(),
            appearance: None,
        }
    }

    pub fn image(mut self, image: Option<Arc<Image>>) -> Self {
        self.image = image;
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for FileHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let title_color = resolve_color(cx, SemanticColor::Label, self.appearance);
        let mut badge = TagBadge::new(TagBadgeDisplay {
            kind: EntityKind::Track,
            label: Some(SharedString::from(self.vm.badge_label.clone())),
        });
        if let Some(appearance) = self.appearance {
            badge = badge.appearance(appearance);
        }

        let mut action_row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(Spacing::SM.scaled(cx))
            .mb(Spacing::SM.scaled(cx))
            .child(badge);
        for action in self.actions {
            action_row = action_row.child(action);
        }

        let mut path = MultilineText::new(SharedString::from(self.vm.path))
            .max_lines(2)
            .size(FontSize::Micro)
            .color(SemanticColor::SecondaryLabel);
        if let Some(appearance) = self.appearance {
            path = path.appearance(appearance);
        }

        div()
            .flex()
            .flex_row()
            .items_start()
            .gap(Spacing::LG.scaled(cx))
            .child(Thumbnail::new(EntityKind::Track, ThumbnailSize::Lg).image(self.image))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(action_row)
                    .child(
                        div()
                            .text_size(FontSize::Title3.scaled(cx))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(title_color)
                            .child(SharedString::from(self.vm.title)),
                    )
                    .child(path),
            )
    }
}
