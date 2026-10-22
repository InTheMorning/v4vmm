//! Shared release/feed detail surface.
//!
//! ## Display contract: `ReleaseTrackSectionDisplay`
//!
//! Discover feeds and Library albums represent the same user-facing release
//! shape. This composite owns the structural order so modes can provide
//! different actions and panels without drifting into different page skeletons.

#![warn(clippy::pedantic)]

use gpui::{
    div, prelude::*, AnyElement, App, IntoElement, ParentElement, RenderOnce, ScrollHandle,
    SharedString, Window,
};

use crate::ui::composites::music_column_scroll::{
    music_column_scrollbar, music_column_tracked_scrollbar,
};
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};

/// Shared release detail layout: header, actions, details, panels, rows.
#[derive(IntoElement)]
#[must_use]
pub struct ReleaseDetailSurface {
    id: SharedString,
    scrollable: bool,
    /// The held scroll handle of the Music detail column (ADR 0084
    /// Decision 10, packet 018). `None` scrolls with no held handle.
    scroll_handle: Option<ScrollHandle>,
    header: Option<ReleaseSurfaceElement>,
    actions: Option<ReleaseSurfaceElement>,
    actions_a11y_label: Option<SharedString>,
    details: Option<ReleaseSurfaceElement>,
    panels: Vec<ReleaseSurfaceElement>,
    track_section: Option<ReleaseTrackSectionDisplay>,
    after_section: Vec<ReleaseSurfaceElement>,
}

/// Display-ready track section for a release/feed detail surface.
pub struct ReleaseTrackSectionDisplay {
    pub title: SharedString,
    pub summary: SharedString,
    pub rows: Vec<ReleaseSurfaceElement>,
}

pub struct ReleaseActionGroupDisplay {
    pub a11y_label: SharedString,
}

impl ReleaseDetailSurface {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            scrollable: false,
            scroll_handle: None,
            header: None,
            actions: None,
            actions_a11y_label: None,
            details: None,
            panels: Vec::new(),
            track_section: None,
            after_section: Vec::new(),
        }
    }

    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }

    /// Scrolls the surface with `scroll_handle`, so the keys of the detail
    /// column can scroll it (ADR 0084 Decision 10, packet 018).
    pub fn scroll_handle(mut self, scroll_handle: Option<ScrollHandle>) -> Self {
        self.scroll_handle = scroll_handle;
        self
    }

    pub fn header(mut self, header: ReleaseSurfaceElement) -> Self {
        self.header = Some(header);
        self
    }

    pub fn actions(
        mut self,
        actions: ReleaseSurfaceElement,
        display: ReleaseActionGroupDisplay,
    ) -> Self {
        self.actions = Some(actions);
        self.actions_a11y_label = Some(display.a11y_label);
        self
    }

    pub fn details(mut self, details: ReleaseSurfaceElement) -> Self {
        self.details = Some(details);
        self
    }

    pub fn panel(mut self, panel: ReleaseSurfaceElement) -> Self {
        self.panels.push(panel);
        self
    }

    pub fn track_section(mut self, section: ReleaseTrackSectionDisplay) -> Self {
        self.track_section = Some(section);
        self
    }

    pub fn after_section(mut self, child: ReleaseSurfaceElement) -> Self {
        self.after_section.push(child);
        self
    }
}

impl RenderOnce for ReleaseDetailSurface {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let scrollable = self.scrollable;
        let id = self.id;
        let mut body = div().flex().flex_col().gap(Spacing::LG.scaled(cx));

        if let Some(header) = self.header {
            body = body.child(header);
        }

        if let Some(actions) = self.actions {
            std::mem::drop(self.actions_a11y_label);
            body = body.child(actions);
        }

        if let Some(details) = self.details {
            body = body.child(details);
        }

        body = body.children(self.panels);

        if let Some(section) = self.track_section {
            body = body.child(track_section(section, cx));
        }

        body = body.children(self.after_section);

        if scrollable {
            // ADR 0084 Decision 1, amended 2026-10-17: the release page
            // scrolls with the shared gutter scrollbar. With a held scroll
            // handle, the handle holds the offset, and the id gives no
            // scroll position. The detail column sets the offset of each
            // history entry (ADR 0084 Decision 9, amended 2026-10-21).
            // With no handle, the id of each release keeps its own offset.
            let scrollbar_id = SharedString::from(format!("{id}-scrollbar"));
            let container = div()
                .id(id)
                .flex_1()
                .min_h_0()
                .min_w_0()
                .p(Spacing::LG.scaled(cx))
                .child(page_scroll_content(cx).child(body));
            match &self.scroll_handle {
                Some(scroll) => music_column_tracked_scrollbar(container, scroll, None),
                None => music_column_scrollbar(container, scrollbar_id),
            }
        } else {
            body.id(id).into_any_element()
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct ReleaseSurfaceElement {
    element: AnyElement,
}

impl ReleaseSurfaceElement {
    pub fn from_element(element: AnyElement) -> Self {
        Self { element }
    }
}

impl RenderOnce for ReleaseSurfaceElement {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.element
    }
}

fn track_section(section: ReleaseTrackSectionDisplay, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(Spacing::SM.scaled(cx))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child(section.title),
                )
                .child(
                    div()
                        .text_size(FontSize::Micro.scaled(cx))
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child(section.summary),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(Spacing::XXS.scaled(cx))
                .children(section.rows),
        )
        .into_any_element()
}
