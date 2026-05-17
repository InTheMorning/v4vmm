//! Discover result list surface.
//!
//! Renders the scrollable search-result rows. `SearchApp` keeps result
//! selection, search pagination, focus, and thumbnail cache ownership.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, ClickEvent, Context, FocusHandle, FontWeight, Image,
    InteractiveElement, ScrollHandle, ScrollWheelEvent, SharedString, Styled,
};

use crate::discover::SearchApp;
use crate::ui::composites::{
    EntityKind, ListRow, ListRowA11yLabel, SkeletonTrackRow, TagBadge, TagBadgeDisplay, Thumbnail,
    ThumbnailSize,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button as UiButton, Label, SectionHeader};
use crate::ui::style::{color, spacing};
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::search::{
    pending_skeleton_count, should_auto_load_more, ResultRowRenderItem,
    SearchLibraryMembershipDisplay, SearchPaneDisplay, SearchResultSectionDisplay,
    AUTO_PAGINATE_THRESHOLD_PX,
};

pub(crate) struct DiscoverResultRow {
    item: ResultRowRenderItem,
    thumbnail: Option<Arc<Image>>,
    library_membership: Option<SearchLibraryMembershipDisplay>,
}

impl DiscoverResultRow {
    #[must_use]
    pub(crate) const fn new(
        item: ResultRowRenderItem,
        thumbnail: Option<Arc<Image>>,
        library_membership: Option<SearchLibraryMembershipDisplay>,
    ) -> Self {
        Self {
            item,
            thumbnail,
            library_membership,
        }
    }
}

pub(crate) struct DiscoverResultListParams<'a> {
    pub(crate) sections: Vec<DiscoverResultSection>,
    pub(crate) selected_key: Option<String>,
    pub(crate) list_focused: bool,
    pub(crate) empty_state: DiscoverResultEmptyState,
    pub(crate) pagination: DiscoverResultPagination,
    pub(crate) pane_display: SearchPaneDisplay,
    pub(crate) list_focus: &'a FocusHandle,
    pub(crate) scroll_handle: &'a ScrollHandle,
}

pub(crate) struct DiscoverResultSection {
    pub(crate) display: SearchResultSectionDisplay,
    pub(crate) rows: Vec<DiscoverResultRow>,
    pub(crate) show_empty: bool,
}

pub(crate) struct DiscoverResultEmptyState {
    pub(crate) is_empty: bool,
    pub(crate) is_loading: bool,
    pub(crate) status_empty: bool,
}

impl DiscoverResultEmptyState {
    #[must_use]
    const fn should_show_empty_message(&self) -> bool {
        self.is_empty && !self.is_loading
    }
}

pub(crate) struct DiscoverResultPagination {
    pub(crate) has_more: bool,
}

pub(crate) fn render_discover_result_list(
    params: DiscoverResultListParams<'_>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let sections = params
        .sections
        .into_iter()
        .map(|section| {
            render_result_section(
                section,
                params.selected_key.as_deref(),
                params.list_focused,
                cx,
            )
        })
        .collect::<Vec<_>>();

    let has_more = params.pagination.has_more;
    let is_loading_for_listener = params.empty_state.is_loading;
    let scroll_for_listener = params.scroll_handle.clone();

    div()
        .id(params.pane_display.results_scroll_id)
        .track_focus(params.list_focus)
        .track_scroll(params.scroll_handle)
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .p(spacing::SM)
        .on_scroll_wheel(cx.listener(
            move |this: &mut SearchApp, _event: &ScrollWheelEvent, _window, cx| {
                if !has_more {
                    return;
                }
                let max_y = f32::from(scroll_for_listener.max_offset().height);
                // GPUI scroll offsets are non-positive when scrolled down.
                let offset_y = f32::from(scroll_for_listener.offset().y);
                let remaining = max_y + offset_y;
                if should_auto_load_more(
                    remaining,
                    AUTO_PAGINATE_THRESHOLD_PX,
                    has_more,
                    is_loading_for_listener,
                ) {
                    this.do_search(true, cx);
                }
            },
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(spacing::SM)
                .children(sections)
                .when(
                    params.empty_state.is_loading && params.empty_state.is_empty,
                    |el| {
                        // Initial cold load: paint skeleton rows so the
                        // pane has structure instead of an empty void.
                        let count = pending_skeleton_count(true, false);
                        el.children((0..count).map(|i| {
                            SkeletonTrackRow::new(("discover-result-skeleton", i))
                                .into_any_element()
                        }))
                    },
                )
                .when(
                    params.empty_state.is_loading && !params.empty_state.is_empty,
                    |el| {
                        // Pagination tail: a few skeleton rows below the
                        // existing results signal "more incoming" without
                        // jumping the operator to a button.
                        let count = pending_skeleton_count(true, true);
                        el.children((0..count).map(|i| {
                            SkeletonTrackRow::new(("discover-result-skeleton-tail", i))
                                .into_any_element()
                        }))
                    },
                )
                .when(
                    params.empty_state.should_show_empty_message()
                        && params.empty_state.status_empty,
                    |el| el.child(render_empty_message(&params.pane_display)),
                )
                .when(
                    params.empty_state.should_show_empty_message()
                        && !params.empty_state.status_empty,
                    |el| el.child(render_empty_message(&params.pane_display)),
                )
                .when(
                    params.pagination.has_more && !params.empty_state.is_loading,
                    |el| {
                        el.child(
                            UiButton::styled(
                                params.pane_display.load_more_button_id,
                                ControlStyle::Ghost,
                            )
                            .label(params.pane_display.load_more_label)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.do_search(true, cx);
                            })),
                        )
                    },
                ),
        )
        .into_any_element()
}

fn render_empty_message(pane_display: &SearchPaneDisplay) -> AnyElement {
    div()
        .text_center()
        .p(spacing::XXL)
        .text_color(color::text_muted())
        .child(div().text_2xl().child(pane_display.empty_icon))
        .child(div().mt(spacing::SM).child(pane_display.empty_label))
        .into_any_element()
}

fn render_result_section(
    section: DiscoverResultSection,
    selected_key: Option<&str>,
    list_focused: bool,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let rows = section
        .rows
        .into_iter()
        .map(|row| {
            render_result_item(
                row.item,
                selected_key,
                row.thumbnail,
                row.library_membership,
                list_focused,
                cx,
            )
        })
        .collect::<Vec<_>>();
    let is_empty = rows.is_empty();

    div()
        .id(section.display.id)
        .flex()
        .flex_col()
        .gap(spacing::XXS)
        .child(SectionHeader::new(section.display.heading))
        .children(rows)
        .when(is_empty && section.show_empty, |el| {
            el.child(
                div()
                    .px(spacing::SM)
                    .py(spacing::XS)
                    .text_color(color::text_muted())
                    .child(section.display.empty_label),
            )
        })
        .into_any_element()
}

fn render_result_item(
    item: ResultRowRenderItem,
    selected_key: Option<&str>,
    thumbnail: Option<Arc<Image>>,
    library_membership: Option<SearchLibraryMembershipDisplay>,
    list_focused: bool,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let ResultRowRenderItem {
        selection_key,
        navigation_target,
        display,
    } = item;
    let element_id = display.element_id;
    let line1 = display.line1;
    let line2 = display.line2;
    let line3 = display.line3;
    let kind_label = display.kind_label;
    let membership_label = library_membership
        .as_ref()
        .map(|membership| membership.label);
    let row_a11y_label = if let Some(label) = membership_label {
        format!("{kind_label}: {line1}, {label}")
    } else {
        format!("{kind_label}: {line1}")
    };
    let is_selected = selected_key == Some(selection_key.as_str());

    let kind = EntityKind::from_legacy_str(&kind_label);

    ListRow::new(SharedString::from(element_id))
        .a11y_label(ListRowA11yLabel {
            label: row_a11y_label.into(),
        })
        .selected(is_selected)
        .focused(is_selected && list_focused)
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            let (source, entity_type, entity_id, feed_guid, title) =
                navigation_target.clone().into_parts();
            this.select_result(source, entity_type, entity_id, feed_guid, title, cx);
        }))
        .child(Thumbnail::new(kind, ThumbnailSize::Sm).image(thumbnail))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    Label::new(line1)
                        .size(FontSize::Micro)
                        .weight(FontWeight::MEDIUM)
                        .truncated(),
                )
                .when(!line2.is_empty(), |el| {
                    el.child(
                        Label::new(line2)
                            .size(FontSize::Micro)
                            .color(SemanticColor::TertiaryLabel)
                            .truncated(),
                    )
                })
                .when(!line3.is_empty(), |el| {
                    el.child(
                        div().opacity(0.7).child(
                            Label::new(line3)
                                .size(FontSize::Micro)
                                .color(SemanticColor::TertiaryLabel)
                                .truncated(),
                        ),
                    )
                })
                .when_some(library_membership, |el, membership| {
                    let color_role = if membership.is_in_library {
                        SemanticColor::Success
                    } else {
                        SemanticColor::SecondaryLabel
                    };
                    el.child(
                        Label::new(membership.label)
                            .size(FontSize::Micro)
                            .color(color_role)
                            .truncated(),
                    )
                }),
        )
        .child(TagBadge::new(TagBadgeDisplay {
            kind,
            label: Some(SharedString::from(kind_label)),
        }))
        .into_any_element()
}
