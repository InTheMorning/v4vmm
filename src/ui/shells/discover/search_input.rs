//! Discover search controls: scope-adjacent filters, status, and recents.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Rgba, SharedString, Styled};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::Size;

use crate::discover::SearchApp;
use crate::ui::composites::{Segment, SegmentDisplay, SegmentedControl};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button as UiButton;
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::style::{color, spacing, typography};
use crate::view_models::search::{IndexControlsVisibility, SearchPaneDisplay, SearchViewModel};

pub(crate) struct DiscoverSearchControlsParams {
    pub(crate) type_filter: usize,
    pub(crate) is_loading: bool,
    pub(crate) fuzzy_search: bool,
    pub(crate) index_controls: IndexControlsVisibility,
    pub(crate) show_recents_command: bool,
    pub(crate) pane_display: SearchPaneDisplay,
    pub(crate) status_color: Rgba,
    pub(crate) status_text: String,
}

pub(crate) fn render_discover_search_controls(
    params: DiscoverSearchControlsParams,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
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
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(spacing::SM)
                .when(params.index_controls.is_visible(), |el| {
                    el.child(render_type_filter_control(params.type_filter, cx))
                }),
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
                        .label(params.pane_display.refresh_button_label)
                        .primary()
                        .scaled(Size::Small, cx)
                        .text_color(color::text_on_accent())
                        .loading(params.is_loading)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.do_search(false, cx);
                        })),
                )
                .when(params.index_controls.is_visible(), |el| {
                    el.child(
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
                })
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

fn render_type_filter_control(
    selected: usize,
    cx: &mut Context<SearchApp>,
) -> SegmentedControl<usize> {
    let type_filter_segments = SearchViewModel::type_filter_options()
        .iter()
        .map(|option| {
            Segment::new(SegmentDisplay {
                id: option.button_id.into(),
                key: option.index,
                label: option.label.into(),
                a11y_label: option.a11y_label.into(),
            })
        })
        .collect::<Vec<_>>();
    let entity = cx.entity();

    SegmentedControl::new(selected)
        .filter_style()
        .segments(type_filter_segments)
        .on_select(move |idx, _window, cx| {
            let idx = *idx;
            entity.update(cx, |this, cx| {
                if this.vm.set_type_filter_if_changed(idx) {
                    let query = this.vm.active_query.clone();
                    cx.notify();
                    if let Some(query) = query {
                        this.rerun_global_search_with_active_filter(query, cx);
                    }
                }
            });
        })
}
