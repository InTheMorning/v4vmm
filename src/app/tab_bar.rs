//! Top-level app tab bar rendering.

use std::sync::Arc;

use gpui::{
    div, img, prelude::*, Context, Image, ImageFormat, IntoElement, ObjectFit, SharedString,
    Styled, Window,
};

use crate::library::LibraryApp;
use crate::ui::theme::layout;
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor, Size as TokenSize, Spacing};

use super::{AppTab, TopApp};

fn app_logo() -> Arc<Image> {
    Arc::new(Image::from_bytes(
        ImageFormat::Png,
        include_bytes!("../assets/music_network_logo.png").to_vec(),
    ))
}

pub(super) fn render_tab_bar(
    app: &mut TopApp,
    playback_bar: impl IntoElement,
    window: &Window,
    cx: &mut Context<TopApp>,
) -> gpui::AnyElement {
    let bg_surface = color(cx, SemanticColor::SecondarySystemBackground);
    let border_subtle = color(cx, SemanticColor::Separator);
    let accent_color = color(cx, SemanticColor::Accent);
    let spacing_xs = Spacing::XS.scaled(cx);
    let spacing_sm = Spacing::SM.scaled(cx);
    let spacing_md = Spacing::MD.scaled(cx);
    let tab_bar_height = TokenSize::RowLg.px();

    div()
        .h(tab_bar_height)
        .flex_shrink_0()
        .bg(bg_surface)
        .border_b_1()
        .border_color(border_subtle)
        .px(spacing_md)
        .flex()
        .flex_row()
        .items_center()
        .gap(spacing_xs)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(spacing_sm)
                .mr(spacing_md)
                .child(
                    div()
                        .w(layout::APP_ICON_SIZE)
                        .h(layout::APP_ICON_SIZE)
                        .rounded(spacing_xs)
                        .overflow_hidden()
                        .flex_shrink_0()
                        .child(
                            img(app_logo())
                                .w(layout::APP_ICON_SIZE)
                                .h(layout::APP_ICON_SIZE)
                                .object_fit(ObjectFit::Cover),
                        ),
                )
                .child(
                    div()
                        .text_size(FontSize::Headline.scaled(cx))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(accent_color)
                        .child("V4V Music Manager"),
                ),
        )
        .child(render_app_tab(
            "Library",
            AppTab::Library,
            app.tab,
            &app.library_tab_focus,
            window,
            cx,
        ))
        .child(render_app_tab(
            "Discover",
            AppTab::Discover,
            app.tab,
            &app.discover_tab_focus,
            window,
            cx,
        ))
        .child(render_app_tab(
            "Settings",
            AppTab::Settings,
            app.tab,
            &app.settings_tab_focus,
            window,
            cx,
        ))
        .child(div().flex_1())
        .child(playback_bar)
        .into_any_element()
}

fn render_app_tab(
    label: &'static str,
    tab: AppTab,
    active: AppTab,
    focus_handle: &gpui::FocusHandle,
    window: &Window,
    cx: &mut Context<TopApp>,
) -> gpui::AnyElement {
    let is_active = tab == active;
    let is_focused = focus_handle.is_focused(window);
    let accent_color = color(cx, SemanticColor::Accent);
    let text_on_accent_color = color(cx, SemanticColor::OnAccent);
    let text_secondary_color = color(cx, SemanticColor::SecondaryLabel);
    let bg_surface_hi_color = color(cx, SemanticColor::TertiarySystemBackground);
    let focus_ring_color = color(cx, SemanticColor::Focus);
    let spacing_md = Spacing::MD.scaled(cx);
    let hit_target_min = layout::HIT_TARGET_MIN;
    let radius_lg = Radius::LG.scaled(cx);

    div()
        .id(SharedString::from(format!("app-tab-{label}")))
        .track_focus(focus_handle)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.tab = tab;
            if tab == AppTab::Library {
                this.library.update(cx, LibraryApp::refresh);
            }
            cx.notify();
        }))
        .px(spacing_md)
        .min_h(hit_target_min)
        .flex()
        .items_center()
        .rounded(radius_lg)
        .when(is_active, |el| {
            el.bg(accent_color).text_color(text_on_accent_color)
        })
        .when(!is_active, |el| {
            el.text_color(text_secondary_color)
                .hover(move |s| s.bg(bg_surface_hi_color))
        })
        .when(is_focused, |el| {
            el.border_2().border_color(focus_ring_color)
        })
        .child(div().text_size(FontSize::Body.scaled(cx)).child(label))
        .into_any_element()
}
