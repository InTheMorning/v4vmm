//! Library thumbnail primitives.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{div, prelude::*, AnyElement, App, Image};

use crate::ui::primitives::Image as ImagePrimitive;
use crate::ui::tokens::{FontSize, Radius};
use crate::ui::{layouts as layout, style::color};
use crate::view_models::library::LibraryViewModel;

pub(crate) fn render_album_thumb(image: Option<Arc<Image>>, size: f32, cx: &App) -> AnyElement {
    let display = LibraryViewModel::album_thumb_display();
    let size = layout::scaled_f32(size, cx);
    if let Some(img_data) = image {
        ImagePrimitive::new(img_data)
            .dimension(size)
            .radius(Radius::SM)
            .into_any_element()
    } else {
        div()
            .w(size)
            .h(size)
            .rounded(Radius::SM.scaled(cx))
            .bg(color::border_subtle())
            .flex()
            .items_center()
            .justify_center()
            .text_size(FontSize::Headline.scaled(cx))
            .flex_shrink_0()
            .child(display.fallback_icon)
            .into_any_element()
    }
}
