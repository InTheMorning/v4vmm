//! Semantic icon catalog for reusable UI iconography.
//!
//! Screens choose [`IconName`] and size intent; this module owns concrete SVG,
//! glyph, and brand-color details.

#![warn(clippy::pedantic)]

use std::sync::{Arc, OnceLock};

use gpui::{
    div, img, prelude::*, AnyElement, App, ClickEvent, Image, ImageFormat, IntoElement, ObjectFit,
    ParentElement, Pixels, RenderOnce, Rgba, SharedString, StatefulInteractiveElement, Styled,
    Window,
};

use crate::ui::layouts as layout;
use crate::ui::primitives::Tooltip;
use crate::ui::style::radius;
use crate::ui::tokens::{FontSize, ScaleFactor};

/// Semantic icon names understood by the design system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconName {
    Add,
    Back,
    Rss,
    Nostr,
    Play,
    Pause,
    Stop,
    Previous,
    Next,
    More,
    DragHandle,
}

impl IconName {
    #[must_use]
    fn image(self) -> Option<Arc<Image>> {
        match self {
            Self::Rss => Some(rss_icon_image()),
            Self::Nostr => Some(nostr_icon_image()),
            Self::Add
            | Self::Back
            | Self::Play
            | Self::Pause
            | Self::Stop
            | Self::Previous
            | Self::Next
            | Self::More
            | Self::DragHandle => None,
        }
    }

    #[must_use]
    fn glyph(self) -> Option<&'static str> {
        match self {
            Self::Add => Some("\u{002B}"),
            Self::Back => Some("\u{2190}"),
            Self::Play => Some("\u{25B6}"),
            Self::Pause => Some("\u{23F8}"),
            Self::Stop => Some("\u{23F9}"),
            Self::Previous => Some("\u{23EE}"),
            Self::Next => Some("\u{23ED}"),
            Self::More => Some("\u{22EF}"),
            Self::DragHandle => Some("\u{2630}"),
            Self::Rss | Self::Nostr => None,
        }
    }

    /// Brand/protocol fill colors used by catalog-owned SVG icons.
    #[must_use]
    pub fn brand_fill(self) -> Option<Rgba> {
        match self {
            Self::Rss => Some(gpui::rgb(0xf3_9a2e)),
            Self::Nostr => Some(gpui::rgb(0x8e_30eb)),
            Self::Add
            | Self::Back
            | Self::Play
            | Self::Pause
            | Self::Stop
            | Self::Previous
            | Self::Next
            | Self::More
            | Self::DragHandle => None,
        }
    }
}

/// Semantic icon size roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconSize {
    /// 14 px icon inside an 18 px action affordance.
    Action,
    /// Transport glyph aligned with body text.
    Transport,
}

impl IconSize {
    #[must_use]
    pub const fn px(self) -> Pixels {
        match self {
            Self::Action => layout::ACTION_ICON_INNER_SIZE,
            Self::Transport => FontSize::Body.px(),
        }
    }

    #[must_use]
    pub fn scaled(self, cx: &App) -> Pixels {
        gpui::px(f32::from(self.px()) * ScaleFactor::current(cx).multiplier())
    }
}

/// Renderable semantic icon.
#[derive(IntoElement)]
#[must_use]
pub struct Icon {
    name: IconName,
    size: IconSize,
    color: Option<Rgba>,
}

impl Icon {
    pub const fn new(name: IconName) -> Self {
        Self {
            name,
            size: IconSize::Action,
            color: None,
        }
    }

    pub const fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    pub const fn color(mut self, color: Rgba) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let size = self.size.scaled(cx);
        if let Some(image) = self.name.image() {
            return img(image)
                .w(size)
                .h(size)
                .object_fit(ObjectFit::Contain)
                .into_any_element();
        }

        let mut icon = div()
            .w(size)
            .h(size)
            .flex()
            .items_center()
            .justify_center()
            .text_size(size)
            .child(SharedString::from(self.name.glyph().unwrap_or_default()));
        if let Some(color) = self.color {
            icon = icon.text_color(color);
        }
        icon.into_any_element()
    }
}

/// Render a clickable RSS icon link using catalog-owned icon assets.
#[must_use]
pub fn render_rss_icon_link(id_seed: &str, url: Option<String>) -> AnyElement {
    let id = SharedString::from(match url.as_deref() {
        Some(u) => format!("rss-link:{id_seed}:{u}"),
        None => format!("rss-link:{id_seed}:missing"),
    });
    let tooltip = url.as_ref().map_or_else(
        || "No RSS feed URL".to_string(),
        |u| format!("Open RSS feed: {u}"),
    );
    let has_url = url.is_some();
    let click_url = url;
    let tooltip = Tooltip::new(tooltip);

    div()
        .id(id)
        .min_w(layout::MIN_HIT_TARGET)
        .min_h(layout::MIN_HIT_TARGET)
        .flex()
        .items_center()
        .justify_center()
        .rounded(radius::SM)
        .overflow_hidden()
        .tooltip(move |window: &mut Window, cx| tooltip.build(window, cx))
        .when(has_url, gpui::Styled::cursor_pointer)
        .when(!has_url, |el| el.opacity(0.45))
        .child(Icon::new(IconName::Rss).size(IconSize::Action))
        .on_click(move |_: &ClickEvent, _window, _cx| {
            if let Some(u) = &click_url {
                let _ = open::that(u);
            }
        })
        .into_any_element()
}

fn rss_icon_image() -> Arc<Image> {
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

fn nostr_icon_image() -> Arc<Image> {
    static NOSTR_ICON: OnceLock<Arc<Image>> = OnceLock::new();

    Arc::clone(NOSTR_ICON.get_or_init(|| {
        Arc::new(Image::from_bytes(
            ImageFormat::Svg,
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 18 18">
<rect width="18" height="18" rx="4" fill="#8e30eb"/>
<path d="M10.8 2.5l-5 7.5h3.4l-1 5.5 5-7.5h-3.4z" fill="#ffffff"/>
</svg>"##
                .to_vec(),
        ))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_protocol_icons_expose_catalog_colors() {
        assert_eq!(IconName::Rss.brand_fill(), Some(gpui::rgb(0xf3_9a2e)));
        assert_eq!(IconName::Nostr.brand_fill(), Some(gpui::rgb(0x8e_30eb)));
    }

    #[test]
    fn transport_icons_are_glyphs_not_svg_assets() {
        assert_eq!(IconName::Add.glyph(), Some("\u{002B}"));
        assert_eq!(IconName::Back.glyph(), Some("\u{2190}"));
        assert_eq!(IconName::Play.glyph(), Some("\u{25B6}"));
        assert_eq!(IconName::Pause.glyph(), Some("\u{23F8}"));
        assert!(IconName::Play.image().is_none());
    }
}
