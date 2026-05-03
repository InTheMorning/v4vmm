//! Shared track detail surface composite.
//!
//! Screens provide a GPUI-free [`TrackDetailVm`], resolved artwork, and typed
//! surface elements for command-bearing slots. This composite owns the common
//! header, summary, description, section, and advanced-panel layout.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{
    div, AnyElement, App, FontWeight, Image, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window,
};

use crate::ui::primitives::MultilineText;
use crate::ui::style::radius;
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::track::TrackHeaderVm;
use crate::view_models::track_detail::{TrackDetailLoadState, TrackDetailSection, TrackDetailVm};

use super::{DetailGrid, DetailRow, DetailTextRow, TrackHeader};

#[derive(IntoElement)]
#[must_use]
pub struct TrackDetailSurface {
    title: String,
    artist: String,
    image: Option<Arc<Image>>,
    load_state: TrackDetailLoadState,
    summary_rows: Vec<crate::view_models::track_detail::TrackDetailSummaryRow>,
    description_label: String,
    description: Option<String>,
    primary_actions: Vec<TrackSurfaceElement>,
    external_links: Vec<TrackSurfaceElement>,
    sections: Vec<TrackDetailSection>,
    section_elements: Vec<TrackSurfaceElement>,
    advanced_panels: Vec<TrackSurfaceElement>,
}

impl TrackDetailSurface {
    pub fn new(vm: &TrackDetailVm<'_>) -> Self {
        Self {
            title: vm.display_title(),
            artist: vm.display_artist(),
            image: None,
            load_state: TrackDetailLoadState::Loaded,
            summary_rows: vm.summary_rows(),
            description_label: vm.labels().description_label().to_string(),
            description: vm.description(),
            primary_actions: Vec::new(),
            external_links: Vec::new(),
            sections: Vec::new(),
            section_elements: Vec::new(),
            advanced_panels: Vec::new(),
        }
    }

    pub fn image(mut self, image: Option<Arc<Image>>) -> Self {
        self.image = image;
        self
    }

    pub fn load_state(mut self, load_state: TrackDetailLoadState) -> Self {
        self.load_state = load_state;
        self
    }

    pub fn primary_actions(mut self, actions: Vec<TrackSurfaceElement>) -> Self {
        self.primary_actions = actions;
        self
    }

    pub fn external_links(mut self, links: Vec<TrackSurfaceElement>) -> Self {
        self.external_links = links;
        self
    }

    pub fn sections(mut self, sections: Vec<TrackDetailSection>) -> Self {
        self.sections = sections;
        self
    }

    pub fn section_elements(mut self, sections: Vec<TrackSurfaceElement>) -> Self {
        self.section_elements = sections;
        self
    }

    pub fn advanced_panels(mut self, panels: Vec<TrackSurfaceElement>) -> Self {
        self.advanced_panels = panels;
        self
    }
}

impl RenderOnce for TrackDetailSurface {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        match self.load_state {
            TrackDetailLoadState::Loaded => render_loaded_surface(self, cx),
            TrackDetailLoadState::Loading => render_surface_state("Loading track...", cx),
            TrackDetailLoadState::Missing => render_surface_state("Track not found", cx),
            TrackDetailLoadState::Failed { reason } => render_surface_state(&reason, cx),
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct TrackSurfaceElement {
    element: AnyElement,
}

impl TrackSurfaceElement {
    pub fn from_element(element: AnyElement) -> Self {
        Self { element }
    }
}

impl RenderOnce for TrackSurfaceElement {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.element
    }
}

fn render_loaded_surface(surface: TrackDetailSurface, cx: &mut App) -> AnyElement {
    let mut stack = div().flex().flex_col().gap(Spacing::LG.scaled(cx)).child(
        TrackHeader::new(TrackHeaderVm {
            title: surface.title,
            artist: surface.artist,
        })
        .image(surface.image),
    );

    if !surface.primary_actions.is_empty() {
        stack = stack.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(Spacing::SM.scaled(cx))
                .children(surface.primary_actions),
        );
    }

    if !surface.external_links.is_empty() {
        stack = stack.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(Spacing::SM.scaled(cx))
                .children(surface.external_links),
        );
    }

    let summary_rows = surface
        .summary_rows
        .into_iter()
        .map(|row| {
            DetailRow::text(DetailTextRow {
                key: row.label.into(),
                value: row.value,
                max_lines: row.max_lines,
            })
        })
        .collect::<Vec<_>>();
    if !summary_rows.is_empty() {
        stack = stack.child(DetailGrid::new(summary_rows));
    }

    if let Some(description) = surface.description {
        stack = stack.child(render_text_section(
            surface.description_label,
            description,
            cx,
        ));
    }

    for section in surface.sections {
        if let Some(empty_label) = section.empty_label {
            stack = stack.child(render_text_section(section.label, empty_label, cx));
        }
    }

    for section in surface.section_elements {
        stack = stack.child(section);
    }

    for panel in surface.advanced_panels {
        stack = stack.child(panel);
    }

    stack.into_any_element()
}

fn render_surface_state(message: &str, cx: &mut App) -> AnyElement {
    div()
        .text_size(FontSize::Micro.scaled(cx))
        .text_color(color(cx, SemanticColor::SecondaryLabel))
        .child(SharedString::from(message.to_string()))
        .into_any_element()
}

fn render_text_section(label: String, value: String, cx: &mut App) -> AnyElement {
    div()
        .border_1()
        .border_color(color(cx, SemanticColor::Separator))
        .rounded(radius::MD)
        .p(Spacing::SM.scaled(cx))
        .child(
            div()
                .text_size(FontSize::Micro.scaled(cx))
                .font_weight(FontWeight::BOLD)
                .text_color(color(cx, SemanticColor::SecondaryLabel))
                .child(SharedString::from(label)),
        )
        .child(
            div()
                .mt(Spacing::XS.scaled(cx))
                .child(MultilineText::new(value).max_lines(3).size(FontSize::Micro)),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::track_detail::{TrackDetailSurfaceContext, TrackDetailVm};
    use crate::views::TrackView;

    #[test]
    fn surface_copies_vm_header_and_summary() {
        let track = TrackView {
            title: Some("Song".to_string()),
            artist: Some("Artist".to_string()),
            album: Some("Album".to_string()),
            ..TrackView::default()
        };
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library);
        let surface = TrackDetailSurface::new(&vm);

        assert_eq!(surface.title, "Song");
        assert_eq!(surface.artist, "Artist");
        assert_eq!(surface.summary_rows[0].label, "Release");
        assert_eq!(surface.summary_rows[0].value, "Album");
    }

    #[test]
    fn surface_load_state_can_be_overridden() {
        let track = TrackView::default();
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover);
        let surface = TrackDetailSurface::new(&vm).load_state(TrackDetailLoadState::Missing);

        assert_eq!(surface.load_state, TrackDetailLoadState::Missing);
    }
}
