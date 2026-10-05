//! Semantic icon catalog for reusable UI iconography.
//!
//! Screens choose [`IconName`] and size intent; this module owns the Lucide
//! mapping and brand-color details. ADR 0083 Decision 10: every icon draws
//! from the Lucide set of `gpui-kit-assets`, through `ComponentIconName`
//! where the default component bundle carries it, or through the complete
//! Lucide catalog otherwise. No icon is a text character.

#![warn(clippy::pedantic)]

use std::sync::{Arc, OnceLock};

use gpui::{
    div, img, prelude::*, AnyElement, App, AssetSource, ClickEvent, Image, ImageFormat,
    IntoElement, ObjectFit, ParentElement, Pixels, RenderOnce, Rgba, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{Icon as ComponentIcon, IconName as ComponentIconName};
use gpui_kit_assets::IconName as LucideIconName;

use crate::ui::layouts as layout;
use crate::ui::primitives::Tooltip;
use crate::ui::style::radius;
use crate::ui::tokens::{FontSize, ScaleFactor};

/// Semantic icon names understood by the design system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconName {
    Add,
    Back,
    ChevronLeft,
    ChevronRight,
    ChevronDown,
    Check,
    Close,
    Info,
    Search,
    Rss,
    Nostr,
    Play,
    Pause,
    Stop,
    Previous,
    Next,
    More,
    DragHandle,
    NotAllowed,
    Warning,
}

impl IconName {
    #[must_use]
    fn image(self) -> Option<Arc<Image>> {
        match self {
            Self::Rss => Some(rss_icon_image()),
            Self::Nostr => Some(nostr_icon_image()),
            Self::Add
            | Self::Back
            | Self::ChevronLeft
            | Self::ChevronRight
            | Self::ChevronDown
            | Self::Check
            | Self::Close
            | Self::Info
            | Self::Search
            | Self::Play
            | Self::Pause
            | Self::Stop
            | Self::Previous
            | Self::Next
            | Self::More
            | Self::DragHandle
            | Self::NotAllowed
            | Self::Warning => None,
        }
    }

    /// The Lucide icon of the default `gpui-component` bundle for a name,
    /// when that bundle carries it. [`Self::extra_icon`] serves the rest.
    #[must_use]
    fn component_icon(self) -> Option<ComponentIconName> {
        match self {
            Self::Add => Some(ComponentIconName::Plus),
            Self::Back => Some(ComponentIconName::ArrowLeft),
            Self::ChevronLeft => Some(ComponentIconName::ChevronLeft),
            Self::ChevronRight => Some(ComponentIconName::ChevronRight),
            Self::ChevronDown => Some(ComponentIconName::ChevronDown),
            Self::Check => Some(ComponentIconName::Check),
            Self::Close => Some(ComponentIconName::Close),
            Self::Info => Some(ComponentIconName::Info),
            Self::Search => Some(ComponentIconName::Search),
            Self::Play => Some(ComponentIconName::Play),
            Self::Pause => Some(ComponentIconName::Pause),
            Self::More => Some(ComponentIconName::Ellipsis),
            Self::Warning => Some(ComponentIconName::TriangleAlert),
            Self::Rss
            | Self::Nostr
            | Self::Stop
            | Self::Previous
            | Self::Next
            | Self::DragHandle
            | Self::NotAllowed => None,
        }
    }

    /// The Lucide icon of the complete `gpui-kit-assets` catalog for a name
    /// the default component bundle does not carry. [`InterfaceAssets`]
    /// serves its SVG path at app start.
    #[must_use]
    fn extra_icon(self) -> Option<LucideIconName> {
        match self {
            Self::Stop => Some(LucideIconName::Square),
            Self::Previous => Some(LucideIconName::SkipBack),
            Self::Next => Some(LucideIconName::SkipForward),
            Self::DragHandle => Some(LucideIconName::GripVertical),
            Self::NotAllowed => Some(LucideIconName::Ban),
            Self::Add
            | Self::Back
            | Self::ChevronLeft
            | Self::ChevronRight
            | Self::ChevronDown
            | Self::Check
            | Self::Close
            | Self::Info
            | Self::Search
            | Self::Rss
            | Self::Nostr
            | Self::Play
            | Self::Pause
            | Self::More
            | Self::Warning => None,
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
            | Self::ChevronLeft
            | Self::ChevronRight
            | Self::ChevronDown
            | Self::Check
            | Self::Close
            | Self::Info
            | Self::Search
            | Self::Play
            | Self::Pause
            | Self::Stop
            | Self::Previous
            | Self::Next
            | Self::More
            | Self::DragHandle
            | Self::NotAllowed
            | Self::Warning => None,
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
            // Trap: this reads `FontSize::Body`'s *unscaled* base pixel
            // value only, to align the glyph with body text at 1.0×. The
            // resolved size below still scales through CHROME, not TYPE —
            // an icon glyph is geometry, per ADR 0039, even when its base
            // happens to borrow a type-role constant.
            Self::Transport => FontSize::Body.px(),
        }
    }

    /// ADR 0039: icon sizing is geometry — this resolves through CHROME,
    /// never the TYPE domain, even for [`Self::Transport`].
    #[must_use]
    pub fn scaled(self, cx: &App) -> Pixels {
        gpui::px(f32::from(self.px()) * ScaleFactor::current(cx).chrome_multiplier())
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
        if let Some(component_icon) = self.name.component_icon() {
            let mut icon = ComponentIcon::new(component_icon).size(size);
            if let Some(color) = self.color {
                icon = icon.text_color(color);
            }
            return icon.into_any_element();
        }

        if let Some(extra_icon) = self.name.extra_icon() {
            let mut icon = ComponentIcon::new(extra_icon).size(size);
            if let Some(color) = self.color {
                icon = icon.text_color(color);
            }
            return icon.into_any_element();
        }

        // Rss and Nostr are the only remaining names; they always resolve a
        // catalog image, so this is the only branch left to try.
        let image = self
            .name
            .image()
            .expect("every IconName resolves through component_icon, extra_icon, or image");
        img(image)
            .w(size)
            .h(size)
            .object_fit(ObjectFit::Contain)
            .into_any_element()
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

// ADR 0083 Decision 10: the small set of Lucide icons this app draws from
// the complete `gpui-kit-assets` catalog, outside the default component
// bundle. The macro embeds only these SVG files, not all 1,830 icons.
gpui_kit_assets::icon_assets!(
    ExtraLucideIcons,
    [Square, SkipBack, SkipForward, GripVertical, Ban]
);

/// The asset source this app registers at start.
///
/// It serves the default `gpui-component` icon bundle, and the extra Lucide
/// icons of [`IconName::extra_icon`] that bundle does not carry.
/// `src/app/bootstrap.rs` registers this source once, through
/// `gpui_platform::application().with_assets(...)`. Registering only the
/// default bundle would leave those extra icons unresolved: a wrong or
/// missing asset path renders nothing, silently.
#[derive(Clone, Copy, Debug, Default)]
pub struct InterfaceAssets;

impl AssetSource for InterfaceAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if let Some(bytes) = ExtraLucideIcons.load(path)? {
            return Ok(Some(bytes));
        }
        gpui_kit_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut paths = gpui_kit_assets::Assets.list(path)?;
        paths.extend(ExtraLucideIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_protocol_icons_expose_catalog_colors() {
        assert_eq!(IconName::Rss.brand_fill(), Some(gpui::rgb(0xf3_9a2e)));
        assert_eq!(IconName::Nostr.brand_fill(), Some(gpui::rgb(0x8e_30eb)));
    }

    const NON_CATALOG_IMAGE_ICON_NAMES: &[IconName] = &[
        IconName::Add,
        IconName::Back,
        IconName::ChevronLeft,
        IconName::ChevronRight,
        IconName::ChevronDown,
        IconName::Check,
        IconName::Close,
        IconName::Info,
        IconName::Search,
        IconName::Play,
        IconName::Pause,
        IconName::Stop,
        IconName::Previous,
        IconName::Next,
        IconName::More,
        IconName::DragHandle,
        IconName::NotAllowed,
        IconName::Warning,
    ];

    /// R83-21: every `IconName` other than `Rss` and `Nostr` resolves to a
    /// Lucide icon, through the default component bundle or the complete
    /// catalog, and never falls back to a catalog image.
    #[test]
    fn every_icon_name_other_than_rss_and_nostr_resolves_to_a_lucide_icon() {
        for name in NON_CATALOG_IMAGE_ICON_NAMES {
            let name = *name;
            assert!(
                name.component_icon().is_some() || name.extra_icon().is_some(),
                "{name:?} must resolve to a Lucide icon"
            );
            assert!(
                name.image().is_none(),
                "{name:?} must not fall back to a catalog image"
            );
        }
    }

    /// `Rss` and `Nostr` keep their catalog-owned brand SVG (task 003
    /// Required Changes item 1) and map through neither Lucide catalog.
    #[test]
    fn rss_and_nostr_keep_their_catalog_image() {
        for name in [IconName::Rss, IconName::Nostr] {
            assert!(name.component_icon().is_none());
            assert!(name.extra_icon().is_none());
            assert!(name.image().is_some());
        }
    }

    #[test]
    fn search_icon_uses_component_vector_asset() {
        assert!(matches!(
            IconName::Search.component_icon(),
            Some(ComponentIconName::Search)
        ));
        assert!(IconName::Search.extra_icon().is_none());
    }

    /// Icons outside the default component bundle resolve through the
    /// complete Lucide catalog, each to its named Lucide icon.
    #[test]
    fn extra_icons_resolve_through_the_complete_lucide_catalog() {
        assert_eq!(IconName::Stop.extra_icon(), Some(LucideIconName::Square));
        assert_eq!(
            IconName::Previous.extra_icon(),
            Some(LucideIconName::SkipBack)
        );
        assert_eq!(
            IconName::Next.extra_icon(),
            Some(LucideIconName::SkipForward)
        );
        assert_eq!(
            IconName::DragHandle.extra_icon(),
            Some(LucideIconName::GripVertical)
        );
        assert_eq!(IconName::NotAllowed.extra_icon(), Some(LucideIconName::Ban));
    }

    /// Confirms the asset source `bootstrap.rs` registers at app start
    /// actually serves every chosen icon path. A wrong path renders
    /// nothing, silently; this test loads each path and checks for bytes.
    #[test]
    fn registered_asset_source_serves_every_icon_name_path() {
        use gpui_component::IconNamed;

        let source = InterfaceAssets;
        for name in NON_CATALOG_IMAGE_ICON_NAMES {
            let name = *name;
            let path = name
                .component_icon()
                .map(IconNamed::path)
                .or_else(|| name.extra_icon().map(IconNamed::path))
                .unwrap_or_else(|| panic!("{name:?} has no Lucide path to load"));
            let bytes = source
                .load(&path)
                .unwrap_or_else(|error| {
                    panic!("{name:?} asset path {path:?} did not load: {error:#}")
                })
                .unwrap_or_else(|| {
                    panic!(
                        "{name:?} asset path {path:?} is not served by the registered asset source"
                    )
                });
            assert!(!bytes.is_empty(), "{name:?} asset path {path:?} is empty");
        }
    }
}
