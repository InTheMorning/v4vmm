//! Library thumbnail primitives.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{div, prelude::*, px, AnyElement, Image};

use crate::ui::primitives::Image as ImagePrimitive;
use crate::ui::style::{color, radius};
use crate::ui::tokens::Radius;
use crate::view_models::library::LibraryViewModel;

pub(crate) fn render_album_thumb(image: Option<Arc<Image>>, size: f32) -> AnyElement {
    let display = LibraryViewModel::album_thumb_display();
    if let Some(img_data) = image {
        ImagePrimitive::new(img_data)
            .dimension(px(size))
            .radius(Radius::SM)
            .into_any_element()
    } else {
        div()
            .w(px(size))
            .h(px(size))
            .rounded(radius::SM)
            .bg(color::border_subtle())
            .flex()
            .items_center()
            .justify_center()
            .text_size(crate::ui::layouts::ACTION_ICON_INNER_SIZE)
            .flex_shrink_0()
            .child(display.fallback_icon)
            .into_any_element()
    }
}
