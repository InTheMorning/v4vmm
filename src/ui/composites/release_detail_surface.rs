//! Shared release/feed detail surface.
//!
//! Discover feeds and Library albums represent the same user-facing release
//! shape. This composite owns the structural order so modes can provide
//! different actions and panels without drifting into different page skeletons.

#![warn(clippy::pedantic)]

use gpui::{
    div, prelude::*, AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Window,
};

use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};

/// Shared release detail layout: header, actions, details, panels, rows.
#[derive(IntoElement)]
#[must_use]
pub struct ReleaseDetailSurface {
    id: SharedString,
    scrollable: bool,
    header: Option<ReleaseSurfaceElement>,
    actions: Option<ReleaseSurfaceElement>,
    details: Option<ReleaseSurfaceElement>,
    panels: Vec<ReleaseSurfaceElement>,
    section_title: Option<SharedString>,
    section_summary: Option<SharedString>,
    section_rows: Vec<ReleaseSurfaceElement>,
    after_section: Vec<ReleaseSurfaceElement>,
}

impl ReleaseDetailSurface {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            scrollable: false,
            header: None,
            actions: None,
            details: None,
            panels: Vec::new(),
            section_title: None,
            section_summary: None,
            section_rows: Vec::new(),
            after_section: Vec::new(),
        }
    }

    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }

    pub fn header(mut self, header: ReleaseSurfaceElement) -> Self {
        self.header = Some(header);
        self
    }

    pub fn actions(mut self, actions: ReleaseSurfaceElement) -> Self {
        self.actions = Some(actions);
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

    pub fn track_section(
        mut self,
        title: impl Into<SharedString>,
        summary: impl Into<SharedString>,
        rows: Vec<ReleaseSurfaceElement>,
    ) -> Self {
        self.section_title = Some(title.into());
        self.section_summary = Some(summary.into());
        self.section_rows = rows;
        self
    }

    pub fn after_section(mut self, child: ReleaseSurfaceElement) -> Self {
        self.after_section.push(child);
        self
    }
}

impl RenderOnce for ReleaseDetailSurface {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let mut root = div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(Spacing::LG.scaled(cx))
            .when(self.scrollable, |el| {
                el.flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_y_scroll()
                    .p(Spacing::LG.scaled(cx))
            });

        if let Some(header) = self.header {
            root = root.child(header);
        }

        if let Some(actions) = self.actions {
            root = root.child(actions);
        }

        if let Some(details) = self.details {
            root = root.child(details);
        }

        root = root.children(self.panels);

        if self.section_title.is_some() || !self.section_rows.is_empty() {
            root = root.child(track_section(
                self.section_title.unwrap_or_else(|| "Tracks".into()),
                self.section_summary.unwrap_or_else(|| "".into()),
                self.section_rows,
                cx,
            ));
        }

        root.children(self.after_section)
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

fn track_section(
    title: SharedString,
    summary: SharedString,
    rows: Vec<ReleaseSurfaceElement>,
    cx: &App,
) -> AnyElement {
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
                        .child(title),
                )
                .child(
                    div()
                        .text_size(FontSize::Micro.scaled(cx))
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child(summary),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(Spacing::XXS.scaled(cx))
                .children(rows),
        )
        .into_any_element()
}
