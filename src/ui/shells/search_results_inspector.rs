//! Search-results inspector shell.
//!
//! ADR 0084 Decision 2 routes global search results into the Music list
//! column. This shell consumes the GPUI-free search-results view model and
//! renders only the body content: the query, the tab control, the rows and
//! the empty states. Frame chrome and breadcrumbs remain owned by
//! `frame_shell`.

#![warn(clippy::pedantic)]
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, AnyElement, App, Image, InteractiveElement, IntoElement, ParentElement, ScrollHandle,
    SharedString, Styled, Window,
};

use crate::ui::composites::music_column_scroll::{
    music_column_scrollbar, music_column_tracked_scrollbar,
};
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::composites::{
    CollectionArtImage, EntityKind, Segment, SegmentDisplay, SegmentedControl, TagBadge,
    TagBadgeDisplay, Thumbnail, ThumbnailSize,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::layouts::OVERLAY_SCROLLBAR_WIDTH;
use crate::ui::primitives::{Button as UiButton, Label, LabelVariant, MultilineText};
use crate::ui::shells::search_result_rows::{
    origin_label, render_pending_result_row, render_result_row, tab_id, SearchResultRowHandlers,
    SearchResultSelectHandler, SearchResultToggleHandler, SearchRowLeading,
};
use crate::ui::tokens::{FontSize, SemanticColor, Size, Spacing};
use crate::view_models::detail_selection::DetailRowKey;
use crate::view_models::search_results::{
    EmptyStateDisplay, IndexDetailDisplay, IndexDetailKind, SearchFailureAction,
    SearchFailureAvailability, SearchPageActionDisplay, SearchPageActionStyle, SearchResultOrigin,
    SearchResultRowVm, SearchResultsInspectorPageVm, SearchResultsTab,
};

type TabSelectHandler = Rc<dyn Fn(SearchResultsTab, &mut Window, &mut App) + 'static>;
type FailureHandler = Rc<dyn Fn(SearchFailureAction, &mut Window, &mut App) + 'static>;
type HeaderActionHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

// ADR 0077 packet 006 deleted this mode's only scoped drill-down caller
// (the old Index name-route feed results). `Tabbed` is the one remaining
// header mode; the type stays named for its one caller in `app.rs`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchResultsHeaderMode {
    Tabbed,
}

/// Callback slots and the visible rows, supplied by the screen or frame
/// owner. The app layer makes the rows with the Library facts of this
/// render (packet 010).
#[derive(Default)]
#[must_use]
pub(crate) struct SearchResultsInspectorSlots {
    tab_select: Option<TabSelectHandler>,
    result_select: Option<SearchResultSelectHandler>,
    row_toggle: Option<SearchResultToggleHandler>,
    failure_action: Option<FailureHandler>,
    header_action: Option<(SearchPageActionDisplay, HeaderActionHandler)>,
    thumbnails: BTreeMap<String, Option<Arc<Image>>>,
    rows: Vec<SearchResultRowVm>,
    arts: BTreeMap<String, CollectionArtImage>,
    scroll: ScrollHandle,
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
        handler: impl Fn(
                SearchResultsTab,
                String,
                Option<DetailRowKey>,
                gpui::Modifiers,
                &mut Window,
                &mut App,
            ) + 'static,
    ) -> Self {
        self.result_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the checkbox callback of a row (ADR 0084 Decision 10).
    pub(crate) fn on_row_toggle(
        mut self,
        handler: impl Fn(DetailRowKey, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.row_toggle = Some(Rc::new(handler));
        self
    }

    /// Supplies the typed action of the header, for example "Save search"
    /// or "Delete…" (packet 012), and its click handler. The handler runs
    /// only when the view model makes the action available.
    pub(crate) fn header_action(
        mut self,
        display: SearchPageActionDisplay,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.header_action = Some((display, Rc::new(handler)));
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

    /// Supplies the visible rows of the active tab.
    pub(crate) fn with_rows(mut self, rows: Vec<SearchResultRowVm>) -> Self {
        self.rows = rows;
        self
    }

    /// Supplies the loaded logo frame art of each row with art, by row id
    /// (packet 028 D3). A row with art and no entry shows the entity color.
    pub(crate) fn with_arts(mut self, arts: BTreeMap<String, CollectionArtImage>) -> Self {
        self.arts = arts;
        self
    }

    /// Supplies the scroll of the rows (packet 028 D2). The owner keeps the
    /// handle, and scrolls the focused row into view after a key.
    pub(crate) fn with_scroll(mut self, scroll: &ScrollHandle) -> Self {
        self.scroll = scroll.clone();
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
    let tab = match header_mode {
        SearchResultsHeaderMode::Tabbed => vm.tab(),
    };
    let header = render_inspector_header(
        InspectorHeader {
            query: vm.query(),
            status_text: vm.status_text(),
            selected: tab,
            tab_select: slots.tab_select.as_ref(),
            action: slots.header_action.as_ref(),
        },
        cx,
    );
    let body = if let Some(empty) = vm.empty_state() {
        render_empty_state(empty, slots.failure_action.as_ref(), cx)
    } else {
        render_active_result_list(tab, vm.is_index_loading(), vm.index_notice(), slots, cx)
    };

    div()
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
        .child(header)
        .child(body)
        .into_any_element()
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

/// The parts of the header of a search page.
#[derive(Clone, Copy)]
struct InspectorHeader<'a> {
    query: &'a str,
    status_text: Option<&'a str>,
    selected: SearchResultsTab,
    tab_select: Option<&'a TabSelectHandler>,
    action: Option<&'a (SearchPageActionDisplay, HeaderActionHandler)>,
}

/// The header at the width of the list column (packet 010 "The view at
/// the width of the list column"): the query as the headline on the first
/// row, with the typed action of packet 012 after it, then the status text,
/// and the tab control on its own row under them. Column text does not
/// call `truncate()` (ADR 0063).
fn render_inspector_header(header: InspectorHeader<'_>, cx: &App) -> AnyElement {
    let mut query_row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::SM.scaled(cx))
        .min_w_0()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .child(Label::new(header.query.to_string()).variant(LabelVariant::Headline)),
        );
    if let Some((display, handler)) = header.action {
        query_row = query_row.child(
            div()
                .flex_shrink_0()
                .child(render_header_action(display, handler)),
        );
    }
    let mut column = div()
        .flex()
        .flex_col()
        .gap(Spacing::XS.scaled(cx))
        .p(Spacing::MD.scaled(cx))
        .child(query_row);
    if let Some(status) = header.status_text {
        column = column.child(
            MultilineText::new(status.to_owned())
                .max_lines(usize::MAX)
                .wrap_lines()
                .size(FontSize::Caption)
                .color(SemanticColor::SecondaryLabel),
        );
    }
    column
        .child(
            div()
                .w_full()
                .min_w_0()
                .child(render_tab_strip(header.selected, header.tab_select)),
        )
        .into_any_element()
}

/// One typed header action. The view model gives the label, the
/// accessibility label, the availability and the style.
fn render_header_action(
    display: &SearchPageActionDisplay,
    handler: &HeaderActionHandler,
) -> AnyElement {
    let style = match display.style {
        SearchPageActionStyle::Plain => ControlStyle::Secondary,
        SearchPageActionStyle::Destructive => ControlStyle::Destructive,
    };
    let button = UiButton::styled(SharedString::from(display.id), style)
        .label(display.label)
        .a11y_label(display.a11y_label.clone())
        .disabled(!display.available);
    if !display.available {
        return button.into_any_element();
    }
    let handler = Rc::clone(handler);
    button
        .on_activate(move |window, cx| handler(window, cx))
        .into_any_element()
}

/// Renders one empty state with no failure action, for example the page
/// of a deleted saved search (packet 012).
pub(crate) fn render_search_empty_state(empty: &EmptyStateDisplay, cx: &App) -> AnyElement {
    render_empty_state(empty, None, cx)
}

fn render_tab_strip(
    selected: SearchResultsTab,
    on_tab_select: Option<&TabSelectHandler>,
) -> SegmentedControl<SearchResultsTab> {
    let mut control = SegmentedControl::new(selected).segments([
        Segment::new(tab_segment_display(SearchResultsTab::Releases)),
        Segment::new(tab_segment_display(SearchResultsTab::Artists)),
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

/// The rows of the active tab. While the `MusicIndex` results load and the
/// tab has no row, three rows wait. `index_notice` is the note of the
/// Index part after the rows, when the view model gives one (walk of
/// 2026-10-21).
fn render_active_result_list(
    tab: SearchResultsTab,
    index_loading: bool,
    index_notice: Option<&EmptyStateDisplay>,
    slots: &SearchResultsInspectorSlots,
    cx: &App,
) -> AnyElement {
    let tab_slug = tab_id(tab);
    let handlers = SearchResultRowHandlers {
        select: slots.result_select.as_ref(),
        toggle: slots.row_toggle.as_ref(),
    };
    let children: Vec<AnyElement> = if index_loading && slots.rows.is_empty() {
        (0..3)
            .map(|index| render_pending_result_row(tab, index))
            .collect()
    } else {
        slots
            .rows
            .iter()
            .map(|row| {
                let leading = if let Some(art) = &row.art {
                    SearchRowLeading::Collection {
                        image: slots
                            .arts
                            .get(&row.id)
                            .cloned()
                            .unwrap_or(CollectionArtImage::EntityColor),
                        kind: art.kind,
                    }
                } else {
                    let href = row.thumbnail_href.as_deref();
                    SearchRowLeading::Thumbnail(
                        href.and_then(|href| slots.thumbnail_for_href(href)),
                    )
                };
                render_result_row(tab, row.clone(), leading, handlers, cx)
            })
            .collect()
    };
    let notice = index_notice.map(|notice| {
        // The note is a child of the scroll container with the rows, so it
        // must not shrink under them (packet 028 review).
        div()
            .flex_shrink_0()
            .pt(Spacing::MD.scaled(cx))
            .child(empty_state_content(
                notice,
                LabelVariant::Caption,
                slots.failure_action.as_ref(),
                cx,
            ))
    });
    // Packet 028 D2: each row is a direct child of the scroll container, in
    // the order of the view model rows. Thus `scroll_to_item` finds the
    // focused row by its index. The note is after the rows. The right
    // padding keeps the clearance of `page_scroll_content` and adds the
    // column inset.
    let container = page_scroll_content(cx)
        .id(SharedString::from(format!(
            "search-results-{tab_slug}-rows"
        )))
        .gap(Spacing::XXS.scaled(cx))
        .pl(Spacing::MD.scaled(cx))
        .pr(OVERLAY_SCROLLBAR_WIDTH + Spacing::SM.scaled(cx) + Spacing::MD.scaled(cx))
        .pb(Spacing::MD.scaled(cx))
        .children(children)
        .children(notice);
    music_column_tracked_scrollbar(container, &slots.scroll, None)
}

fn render_empty_state(
    empty: &EmptyStateDisplay,
    on_failure: Option<&FailureHandler>,
    cx: &App,
) -> AnyElement {
    let content = empty_state_content(empty, LabelVariant::Headline, on_failure, cx);
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

/// The title, the text, the failure actions and the open report of one
/// notice. The empty state and the note of the Index part share it.
fn empty_state_content(
    empty: &EmptyStateDisplay,
    title_variant: LabelVariant,
    on_failure: Option<&FailureHandler>,
    cx: &App,
) -> gpui::Div {
    let mut content = div()
        .w_full()
        .max_w(Size::NoticeWidth.scaled(cx))
        .min_w_0()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .gap(Spacing::MD.scaled(cx))
        .child(Label::new(empty.title.clone()).variant(title_variant))
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
    content
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
            .on_result_select(|_, _, _, _, _, _| {})
            .on_row_toggle(|_, _, _| {});

        assert!(slots.tab_select.is_some());
        assert!(slots.result_select.is_some());
        assert!(slots.row_toggle.is_some());
    }
}
