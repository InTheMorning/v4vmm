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
    CONTENT_PANE_DEFAULT_WIDTH, CONTENT_PANE_MIN_WIDTH, MUSIC_DETAIL_MIN_WIDTH,
    MUSIC_LIST_COLUMN_WIDTH, MUSIC_SIDEBAR_COLLAPSED_WIDTH, MUSIC_SIDEBAR_COLLAPSE_BREAKPOINT,
    MUSIC_SIDEBAR_WIDTH, MUSIC_SINGLE_COLUMN_BREAKPOINT, WORKSPACE_QUEUE_COLLAPSE_BREAKPOINT,
    WORKSPACE_SECONDARY_DETAIL_COLLAPSE_BREAKPOINT,
};
use crate::ui::tokens::{resolve_color, FontSize, SemanticColor, Spacing};
use crate::view_models::workspace::{
    BreadcrumbDisplay, ContentFilter, ContentViewMode, ContentViewModeControlDisplay,
    FilterChipStripDisplay, FrameHeaderActionDisplay, FrameNavigationEntry, FrameNavigationState,
    FrameShellDisplay, MusicColumns, MusicDestination, MusicSidebarPreference, MusicSidebarState,
    TrackKey, WorkspaceFrameKind, WorkspaceFrameState, WorkspaceLayout,
};

type WorkspaceFilterSelectHandler = Rc<dyn Fn(ContentFilter, &mut Window, &mut App) + 'static>;
type WorkspaceViewModeSelectHandler = Rc<dyn Fn(ContentViewMode, &mut Window, &mut App) + 'static>;
type WorkspaceHeaderActionHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type WorkspaceBreadcrumbSelectHandler =
    Rc<dyn Fn(FrameNavigationEntry, &mut Window, &mut App) + 'static>;
type WorkspaceBreadcrumbLabeler = Rc<dyn Fn(&FrameNavigationEntry) -> String + 'static>;
type WorkspaceBackSelectHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type WorkspaceForwardSelectHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

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
    content_list_view_mode_control: Option<ContentViewModeControlDisplay>,
    content_list_header_action: Option<FrameHeaderActionDisplay>,
    detail_header_action: Option<FrameHeaderActionDisplay>,
    content_list_filter_chip_strip: Option<FilterChipStripDisplay>,
    on_content_list_filter_select: Option<WorkspaceFilterSelectHandler>,
    on_content_list_view_mode_select: Option<WorkspaceViewModeSelectHandler>,
    on_content_list_header_action: Option<WorkspaceHeaderActionHandler>,
    on_detail_header_action: Option<WorkspaceHeaderActionHandler>,
    on_content_list_breadcrumb_select: Option<WorkspaceBreadcrumbSelectHandler>,
    content_list_breadcrumb_labeler: Option<WorkspaceBreadcrumbLabeler>,
    on_content_list_back_select: Option<WorkspaceBackSelectHandler>,
    on_content_list_forward_select: Option<WorkspaceForwardSelectHandler>,
    on_detail_breadcrumb_select: Option<WorkspaceBreadcrumbSelectHandler>,
    detail_breadcrumb_labeler: Option<WorkspaceBreadcrumbLabeler>,
    on_detail_back_select: Option<WorkspaceBackSelectHandler>,
    on_detail_forward_select: Option<WorkspaceForwardSelectHandler>,
    content_pane_width: Option<Pixels>,
    on_content_pane_resize_start: Option<WorkspaceContentPaneResizeStartHandler>,
    on_content_pane_resize_move: Option<WorkspaceContentPaneResizeMoveHandler>,
    on_content_pane_resize_end: Option<WorkspaceContentPaneResizeHandler>,
    source_list_sidebar_preference: MusicSidebarPreference,
}

impl WorkspaceSlots {
    /// Creates an empty slot map.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies content for the source-list frame (ADR 0084 Decision 1). The
    /// Music mount is the only caller today.
    pub(crate) fn source_list_slot(mut self, content: impl IntoElement) -> Self {
        self.source_list = Some(content.into_any_element());
        self
    }

    /// Supplies the sidebar collapse preference (ADR 0084 Decision 1). The
    /// shell resolves the sidebar column's width from this preference and
    /// the workspace width, with the same pure function the Library's own
    /// sidebar render uses, so both agree on one collapsed state.
    pub(crate) fn source_list_sidebar_preference(
        mut self,
        preference: MusicSidebarPreference,
    ) -> Self {
        self.source_list_sidebar_preference = preference;
        self
    }

    /// Supplies content for the content-list frame.
    pub(crate) fn content_list(mut self, content: impl IntoElement) -> Self {
        self.content_list = Some(content.into_any_element());
        self
    }

    /// Supplies content for the detail frame (ADR 0084 Decision 1). The
    /// Music mount is the only caller today.
    pub(crate) fn detail_slot(mut self, content: impl IntoElement) -> Self {
        self.detail = Some(content.into_any_element());
        self
    }

    /// Takes the content of the detail frame, so the caller can wrap it
    /// and supply it again (ADR 0084 packet 018: the focus and the action
    /// bar of the Music detail column).
    pub(crate) fn take_detail_slot(&mut self) -> Option<AnyElement> {
        self.detail.take()
    }

    /// Supplies frame-local content view-mode chrome for the content-list frame.
    pub(crate) fn content_list_view_mode_control(
        mut self,
        display: ContentViewModeControlDisplay,
    ) -> Self {
        self.content_list_view_mode_control = Some(display);
        self
    }

    /// Supplies the content-list frame's trailing header action (ADR
    /// 0084 Decision 10 Amendment, 2026-10-19): the Music list column's
    /// "Select" and "Done" button.
    pub(crate) fn content_list_header_action(mut self, display: FrameHeaderActionDisplay) -> Self {
        self.content_list_header_action = Some(display);
        self
    }

    /// Supplies the content-list header action's click callback.
    pub(crate) fn on_content_list_header_action(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_content_list_header_action = Some(Rc::new(handler));
        self
    }

    /// Supplies the detail frame's trailing header action (ADR 0084
    /// packet 018): the Music detail column's "Select" and "Done" button.
    pub(crate) fn detail_header_action(mut self, display: FrameHeaderActionDisplay) -> Self {
        self.detail_header_action = Some(display);
        self
    }

    /// Supplies the detail header action's click callback.
    pub(crate) fn on_detail_header_action(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_detail_header_action = Some(Rc::new(handler));
        self
    }

    /// Supplies frame-local chip filter chrome for the content-list frame.
    pub(crate) fn content_list_filter_chip_strip(
        mut self,
        display: FilterChipStripDisplay,
    ) -> Self {
        self.content_list_filter_chip_strip = Some(display);
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

    /// Supplies the content-list view-mode callback.
    pub(crate) fn on_content_list_view_mode_select(
        mut self,
        handler: impl Fn(ContentViewMode, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_content_list_view_mode_select = Some(Rc::new(handler));
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

    /// Supplies the content-list forward button callback.
    pub(crate) fn on_content_list_forward_select(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_content_list_forward_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the detail frame's breadcrumb selection callback (ADR 0084
    /// Decision 9: the breadcrumb moves to the detail column in Music).
    pub(crate) fn on_detail_breadcrumb_select(
        mut self,
        handler: impl Fn(FrameNavigationEntry, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_detail_breadcrumb_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the detail frame's breadcrumb label projection.
    pub(crate) fn detail_breadcrumb_labeler(
        mut self,
        labeler: impl Fn(&FrameNavigationEntry) -> String + 'static,
    ) -> Self {
        self.detail_breadcrumb_labeler = Some(Rc::new(labeler));
        self
    }

    /// Supplies the detail frame's back button callback.
    pub(crate) fn on_detail_back_select(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_detail_back_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the detail frame's forward button callback.
    pub(crate) fn on_detail_forward_select(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_detail_forward_select = Some(Rc::new(handler));
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
            WorkspaceFrameKind::ContentList => self.content_list.take(),
            // ADR 0084 Decision 1: the Music mount fills the source-list and
            // detail slots.
            WorkspaceFrameKind::SourceList => self.source_list.take(),
            WorkspaceFrameKind::Detail => self.detail.take(),
            // ADR 0081 Decision 2: a slot is added when a screen fills it.
            WorkspaceFrameKind::QueueNowPlaying => None,
        }
        .unwrap_or_else(|| placeholder(frame, cx))
    }

    fn filter_chip_strip_for(&self, kind: WorkspaceFrameKind) -> Option<FilterChipStripDisplay> {
        match kind {
            WorkspaceFrameKind::ContentList => self.content_list_filter_chip_strip.clone(),
            // ADR 0081 Decision 4: a Detail filter strip needs a new ADR.
            WorkspaceFrameKind::SourceList
            | WorkspaceFrameKind::Detail
            | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }

    fn view_mode_control_for(
        &self,
        kind: WorkspaceFrameKind,
    ) -> Option<ContentViewModeControlDisplay> {
        match kind {
            WorkspaceFrameKind::ContentList => self.content_list_view_mode_control.clone(),
            WorkspaceFrameKind::SourceList
            | WorkspaceFrameKind::Detail
            | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }

    fn header_action_for(&self, kind: WorkspaceFrameKind) -> Option<FrameHeaderActionDisplay> {
        match kind {
            WorkspaceFrameKind::ContentList => self.content_list_header_action.clone(),
            WorkspaceFrameKind::Detail => self.detail_header_action.clone(),
            WorkspaceFrameKind::SourceList | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }

    fn header_action_handler_for(
        &self,
        kind: WorkspaceFrameKind,
    ) -> Option<WorkspaceHeaderActionHandler> {
        match kind {
            WorkspaceFrameKind::ContentList => self.on_content_list_header_action.clone(),
            WorkspaceFrameKind::Detail => self.on_detail_header_action.clone(),
            WorkspaceFrameKind::SourceList | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }

    fn filter_select_handler_for(
        &self,
        kind: WorkspaceFrameKind,
    ) -> Option<WorkspaceFilterSelectHandler> {
        match kind {
            WorkspaceFrameKind::ContentList => self.on_content_list_filter_select.clone(),
            WorkspaceFrameKind::SourceList
            | WorkspaceFrameKind::Detail
            | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }

    fn view_mode_select_handler_for(
        &self,
        kind: WorkspaceFrameKind,
    ) -> Option<WorkspaceViewModeSelectHandler> {
        match kind {
            WorkspaceFrameKind::ContentList => self.on_content_list_view_mode_select.clone(),
            WorkspaceFrameKind::SourceList
            | WorkspaceFrameKind::Detail
            | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }

    fn breadcrumb_select_handler_for(
        &self,
        kind: WorkspaceFrameKind,
    ) -> Option<WorkspaceBreadcrumbSelectHandler> {
        match kind {
            WorkspaceFrameKind::ContentList => self.on_content_list_breadcrumb_select.clone(),
            WorkspaceFrameKind::Detail => self.on_detail_breadcrumb_select.clone(),
            WorkspaceFrameKind::SourceList | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }

    fn breadcrumb_labeler_for(
        &self,
        kind: WorkspaceFrameKind,
    ) -> Option<WorkspaceBreadcrumbLabeler> {
        match kind {
            WorkspaceFrameKind::ContentList => self.content_list_breadcrumb_labeler.clone(),
            WorkspaceFrameKind::Detail => self.detail_breadcrumb_labeler.clone(),
            WorkspaceFrameKind::SourceList | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }

    fn back_select_handler_for(
        &self,
        kind: WorkspaceFrameKind,
    ) -> Option<WorkspaceBackSelectHandler> {
        match kind {
            WorkspaceFrameKind::ContentList => self.on_content_list_back_select.clone(),
            WorkspaceFrameKind::Detail => self.on_detail_back_select.clone(),
            WorkspaceFrameKind::SourceList | WorkspaceFrameKind::QueueNowPlaying => None,
        }
    }

    fn forward_select_handler_for(
        &self,
        kind: WorkspaceFrameKind,
    ) -> Option<WorkspaceForwardSelectHandler> {
        match kind {
            WorkspaceFrameKind::ContentList => self.on_content_list_forward_select.clone(),
            WorkspaceFrameKind::Detail => self.on_detail_forward_select.clone(),
            WorkspaceFrameKind::SourceList | WorkspaceFrameKind::QueueNowPlaying => None,
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
        let mut source_list_element: Option<AnyElement> = None;
        let mut detail_element: Option<AnyElement> = None;

        // ADR 0084 Decision 1: a `SourceList` frame exists only in the Music
        // three-column layout. `visible_workspace_layout` gives Settings and
        // Show the content-list frame only.
        let is_music_three_column_layout = self
            .layout
            .frames()
            .iter()
            .any(|frame| frame.kind() == WorkspaceFrameKind::SourceList);
        let content_list_frame_id = self
            .layout
            .frames()
            .iter()
            .find(|frame| frame.kind() == WorkspaceFrameKind::ContentList)
            .map(WorkspaceFrameState::id);
        let content_list_navigation =
            content_list_frame_id.and_then(|id| self.layout.frame_nav(id));
        // ADR 0084 Decision 1: "page open" means the location's current
        // entry is a page, not a destination root.
        let page_open =
            content_list_navigation.is_some_and(|nav| nav.current().opens_detail_page());
        let music_columns = MusicColumns::for_width(
            f32::from(workspace_width),
            f32::from(MUSIC_SINGLE_COLUMN_BREAKPOINT),
            page_open,
        );
        // ADR 0084 Decision 1: the sidebar width comes from its resolved
        // collapse state. The Library's own sidebar render resolves the
        // same state, from the same preference and workspace width, through
        // the same pure function.
        let sidebar_state = crate::view_models::workspace::resolve_music_sidebar_state(
            self.slots.source_list_sidebar_preference,
            f32::from(workspace_width),
            f32::from(MUSIC_SIDEBAR_COLLAPSE_BREAKPOINT),
        );
        let sidebar_width = match sidebar_state {
            MusicSidebarState::Expanded => MUSIC_SIDEBAR_WIDTH,
            MusicSidebarState::Collapsed => MUSIC_SIDEBAR_COLLAPSED_WIDTH,
        };

        for frame in self.layout.frames() {
            let frame_kind = frame.kind();
            if should_collapse_frame(frame_kind, workspace_width, is_music_three_column_layout) {
                collapsed_frames.push(frame.title().to_owned());
                continue;
            }
            if is_music_three_column_layout {
                let hidden_at_this_width = match frame_kind {
                    WorkspaceFrameKind::ContentList => !music_columns.shows_list(),
                    WorkspaceFrameKind::Detail => !music_columns.shows_detail(),
                    WorkspaceFrameKind::SourceList | WorkspaceFrameKind::QueueNowPlaying => false,
                };
                if hidden_at_this_width {
                    continue;
                }
            }

            let filter_chip_strip = self.slots.filter_chip_strip_for(frame_kind);
            let view_mode_control = self.slots.view_mode_control_for(frame_kind);
            let header_action = self.slots.header_action_for(frame_kind);
            let on_filter_select = self.slots.filter_select_handler_for(frame_kind);
            let on_view_mode_select = self.slots.view_mode_select_handler_for(frame_kind);
            let on_header_action = self.slots.header_action_handler_for(frame_kind);
            let content = self.slots.take(frame.kind(), frame, cx);
            let fallback_navigation = FrameNavigationState::new(navigation_entry_for(frame_kind));
            // ADR 0084 Decision 9: in the Music three-column layout, the
            // Detail frame chrome projects the content-list frame's one
            // navigation stack, not a stack of its own.
            let detail_projects_content_list_nav =
                is_music_three_column_layout && matches!(frame_kind, WorkspaceFrameKind::Detail);
            let navigation = if detail_projects_content_list_nav {
                content_list_navigation.unwrap_or(&fallback_navigation)
            } else {
                self.layout
                    .frame_nav(frame.id())
                    .unwrap_or(&fallback_navigation)
            };
            let mut display = FrameShellDisplay::from_frame(frame, navigation, false);
            if detail_projects_content_list_nav {
                // ADR 0084 Decision 9: Back and Forward always show in the
                // Music detail column.
                display = display.with_history_controls_always_shown();
            } else if is_music_three_column_layout
                && matches!(
                    frame_kind,
                    WorkspaceFrameKind::ContentList | WorkspaceFrameKind::SourceList
                )
            {
                // ADR 0084 Decision 1: the Music sidebar and list columns
                // show neither control.
                display = display.with_history_controls_hidden();
            }
            if let Some(filter_chip_strip) = filter_chip_strip {
                display = display.with_filter_chip_strip(filter_chip_strip);
            }
            if let Some(view_mode_control) = view_mode_control {
                display = display.with_view_mode_control(view_mode_control);
            }
            if let Some(header_action) = header_action {
                display = display.with_header_action(header_action);
            }
            if should_render_breadcrumb(frame_kind, navigation, is_music_three_column_layout) {
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
            if let Some(handler) = on_view_mode_select {
                shell_slots = shell_slots.on_view_mode_select(move |view_mode, window, cx| {
                    handler(view_mode, window, cx);
                });
            }
            if let Some(handler) = on_header_action {
                shell_slots = shell_slots.on_header_action(move |window, cx| {
                    handler(window, cx);
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
            if let Some(handler) = self.slots.forward_select_handler_for(frame_kind) {
                shell_slots = shell_slots.on_forward(move |window, cx| {
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
                WorkspaceFrameKind::SourceList => {
                    source_list_element = Some(frame_container.into_any_element());
                }
                WorkspaceFrameKind::Detail => {
                    detail_element = Some(frame_container.into_any_element());
                }
            }
        }

        // ADR 0084 Decision 1: the Music layout is one row: the sidebar at
        // its token width, the list at its token width, and the detail with
        // the remaining width.
        let layout_element: AnyElement = if is_music_three_column_layout {
            let mut row = div()
                .size_full()
                .flex()
                .flex_row()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .gap(Spacing::SM.scaled(cx))
                .p(Spacing::SM.scaled(cx))
                .overflow_hidden();

            if let Some(source_list) = source_list_element {
                row = row.child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_shrink_0()
                        .w(sidebar_width)
                        .h_full()
                        .min_h_0()
                        .child(source_list),
                );
            }
            if let Some(content_list) = content_list_element {
                row = row.child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_shrink_0()
                        .w(MUSIC_LIST_COLUMN_WIDTH)
                        .h_full()
                        .min_h_0()
                        .child(content_list),
                );
            }
            if let Some(detail) = detail_element {
                row = row.child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w(MUSIC_DETAIL_MIN_WIDTH)
                        .h_full()
                        .min_h_0()
                        .child(detail),
                );
            }

            row.into_any_element()
        } else {
            let content_pane_width = self
                .slots
                .content_pane_width
                .unwrap_or(CONTENT_PANE_DEFAULT_WIDTH);

            let has_secondary_frames = queue_element.is_some();

            match content_list_element {
                Some(content) if has_secondary_frames => {
                    let mut split = SplitPane::new("workspace-split")
                        .leading_width(content_pane_width)
                        .leading_min_width(CONTENT_PANE_MIN_WIDTH)
                        .leading(content)
                        .trailing(queue_element.expect("secondary frame exists"));

                    if let Some(handler) = self.slots.on_content_pane_resize_start {
                        split = split.on_resize_start(
                            move |_event: &gpui::MouseDownEvent, window, cx| {
                                handler(window, cx);
                            },
                        );
                    }

                    if let Some(handler) = self.slots.on_content_pane_resize_move {
                        split = split.on_resize_move(
                            move |event: &gpui::MouseMoveEvent, window, cx| {
                                let x = f32::from(event.position.x);
                                handler(x, window, cx);
                            },
                        );
                    }

                    if let Some(handler) = self.slots.on_content_pane_resize_end {
                        split =
                            split.on_resize_end(move |_event: &gpui::MouseUpEvent, window, cx| {
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
                }
                content_list_element => {
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
                }
            }
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

fn should_render_breadcrumb(
    kind: WorkspaceFrameKind,
    nav: &FrameNavigationState,
    music_three_column: bool,
) -> bool {
    if music_three_column {
        // ADR 0084 Decision 1: in Music, only the detail column shows the
        // location breadcrumb. The sidebar and list columns show none.
        return matches!(kind, WorkspaceFrameKind::Detail) && nav.has_location_path();
    }
    let is_detail_search = matches!(kind, WorkspaceFrameKind::Detail)
        && matches!(nav.current(), FrameNavigationEntry::Search(_));
    // ADR 0084 Decision 9: the breadcrumb shows the location path, not the
    // history.
    let is_content_list_with_path =
        matches!(kind, WorkspaceFrameKind::ContentList) && nav.has_location_path();
    is_detail_search || is_content_list_with_path
}

fn should_collapse_frame(
    kind: WorkspaceFrameKind,
    workspace_width: Pixels,
    music_three_column: bool,
) -> bool {
    match kind {
        WorkspaceFrameKind::QueueNowPlaying => {
            workspace_width < WORKSPACE_QUEUE_COLLAPSE_BREAKPOINT
        }
        // ADR 0084 Decision 1: the Detail frame appears only in the Music
        // three-column layout, where `MusicColumns` already decides this
        // frame's visibility at the current width. This width-based rule
        // predates that frame having a screen, so it stays inert there.
        WorkspaceFrameKind::Detail => {
            !music_three_column && workspace_width < WORKSPACE_SECONDARY_DETAIL_COLLAPSE_BREAKPOINT
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
            FrameNavigationEntry::Destination(MusicDestination::Discover)
        }
        WorkspaceFrameKind::Detail => {
            FrameNavigationEntry::Track(TrackKey::from_index_track_target(None, String::new()))
        }
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

#[cfg(test)]
mod tests {
    use super::{should_collapse_frame, should_render_breadcrumb};
    use crate::ui::layouts::{
        WORKSPACE_QUEUE_COLLAPSE_BREAKPOINT, WORKSPACE_SECONDARY_DETAIL_COLLAPSE_BREAKPOINT,
    };
    use crate::view_models::workspace::{
        FrameNavigationEntry, FrameNavigationState, MusicDestination, TrackKey, WorkspaceFrameKind,
    };
    use gpui::px;

    fn track(id: i64) -> FrameNavigationEntry {
        FrameNavigationEntry::Track(TrackKey::from_index_track_target(None, id.to_string()))
    }

    /// C4-04: in the Music three-column layout, the breadcrumb attaches to
    /// the Detail frame, not to the `ContentList` frame, for the same
    /// navigation state.
    #[test]
    fn adr_0084_columns_c4_04_music_breadcrumb_attaches_to_detail_only() {
        let mut nav = FrameNavigationState::new(FrameNavigationEntry::Destination(
            MusicDestination::Discover,
        ));
        nav.push(track(1));

        assert!(should_render_breadcrumb(
            WorkspaceFrameKind::Detail,
            &nav,
            true
        ));
        assert!(!should_render_breadcrumb(
            WorkspaceFrameKind::ContentList,
            &nav,
            true
        ));
        assert!(!should_render_breadcrumb(
            WorkspaceFrameKind::SourceList,
            &nav,
            true
        ));
    }

    /// ADR 0084 Decision 1: the width-based Detail collapse rule stays
    /// inert in the Music three-column layout, where `MusicColumns`
    /// decides this frame's visibility instead.
    #[test]
    fn adr_0084_columns_detail_width_collapse_is_inert_in_music_three_column_layout() {
        let narrow = WORKSPACE_SECONDARY_DETAIL_COLLAPSE_BREAKPOINT - px(1.0);
        assert!(!should_collapse_frame(
            WorkspaceFrameKind::Detail,
            narrow,
            true
        ));
        assert!(should_collapse_frame(
            WorkspaceFrameKind::Detail,
            narrow,
            false
        ));
    }

    /// `should_collapse_frame` keeps the Queue frame's own width-based
    /// collapse, in or out of the Music three-column layout.
    #[test]
    fn should_collapse_frame_keeps_queue_breakpoint_in_both_layouts() {
        let narrow = WORKSPACE_QUEUE_COLLAPSE_BREAKPOINT - px(1.0);
        assert!(should_collapse_frame(
            WorkspaceFrameKind::QueueNowPlaying,
            narrow,
            false
        ));
        assert!(should_collapse_frame(
            WorkspaceFrameKind::QueueNowPlaying,
            narrow,
            true
        ));
    }
}
