//! Slot-based entity-detail shells.
//!
//! Shared GPUI layout lives here; screen modules still own click handlers,
//! popover state, image-cache resolution, and command dispatch.

#![warn(clippy::pedantic)]

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, AnyElement, App, ClickEvent, ClipboardItem, Image, InteractiveElement, IntoElement,
    ParentElement, SharedString, Styled, Window,
};

use crate::ui::composites::{
    identity_action_button, DetailGrid, DetailHeader, DetailHeaderDataRow, DetailHeaderDisplay,
    DetailRow, DetailTextRow, EntityKind, IdentityActionButtonDisplay, IdentityActionKind, ListRow,
    ListRowA11yLabel, ReleaseActionGroupDisplay, ReleaseDetailSurface, ReleaseSurfaceElement,
    ReleaseTrackSectionDisplay, Thumbnail, ThumbnailSize, TrackRow,
};
use crate::ui::primitives::Label;
use crate::ui::style::{color, spacing, typography};
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::entity_detail::{
    ContributorListVm, ContributorPersonVm, ContributorRoleRowVm, ContributorRowVm,
    EntitySurfaceKind, IdentityActionDisplay, IdentityActionDisplayKind, ReleaseDetailPageVm,
    ReleaseHeroVm, ReleasePanelKind, ReleasePanelVm, ReleaseTextPanelDisplay, SharedTrackRowVm,
};

type ReleaseTrackRowClick = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(Default)]
pub struct ContributorRowSlot {
    pub thumbnail: Option<Arc<Image>>,
    pub actions: Vec<ReleaseSurfaceElement>,
}

#[derive(Default)]
pub struct ReleaseTrackRowSlot {
    pub thumbnail: Option<Arc<Image>>,
    pub on_click: Option<ReleaseTrackRowClick>,
    pub actions: Vec<ReleaseSurfaceElement>,
    pub popover: Option<ReleaseSurfaceElement>,
}

impl ReleaseTrackRowSlot {
    #[must_use]
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

#[derive(Default)]
pub struct ReleaseDetailBehaviorSlots {
    pub hero_image: Option<Arc<Image>>,
    pub primary_actions: Vec<ReleaseSurfaceElement>,
    pub identity_actions: Vec<ReleaseSurfaceElement>,
    pub action_overlays: Vec<ReleaseSurfaceElement>,
    pub description_panel: Option<ReleaseSurfaceElement>,
    pub track_rows: Option<Vec<ReleaseSurfaceElement>>,
    pub after_section: Vec<ReleaseSurfaceElement>,
}

#[must_use]
pub fn render_release_detail_shell(
    page: &ReleaseDetailPageVm<'_>,
    slots: ReleaseDetailBehaviorSlots,
) -> AnyElement {
    let mut surface = ReleaseDetailSurface::new(page.detail_scroll_id)
        .scrollable(true)
        .header(ReleaseSurfaceElement::from_element(render_contract_header(
            &page.hero,
            slots.hero_image,
        )))
        .details(ReleaseSurfaceElement::from_element(render_summary_facts(
            &page.summary_facts,
        )));

    if let Some(actions) = render_action_slots(slots.primary_actions, slots.identity_actions) {
        surface = surface.actions(
            actions,
            ReleaseActionGroupDisplay {
                a11y_label: page.actions_a11y_label.into(),
            },
        );
    }

    for overlay in slots.action_overlays {
        surface = surface.panel(overlay);
    }

    let mut description_panel = slots.description_panel;
    for panel in &page.panels {
        if panel.kind == ReleasePanelKind::Description {
            if let Some(description_panel) = description_panel.take() {
                surface = surface.panel(description_panel);
            } else {
                surface = surface.panel(ReleaseSurfaceElement::from_element(render_release_panel(
                    panel,
                )));
            }
        } else {
            surface = surface.panel(ReleaseSurfaceElement::from_element(render_release_panel(
                panel,
            )));
        }
    }

    let rows = slots
        .track_rows
        .unwrap_or_else(|| render_track_rows(page.tracks.rows()));
    if !rows.is_empty() {
        surface = surface.track_section(ReleaseTrackSectionDisplay {
            title: page.tracks.title().into(),
            summary: page.tracks.summary().into(),
            rows,
        });
    }

    for child in slots.after_section {
        surface = surface.after_section(child);
    }

    surface.into_any_element()
}

#[must_use]
pub fn render_feed_identity_actions(page: &ReleaseDetailPageVm<'_>) -> Vec<ReleaseSurfaceElement> {
    page.identity_actions
        .iter()
        .filter_map(|action| {
            let display = action.identity_display(page.identity_action_prefix)?;
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

            Some(ReleaseSurfaceElement::from_element(
                button.into_any_element(),
            ))
        })
        .collect()
}

pub fn render_contributor_panel(
    id: impl Into<SharedString>,
    title: impl Into<SharedString>,
    contributors: ContributorListVm<'_>,
    row_slot: impl FnMut(&ContributorRowVm<'_>) -> ContributorRowSlot,
) -> Option<AnyElement> {
    if contributors.is_empty() {
        return None;
    }

    Some(
        div()
            .id(id.into())
            .flex()
            .flex_col()
            .gap(spacing::SM)
            .child(
                div()
                    .text_size(typography::SIZE_CAPTION)
                    .text_color(color::text_muted())
                    .child(title.into()),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(spacing::XXS)
                    .children(render_contributor_rows(contributors, row_slot)),
            )
            .into_any_element(),
    )
}

pub fn render_release_track_row(
    id: impl Into<SharedString>,
    row: SharedTrackRowVm<'_>,
    slot: ReleaseTrackRowSlot,
) -> ReleaseSurfaceElement {
    let mut track_row = TrackRow::from_shared_track_row(id.into(), row).thumbnail(slot.thumbnail);

    if let Some(on_click) = slot.on_click {
        track_row = track_row.on_click(move |event, window, cx| on_click(event, window, cx));
    }

    for action in slot.actions {
        track_row = track_row.trailing_child(action);
    }

    let row = track_row.into_any_element();
    if let Some(popover) = slot.popover {
        ReleaseSurfaceElement::from_element(
            div()
                .flex()
                .flex_col()
                .gap(spacing::XXS)
                .child(row)
                .child(popover)
                .into_any_element(),
        )
    } else {
        ReleaseSurfaceElement::from_element(row)
    }
}

pub fn render_contributor_rows(
    contributors: ContributorListVm<'_>,
    mut row_slot: impl FnMut(&ContributorRowVm<'_>) -> ContributorRowSlot,
) -> Vec<AnyElement> {
    let mut rows = Vec::new();
    for person in contributors.people() {
        let Some(primary) = person.primary() else {
            continue;
        };
        let slot = row_slot(primary);
        rows.push(render_contributor_person_row(&person, slot));
        for role in person.role_rows() {
            rows.push(render_contributor_role_row(role));
        }
    }
    rows
}

fn render_contributor_person_row(
    person: &ContributorPersonVm<'_>,
    slot: ContributorRowSlot,
) -> AnyElement {
    if person.primary().is_none() {
        return div().into_any_element();
    }
    let display = person.row_display();
    let name = display.name;
    let row_a11y_label = format!("Contributor: {name}");
    let mut detail = div().flex_1().min_w_0().child(
        Label::new(name)
            .size(FontSize::Micro)
            .weight(gpui::FontWeight::MEDIUM)
            .truncated(),
    );

    if let Some(href) = display.href {
        detail = detail.child(
            Label::new(href)
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel)
                .truncated(),
        );
    }

    let mut row = ListRow::compact(SharedString::from(display.id))
        .a11y_label(ListRowA11yLabel {
            label: row_a11y_label.into(),
        })
        .child(Thumbnail::new(EntityKind::Artist, ThumbnailSize::Sm).image(slot.thumbnail))
        .child(detail);

    if !slot.actions.is_empty() {
        row = row.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(spacing::XS)
                .flex_shrink_0()
                .children(slot.actions),
        );
    }

    row.into_any_element()
}

fn render_contributor_role_row(role: ContributorRoleRowVm) -> AnyElement {
    div()
        .id(SharedString::from(role.id))
        .pl(spacing::XL)
        .text_size(typography::SIZE_MICRO)
        .text_color(color::text_muted())
        .child(SharedString::from(role.label))
        .into_any_element()
}

fn render_contract_header(hero: &ReleaseHeroVm<'_>, hero_image: Option<Arc<Image>>) -> AnyElement {
    let display = hero.display();
    DetailHeader::new(DetailHeaderDisplay {
        kind: entity_kind(display.kind),
        title: display.title.into(),
        subtitle: display.subtitle.map(Into::into),
        data_rows: display
            .data_rows
            .into_iter()
            .map(|row| DetailHeaderDataRow {
                label: row.label.into(),
                value: row.value.into(),
                max_lines: row.max_lines,
            })
            .collect(),
    })
    .image(hero_image)
    .into_any_element()
}

fn render_summary_facts(facts: &[crate::view_models::entity_detail::ReleaseFactVm]) -> AnyElement {
    DetailGrid::new(
        facts
            .iter()
            .map(|fact| {
                DetailRow::text(DetailTextRow {
                    key: fact.key.into(),
                    value: fact.value.clone(),
                    max_lines: 6,
                })
            })
            .collect(),
    )
    .into_any_element()
}

fn render_release_panel(panel: &ReleasePanelVm) -> AnyElement {
    match panel.kind {
        ReleasePanelKind::Description => {
            let display = panel
                .text_display()
                .expect("description panels carry text display");
            render_text_panel(display)
        }
        ReleasePanelKind::Identity => DetailGrid::new(
            panel
                .rows
                .iter()
                .map(|row| {
                    DetailRow::text(DetailTextRow {
                        key: row.key.into(),
                        value: row.value.clone(),
                        max_lines: 6,
                    })
                })
                .collect(),
        )
        .into_any_element(),
    }
}

fn render_text_panel(display: ReleaseTextPanelDisplay) -> AnyElement {
    div()
        .border_1()
        .border_color(color::border_subtle())
        .rounded(crate::ui::style::radius::MD)
        .p(spacing::SM)
        .child(
            div()
                .text_size(typography::SIZE_MICRO)
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(color::text_muted())
                .child(display.title),
        )
        .child(
            div().mt(spacing::XS).child(
                crate::ui::primitives::MultilineText::new(display.body)
                    .max_lines(3)
                    .size(FontSize::Micro)
                    .line_height(typography::LINE_DETAIL)
                    .color(SemanticColor::Label),
            ),
        )
        .into_any_element()
}

fn render_action_slots(
    primary_actions: Vec<ReleaseSurfaceElement>,
    identity_actions: Vec<ReleaseSurfaceElement>,
) -> Option<ReleaseSurfaceElement> {
    if primary_actions.is_empty() && identity_actions.is_empty() {
        return None;
    }

    let mut row = div()
        .flex()
        .flex_row()
        .items_start()
        .justify_between()
        .gap(spacing::SM)
        .flex_wrap();
    if !primary_actions.is_empty() {
        row = row.child(
            div()
                .flex()
                .flex_col()
                .gap(spacing::XS)
                .children(primary_actions),
        );
    }
    if !identity_actions.is_empty() {
        row = row.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(spacing::XS)
                .flex_wrap()
                .children(identity_actions),
        );
    }
    Some(ReleaseSurfaceElement::from_element(row.into_any_element()))
}

fn render_track_rows(rows: Vec<SharedTrackRowVm<'_>>) -> Vec<ReleaseSurfaceElement> {
    rows.into_iter()
        .map(|row| {
            render_release_track_row(
                SharedString::from(row.element_id()),
                row,
                ReleaseTrackRowSlot::default(),
            )
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
    use crate::view_models::entity_detail::EntitySurfaceContext;

    #[test]
    fn behavior_slots_start_empty() {
        let slots = ReleaseDetailBehaviorSlots::default();

        assert!(slots.hero_image.is_none());
        assert!(slots.primary_actions.is_empty());
        assert!(slots.identity_actions.is_empty());
        assert!(slots.action_overlays.is_empty());
        assert!(slots.track_rows.is_none());
        assert!(slots.after_section.is_empty());
    }

    #[test]
    fn release_track_row_slot_starts_empty() {
        let slot = ReleaseTrackRowSlot::default();

        assert!(slot.thumbnail.is_none());
        assert!(slot.on_click.is_none());
        assert!(slot.actions.is_empty());
        assert!(slot.popover.is_none());
    }

    #[test]
    fn contributor_panel_skips_empty_lists() {
        let contributors = ContributorListVm::new(&[], EntitySurfaceContext::Discover);

        assert!(
            render_contributor_panel("contributors", "Contributors", contributors, |_| {
                ContributorRowSlot::default()
            },)
            .is_none()
        );
    }

    #[test]
    fn contributor_rows_use_shared_projection_groups() {
        let contributors = [crate::views::ContributorView {
            name: Some("Alice".into()),
            role: Some("vocals".into()),
            group_name: Some("Band".into()),
            href: Some("https://example.test/alice".into()),
            ..Default::default()
        }];
        let rows = render_contributor_rows(
            ContributorListVm::new(&contributors, EntitySurfaceContext::Discover),
            |_| ContributorRowSlot::default(),
        );

        assert_eq!(rows.len(), 2);
    }
}
