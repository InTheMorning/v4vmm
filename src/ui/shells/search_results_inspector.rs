//! Search-results inspector shell.
//!
//! ADR 0048 routes global search results into the workspace `ContentList` frame. This
//! shell consumes the GPUI-free search-results inspector VM and renders only
//! the body content: tab selection, paged rows, and empty states. Frame chrome,
//! breadcrumbs, and source filters remain owned by `frame_shell`.

#![warn(clippy::pedantic)]
use std::rc::Rc;

use gpui::{
    div, AnyElement, App, ClickEvent, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window,
};

use crate::runtime::paged_list_vm::{PagedListVm, RowSlot};
use crate::ui::composites::{
    EntityKind, Segment, SegmentDisplay, SegmentedControl, TagBadge, TagBadgeDisplay, Thumbnail,
    ThumbnailSize,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button as UiButton, Label, LabelVariant};
use crate::ui::shells::entity::{
    render_feed_identity_actions, render_release_detail_shell, ReleaseDetailBehaviorSlots,
};
use crate::ui::shells::search_result_rows::{
    artist_fields, feed_fields, origin_label, render_pending_result_row, render_result_row, tab_id,
    track_fields, SearchResultRowFields, SearchResultSelectHandler,
};
use crate::ui::shells::track::{
    build_track_detail_surface, render_track_page_identity_actions, TrackDetailBehaviorSlots,
};
use crate::ui::tokens::{FontSize, SemanticColor, Spacing};
use crate::view_models::entity_detail::{EntitySurfaceContext, ReleaseDetailVm};
use crate::view_models::search_results::{
    EmptyStateDisplay, IndexDetailDisplay, IndexDetailKind, SearchResultItemId, SearchResultOrigin,
    SearchResultsInspectorPageVm, SearchResultsTab,
};
use crate::view_models::track_detail::{TrackDetailSurfaceContext, TrackDetailVm};
use crate::view_models::workspace::ContentFilter;

type TabSelectHandler = Rc<dyn Fn(SearchResultsTab, &mut Window, &mut App) + 'static>;
type ResultSelectHandler = SearchResultSelectHandler;
type ClearFilterHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchResultsHeaderMode {
    Tabbed,
    Scoped {
        tab: SearchResultsTab,
        filter: ContentFilter,
    },
}

/// Callback slots supplied by the screen or frame owner.
#[derive(Default)]
#[must_use]
pub(crate) struct SearchResultsInspectorSlots {
    tab_select: Option<TabSelectHandler>,
    result_select: Option<ResultSelectHandler>,
    clear_filter: Option<ClearFilterHandler>,
}

impl SearchResultsInspectorSlots {
    /// Creates empty search-results inspector slots.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies the tab-selection callback.
    pub(crate) fn on_tab_select(
        mut self,
        handler: impl Fn(SearchResultsTab, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.tab_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the result-row activation callback.
    pub(crate) fn on_result_select(
        mut self,
        handler: impl Fn(SearchResultsTab, String, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.result_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the clear-filter callback for filtered empty states.
    pub(crate) fn on_clear_filter(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.clear_filter = Some(Rc::new(handler));
        self
    }
}

/// Renders a search-results inspector shell.
pub(crate) fn render_search_results_inspector(
    vm: &SearchResultsInspectorPageVm,
    slots: &SearchResultsInspectorSlots,
    header_mode: SearchResultsHeaderMode,
    cx: &App,
) -> AnyElement {
    let (tab, filter, empty_state) = match header_mode {
        SearchResultsHeaderMode::Tabbed => (vm.tab(), vm.filter(), vm.empty_state().cloned()),
        SearchResultsHeaderMode::Scoped { tab, filter } => {
            (tab, filter, vm.empty_state_for_scope(tab, filter))
        }
    };
    render_search_results_inspector_with_scope(
        vm,
        slots,
        tab,
        filter,
        empty_state.as_ref(),
        header_mode,
        cx,
    )
}

fn render_search_results_inspector_with_scope(
    vm: &SearchResultsInspectorPageVm,
    slots: &SearchResultsInspectorSlots,
    tab: SearchResultsTab,
    filter: ContentFilter,
    empty_state: Option<&EmptyStateDisplay>,
    header_mode: SearchResultsHeaderMode,
    cx: &App,
) -> AnyElement {
    let query = vm.query().to_string();
    let on_result_select = slots.result_select.as_ref();
    let body = if let Some(empty) = empty_state {
        render_empty_state(empty, slots.clear_filter.as_ref(), cx)
    } else {
        render_active_result_list(vm, tab, filter, on_result_select, cx)
    };

    let inspector = div()
        .id(SharedString::from(format!(
            "search-results-inspector-{}",
            tab_id(tab)
        )))
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_hidden()
        .child(render_inspector_header(
            &query,
            tab,
            filter.label(),
            slots.tab_select.as_ref(),
            header_mode,
            cx,
        ));

    inspector.child(body).into_any_element()
}

/// Renders a remote Index detail page reached from search-result drill-down.
pub(crate) fn render_index_detail_display(display: &IndexDetailDisplay, cx: &App) -> AnyElement {
    if let Some(feed) = display.feed.as_ref() {
        return render_index_feed_detail(feed, ReleaseDetailBehaviorSlots::default());
    }
    if let Some(track) = display.track.as_ref() {
        return render_index_track_detail(track, TrackDetailBehaviorSlots::default(), cx);
    }

    let kind = entity_kind_for_index_detail(display.kind);
    let mut heading = div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(Spacing::XS.scaled(cx))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(Spacing::XS.scaled(cx))
                .child(TagBadge::new(TagBadgeDisplay {
                    kind,
                    label: Some(SharedString::from(display.kind.label())),
                }))
                .child(origin_label(SearchResultOrigin::Index)),
        )
        .child(
            Label::new(display.title.clone())
                .variant(LabelVariant::Title)
                .truncated(),
        );
    if !display.secondary_text.is_empty() {
        heading = heading.child(
            Label::new(display.secondary_text.clone())
                .variant(LabelVariant::Caption)
                .truncated(),
        );
    }

    div()
        .id(SharedString::from(format!(
            "index-detail-{}",
            detail_id_suffix(display.kind)
        )))
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .p(Spacing::MD.scaled(cx))
        .gap(Spacing::LG.scaled(cx))
        .child(
            div()
                .flex()
                .flex_row()
                .items_start()
                .gap(Spacing::MD.scaled(cx))
                .child(Thumbnail::new(kind, ThumbnailSize::Lg))
                .child(heading),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(Spacing::XS.scaled(cx))
                .child(detail_metadata_row("Source", "Index", cx))
                .child(detail_metadata_row("ID", &display.id, cx)),
        )
        .into_any_element()
}

/// Renders a remote Index feed through the shared release-detail shell.
pub(crate) fn render_index_feed_detail(
    feed: &crate::views::FeedView,
    mut slots: ReleaseDetailBehaviorSlots,
) -> AnyElement {
    let projection = ReleaseDetailVm::new(feed, EntitySurfaceContext::Library);
    let page = projection.page();
    slots.identity_actions = render_feed_identity_actions(&page);
    render_release_detail_shell(&page, slots)
}

/// Renders a remote Index track through the shared track-detail shell.
pub(crate) fn render_index_track_detail(
    track: &crate::views::TrackView,
    mut slots: TrackDetailBehaviorSlots,
    cx: &App,
) -> AnyElement {
    let page = TrackDetailVm::new(track, TrackDetailSurfaceContext::Discover).page();
    slots.external_links = render_track_page_identity_actions(&page);

    div()
        .id(SharedString::from("index-detail-track"))
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .p(Spacing::MD.scaled(cx))
        .child(build_track_detail_surface(&page, slots))
        .into_any_element()
}

fn render_inspector_header(
    query: &str,
    selected: SearchResultsTab,
    filter_label: &str,
    tab_select: Option<&TabSelectHandler>,
    header_mode: SearchResultsHeaderMode,
    cx: &App,
) -> AnyElement {
    let mut header_row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::SM.scaled(cx))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    Label::new(query.to_string())
                        .variant(LabelVariant::Headline)
                        .truncated(),
                )
                .child(
                    Label::new(filter_label.to_string())
                        .variant(LabelVariant::Caption)
                        .truncated(),
                ),
        );

    if header_mode == SearchResultsHeaderMode::Tabbed {
        header_row = header_row.child(render_tab_strip(selected, tab_select));
    }

    div()
        .flex()
        .flex_col()
        .gap(Spacing::XS.scaled(cx))
        .p(Spacing::MD.scaled(cx))
        .child(header_row)
        .into_any_element()
}

fn render_tab_strip(
    selected: SearchResultsTab,
    on_tab_select: Option<&TabSelectHandler>,
) -> SegmentedControl<SearchResultsTab> {
    let mut control = SegmentedControl::new(selected).segments([
        Segment::new(tab_segment_display(SearchResultsTab::Artists)),
        Segment::new(tab_segment_display(SearchResultsTab::Feeds)),
        Segment::new(tab_segment_display(SearchResultsTab::Tracks)),
    ]);

    if let Some(handler) = on_tab_select.cloned() {
        control = control.on_select(move |tab, window, cx| {
            handler(*tab, window, cx);
        });
    }

    control
}

fn tab_segment_display(tab: SearchResultsTab) -> SegmentDisplay<SearchResultsTab> {
    SegmentDisplay {
        id: SharedString::from(format!("search-results-tab-{}", tab_id(tab))).into(),
        key: tab,
        label: SharedString::from(tab.label()),
        a11y_label: SharedString::from(format!("Show {} search results", tab.label())),
    }
}

fn detail_metadata_row(label: &str, value: &str, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(Spacing::SM.scaled(cx))
        .child(
            Label::new(label.to_string())
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel),
        )
        .child(
            div().flex_1().min_w_0().child(
                Label::new(value.to_string())
                    .size(FontSize::Micro)
                    .truncated(),
            ),
        )
        .into_any_element()
}

fn render_active_result_list(
    vm: &SearchResultsInspectorPageVm,
    tab: SearchResultsTab,
    filter: ContentFilter,
    on_result_select: Option<&ResultSelectHandler>,
    cx: &App,
) -> AnyElement {
    let show_loading_placeholders =
        vm.is_index_loading() && matches!(filter, ContentFilter::All | ContentFilter::Index);
    match tab {
        SearchResultsTab::Artists => render_result_window(
            vm.artists().window(filter),
            SearchResultsTab::Artists,
            EntityKind::Artist,
            show_loading_placeholders,
            on_result_select,
            artist_fields,
            cx,
        ),
        SearchResultsTab::Feeds => render_result_window(
            vm.feeds().window(filter),
            SearchResultsTab::Feeds,
            EntityKind::Feed,
            show_loading_placeholders,
            on_result_select,
            feed_fields,
            cx,
        ),
        SearchResultsTab::Tracks => render_result_window(
            vm.tracks().window(filter),
            SearchResultsTab::Tracks,
            EntityKind::Track,
            show_loading_placeholders,
            on_result_select,
            track_fields,
            cx,
        ),
    }
}

fn render_result_window<Row>(
    window: &PagedListVm<SearchResultItemId, Row>,
    tab: SearchResultsTab,
    kind: EntityKind,
    show_loading_placeholders: bool,
    on_result_select: Option<&ResultSelectHandler>,
    fields_for: fn(&Row) -> SearchResultRowFields<'_>,
    cx: &App,
) -> AnyElement {
    let show_loading_placeholders = show_loading_placeholders && window.total() == 0;
    let visible_rows = if show_loading_placeholders {
        3
    } else {
        window.total().min(window.page_size())
    };

    div()
        .id(SharedString::from(format!(
            "search-results-{}-rows",
            tab_id(tab)
        )))
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .px(Spacing::MD.scaled(cx))
        .pb(Spacing::MD.scaled(cx))
        .gap(Spacing::XXS.scaled(cx))
        .children((0..visible_rows).map(|index| {
            if show_loading_placeholders {
                render_pending_result_row(tab, kind, index)
            } else {
                match window.peek_row(index) {
                    RowSlot::Ready(row) => render_result_row(
                        tab,
                        kind,
                        fields_for(row.as_ref()),
                        None,
                        on_result_select,
                    ),
                    RowSlot::Pending(placeholder) => {
                        render_pending_result_row(tab, kind, placeholder.index)
                    }
                }
            }
        }))
        .into_any_element()
}

fn render_empty_state(
    empty: &EmptyStateDisplay,
    on_clear_filter: Option<&ClearFilterHandler>,
    cx: &App,
) -> AnyElement {
    let mut content = div()
        .flex()
        .flex_col()
        .items_center()
        .gap(Spacing::XS.scaled(cx))
        .child(Label::new(empty.title.clone()).variant(LabelVariant::Headline))
        .child(
            Label::new(empty.secondary.clone())
                .variant(LabelVariant::Caption)
                .truncated(),
        );

    if let Some(action_id) = empty.clear_filter_action_id {
        let mut clear_button = UiButton::styled(SharedString::from(action_id), ControlStyle::Ghost)
            .label("Show all")
            .a11y_label("Clear search result filter");
        if let Some(handler) = on_clear_filter.cloned() {
            clear_button = clear_button.on_click(move |_: &ClickEvent, window, cx| {
                handler(window, cx);
            });
        } else {
            clear_button = clear_button.disabled(true);
        }
        content = content.child(clear_button);
    }

    div()
        .flex()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .items_center()
        .justify_center()
        .p(Spacing::MD.scaled(cx))
        .child(content)
        .into_any_element()
}

const fn entity_kind_for_index_detail(kind: IndexDetailKind) -> EntityKind {
    match kind {
        IndexDetailKind::Feed => EntityKind::Feed,
        IndexDetailKind::Track => EntityKind::Track,
    }
}

const fn detail_id_suffix(kind: IndexDetailKind) -> &'static str {
    match kind {
        IndexDetailKind::Feed => "feed",
        IndexDetailKind::Track => "track",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_accept_callbacks() {
        let slots = SearchResultsInspectorSlots::new()
            .on_tab_select(|_, _, _| {})
            .on_result_select(|_, _, _, _| {})
            .on_clear_filter(|_, _| {});

        assert!(slots.tab_select.is_some());
        assert!(slots.result_select.is_some());
        assert!(slots.clear_filter.is_some());
    }
}
