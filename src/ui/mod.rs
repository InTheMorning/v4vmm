pub mod composites;
pub mod contrast;
pub mod detail_row;
pub mod playlist_popover;
pub mod primitives;
pub mod sizable_bridge;
pub mod text;
pub mod theme;
pub mod theme_bridge;
pub mod tokens;

use std::sync::{Arc, OnceLock};

use gpui::{
    div, img, prelude::*, px, AnyElement, ClickEvent, Image, ImageFormat, InteractiveElement,
    IntoElement, ObjectFit, ParentElement, SharedString, StatefulInteractiveElement, Styled,
    Window,
};
use gpui_component::tooltip::Tooltip;

use theme::radius;

pub fn rss_icon_image() -> Arc<Image> {
    static RSS_ICON: OnceLock<Arc<Image>> = OnceLock::new();
    Arc::clone(RSS_ICON.get_or_init(|| {
        Arc::new(Image::from_bytes(
            ImageFormat::Svg,
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 18 18">
<rect width="18" height="18" rx="4" fill="#f39a2e"/>
<circle cx="5" cy="13" r="1.7" fill="#ffffff"/>
<path d="M4 9.4A4.6 4.6 0 0 1 8.6 14" fill="none" stroke="#ffffff" stroke-width="2" stroke-linecap="round"/>
<path d="M4 5.2A8.8 8.8 0 0 1 12.8 14" fill="none" stroke="#ffffff" stroke-width="2" stroke-linecap="round"/>
</svg>"##
                .to_vec(),
        ))
    }))
}

pub fn render_rss_icon_link(id_seed: &str, url: Option<String>) -> AnyElement {
    let id = SharedString::from(match url.as_deref() {
        Some(u) => format!("rss-link:{id_seed}:{u}"),
        None => format!("rss-link:{id_seed}:missing"),
    });
    let tooltip = url.as_ref().map_or_else(
        || "No RSS feed URL".to_string(),
        |u| format!("Open RSS feed: {u}"),
    );
    let click_url = url.clone();
    div()
        .id(id)
        .w(px(18.0))
        .h(px(18.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(radius::SM)
        .overflow_hidden()
        .tooltip(move |window: &mut Window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .when(url.is_some(), |el| el.cursor_pointer())
        .when(url.is_none(), |el| el.opacity(0.45))
        .child(
            img(rss_icon_image())
                .w(px(14.0))
                .h(px(14.0))
                .object_fit(ObjectFit::Contain),
        )
        .on_click(move |_: &ClickEvent, _window, _cx| {
            if let Some(u) = &click_url {
                let _ = open::that(u);
            }
        })
        .into_any_element()
}
