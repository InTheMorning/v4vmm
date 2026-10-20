//! Search-results inspector shell.
//!
//! ADR 0048 routes global search results into the workspace `ContentList` frame. This
//! shell consumes the GPUI-free search-results inspector VM and renders only
//! the body content: tab selection, paged rows, and empty states. Frame chrome,
//! breadcrumbs, and source filters remain owned by `frame_shell`.

#![warn(clippy::pedantic)]
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, AnyElement, App, ClickEvent, Image, InteractiveElement, IntoElement, ParentElement,
    SharedString, Styled, Window,
};

use crate::runtime::paged_list_vm::{PagedListVm, RowSlot};
use crate::ui::composites::music_column_scroll::{
    music_column_scrollbar, music_column_tracked_scrollbar,
};
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::composites::{
    EntityKind, Segment, SegmentDisplay, SegmentedControl, TagBadge, TagBadgeDisplay, Thumbnail,
    ThumbnailSize,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button as UiButton, Label, LabelVariant, MultilineText};
use crate::ui::shells::search_result_rows::{
    artist_fields, feed_fields, origin_label, render_pending_result_row, render_result_row, tab_id,
    track_fields, SearchResultRowFields, SearchResultSelectHandler,
};
use crate::ui::shells::track::{
    build_track_detail_surface, render_track_page_identity_actions, TrackDetailBehaviorSlots,
};
use crate::ui::tokens::{FontSize, SemanticColor, Size, Spacing};
use crate::view_models::search_results::{
    EmptyStateDisplay, IndexDetailDisplay, IndexDetailKind, SearchFailureAction,
    SearchFailureAvailability, SearchResultItemId, SearchResultOrigin,
    SearchResultsInspectorPageVm, SearchResultsTab,
};
use crate::view_models::track_detail::{TrackDetailSurfaceContext, TrackDetailVm};
use crate::view_models::workspace::ContentFilter;

type TabSelectHandler = Rc<dyn Fn(SearchResultsTab, &mut Window, &mut App) + 'static>;
type ResultSelectHandler = SearchResultSelectHandler;
type ClearFilterHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type FailureHandler = Rc<dyn Fn(SearchFailureAction, &mut Window, &mut App) + 'static>;

// ADR 0077 packet 006 deleted this mode's only scoped drill-down caller
// (the old Index name-route feed results). `Tabbed` is the one remaining
// header mode; the type stays named for its one caller in `app.rs`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchResultsHeaderMode {
    Tabbed,
}

/// Callback slots supplied by the screen or frame owner.
#[derive(Default)]
#[must_use]
pub(crate) struct SearchResultsInspectorSlots {
    tab_select: Option<TabSelectHandler>,
    result_select: Option<ResultSelectHandler>,
    clear_filter: Option<ClearFilterHandler>,
    failure_action: Option<FailureHandler>,
    thumbnails: BTreeMap<String, Option<Arc<Image>>>,
}

impl SearchResultsInspectorSlots {
    /// Creates empty search-results inspector slots.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies disclosure and clipboard wiring for the VM's failure actions.
    pub(crate) fn on_failure_action(
        mut self,
        handler: impl Fn(SearchFailureAction, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.failure_action = Some(Rc::new(handler));
        self
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

    /// Supplies resolved row thumbnails keyed by row thumbnail href.
    pub(crate) fn with_thumbnails(
        mut self,
        thumbnails: BTreeMap<String, Option<Arc<Image>>>,
    ) -> Self {
        self.thumbnails = thumbnails;
        self
    }

    fn thumbnail_for_href(&self, href: &str) -> Option<Arc<Image>> {
        self.thumbnails.get(href).cloned().flatten()
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
        render_empty_state(
            empty,
            slots.clear_filter.as_ref(),
            slots.failure_action.as_ref(),
            cx,
        )
    } else {
        render_active_result_list(vm, tab, filter, slots, on_result_select, cx)
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

    let suffix = detail_id_suffix(display.kind);
    let container = div()
        .id(SharedString::from(format!("index-detail-{suffix}")))
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .p(Spacing::MD.scaled(cx))
        .child(
            page_scroll_content(cx)
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
                ),
        );
    music_column_scrollbar(
        container,
        SharedString::from(format!("index-detail-{suffix}-scrollbar")),
    )
}

/// Renders a remote Index track through the shared track-detail shell.
pub(crate) fn render_index_track_detail(
    track: &crate::views::TrackView,
    mut slots: TrackDetailBehaviorSlots,
    cx: &App,
) -> AnyElement {
    let page = TrackDetailVm::new(track, TrackDetailSurfaceContext::Discover).page();
    slots.external_links = render_track_page_identity_actions(&page);
    let scroll = slots.scroll_handle.take();

    let container = div()
        .id(SharedString::from("index-detail-track"))
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .p(Spacing::MD.scaled(cx))
        .child(page_scroll_content(cx).child(build_track_detail_surface(&page, slots)));
    // ADR 0084 packet 018: in the Music detail column, the page holds the
    // scroll handle of the column, so its keys can scroll it.
    match &scroll {
        Some(scroll) => music_column_tracked_scrollbar(container, scroll, None),
        None => music_column_scrollbar(container, "index-detail-track-scrollbar"),
    }
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
    slots: &SearchResultsInspectorSlots,
    on_result_select: Option<&ResultSelectHandler>,
    cx: &App,
) -> AnyElement {
    let show_loading_placeholders =
        vm.is_index_loading() && matches!(filter, ContentFilter::All | ContentFilter::Index);
    match tab {
        SearchResultsTab::Artists => render_result_window(
            vm.artists().window(filter),
            SearchResultsTab::Artists,
            show_loading_placeholders,
            slots,
            on_result_select,
            artist_fields,
            cx,
        ),
        SearchResultsTab::Feeds => render_result_window(
            vm.feeds().window(filter),
            SearchResultsTab::Feeds,
            show_loading_placeholders,
            slots,
            on_result_select,
            feed_fields,
            cx,
        ),
        SearchResultsTab::Tracks => render_result_window(
            vm.tracks().window(filter),
            SearchResultsTab::Tracks,
            show_loading_placeholders,
            slots,
            on_result_select,
            track_fields,
            cx,
        ),
    }
}

fn render_result_window<Row>(
    window: &PagedListVm<SearchResultItemId, Row>,
    tab: SearchResultsTab,
    show_loading_placeholders: bool,
    slots: &SearchResultsInspectorSlots,
    on_result_select: Option<&ResultSelectHandler>,
    fields_for: fn(&Row) -> SearchResultRowFields<'_>,
    cx: &App,
) -> AnyElement {
    let kind = entity_kind_for_tab(tab);
    let show_loading_placeholders = show_loading_placeholders && window.total() == 0;
    let visible_rows = if show_loading_placeholders {
        3
    } else {
        window.total().min(window.page_size())
    };

    let tab_slug = tab_id(tab);
    let container = div()
        .id(SharedString::from(format!(
            "search-results-{tab_slug}-rows"
        )))
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .px(Spacing::MD.scaled(cx))
        .pb(Spacing::MD.scaled(cx))
        .child(
            page_scroll_content(cx)
                .gap(Spacing::XXS.scaled(cx))
                .children((0..visible_rows).map(|index| {
                    if show_loading_placeholders {
                        render_pending_result_row(tab, kind, index)
                    } else {
                        match window.peek_row(index) {
                            RowSlot::Ready(row) => {
                                let fields = fields_for(row.as_ref());
                                let thumbnail = fields
                                    .thumbnail_href
                                    .and_then(|href| slots.thumbnail_for_href(href));
                                render_result_row(tab, kind, fields, thumbnail, on_result_select)
                            }
                            RowSlot::Pending(placeholder) => {
                                render_pending_result_row(tab, kind, placeholder.index)
                            }
                        }
                    }
                })),
        );
    music_column_scrollbar(
        container,
        SharedString::from(format!("search-results-{tab_slug}-rows-scrollbar")),
    )
}

const fn entity_kind_for_tab(tab: SearchResultsTab) -> EntityKind {
    match tab {
        SearchResultsTab::Artists => EntityKind::Artist,
        SearchResultsTab::Feeds => EntityKind::Feed,
        SearchResultsTab::Tracks => EntityKind::Track,
    }
}

fn render_empty_state(
    empty: &EmptyStateDisplay,
    on_clear_filter: Option<&ClearFilterHandler>,
    on_failure: Option<&FailureHandler>,
    cx: &App,
) -> AnyElement {
    let mut content = div()
        .w_full()
        .max_w(Size::NoticeWidth.scaled(cx))
        .min_w_0()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .gap(Spacing::MD.scaled(cx))
        .child(Label::new(empty.title.clone()).variant(LabelVariant::Headline))
        .child(
            MultilineText::new(empty.secondary.clone())
                .max_lines(usize::MAX)
                .wrap_lines()
                .size(FontSize::Body)
                .color(SemanticColor::SecondaryLabel),
        );

    if let Some(failure) = &empty.failure {
        if let Some(handler) = on_failure {
            let mut actions = div().flex().flex_wrap().gap(Spacing::SM.scaled(cx));
            for intent in [
                SearchFailureAction::ToggleDetails,
                SearchFailureAction::CopyReport,
            ] {
                let display = failure.action(intent);
                let handler = handler.clone();
                actions = actions.child(
                    UiButton::styled(
                        SharedString::from(format!("search-failure-{intent:?}")),
                        ControlStyle::Secondary,
                    )
                    .label(display.label)
                    .a11y_label(display.a11y_label)
                    .disabled(display.availability != SearchFailureAvailability::Available)
                    .on_activate(move |window, cx| handler(display.action, window, cx)),
                );
            }
            content = content.child(actions);
        }
        if let Some(report) = failure.visible_report() {
            content = content.child(
                MultilineText::new(report.to_owned())
                    .max_lines(usize::MAX)
                    .wrap_lines()
                    .size(FontSize::Caption)
                    .color(SemanticColor::SecondaryLabel),
            );
        }
    }

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

    let container = div()
        .id("search-results-notice")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .items_center()
        .p(Spacing::MD.scaled(cx))
        .child(
            page_scroll_content(cx)
                .w_full()
                .items_center()
                .child(content),
        );
    music_column_scrollbar(container, "search-results-notice-scrollbar")
}

const fn entity_kind_for_index_detail(kind: IndexDetailKind) -> EntityKind {
    match kind {
        IndexDetailKind::Release => EntityKind::Release,
        IndexDetailKind::Track => EntityKind::Track,
    }
}

/// The element id suffix of a detail page. The id of a release page keeps
/// the suffix "feed", so that each element id stays the same.
const fn detail_id_suffix(kind: IndexDetailKind) -> &'static str {
    match kind {
        IndexDetailKind::Release => "feed",
        IndexDetailKind::Track => "track",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// F6 of the walk of 2026-10-20 (ADR 0084 Decision 5): the loading and
    /// failed header of an Index release page shows the tag "release".
    #[test]
    fn adr_0084_walk_fix_index_release_header_kind_label_is_release() {
        assert_eq!(IndexDetailKind::Release.label(), "release");
        assert_eq!(
            entity_kind_for_index_detail(IndexDetailKind::Release).label(),
            "release"
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
