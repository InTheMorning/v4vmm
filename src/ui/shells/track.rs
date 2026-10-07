#![warn(clippy::pedantic)]
//! Shared track-detail identity and surface assembly.
//!
//! Thin screen-level glue: renders a track page's identity actions and feed
//! identity section, then assembles the shared `TrackDetailSurface` from
//! typed behavior slots. The Library track page and the Index track page
//! both call into this module. All layout lives inside shared composites;
//! this module only wires callbacks.

use std::rc::Rc;
use std::sync::Arc;

use gpui::{prelude::*, App, ClipboardItem, Image, SharedString, Window};

use crate::ui::composites::{
    identity_action_button, render_feed_identity_panel, IdentityActionButtonDisplay,
    IdentityActionKind, TrackDetailSurface, TrackSurfaceElement,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button;
use crate::view_models::entity_detail::{
    EntityActionVm, IdentityActionDisplay, IdentityActionDisplayKind,
};
use crate::view_models::track_detail::{
    TrackDetailLoadState, TrackDetailPageVm, TrackDetailSection, TrackNameLinkTarget,
    TrackNameLinkVm,
};

#[derive(Default)]
pub(crate) struct TrackDetailBehaviorSlots {
    pub hero_image: Option<Arc<Image>>,
    pub load_state: Option<TrackDetailLoadState>,
    pub name_links: Vec<TrackSurfaceElement>,
    pub primary_actions: Vec<TrackSurfaceElement>,
    pub external_links: Vec<TrackSurfaceElement>,
    pub description_panel: Option<TrackSurfaceElement>,
    pub sections: Vec<TrackDetailSection>,
    pub section_elements: Vec<TrackSurfaceElement>,
    pub advanced_panels: Vec<TrackSurfaceElement>,
}

#[must_use]
pub(crate) fn render_track_page_identity_actions(
    page: &TrackDetailPageVm<'_>,
) -> Vec<TrackSurfaceElement> {
    render_track_identity_actions(page.identity_actions(), page.identity_action_prefix())
}

/// Renders the feed identity section of a track page, apart from the
/// track's own header (ADR 0075 Decision B, packet 022). `None` when the
/// page has no feed identity to show. The box and its scaled tokens live in
/// the shared `render_feed_identity_panel` composite; this function only
/// composes the view model's data into it.
#[must_use]
pub(crate) fn render_track_feed_identity_section(
    page: &TrackDetailPageVm<'_>,
    cx: &mut App,
) -> Option<TrackSurfaceElement> {
    let section = page.feed_identity_section()?;
    let actions =
        render_track_identity_actions(section.actions, page.feed_identity_action_prefix());
    if actions.is_empty() {
        return None;
    }
    Some(TrackSurfaceElement::from_element(
        render_feed_identity_panel(section.owner_label, actions, cx),
    ))
}

/// Renders the name links under the track title (ADR 0083 Decision 5).
/// `on_open` runs the navigation of the origin: a Library page and an Index
/// page open different pages for the same target kind.
#[must_use]
pub(crate) fn render_track_name_links(
    page: &TrackDetailPageVm<'_>,
    on_open: impl Fn(&TrackNameLinkTarget, &mut Window, &mut App) + 'static,
) -> Vec<TrackSurfaceElement> {
    let on_open = Rc::new(on_open);
    page.detail()
        .name_links()
        .into_iter()
        .enumerate()
        .map(|(index, link)| {
            let on_open = Rc::clone(&on_open);
            let TrackNameLinkVm {
                label,
                a11y_label,
                target,
            } = link;
            let button = Button::styled(
                SharedString::from(format!("track-name-link-{index}")),
                ControlStyle::Ghost,
            )
            .label(label)
            .a11y_label(a11y_label)
            .on_click(move |_, window, cx| on_open(&target, window, cx));
            TrackSurfaceElement::from_element(button.into_any_element())
        })
        .collect()
}

fn render_track_identity_actions(
    actions: Vec<EntityActionVm>,
    identity_action_prefix: &str,
) -> Vec<TrackSurfaceElement> {
    actions
        .into_iter()
        .filter_map(|action| {
            let display = action.identity_display(identity_action_prefix)?;
            let IdentityActionDisplay {
                id,
                kind,
                payload,
                a11y_label,
            } = display;
            let kind = match kind {
                IdentityActionDisplayKind::Website => IdentityActionKind::Website,
                IdentityActionDisplayKind::Nostr => IdentityActionKind::Nostr,
                IdentityActionDisplayKind::Rss => IdentityActionKind::Rss,
            };
            let payload_for_click = payload;
            let button = identity_action_button(IdentityActionButtonDisplay {
                id: SharedString::from(id),
                kind,
                a11y_label: SharedString::from(a11y_label),
            })
            .on_click(move |_, _, cx| match kind {
                IdentityActionKind::Website | IdentityActionKind::Rss => {
                    let _ = open::that(&payload_for_click);
                }
                IdentityActionKind::Nostr => {
                    cx.write_to_clipboard(ClipboardItem::new_string(payload_for_click.clone()));
                }
            });

            Some(TrackSurfaceElement::from_element(button.into_any_element()))
        })
        .collect()
}

pub(crate) fn build_track_detail_surface(
    page: &TrackDetailPageVm<'_>,
    slots: TrackDetailBehaviorSlots,
) -> TrackDetailSurface {
    let mut surface = TrackDetailSurface::new(page.detail()).image(slots.hero_image);

    if let Some(load_state) = slots.load_state {
        surface = surface.load_state(load_state);
    }

    if !slots.name_links.is_empty() {
        surface = surface.name_links(slots.name_links);
    }

    if !slots.primary_actions.is_empty() {
        surface = surface.primary_actions(slots.primary_actions);
    }

    if !slots.external_links.is_empty() {
        surface = surface.external_links(slots.external_links);
    }

    if let Some(description_panel) = slots.description_panel {
        surface = surface.description_panel(description_panel);
    }

    if !slots.sections.is_empty() {
        surface = surface.sections(slots.sections);
    }

    if !slots.section_elements.is_empty() {
        surface = surface.section_elements(slots.section_elements);
    }

    if !slots.advanced_panels.is_empty() {
        surface = surface.advanced_panels(slots.advanced_panels);
    }

    surface
}
