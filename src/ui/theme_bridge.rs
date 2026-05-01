//! Bridge between v4vmm's design tokens and `gpui-component`'s theme.
//!
//! `gpui-component` ships its own light/dark palettes. Without intervention
//! every component (`Button`, `Popover`, `Input`, `Sidebar`, …) renders with
//! those defaults — the root cause of every contrast bug we have shipped on
//! our dark canvas.
//!
//! [`install_theme`] is called once during application startup right after
//! [`gpui_component::init`]. It:
//!
//! 1. Calls `Theme::change(ThemeMode, …)` so the global is initialised.
//! 2. Overwrites every relevant field of `ThemeColor` with the value our
//!    [`SemanticColor`] palette would resolve to.
//!
//! After this call every gpui-component widget renders with our colors
//! automatically — no per-component overrides required.

#![warn(clippy::pedantic)]

use gpui::{App, Hsla, Rgba, Window};
use gpui_component::{Theme, ThemeMode};

use crate::config::UiScale;
use crate::theme_profile::ThemeProfile;
use crate::ui::theme_profiles::{appearance_for_profile, effective_profile, resolve_profile_color};
use crate::ui::tokens::{Appearance, ScaleFactor, SemanticColor};

impl From<UiScale> for ScaleFactor {
    fn from(s: UiScale) -> Self {
        match s {
            UiScale::XSmall => Self::XSmall,
            UiScale::Small => Self::Small,
            UiScale::Medium => Self::Medium,
            UiScale::Large => Self::Large,
            UiScale::XLarge => Self::XLarge,
        }
    }
}

/// Install v4vmm's palette and UI scale into the global `gpui-component`
/// theme + the application context.
///
/// Idempotent: safe to call from every window-construction path, and to
/// re-call when the user changes appearance or scale at runtime.
pub fn install_theme(profile: ThemeProfile, scale: ScaleFactor, cx: &mut App) {
    install_theme_for_appearance(profile, scale, cx.window_appearance().into(), None, cx);
}

/// Install v4vmm's palette using a concrete window's platform appearance.
///
/// Use this when the caller has a `Window`; it is the most accurate source on
/// Linux and mirrors `gpui-component`'s own system-appearance strategy.
pub fn install_theme_for_window(
    profile: ThemeProfile,
    scale: ScaleFactor,
    window: &mut Window,
    cx: &mut App,
) {
    install_theme_for_appearance(profile, scale, window.appearance().into(), Some(window), cx);
}

#[expect(
    clippy::too_many_lines,
    reason = "flat field-by-field assignment list is clearer than splitting"
)]
fn install_theme_for_appearance(
    profile: ThemeProfile,
    scale: ScaleFactor,
    system_appearance: Appearance,
    window: Option<&mut Window>,
    cx: &mut App,
) {
    let appearance = appearance_for_profile(profile, system_appearance);
    let concrete_profile = effective_profile(profile, system_appearance);
    crate::ui::style::install_profile(concrete_profile);

    // Install the bundled Environment so every component can read appearance
    // and scale through the single SwiftUI-style accessor. Mirrors the scale
    // into the legacy `ScaleFactor` global for any code still reading it
    // directly during the migration.
    crate::ui::tokens::Environment {
        profile,
        appearance,
        scale,
    }
    .install(cx);

    let mode = match appearance {
        Appearance::Dark => ThemeMode::Dark,
        Appearance::Light => ThemeMode::Light,
    };

    // Initialise / reset gpui-component's defaults for this mode.
    Theme::change(mode, window, cx);

    let theme = Theme::global_mut(cx);

    // Backgrounds.
    let bg = hsla(resolve(concrete_profile, SemanticColor::SystemBackground));
    let bg2 = hsla(resolve(
        concrete_profile,
        SemanticColor::SecondarySystemBackground,
    ));
    let bg3 = hsla(resolve(
        concrete_profile,
        SemanticColor::TertiarySystemBackground,
    ));

    // Labels.
    let label = hsla(resolve(concrete_profile, SemanticColor::Label));
    let label2 = hsla(resolve(concrete_profile, SemanticColor::SecondaryLabel));

    // Fills / accents / borders.
    let fill = hsla(resolve(concrete_profile, SemanticColor::SystemFill));
    let separator = hsla(resolve(concrete_profile, SemanticColor::Separator));
    let opaque_sep = hsla(resolve(concrete_profile, SemanticColor::OpaqueSeparator));
    let accent = hsla(resolve(concrete_profile, SemanticColor::Accent));
    let accent_h = hsla(resolve(concrete_profile, SemanticColor::AccentHover));
    let accent_p = hsla(resolve(concrete_profile, SemanticColor::AccentPressed));
    let on_accent = hsla(resolve(concrete_profile, SemanticColor::OnAccent));
    let focus = hsla(resolve(concrete_profile, SemanticColor::Focus));
    let selected = hsla(resolve(concrete_profile, SemanticColor::SelectedContent));

    // Status.
    let success = hsla(resolve(concrete_profile, SemanticColor::Success));
    let danger = hsla(resolve(concrete_profile, SemanticColor::Danger));
    let warning = hsla(resolve(concrete_profile, SemanticColor::Warning));
    let info = hsla(resolve(concrete_profile, SemanticColor::Info));

    // Window / canvas.
    theme.background = bg;
    theme.foreground = label;

    // Popover surfaces — the bug that started this overhaul.
    theme.popover = bg2;
    theme.popover_foreground = label;

    // Primary buttons (filled).
    theme.primary = accent;
    theme.primary_hover = accent_h;
    theme.primary_active = accent_p;
    theme.primary_foreground = on_accent;

    // Secondary buttons (tinted / ghost surface).
    theme.secondary = fill;
    theme.secondary_hover = bg3;
    theme.secondary_active = opaque_sep;
    theme.secondary_foreground = label;

    // Muted text & surfaces.
    theme.muted = bg2;
    theme.muted_foreground = label2;

    // Inputs.
    theme.input = bg2;
    theme.border = separator;
    theme.ring = focus;
    theme.caret = label;
    theme.selection = selected;

    // Accent + link.
    theme.accent = accent;
    theme.accent_foreground = on_accent;
    theme.link = accent;
    theme.link_hover = accent_h;
    theme.link_active = accent_p;

    // Status colors.
    theme.success = success;
    theme.success_hover = success;
    theme.success_active = success;
    theme.success_foreground = on_accent;
    theme.danger = danger;
    theme.danger_hover = danger;
    theme.danger_active = danger;
    theme.danger_foreground = on_accent;
    theme.warning = warning;
    theme.warning_hover = warning;
    theme.warning_active = warning;
    theme.warning_foreground = on_accent;
    theme.info = info;
    theme.info_hover = info;
    theme.info_active = info;
    theme.info_foreground = on_accent;

    // Sidebar — give it a slightly raised surface so it reads as a panel.
    theme.sidebar = bg2;
    theme.sidebar_foreground = label;
    theme.sidebar_border = separator;
    theme.sidebar_accent = bg3;
    theme.sidebar_accent_foreground = label;
    theme.sidebar_primary = accent;
    theme.sidebar_primary_foreground = on_accent;

    // Lists.
    theme.list = bg;
    theme.list_hover = bg2;
    theme.list_active = selected;
    theme.list_active_border = accent;
    theme.list_even = bg2;
    theme.list_head = bg3;

    // Tables.
    theme.table = bg;
    theme.table_hover = bg2;
    theme.table_active = selected;
    theme.table_active_border = accent;
    theme.table_even = bg2;
    theme.table_head = bg3;
    theme.table_head_foreground = label2;
    theme.table_row_border = separator;

    // Title bar / window chrome.
    theme.title_bar = bg;
    theme.title_bar_border = separator;
    theme.window_border = separator;

    // Tabs.
    theme.tab = bg;
    theme.tab_active = bg2;
    theme.tab_active_foreground = label;
    theme.tab_bar = bg;
    theme.tab_bar_segmented = bg2;

    // Misc surfaces that would otherwise show light defaults.
    theme.accordion = bg2;
    theme.accordion_hover = bg3;
    theme.group_box = bg2;
    theme.group_box_foreground = label;
    theme.description_list_label = bg2;
    theme.description_list_label_foreground = label2;
    theme.drop_target = selected;
    theme.skeleton = bg3;
    theme.scrollbar = bg2;
    theme.scrollbar_thumb = opaque_sep;
    theme.scrollbar_thumb_hover = label2;

    theme.overlay = scrim();
    theme.drag_border = accent;

    // Force every open window to re-render so the new scale + colors are
    // picked up immediately. Without this, the next user interaction (or a
    // background timer) would be needed before any visual change.
    cx.refresh_windows();
}

fn resolve(profile: ThemeProfile, token: SemanticColor) -> Rgba {
    resolve_profile_color(profile, token)
}

/// Convert our `Rgba` token output to gpui's `Hsla` that the theme stores.
#[inline]
fn hsla(c: Rgba) -> Hsla {
    c.into()
}

/// Translucent black used as overlay scrim under modals.
fn scrim() -> Hsla {
    Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.5,
    }
    .into()
}
