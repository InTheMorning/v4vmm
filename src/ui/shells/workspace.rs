//! Workspace-frame shell.
//!
//! ADR 0046 Task 007 keeps existing Library, Search, and Settings screens
//! mounted whole. This shell only arranges workspace frames and delegates frame
//! chrome to the shared `frame_shell` composite.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder, transparent_black, AnyElement, App, IntoElement, ParentElement,
    Pixels, RenderOnce, Styled, Window,
};

use crate::ui::composites::{frame_shell, FrameShellSlots, SplitPane};
use crate::ui::icons::{Icon, IconName, IconSize};
use crate::ui::layouts::{
    CONTENT_PANE_DEFAULT_WIDTH, CONTENT_PANE_MIN_WIDTH, WORKSPACE_QUEUE_COLLAPSE_BREAKPOINT,
    WORKSPACE_SECONDARY_DETAIL_COLLAPSE_BREAKPOINT,
};
use crate::ui::tokens::{resolve_color, FontSize, SemanticColor, Spacing};
use crate::view_models::workspace::{
    BreadcrumbDisplay, ContentFilter, FilterChipStripDisplay, FrameNavigationEntry,
    FrameNavigationState, FrameShellDisplay, WorkspaceFrameKind, WorkspaceFrameState,
    WorkspaceLayout,
};

type WorkspaceFilterSelectHandler = Rc<dyn Fn(ContentFilter, &mut Window, &mut App) + 'static>;
type WorkspaceBreadcrumbSelectHandler =
    Rc<dyn Fn(FrameNavigationEntry, &mut Window, &mut App) + 'static>;
type WorkspaceBreadcrumbLabeler = Rc<dyn Fn(&FrameNavigationEntry) -> String + 'static>;
type WorkspaceBackSelectHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

type WorkspaceContentPaneResizeHandler = Box<dyn Fn(Pixels, &mut Window, &mut App) + 'static>;
type WorkspaceContentPaneResizeStartHandler = Box<dyn Fn(&mut Window, &mut App) + 'static>;
type WorkspaceContentPaneResizeMoveHandler = Box<dyn Fn(f32, &mut Window, &mut App) + 'static>;

/// Whole-screen content mounts keyed by transitional workspace frame kind.
#[derive(Default)]
#[must_use]
pub(crate) struct WorkspaceSlots {
    source_list: Option<AnyElement>,
    content_list: Option<AnyElement>,
    detail: Option<AnyElement>,
    queue_now_playing: Option<AnyElement>,
    content_list_filter_chip_strip: Option<FilterChipStripDisplay>,
    detail_filter_chip_strip: Option<FilterChipStripDisplay>,
    on_content_list_filter_select: Option<WorkspaceFilterSelectHandler>,
    on_detail_filter_select: Option<WorkspaceFilterSelectHandler>,
    on_content_list_breadcrumb_select: Option<WorkspaceBreadcrumbSelectHandler>,
    content_list_breadcrumb_labeler: Option<WorkspaceBreadcrumbLabeler>,
    on_content_list_back_select: Option<WorkspaceBackSelectHandler>,
    content_pane_width: Option<Pixels>,
    on_content_pane_resize_start: Option<WorkspaceContentPaneResizeStartHandler>,
    on_content_pane_resize_move: Option<WorkspaceContentPaneResizeMoveHandler>,
    on_content_pane_resize_end: Option<WorkspaceContentPaneResizeHandler>,
}

impl WorkspaceSlots {
    /// Creates an empty slot map.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies content for the source-list frame.
    #[expect(dead_code, reason = "ADR 0046 Task 008+ wires source-list content")]
    pub(crate) fn source_list(mut self, content: impl IntoElement) -> Self {
        self.source_list = Some(content.into_any_element());
        self
    }

    /// Supplies content for the content-list frame.
    pub(crate) fn content_list(mut self, content: impl IntoElement) -> Self {
        self.content_list = Some(content.into_any_element());
        self
    }

    /// Supplies content for the detail frame.
    #[expect(
        dead_code,
        reason = "Stage 5: Detail frame reserved for future workflows"
    )]
    pub(crate) fn detail(mut self, content: impl IntoElement) -> Self {
        self.detail = Some(content.into_any_element());
        self
    }

    /// Supplies content for the queue/now-playing frame.
    pub(crate) fn queue_now_playing(mut self, content: impl IntoElement) -> Self {
        self.queue_now_playing = Some(content.into_any_element());
        self
    }

    /// Supplies frame-local filter chrome for the content-list frame.
    pub(crate) fn content_list_filter_chip_strip(
        mut self,
        display: FilterChipStripDisplay,
    ) -> Self {
        self.content_list_filter_chip_strip = Some(display);
        self
    }

    /// Supplies frame-local filter chrome for the detail frame.
    #[expect(
        dead_code,
        reason = "Stage 5: Detail frame reserved for future workflows"
    )]
    pub(crate) fn detail_filter_chip_strip(mut self, display: FilterChipStripDisplay) -> Self {
        self.detail_filter_chip_strip = Some(display);
        self
    }

    /// Supplies the content-list filter callback.
    pub(crate) fn on_content_list_filter_select(
        mut self,
        handler: impl Fn(ContentFilter, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_content_list_filter_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the detail-frame filter callback.
    #[expect(
        dead_code,
        reason = "Stage 5: Detail frame reserved for future workflows"
    )]
    pub(crate) fn on_detail_filter_select(
        mut self,
        handler: impl Fn(ContentFilter, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_detail_filter_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the content-list breadcrumb selection callback.
    pub(crate) fn on_content_list_breadcrumb_select(
        mut self,
        handler: impl Fn(FrameNavigationEntry, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_content_list_breadcrumb_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the content-list breadcrumb label projection.
    pub(crate) fn content_list_breadcrumb_labeler(
        mut self,
        labeler: impl Fn(&FrameNavigationEntry) -> String + 'static,
    ) -> Self {
        self.content_list_breadcrumb_labeler = Some(Rc::new(labeler));
        self
    }

    /// Supplies the content-list back button callback.
    pub(crate) fn on_content_list_back_select(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_content_list_back_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the content pane width for the `SplitPane`.
    pub(crate) fn content_pane_width(mut self, width: Pixels) -> Self {
        self.content_pane_width = Some(width);
        self
    }

    /// Supplies the content pane resize-start callback.
    pub(crate) fn on_content_pane_resize_start<F>(mut self, handler: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.on_content_pane_resize_start = Some(Box::new(handler));
        self
    }

    /// Supplies the content pane resize-move callback.
    pub(crate) fn on_content_pane_resize_move<F>(mut self, handler: F) -> Self
    where
        F: Fn(f32, &mut Window, &mut App) + 'static,
    {
        self.on_content_pane_resize_move = Some(Box::new(handler));
        self
    }

    /// Supplies the content pane resize-end callback.
    pub(crate) fn on_content_pane_resize_end<F>(mut self, handler: F) -> Self
    where
        F: Fn(Pixels, &mut Window, &mut App) + 'static,
    {
        self.on_content_pane_resize_end = Some(Box::new(handler));
        self
    }

    fn take(
        &mut self,
        kind: WorkspaceFrameKind,
        frame: &WorkspaceFrameState,
        cx: &App,
    ) -> AnyElement {
        match kind {
            WorkspaceFrameKind::SourceList => self.source_list.take(),
            WorkspaceFrameKind::ContentList => self.content_list.take(),
            WorkspaceFrameKind::Detail => self.detail.take(),
            WorkspaceFrameKind::QueueNowPlaying => self.queue_now_playing.take(),
        }
        .unwrap_or_else(|| placeholder(frame, cx))
    }

    fn filter_chip_strip_for(&self, kind: WorkspaceFrameKind) -> Option<FilterChipStripDisplay> {
        match kind {
            WorkspaceFrameKind::ContentList => self.content_list_filter_chip_strip.clone(),
            WorkspaceFrameKind::Detail => self.detail_filter_chip_strip.clone(),
            WorkspaceFrameKind::SourceList | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }

    fn filter_select_handler_for(
        &self,
        kind: WorkspaceFrameKind,
    ) -> Option<WorkspaceFilterSelectHandler> {
        match kind {
            WorkspaceFrameKind::ContentList => self.on_content_list_filter_select.clone(),
            WorkspaceFrameKind::Detail => self.on_detail_filter_select.clone(),
            WorkspaceFrameKind::SourceList | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }

    fn breadcrumb_select_handler_for(
        &self,
        kind: WorkspaceFrameKind,
    ) -> Option<WorkspaceBreadcrumbSelectHandler> {
        match kind {
            WorkspaceFrameKind::ContentList => self.on_content_list_breadcrumb_select.clone(),
            WorkspaceFrameKind::Detail
            | WorkspaceFrameKind::SourceList
            | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }

    fn breadcrumb_labeler_for(
        &self,
        kind: WorkspaceFrameKind,
    ) -> Option<WorkspaceBreadcrumbLabeler> {
        match kind {
            WorkspaceFrameKind::ContentList => self.content_list_breadcrumb_labeler.clone(),
            WorkspaceFrameKind::Detail
            | WorkspaceFrameKind::SourceList
            | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }

    fn back_select_handler_for(
        &self,
        kind: WorkspaceFrameKind,
    ) -> Option<WorkspaceBackSelectHandler> {
        match kind {
            WorkspaceFrameKind::ContentList => self.on_content_list_back_select.clone(),
            WorkspaceFrameKind::Detail
            | WorkspaceFrameKind::SourceList
            | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }
}

/// Renders a workspace layout using shared frame chrome.
pub(crate) fn render_workspace(
    layout: &WorkspaceLayout,
    slots: WorkspaceSlots,
    _cx: &mut App,
) -> impl IntoElement {
    WorkspaceShell {
        layout: layout.clone(),
        slots,
    }
}

#[derive(IntoElement)]
struct WorkspaceShell {
    layout: WorkspaceLayout,
    slots: WorkspaceSlots,
}

impl RenderOnce for WorkspaceShell {
    #[allow(clippy::too_many_lines)]
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let workspace_width = window.bounds().size.width;
        let focus_color = resolve_color(cx, SemanticColor::Focus, None);
        let mut collapsed_frames = Vec::new();
        let mut content_list_element: Option<AnyElement> = None;
        let mut queue_element: Option<AnyElement> = None;

        for frame in self.layout.frames() {
            let frame_kind = frame.kind();
            if should_collapse_frame(frame_kind, workspace_width) {
                collapsed_frames.push(frame.title().to_owned());
                continue;
            }

            let filter_chip_strip = self.slots.filter_chip_strip_for(frame_kind);
            let on_filter_select = self.slots.filter_select_handler_for(frame_kind);
            let content = self.slots.take(frame.kind(), frame, cx);
            let fallback_navigation = FrameNavigationState::new(navigation_entry_for(frame_kind));
            let navigation = self
                .layout
                .frame_nav(frame.id())
                .unwrap_or(&fallback_navigation);
            let mut display = FrameShellDisplay::from_frame(frame, navigation, false);
            if let Some(filter_chip_strip) = filter_chip_strip {
                display = display.with_filter_chip_strip(filter_chip_strip);
            }
            if should_render_breadcrumb(frame_kind, navigation) {
                let breadcrumb_id = format!("workspace-frame-{}-breadcrumb", frame.id().value());
                let breadcrumb = if let Some(labeler) =
                    self.slots.breadcrumb_labeler_for(frame_kind)
                {
                    BreadcrumbDisplay::project(breadcrumb_id, navigation, |entry| labeler(entry))
                } else {
                    BreadcrumbDisplay::project(
                        breadcrumb_id,
                        navigation,
                        FrameNavigationEntry::display_label,
                    )
                };
                display = display.with_breadcrumb(breadcrumb);
            }
            let mut shell_slots = FrameShellSlots::new().content(content);
            if let Some(handler) = on_filter_select {
                shell_slots = shell_slots.on_filter_select(move |filter, window, cx| {
                    handler(filter, window, cx);
                });
            }
            if let Some(handler) = self.slots.breadcrumb_select_handler_for(frame_kind) {
                shell_slots = shell_slots.on_breadcrumb_select(move |entry, window, cx| {
                    handler(entry, window, cx);
                });
            }
            if let Some(handler) = self.slots.back_select_handler_for(frame_kind) {
                shell_slots = shell_slots.on_back(move |window, cx| {
                    handler(window, cx);
                });
            }

            let frame_element = frame_shell(display, shell_slots);

            let frame_container = div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .border_1()
                .border_color(transparent_black())
                .when(frame.is_focused(), |el| {
                    el.border_2().border_color(focus_color)
                })
                .child(frame_element);

            match frame_kind {
                WorkspaceFrameKind::ContentList => {
                    content_list_element = Some(frame_container.into_any_element());
                }
                WorkspaceFrameKind::QueueNowPlaying => {
                    queue_element = Some(frame_container.into_any_element());
                }
                _ => {}
            }
        }

        let content_pane_width = self
            .slots
            .content_pane_width
            .unwrap_or(CONTENT_PANE_DEFAULT_WIDTH);

        let has_both_frames = content_list_element.is_some() && queue_element.is_some();

        let layout_element: AnyElement = if has_both_frames {
            let content = content_list_element.unwrap();
            let queue = queue_element.unwrap();

            let mut split = SplitPane::new("workspace-split")
                .leading_width(content_pane_width)
                .leading_min_width(CONTENT_PANE_MIN_WIDTH)
                .leading(content)
                .trailing(queue);

            if let Some(handler) = self.slots.on_content_pane_resize_start {
                split = split.on_resize_start(move |_event: &gpui::MouseDownEvent, window, cx| {
                    handler(window, cx);
                });
            }

            if let Some(handler) = self.slots.on_content_pane_resize_move {
                split = split.on_resize_move(move |event: &gpui::MouseMoveEvent, window, cx| {
                    let x = f32::from(event.position.x);
                    handler(x, window, cx);
                });
            }

            if let Some(handler) = self.slots.on_content_pane_resize_end {
                split = split.on_resize_end(move |_event: &gpui::MouseUpEvent, window, cx| {
                    handler(content_pane_width, window, cx);
                });
            }

            div()
                .size_full()
                .flex()
                .flex_col()
                .min_h_0()
                .min_w_0()
                .p(Spacing::SM.scaled(cx))
                .overflow_hidden()
                .child(split)
                .into_any_element()
        } else {
            let mut fallback_row = div()
                .size_full()
                .flex()
                .flex_row()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .gap(Spacing::SM.scaled(cx))
                .p(Spacing::SM.scaled(cx))
                .overflow_hidden();

            if let Some(content) = content_list_element {
                fallback_row = fallback_row.child(content);
            }
            if let Some(queue) = queue_element {
                fallback_row = fallback_row.child(queue);
            }

            fallback_row.into_any_element()
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .min_w_0()
            .child(layout_element)
            .when(!collapsed_frames.is_empty(), |el| {
                el.child(collapsed_frames_hint(&collapsed_frames, cx))
            })
    }
}

fn should_render_breadcrumb(kind: WorkspaceFrameKind, nav: &FrameNavigationState) -> bool {
    let is_detail_search = matches!(kind, WorkspaceFrameKind::Detail)
        && matches!(nav.current(), FrameNavigationEntry::Search(_));
    let is_content_list_with_history =
        matches!(kind, WorkspaceFrameKind::ContentList) && nav.has_history();
    is_detail_search || is_content_list_with_history
}

fn should_collapse_frame(kind: WorkspaceFrameKind, workspace_width: Pixels) -> bool {
    match kind {
        WorkspaceFrameKind::QueueNowPlaying => {
            workspace_width < WORKSPACE_QUEUE_COLLAPSE_BREAKPOINT
        }
        WorkspaceFrameKind::Detail => {
            workspace_width < WORKSPACE_SECONDARY_DETAIL_COLLAPSE_BREAKPOINT
        }
        WorkspaceFrameKind::SourceList | WorkspaceFrameKind::ContentList => false,
    }
}

fn collapsed_frames_hint(frame_titles: &[String], cx: &App) -> AnyElement {
    div()
        .flex()
        .gap(Spacing::XS.scaled(cx))
        .flex_shrink_0()
        .px(Spacing::SM.scaled(cx))
        .pb(Spacing::SM.scaled(cx))
        .text_size(FontSize::Caption.scaled(cx))
        .text_color(resolve_color(cx, SemanticColor::TertiaryLabel, None))
        .child(Icon::new(IconName::ChevronLeft).size(IconSize::Transport))
        .child(format!("Off-screen: {}", frame_titles.join(", ")))
        .into_any_element()
}

fn navigation_entry_for(kind: WorkspaceFrameKind) -> FrameNavigationEntry {
    match kind {
        WorkspaceFrameKind::SourceList | WorkspaceFrameKind::ContentList => {
            FrameNavigationEntry::SourceList
        }
        WorkspaceFrameKind::Detail => FrameNavigationEntry::TrackDetail(0),
        WorkspaceFrameKind::QueueNowPlaying => FrameNavigationEntry::QueueNowPlaying,
    }
}

fn placeholder(frame: &WorkspaceFrameState, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_1()
        .min_h_0()
        .items_center()
        .justify_center()
        .text_size(FontSize::Caption.scaled(cx))
        .text_color(resolve_color(cx, SemanticColor::TertiaryLabel, None))
        .child(frame.title().to_owned())
        .into_any_element()
}
