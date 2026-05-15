//! Top-level app tab bar rendering.

use std::sync::Arc;

use gpui::{
    div, img, prelude::*, Context, Image, ImageFormat, IntoElement, ObjectFit, SharedString,
    Styled, Window,
};
use gpui_component::input::Input;
use gpui_component::{IconName as InputIconName, Size};

use crate::library::LibraryApp;
use crate::ui::composites::{Segment, SegmentDisplay, SegmentedControl};
use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::layouts as layout;
use crate::ui::primitives::{
    Button as UiButton, ContextMenu, ContextMenuItem, ContextMenuItemDisplay, ContextMenuScope,
    Tooltip,
};
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
    let toolbar_width = window.bounds().size.width;
    let now_playing_width = if toolbar_width >= layout::APP_TOOLBAR_NOW_PLAYING_COMPACT_BREAKPOINT {
        TokenSize::ColumnTall.scaled(cx)
    } else if toolbar_width >= layout::APP_TOOLBAR_GLOBAL_SEARCH_COMPACT_BREAKPOINT {
        TokenSize::ColumnRegular.scaled(cx)
    } else {
        TokenSize::MenuRegular.scaled(cx)
    };
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
            &display.global_search,
            app,
            toolbar_width,
            cx,
        ))
        .child(
            div()
                .id(display.now_playing.id)
                .w(now_playing_width)
                .min_w(now_playing_width)
                .max_w(now_playing_width)
                .h(TokenSize::RowMd.scaled(cx))
                .flex_shrink_0()
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
    display: &GlobalSearchDisplay,
    app: &TopApp,
    toolbar_width: gpui::Pixels,
    cx: &mut Context<TopApp>,
) -> gpui::AnyElement {
    let show_scope_controls = toolbar_width >= layout::APP_TOOLBAR_SCOPE_BREAKPOINT;
    let use_compact_search = toolbar_width < layout::APP_TOOLBAR_GLOBAL_SEARCH_COMPACT_BREAKPOINT;
    let segments = display
        .scopes
        .iter()
        .map(|scope| {
            Segment::new(SegmentDisplay {
                id: scope.id.into(),
                key: scope.scope,
                label: scope.label.into(),
                a11y_label: scope.a11y_label.into(),
            })
        })
        .collect::<Vec<_>>();
    let active_scope_label = display
        .scopes
        .iter()
        .find(|scope| scope.scope == app.global_search_scope)
        .map_or("All", |scope| scope.label);
    let entity = cx.entity();
    let segmented_entity = entity.clone();
    let menu_entity = entity.clone();

    let search_input = div()
        .id(display.input_id)
        .flex_1()
        .min_w_0()
        .max_w(TokenSize::ColumnTall.scaled(cx))
        .child(
            Input::new(&app.global_search_input)
                .prefix(InputIconName::Search)
                .cleanable(true)
                .scaled(Size::Small, cx),
        );

    let mut toolbar_search = div()
        .id(center_id)
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .justify_center()
        .gap(Spacing::XS.scaled(cx))
        .overflow_hidden()
        .child(search_input);

    if use_compact_search {
        toolbar_search = toolbar_search
            .child(render_global_search_scope_menu(
                display.scope_menu_id,
                display.scope_menu_a11y_label,
                "",
                display.search_button_label,
                display.search_button_a11y_label,
                false,
                display.scopes.clone(),
                app.global_search_scope,
                &menu_entity,
            ))
            .child(render_global_search_submit_button(display, true, cx));
    } else {
        toolbar_search = toolbar_search
            .when(show_scope_controls, |el| {
                el.child(
                    SegmentedControl::new(app.global_search_scope)
                        .filter_style()
                        .segments(segments)
                        .on_select(move |scope: &GlobalSearchScope, _window, cx| {
                            let scope = *scope;
                            segmented_entity.update(cx, |this, cx| {
                                this.set_global_search_scope(scope, cx);
                            });
                        }),
                )
            })
            .when(!show_scope_controls, |el| {
                el.child(render_global_search_scope_menu(
                    display.scope_menu_id,
                    display.scope_menu_a11y_label,
                    active_scope_label,
                    display.search_button_label,
                    display.search_button_a11y_label,
                    false,
                    display.scopes.clone(),
                    app.global_search_scope,
                    &menu_entity,
                ))
            })
            .child(render_global_search_submit_button(display, false, cx));
    }

    toolbar_search.into_any_element()
}

fn render_global_search_submit_button(
    display: &GlobalSearchDisplay,
    compact: bool,
    cx: &mut Context<TopApp>,
) -> UiButton {
    let mut button = if compact {
        UiButton::styled(display.search_button_id, ControlStyle::ToolbarIcon)
            .leading_icon(IconName::Search)
            .tooltip(display.search_button_a11y_label)
    } else {
        UiButton::styled(display.search_button_id, ControlStyle::Primary)
            .label(display.search_button_label)
    }
    .a11y_label(display.search_button_a11y_label);

    button = button.on_click(cx.listener(|this, _, _, cx| {
        this.submit_global_search(cx);
    }));
    button
}

#[expect(
    clippy::too_many_arguments,
    reason = "toolbar overflow keeps VM-owned display strings at the call site"
)]
fn render_global_search_scope_menu(
    id: &'static str,
    a11y_label: &'static str,
    trigger_label: &'static str,
    search_label: &'static str,
    search_a11y_label: &'static str,
    include_search_action: bool,
    scopes: [crate::view_models::app_toolbar::GlobalSearchScopeDisplay; 3],
    active_scope: GlobalSearchScope,
    entity: &gpui::Entity<TopApp>,
) -> gpui::AnyElement {
    let mut items = Vec::new();

    if include_search_action {
        let entity = entity.clone();
        items.push(
            ContextMenuItem::new(ContextMenuItemDisplay {
                id: SharedString::from(format!("{id}:submit")),
                label: SharedString::from(search_label),
                a11y_label: SharedString::from(search_a11y_label),
                destructive: false,
                disabled: false,
            })
            .on_select(move |_window, cx| {
                entity.update(cx, |this, cx| {
                    this.submit_global_search(cx);
                });
            }),
        );
    }

    items.extend(scopes.into_iter().map(|scope| {
        let mut item = ContextMenuItem::new(ContextMenuItemDisplay {
            id: SharedString::from(scope.id),
            label: SharedString::from(scope.label),
            a11y_label: SharedString::from(scope.a11y_label),
            destructive: false,
            disabled: scope.scope == active_scope,
        });
        if scope.scope != active_scope {
            let selected_scope = scope.scope;
            let entity = entity.clone();
            item = item.on_select(move |_window, cx| {
                entity.update(cx, |this, cx| {
                    this.set_global_search_scope(selected_scope, cx);
                });
            });
        }
        item
    }));

    ContextMenu::new(
        SharedString::from(id),
        ContextMenuScope::GlobalSearchScope,
        SharedString::from(a11y_label),
    )
    .trigger_label(SharedString::from(trigger_label))
    .items(items)
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
        .flex_shrink_0()
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
        AppToolbarTabKey::Search => AppTab::Search,
        AppToolbarTabKey::Settings => AppTab::Settings,
    }
}

fn focus_handle_for_key(key: AppToolbarTabKey, app: &TopApp) -> &gpui::FocusHandle {
    match key {
        AppToolbarTabKey::Library => &app.library_tab_focus,
        AppToolbarTabKey::Search => &app.search_tab_focus,
        AppToolbarTabKey::Settings => &app.settings_tab_focus,
    }
}
