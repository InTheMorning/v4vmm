//! Search-results inspector shell.
//!
//! ADR 0048 routes global search results into the workspace `ContentList` frame. This
//! shell consumes the GPUI-free search-results inspector VM and renders only
//! the body content: tab selection, paged rows, and empty states. Frame chrome,
//! breadcrumbs, and source filters remain owned by `frame_shell`.

#![warn(clippy::pedantic)]
use std::rc::Rc;

use gpui::{
    div, AnyElement, App, ClickEvent, FontWeight, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window,
};

use crate::runtime::paged_list_vm::{PagedListVm, RowSlot};
use crate::ui::composites::{
    EntityKind, ListRow, ListRowA11yLabel, Segment, SegmentDisplay, SegmentedControl, TagBadge,
    TagBadgeDisplay, Thumbnail, ThumbnailSize,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button as UiButton, Label, LabelVariant};
use crate::ui::shells::entity::{
    render_feed_identity_actions, render_release_detail_shell, ReleaseDetailBehaviorSlots,
};
use crate::ui::tokens::{FontSize, SemanticColor, Spacing};
use crate::view_models::entity_detail::{EntitySurfaceContext, ReleaseDetailVm};
use crate::view_models::search_results::{
    ArtistResultDisplay, EmptyStateDisplay, FeedResultDisplay, IndexDetailDisplay, IndexDetailKind,
    SearchResultItemId, SearchResultOrigin, SearchResultsInspectorPageVm, SearchResultsTab,
    TrackResultDisplay,
};
use crate::view_models::workspace::ContentFilter;

type TabSelectHandler = Rc<dyn Fn(SearchResultsTab, &mut Window, &mut App) + 'static>;
type ResultSelectHandler = Rc<dyn Fn(SearchResultsTab, String, &mut Window, &mut App) + 'static>;
type ClearFilterHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SearchResultsHeaderMode {
    Tabbed,
    Scoped,
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
    cx: &App,
) -> AnyElement {
    render_search_results_inspector_with_scope(
        vm,
        slots,
        vm.tab(),
        vm.filter(),
        vm.empty_state(),
        SearchResultsHeaderMode::Tabbed,
        cx,
    )
}

/// Renders a search-results inspector using an explicit tab/filter scope.
pub(crate) fn render_search_results_inspector_scoped(
    vm: &SearchResultsInspectorPageVm,
    slots: &SearchResultsInspectorSlots,
    tab: SearchResultsTab,
    filter: ContentFilter,
    cx: &App,
) -> AnyElement {
    let empty_state = vm.empty_state_for_scope(tab, filter);
    render_search_results_inspector_with_scope(
        vm,
        slots,
        tab,
        filter,
        empty_state.as_ref(),
        SearchResultsHeaderMode::Scoped,
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
    fields_for: fn(&Row) -> ResultRowFields<'_>,
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
                    RowSlot::Ready(row) => {
                        render_result_row(tab, kind, fields_for(row.as_ref()), on_result_select)
                    }
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

fn render_result_row(
    tab: SearchResultsTab,
    kind: EntityKind,
    fields: ResultRowFields<'_>,
    on_result_select: Option<&ResultSelectHandler>,
) -> AnyElement {
    let row_id = fields.id.to_string();
    let element_id = SharedString::from(format!("search-results-{}-{row_id}", tab_id(tab)));
    let mut row = ListRow::new(element_id)
        .a11y_label(ListRowA11yLabel {
            label: SharedString::from(fields.a11y_label.to_string()),
        })
        .child(Thumbnail::new(kind, ThumbnailSize::Sm))
        .child(result_row_text(fields))
        .child(origin_label(fields.origin))
        .child(TagBadge::new(TagBadgeDisplay {
            kind,
            label: Some(SharedString::from(kind.label())),
        }));

    if let Some(handler) = on_result_select.cloned() {
        row = row.on_click(move |_: &ClickEvent, window, cx| {
            handler(tab, row_id.clone(), window, cx);
        });
    }

    row.into_any_element()
}

fn result_row_text(fields: ResultRowFields<'_>) -> AnyElement {
    let mut text = div().flex_1().min_w_0().child(
        Label::new(fields.label.to_string())
            .size(FontSize::Micro)
            .weight(FontWeight::MEDIUM)
            .truncated(),
    );

    if !fields.secondary_text.is_empty() {
        text = text.child(
            Label::new(fields.secondary_text.to_string())
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel)
                .truncated(),
        );
    }

    text.into_any_element()
}

fn origin_label(origin: SearchResultOrigin) -> Label {
    Label::new(origin_label_text(origin))
        .size(FontSize::Micro)
        .color(origin_label_color(origin))
        .truncated()
}

fn render_pending_result_row(tab: SearchResultsTab, kind: EntityKind, index: usize) -> AnyElement {
    ListRow::new(SharedString::from(format!(
        "search-results-{}-pending-{index}",
        tab_id(tab)
    )))
    .a11y_label(ListRowA11yLabel {
        label: SharedString::from("Loading search result"),
    })
    .child(Thumbnail::new(kind, ThumbnailSize::Sm))
    .child(
        div().flex_1().min_w_0().child(
            Label::new("Loading result")
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel)
                .truncated(),
        ),
    )
    .child(TagBadge::new(TagBadgeDisplay {
        kind,
        label: Some(SharedString::from(kind.label())),
    }))
    .into_any_element()
}

#[derive(Clone, Copy)]
struct ResultRowFields<'a> {
    id: &'a str,
    label: &'a str,
    secondary_text: &'a str,
    a11y_label: &'a str,
    origin: SearchResultOrigin,
}

fn artist_fields(row: &ArtistResultDisplay) -> ResultRowFields<'_> {
    ResultRowFields {
        id: &row.id,
        label: &row.label,
        secondary_text: &row.secondary_text,
        a11y_label: &row.a11y_label,
        origin: row.origin,
    }
}

fn feed_fields(row: &FeedResultDisplay) -> ResultRowFields<'_> {
    ResultRowFields {
        id: &row.id,
        label: &row.label,
        secondary_text: &row.secondary_text,
        a11y_label: &row.a11y_label,
        origin: row.origin,
    }
}

fn track_fields(row: &TrackResultDisplay) -> ResultRowFields<'_> {
    ResultRowFields {
        id: &row.id,
        label: &row.label,
        secondary_text: &row.secondary_text,
        a11y_label: &row.a11y_label,
        origin: row.origin,
    }
}

const fn tab_id(tab: SearchResultsTab) -> &'static str {
    match tab {
        SearchResultsTab::Artists => "artists",
        SearchResultsTab::Feeds => "feeds",
        SearchResultsTab::Tracks => "tracks",
    }
}

const fn origin_label_text(origin: SearchResultOrigin) -> &'static str {
    match origin {
        SearchResultOrigin::Library => "In Library",
        SearchResultOrigin::Index => "Index",
    }
}

const fn origin_label_color(origin: SearchResultOrigin) -> SemanticColor {
    match origin {
        SearchResultOrigin::Library => SemanticColor::Accent,
        SearchResultOrigin::Index => SemanticColor::TertiaryLabel,
    }
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
    fn tab_ids_are_stable() {
        assert_eq!(tab_id(SearchResultsTab::Artists), "artists");
        assert_eq!(tab_id(SearchResultsTab::Feeds), "feeds");
        assert_eq!(tab_id(SearchResultsTab::Tracks), "tracks");
    }

    #[test]
    fn origin_labels_use_membership_language() {
        assert_eq!(origin_label_text(SearchResultOrigin::Library), "In Library");
        assert_eq!(origin_label_text(SearchResultOrigin::Index), "Index");
        assert_eq!(
            origin_label_color(SearchResultOrigin::Library),
            SemanticColor::Accent
        );
    }

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
