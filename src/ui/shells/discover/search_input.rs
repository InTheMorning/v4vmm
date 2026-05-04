//! Discover search input bar: text field, fuzzy toggle, and filter buttons.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, Context, Entity, FontWeight, Rgba, SharedString, Styled};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputState};
use gpui_component::Size;

use crate::search::SearchApp;
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button as UiButton;
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::style::{color, spacing, typography};
use crate::view_models::search::{normalized_search_query, SearchPaneDisplay, SearchViewModel};

pub(crate) struct DiscoverSearchInputParams {
    pub(crate) input: Entity<InputState>,
    pub(crate) type_filter: usize,
    pub(crate) is_loading: bool,
    pub(crate) fuzzy_search: bool,
    pub(crate) show_recents_command: bool,
    pub(crate) pane_display: SearchPaneDisplay,
    pub(crate) status_color: Rgba,
    pub(crate) status_text: String,
}

pub(crate) fn render_discover_search_input(
    params: DiscoverSearchInputParams,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let type_filters: Vec<AnyElement> = SearchViewModel::type_filter_options()
        .iter()
        .map(|option| {
            render_filter_button(
                option.index,
                option.label,
                option.index == params.type_filter,
                cx,
            )
        })
        .collect();
    let search_label = params.pane_display.search_button_label;

    div()
        .p(spacing::MD)
        .border_b_1()
        .border_color(color::border_subtle())
        .flex()
        .flex_col()
        .gap(spacing::SM)
        .child(
            div()
                .text_size(typography::SIZE_MICRO)
                .font_weight(FontWeight::BOLD)
                .text_color(color::text_muted())
                .child(params.pane_display.heading),
        )
        .child(
            Input::new(&params.input)
                .cleanable(true)
                .scaled(Size::Small, cx),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(spacing::SM)
                .children(type_filters),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(spacing::SM)
                .child(
                    // CONTROL-COMPAT(reason): native Button does not yet expose loading state.
                    Button::new(params.pane_display.search_button_id)
                        .label(search_label)
                        .primary()
                        .scaled(Size::Small, cx)
                        .text_color(color::text_on_accent())
                        .loading(params.is_loading)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.do_search(false, cx);
                        })),
                )
                .child(
                    UiButton::styled(
                        params.pane_display.fuzzy_toggle_id,
                        if params.fuzzy_search {
                            ControlStyle::Pill
                        } else {
                            ControlStyle::Ghost
                        },
                    )
                    .label(params.pane_display.fuzzy_toggle_label)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_fuzzy_search(cx);
                    })),
                )
                .when(params.show_recents_command, |el| {
                    el.child(
                        UiButton::styled(
                            params.pane_display.recents_button_id,
                            ControlStyle::Ghost,
                        )
                        .label(params.pane_display.recents_button_label)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.show_recent_feeds(window, cx);
                        })),
                    )
                }),
        )
        .child(
            div()
                .text_xs()
                .text_color(params.status_color)
                .child(SharedString::from(params.status_text)),
        )
        .into_any_element()
}

fn render_filter_button(
    idx: usize,
    label: &'static str,
    selected: bool,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    UiButton::styled(
        ("type-filter", idx),
        if selected {
            ControlStyle::Pill
        } else {
            ControlStyle::Ghost
        },
    )
    .label(label)
    .on_click(cx.listener(move |this, _, _, cx| {
        if this.vm.set_type_filter_if_changed(idx) {
            let has_query = normalized_search_query(&this.input.read(cx).value()).is_some();
            cx.notify();
            if has_query {
                this.do_search(false, cx);
            }
        }
    }))
    .into_any_element()
}
