//! Shared release/feed detail surface.
//!
//! Discover feeds and Library albums represent the same user-facing release
//! shape. This composite owns the structural order so modes can provide
//! different actions and panels without drifting into different page skeletons.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, IntoElement, ParentElement, RenderOnce, SharedString};

use crate::ui::style::{color, spacing, typography};

/// Shared release detail layout: header, actions, details, panels, rows.
#[derive(IntoElement)]
#[must_use]
pub struct ReleaseDetailSurface {
    id: SharedString,
    scrollable: bool,
    header: Option<AnyElement>,
    actions: Option<AnyElement>,
    details: Option<AnyElement>,
    panels: Vec<AnyElement>,
    section_title: Option<SharedString>,
    section_summary: Option<SharedString>,
    section_rows: Vec<AnyElement>,
    after_section: Vec<AnyElement>,
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

    pub fn header(mut self, header: AnyElement) -> Self {
        self.header = Some(header);
        self
    }

    pub fn actions(mut self, actions: AnyElement) -> Self {
        self.actions = Some(actions);
        self
    }

    pub fn details(mut self, details: AnyElement) -> Self {
        self.details = Some(details);
        self
    }

    pub fn panel(mut self, panel: AnyElement) -> Self {
        self.panels.push(panel);
        self
    }

    pub fn track_section(
        mut self,
        title: impl Into<SharedString>,
        summary: impl Into<SharedString>,
        rows: Vec<AnyElement>,
    ) -> Self {
        self.section_title = Some(title.into());
        self.section_summary = Some(summary.into());
        self.section_rows = rows;
        self
    }

    pub fn after_section(mut self, child: AnyElement) -> Self {
        self.after_section.push(child);
        self
    }
}

impl RenderOnce for ReleaseDetailSurface {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let mut root =
            div()
                .id(self.id)
                .flex()
                .flex_col()
                .gap(spacing::LG)
                .when(self.scrollable, |el| {
                    el.flex_1()
                        .min_h_0()
                        .min_w_0()
                        .overflow_y_scroll()
                        .p(spacing::LG)
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
            ));
        }

        root.children(self.after_section)
    }
}

fn track_section(title: SharedString, summary: SharedString, rows: Vec<AnyElement>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(spacing::SM)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(typography::SIZE_CAPTION)
                        .text_color(color::text_muted())
                        .child(title),
                )
                .child(
                    div()
                        .text_size(typography::SIZE_MICRO)
                        .text_color(color::text_muted())
                        .child(summary),
                ),
        )
        .child(div().flex().flex_col().gap(spacing::XXS).children(rows))
        .into_any_element()
}
