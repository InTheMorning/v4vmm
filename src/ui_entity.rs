//! Slot-based entity-detail shells.
//!
//! Shared GPUI layout lives here; screen modules still own click handlers,
//! popover state, image-cache resolution, and command dispatch.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{div, AnyElement, Image, IntoElement, ParentElement, SharedString, Styled};

use crate::ui::composites::{
    DetailGrid, DetailHeader, DetailRow, EntityKind, ReleaseDetailSurface, TrackRow,
};
use crate::ui::style::spacing;
use crate::view_models::entity_detail::{EntitySurfaceKind, ReleaseDetailVm, SharedTrackRowVm};

#[derive(Default)]
pub struct TrackRowActionSlot {
    pub actions: Vec<AnyElement>,
}

pub struct TrackSectionSlot {
    pub summary: SharedString,
    pub rows: Vec<AnyElement>,
}

#[derive(Default)]
pub struct ReleaseDetailSlots {
    pub header: Option<AnyElement>,
    pub header_image: Option<Arc<Image>>,
    pub action_row: Option<AnyElement>,
    pub identity_actions: Vec<AnyElement>,
    pub details: Option<AnyElement>,
    pub panels: Vec<AnyElement>,
    pub track_actions: Vec<TrackRowActionSlot>,
    pub track_section: Option<TrackSectionSlot>,
    pub after_section: Vec<AnyElement>,
}

#[must_use]
pub fn render_release_detail_shell(
    id: impl Into<SharedString>,
    projection: &ReleaseDetailVm<'_>,
    slots: ReleaseDetailSlots,
) -> AnyElement {
    let header = slots
        .header
        .unwrap_or_else(|| render_default_header(projection, slots.header_image));
    let details = slots
        .details
        .unwrap_or_else(|| render_default_details(projection));

    let mut surface = ReleaseDetailSurface::new(id)
        .scrollable(true)
        .header(header)
        .details(details);

    if let Some(actions) = render_action_slots(slots.action_row, slots.identity_actions) {
        surface = surface.actions(actions);
    }

    for panel in slots.panels {
        surface = surface.panel(panel);
    }

    if let Some(section) = slots.track_section {
        surface = surface.track_section("Tracks", section.summary, section.rows);
    } else {
        let track_list = projection.track_list();
        let rows = track_list.rows();
        if !rows.is_empty() {
            surface = surface.track_section(
                "Tracks",
                track_list.summary(),
                render_track_rows(rows, slots.track_actions),
            );
        }
    }

    for child in slots.after_section {
        surface = surface.after_section(child);
    }

    surface.into_any_element()
}

fn render_default_header(
    projection: &ReleaseDetailVm<'_>,
    header_image: Option<Arc<Image>>,
) -> AnyElement {
    let header = projection.header();
    let mut header_el =
        DetailHeader::new(entity_kind(header.kind), header.title).image(header_image);
    if let Some(subtitle) = header.subtitle {
        header_el = header_el.subtitle(subtitle);
    }
    header_el.into_any_element()
}

fn render_default_details(projection: &ReleaseDetailVm<'_>) -> AnyElement {
    DetailGrid::new(
        projection
            .detail_rows()
            .into_iter()
            .map(|row| DetailRow::text(row.key, row.value, 6))
            .collect(),
    )
    .into_any_element()
}

fn render_action_slots(
    action_row: Option<AnyElement>,
    identity_actions: Vec<AnyElement>,
) -> Option<AnyElement> {
    if action_row.is_none() && identity_actions.is_empty() {
        return None;
    }

    let mut row = div().flex().flex_row().items_center().gap(spacing::SM);
    if let Some(action_row) = action_row {
        row = row.child(action_row);
    }
    row = row.children(identity_actions);
    Some(row.into_any_element())
}

fn render_track_rows(
    rows: Vec<SharedTrackRowVm<'_>>,
    mut slots: Vec<TrackRowActionSlot>,
) -> Vec<AnyElement> {
    rows.into_iter()
        .enumerate()
        .map(|(index, row)| {
            let mut track_row = TrackRow::new(SharedString::from(format!("entity-track:{index}")))
                .number(row.number_label())
                .title(row.title())
                .duration(row.duration_display());

            if let Some(slot) = slots.get_mut(index) {
                for action in std::mem::take(&mut slot.actions) {
                    track_row = track_row.trailing_child(action);
                }
            }

            track_row.into_any_element()
        })
        .collect()
}

fn entity_kind(kind: EntitySurfaceKind) -> EntityKind {
    match kind {
        EntitySurfaceKind::Artist => EntityKind::Artist,
        EntitySurfaceKind::Feed => EntityKind::Feed,
        EntitySurfaceKind::Track => EntityKind::Track,
        EntitySurfaceKind::Contributor => EntityKind::Generic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_slots_start_empty() {
        let slots = ReleaseDetailSlots::default();

        assert!(slots.header.is_none());
        assert!(slots.header_image.is_none());
        assert!(slots.action_row.is_none());
        assert!(slots.identity_actions.is_empty());
        assert!(slots.details.is_none());
        assert!(slots.panels.is_empty());
        assert!(slots.track_actions.is_empty());
        assert!(slots.track_section.is_none());
        assert!(slots.after_section.is_empty());
    }
}
