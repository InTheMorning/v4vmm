//! Track metadata-grid composite.
//!
//! Owns the shared grid shell for RSS / tag / `MusicBrainz` comparison rows.
//! Screens build the individual cells because their drag/drop and edit
//! callbacks are screen-specific; this composite keeps column headings,
//! spacing, and grid layout consistent.

#![warn(clippy::pedantic)]

use gpui::{
    div, px, AnyElement, App, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window,
};

use crate::ui::tokens::{resolve_color, Appearance, FontSize, SemanticColor, Spacing};
use crate::view_models::track_metadata_grid::{TrackMetadataGridHeading, TrackMetadataGridVm};

#[derive(IntoElement)]
#[must_use]
pub struct TrackMetadataGrid {
    vm: TrackMetadataGridVm,
    cells: Vec<AnyElement>,
    appearance: Option<Appearance>,
}

impl TrackMetadataGrid {
    pub fn new(vm: TrackMetadataGridVm) -> Self {
        Self {
            vm,
            cells: Vec::new(),
            appearance: None,
        }
    }

    pub fn cells(mut self, cells: Vec<AnyElement>) -> Self {
        self.cells = cells;
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for TrackMetadataGrid {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut cells = Vec::with_capacity(self.vm.headings().len() + self.cells.len());
        cells.extend(
            self.vm
                .headings()
                .iter()
                .map(|heading| metadata_heading_cell(heading, self.appearance, cx)),
        );
        cells.extend(self.cells);

        div()
            .grid()
            .grid_cols(self.vm.columns())
            .gap_x(Spacing::XL.scaled(cx))
            .gap_y(Spacing::SM.scaled(cx))
            .children(cells)
    }
}

fn metadata_heading_cell(
    heading: &TrackMetadataGridHeading,
    appearance: Option<Appearance>,
    cx: &mut App,
) -> AnyElement {
    div()
        .pl(px(heading.indent))
        .text_color(resolve_color(cx, SemanticColor::SecondaryLabel, appearance))
        .font_weight(FontWeight::BOLD)
        .text_size(FontSize::Micro.scaled(cx))
        .child(SharedString::from(heading.label.clone()))
        .into_any_element()
}
