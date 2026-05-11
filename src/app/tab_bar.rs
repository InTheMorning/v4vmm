//! Top-level app tab bar rendering.

use std::sync::Arc;

use gpui::{
    div, img, prelude::*, Context, Image, ImageFormat, IntoElement, ObjectFit, SharedString,
    Styled, Window,
};
use gpui_component::input::Input;
use gpui_component::Size;

use crate::library::LibraryApp;
use crate::ui::composites::{Segment, SegmentDisplay, SegmentedControl};
use crate::ui::control_styles::ControlStyle;
use crate::ui::layouts as layout;
use crate::ui::primitives::{Button as UiButton, Tooltip};
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor, Size as TokenSize, Spacing};
use crate::view_models::app_toolbar::{
    AppToolbarTabDisplay, AppToolbarTabKey, AppToolbarVm, GlobalSearchDisplay, GlobalSearchScope,
};

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
    let display = AppToolbarVm::new().display();
    let bg_surface = color(cx, SemanticColor::SecondarySystemBackground);
    let frame_bg = color(cx, SemanticColor::TertiarySystemBackground);
    let border_subtle = color(cx, SemanticColor::Separator);
    let spacing_xs = Spacing::XS.scaled(cx);
    let spacing_sm = Spacing::SM.scaled(cx);
    let spacing_md = Spacing::MD.scaled(cx);
    let tab_bar_height = TokenSize::RowLg.px();
    let frame_radius = Radius::LG.scaled(cx);
    let mark_tooltip = Tooltip::new(display.mark_a11y_label);
    let now_playing_tooltip = Tooltip::new(display.now_playing.a11y_label);

    let mut toolbar = div()
        .id(display.id)
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
                .id(display.leading_id)
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
                        .opacity(0.82)
                        .id(display.mark_id)
                        .tooltip(move |window, cx| mark_tooltip.build(window, cx))
                        .child(
                            img(app_logo())
                                .w(layout::APP_ICON_SIZE)
                                .h(layout::APP_ICON_SIZE)
                                .object_fit(ObjectFit::Cover),
                        ),
                ),
        );

    for tab in display.tabs {
        toolbar = toolbar.child(render_app_tab(tab, app.tab, app, window, cx));
    }

    toolbar
        .child(render_global_search(
            display.center_id,
            display.global_search,
            app,
            cx,
        ))
        .child(
            div()
                .id(display.now_playing.id)
                .w(TokenSize::ColumnTall.scaled(cx))
                .min_w(TokenSize::ColumnShort.scaled(cx))
                .max_w(TokenSize::ColumnTall.scaled(cx))
                .h(TokenSize::RowMd.scaled(cx))
                .border_1()
                .border_color(border_subtle)
                .rounded(frame_radius)
                .bg(frame_bg)
                .px(spacing_sm)
                .flex()
                .items_center()
                .overflow_hidden()
                .tooltip(move |window, cx| now_playing_tooltip.build(window, cx))
                .child(playback_bar),
        )
        .into_any_element()
}

fn render_global_search(
    center_id: &'static str,
    display: GlobalSearchDisplay,
    app: &TopApp,
    cx: &mut Context<TopApp>,
) -> gpui::AnyElement {
    let segments = display
        .scopes
        .into_iter()
        .map(|scope| {
            Segment::new(SegmentDisplay {
                id: scope.id.into(),
                key: scope.scope,
                label: scope.label.into(),
                a11y_label: scope.a11y_label.into(),
            })
        })
        .collect::<Vec<_>>();
    let entity = cx.entity();

    div()
        .id(center_id)
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .justify_center()
        .gap(Spacing::XS.scaled(cx))
        .child(
            div()
                .id(display.input_id)
                .w(TokenSize::ColumnTall.scaled(cx))
                .max_w(TokenSize::ColumnTall.scaled(cx))
                .min_w(TokenSize::ColumnShort.scaled(cx))
                .child(
                    Input::new(&app.global_search_input)
                        .cleanable(true)
                        .scaled(Size::Small, cx),
                ),
        )
        .child(
            SegmentedControl::new(app.global_search_scope)
                .filter_style()
                .segments(segments)
                .on_select(move |scope: &GlobalSearchScope, _window, cx| {
                    let scope = *scope;
                    entity.update(cx, |this, cx| {
                        this.set_global_search_scope(scope, cx);
                    });
                }),
        )
        .child(
            UiButton::styled(display.search_button_id, ControlStyle::Primary)
                .label("Search")
                .a11y_label(display.search_button_a11y_label)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.submit_global_search(cx);
                })),
        )
        .into_any_element()
}

fn render_app_tab(
    display: AppToolbarTabDisplay,
    active: AppTab,
    app: &TopApp,
    window: &Window,
    cx: &mut Context<TopApp>,
) -> gpui::AnyElement {
    let tab = app_tab_for_key(display.key);
    let focus_handle = focus_handle_for_key(display.key, app);
    let is_active = tab == active;
    let is_focused = focus_handle.is_focused(window);
    let accent_color = color(cx, SemanticColor::Accent);
    let text_on_accent_color = color(cx, SemanticColor::OnAccent);
    let text_secondary_color = color(cx, SemanticColor::SecondaryLabel);
    let bg_surface_hi_color = color(cx, SemanticColor::TertiarySystemBackground);
    let focus_ring_color = color(cx, SemanticColor::Focus);
    let spacing_md = Spacing::MD.scaled(cx);
    let hit_target_min = layout::MIN_HIT_TARGET;
    let radius_lg = Radius::LG.scaled(cx);
    let tooltip = Tooltip::new(display.a11y_label);

    div()
        .id(display.id)
        .track_focus(focus_handle)
        .tooltip(move |window, cx| tooltip.build(window, cx))
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
        .child(
            div()
                .text_size(FontSize::Body.scaled(cx))
                .child(SharedString::from(display.label)),
        )
        .into_any_element()
}

const fn app_tab_for_key(key: AppToolbarTabKey) -> AppTab {
    match key {
        AppToolbarTabKey::Library => AppTab::Library,
        AppToolbarTabKey::Discover => AppTab::Discover,
        AppToolbarTabKey::Settings => AppTab::Settings,
    }
}

fn focus_handle_for_key(key: AppToolbarTabKey, app: &TopApp) -> &gpui::FocusHandle {
    match key {
        AppToolbarTabKey::Library => &app.library_tab_focus,
        AppToolbarTabKey::Discover => &app.discover_tab_focus,
        AppToolbarTabKey::Settings => &app.settings_tab_focus,
    }
}
