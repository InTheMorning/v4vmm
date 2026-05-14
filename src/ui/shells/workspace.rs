//! Workspace-frame shell.
//!
//! ADR 0046 Task 007 keeps existing Library, Search, and Settings screens
//! mounted whole. This shell only arranges workspace frames and delegates frame
//! chrome to the shared `frame_shell` composite.

#![warn(clippy::pedantic)]

use gpui::{div, AnyElement, App, IntoElement, ParentElement, Styled};

use crate::ui::composites::{frame_shell, FrameShellSlots};
use crate::ui::tokens::{resolve_color, FontSize, SemanticColor, Size, Spacing};
use crate::view_models::workspace::{
    FrameNavigationEntry, FrameNavigationState, FrameShellDisplay, WorkspaceFrameKind,
    WorkspaceFrameState, WorkspaceLayout,
};

/// Whole-screen content mounts keyed by transitional workspace frame kind.
#[derive(Default)]
#[must_use]
pub(crate) struct WorkspaceSlots {
    source_list: Option<AnyElement>,
    content_list: Option<AnyElement>,
    detail: Option<AnyElement>,
    queue_now_playing: Option<AnyElement>,
}

#[expect(
    dead_code,
    reason = "ADR 0046 Task 007 defines all frame slots before later tasks fill them"
)]
impl WorkspaceSlots {
    /// Creates an empty slot map.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies content for the source-list frame.
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
    pub(crate) fn detail(mut self, content: impl IntoElement) -> Self {
        self.detail = Some(content.into_any_element());
        self
    }

    /// Supplies content for the queue/now-playing frame.
    pub(crate) fn queue_now_playing(mut self, content: impl IntoElement) -> Self {
        self.queue_now_playing = Some(content.into_any_element());
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
}

/// Renders a workspace layout using shared frame chrome.
pub(crate) fn render_workspace(
    layout: &WorkspaceLayout,
    mut slots: WorkspaceSlots,
    cx: &mut App,
) -> impl IntoElement {
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

    for frame in layout.frames() {
        let frame_kind = frame.kind();
        let content = slots.take(frame.kind(), frame, cx);
        let navigation = FrameNavigationState::new(navigation_entry_for(frame_kind));
        let display = FrameShellDisplay::from_frame(frame, &navigation, false);
        let frame_container = if matches!(frame_kind, WorkspaceFrameKind::QueueNowPlaying) {
            div()
                .flex()
                .flex_col()
                .w(Size::ColumnRegular.scaled(cx))
                .flex_shrink_0()
                .min_w_0()
                .min_h_0()
        } else {
            div().flex().flex_col().flex_1().min_w_0().min_h_0()
        };
        row = row.child(frame_container.child(frame_shell(
            display,
            FrameShellSlots::new().content(content),
        )));
    }

    row
}

fn navigation_entry_for(kind: WorkspaceFrameKind) -> FrameNavigationEntry {
    match kind {
        WorkspaceFrameKind::SourceList => FrameNavigationEntry::SourceList,
        WorkspaceFrameKind::ContentList => FrameNavigationEntry::Search(String::new()),
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
