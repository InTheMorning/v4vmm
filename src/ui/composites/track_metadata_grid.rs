//! Track metadata-grid composite.
//!
//! Owns the shared grid shell for RSS / tag / `MusicBrainz` comparison rows.
//! Screens build the individual cells because their drag/drop and edit
//! callbacks are screen-specific; this composite keeps column headings,
//! spacing, and grid layout consistent.

#![warn(clippy::pedantic)]

use gpui::{
    div, px, AnyElement, App, ClickEvent, ElementId, FontWeight, IntoElement, ParentElement,
    RenderOnce, Rgba, SharedString, Styled, Window,
};

use super::disclosure_group::{DisclosureGroup, DisclosureGroupDisplay};

use crate::ui::layouts as layout;
use crate::ui::primitives::MultilineText;
use crate::ui::tokens::{resolve_color, Appearance, FontSize, Radius, SemanticColor, Spacing};
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

#[derive(IntoElement)]
#[must_use]
pub struct TrackMetadataGroupCell {
    label: SharedString,
    columns: u16,
    disclosure: Option<AnyElement>,
    appearance: Option<Appearance>,
}

/// Display-ready group-heading fields for a metadata grid row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackMetadataGroupDisplay {
    pub label: SharedString,
    pub columns: u16,
}

impl TrackMetadataGroupCell {
    pub fn new(display: TrackMetadataGroupDisplay) -> Self {
        Self {
            label: display.label,
            columns: display.columns,
            disclosure: None,
            appearance: None,
        }
    }

    pub fn disclosure(mut self, disclosure: impl IntoElement) -> Self {
        self.disclosure = Some(disclosure.into_any_element());
        self
    }

    pub fn disclosure_group(
        mut self,
        id: impl Into<ElementId>,
        collapsed: bool,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.disclosure = Some(
            DisclosureGroup::new(DisclosureGroupDisplay {
                id: id.into(),
                label: self.label.clone(),
                a11y_label: format!("Toggle {} metadata group", self.label).into(),
            })
            .collapsed(collapsed)
            .on_toggle(handler)
            .into_any_element(),
        );
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for TrackMetadataGroupCell {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut cell = div().col_span(self.columns).mt(Spacing::XS.scaled(cx));
        if let Some(disclosure) = self.disclosure {
            cell = cell.child(disclosure);
        } else {
            cell = cell.child(
                div()
                    .text_size(FontSize::Micro.scaled(cx))
                    .font_weight(FontWeight::BOLD)
                    .text_color(resolve_color(
                        cx,
                        SemanticColor::SecondaryLabel,
                        self.appearance,
                    ))
                    .child(self.label),
            );
        }
        cell
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct TrackMetadataFieldCell {
    label: SharedString,
    value: AnyElement,
    appearance: Option<Appearance>,
}

/// Display-ready field-label and value element for a metadata grid cell.
pub struct TrackMetadataFieldDisplay {
    pub label: SharedString,
    pub value: AnyElement,
}

impl std::fmt::Debug for TrackMetadataFieldDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackMetadataFieldDisplay")
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl TrackMetadataFieldCell {
    pub fn new(display: TrackMetadataFieldDisplay) -> Self {
        Self {
            label: display.label,
            value: display.value,
            appearance: None,
        }
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for TrackMetadataFieldCell {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_start()
            .gap(Spacing::SM.scaled(cx))
            .child(
                div()
                    .w(layout::COMPACT_COLUMN_WIDTH)
                    .flex_shrink_0()
                    .text_color(resolve_color(cx, SemanticColor::Label, self.appearance))
                    .text_size(FontSize::Micro.scaled(cx))
                    .child(self.label),
            )
            .child(div().flex_1().min_w_0().child(self.value))
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct TrackMetadataSourceCell {
    value: AnyElement,
    border: Option<Rgba>,
}

impl TrackMetadataSourceCell {
    pub fn new(value: impl IntoElement) -> Self {
        Self {
            value: value.into_any_element(),
            border: None,
        }
    }

    pub fn border_color(mut self, border: Rgba) -> Self {
        self.border = Some(border);
        self
    }
}

impl RenderOnce for TrackMetadataSourceCell {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut cell = div()
            .pl(Spacing::MD.scaled(cx))
            .min_w_0()
            .rounded(Radius::SM.scaled(cx))
            .child(self.value);
        if let Some(border) = self.border {
            cell = cell.border_1().border_color(border);
        }
        cell
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct TrackMetadataTagCell {
    frame_label: Option<SharedString>,
    frame_color: Option<Rgba>,
    value: AnyElement,
    appearance: Option<Appearance>,
}

/// Display-ready tag-frame label and color for a metadata grid tag cell.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackMetadataFrameDisplay {
    pub label: SharedString,
    pub color: Option<Rgba>,
}

/// Display-ready tag value and optional frame metadata.
pub struct TrackMetadataTagDisplay {
    pub value: AnyElement,
    pub frame: Option<TrackMetadataFrameDisplay>,
}

impl std::fmt::Debug for TrackMetadataTagDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackMetadataTagDisplay")
            .field("frame", &self.frame)
            .finish_non_exhaustive()
    }
}

impl TrackMetadataTagCell {
    pub fn new(display: TrackMetadataTagDisplay) -> Self {
        let frame = display.frame;
        Self {
            frame_label: frame.as_ref().map(|frame| frame.label.clone()),
            frame_color: frame.and_then(|frame| frame.color),
            value: display.value,
            appearance: None,
        }
    }

    pub fn frame_color(mut self, color: Rgba) -> Self {
        self.frame_color = Some(color);
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for TrackMetadataTagCell {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_start()
            .gap(Spacing::XS.scaled(cx))
            .child(
                div()
                    .w(layout::METADATA_LABEL_WIDTH)
                    .flex_shrink_0()
                    .text_color(self.frame_color.unwrap_or_else(|| {
                        resolve_color(cx, SemanticColor::SecondaryLabel, self.appearance)
                    }))
                    .text_size(FontSize::Micro.scaled(cx))
                    .child(self.frame_label.unwrap_or_default()),
            )
            .child(div().flex_1().min_w_0().child(self.value))
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct TrackMetadataTextValue {
    value: SharedString,
    color: Option<Rgba>,
    max_lines: usize,
}

/// Display-ready text value for metadata grid body cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackMetadataTextDisplay {
    pub value: SharedString,
}

impl TrackMetadataTextValue {
    pub fn new(display: TrackMetadataTextDisplay) -> Self {
        Self {
            value: display.value,
            color: None,
            max_lines: 4,
        }
    }

    pub fn color_raw(mut self, color: Rgba) -> Self {
        self.color = Some(color);
        self
    }

    pub const fn max_lines(mut self, max_lines: usize) -> Self {
        self.max_lines = max_lines;
        self
    }
}

impl RenderOnce for TrackMetadataTextValue {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut cell = MultilineText::new(self.value.to_string())
            .max_lines(self.max_lines)
            .size(FontSize::Micro);
        if let Some(color) = self.color {
            cell = cell.color_raw(color);
        }
        cell
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_cell_uses_display_contract() {
        let cell = TrackMetadataGroupCell::new(TrackMetadataGroupDisplay {
            label: SharedString::from("ID3 Frames"),
            columns: 3,
        });

        assert_eq!(cell.label, SharedString::from("ID3 Frames"));
        assert_eq!(cell.columns, 3);
    }

    #[test]
    fn group_cell_builds_disclosure_from_display_label() {
        let cell = TrackMetadataGroupCell::new(TrackMetadataGroupDisplay {
            label: SharedString::from("ID3 Frames"),
            columns: 3,
        })
        .disclosure_group("id3-frames", true, |_, _, _| {});

        assert_eq!(cell.label, SharedString::from("ID3 Frames"));
        assert_eq!(cell.columns, 3);
        assert!(cell.disclosure.is_some());
    }

    #[test]
    fn field_cell_uses_display_contract() {
        let cell = TrackMetadataFieldCell::new(TrackMetadataFieldDisplay {
            label: SharedString::from("Title"),
            value: div().into_any_element(),
        });

        assert_eq!(cell.label, SharedString::from("Title"));
    }

    #[test]
    fn tag_cell_uses_display_contract() {
        let color = Rgba {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        let cell = TrackMetadataTagCell::new(TrackMetadataTagDisplay {
            value: div().into_any_element(),
            frame: Some(TrackMetadataFrameDisplay {
                label: SharedString::from("TIT2"),
                color: Some(color),
            }),
        });

        assert_eq!(cell.frame_label, Some(SharedString::from("TIT2")));
        assert_eq!(cell.frame_color, Some(color));
    }

    #[test]
    fn text_value_uses_display_contract() {
        let cell = TrackMetadataTextValue::new(TrackMetadataTextDisplay {
            value: SharedString::from("Album title"),
        });

        assert_eq!(cell.value, SharedString::from("Album title"));
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
