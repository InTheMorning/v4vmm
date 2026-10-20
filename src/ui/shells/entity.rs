//! Slot-based entity-detail shells.
//!
//! Shared GPUI layout lives here; screen modules still own click handlers,
//! popover state, image-cache resolution, and command dispatch.

#![warn(clippy::pedantic)]

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, AnyElement, App, ClipboardItem, Image, InteractiveElement, IntoElement, ParentElement,
    ScrollHandle, SharedString, Styled, Window,
};

use crate::media::cover_color::CoverColor;
use crate::ui::composites::{
    identity_action_button, CoverBackdrop, DetailGrid, DetailHeader, DetailHeaderDataRow,
    DetailHeaderDisplay, DetailRow, DetailTextRow, EntityKind, IdentityActionButtonDisplay,
    IdentityActionKind, ListRow, ListRowA11yLabel, ReleaseActionGroupDisplay, ReleaseDetailSurface,
    ReleaseSurfaceElement, ReleaseTrackSectionDisplay, Thumbnail, ThumbnailSize,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button, Label, LabelVariant};
use crate::ui::style::{color, spacing, typography};
use crate::ui::tokens::{FontSize, SemanticColor, Spacing};
use crate::view_models::album_page::{AlbumNameLinkTarget, AlbumNameLinkVm};
use crate::view_models::entity_detail::{
    ContributorListVm, ContributorPersonVm, ContributorRoleRowVm, ContributorRowVm,
    EntitySurfaceKind, IdentityActionDisplay, IdentityActionDisplayKind, NamedFeedRowVm,
    ReleaseDetailPageVm, ReleaseHeroVm, ReleasePanelKind, ReleasePanelVm, ReleaseTextPanelDisplay,
};

#[derive(Default)]
pub struct ContributorRowSlot {
    pub thumbnail: Option<Arc<Image>>,
    pub actions: Vec<ReleaseSurfaceElement>,
}

/// The cover size of the album page header (ADR 0083 Decision 4).
pub const RELEASE_HEADER_COVER: ThumbnailSize = ThumbnailSize::XXl;

#[derive(Default)]
pub struct ReleaseDetailBehaviorSlots {
    pub hero_image: Option<Arc<Image>>,
    /// The main color of the cover. The screen resolves it like the cover.
    pub cover_color: Option<CoverColor>,
    /// The name links under the album title.
    pub name_links: Vec<AnyElement>,
    /// The named feed rows of ADR 0085 Decisions 5 and 6, each other than
    /// the artist feed. `None` when the release has none.
    pub named_feed_rows: Option<ReleaseSurfaceElement>,
    pub primary_actions: Vec<ReleaseSurfaceElement>,
    pub identity_actions: Vec<ReleaseSurfaceElement>,
    pub action_overlays: Vec<ReleaseSurfaceElement>,
    pub description_panel: Option<ReleaseSurfaceElement>,
    pub track_rows: Option<Vec<ReleaseSurfaceElement>>,
    pub after_section: Vec<ReleaseSurfaceElement>,
    /// The held scroll handle of the Music detail column (ADR 0084
    /// Decision 10, packet 018).
    pub scroll_handle: Option<ScrollHandle>,
}

#[must_use]
pub fn render_release_detail_shell(
    page: &ReleaseDetailPageVm<'_>,
    slots: ReleaseDetailBehaviorSlots,
) -> AnyElement {
    let mut surface = ReleaseDetailSurface::new(page.detail_scroll_key.clone())
        .scrollable(true)
        .scroll_handle(slots.scroll_handle)
        .header(ReleaseSurfaceElement::from_element(
            CoverBackdrop::new(
                slots.cover_color,
                render_contract_header(&page.hero, slots.hero_image, slots.name_links),
            )
            .into_any_element(),
        ))
        .details(ReleaseSurfaceElement::from_element(render_summary_facts(
            &page.summary_facts,
        )));

    if let Some(named_feed_rows) = slots.named_feed_rows {
        surface = surface.panel(named_feed_rows);
    }

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

    let rows = slots.track_rows.unwrap_or_default();
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

fn render_contract_header(
    hero: &ReleaseHeroVm<'_>,
    hero_image: Option<Arc<Image>>,
    name_links: Vec<AnyElement>,
) -> AnyElement {
    let display = hero.display();
    DetailHeader::new(DetailHeaderDisplay {
        kind: entity_kind(display.kind),
        title: display.title.into(),
        subtitle: None,
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
    // ADR 0083 Decision 3: this header is always the album (release) page.
    .title_size(FontSize::Display)
    .cover_size(RELEASE_HEADER_COVER)
    .name_links(name_links)
    .into_any_element()
}

/// The name links under an album title (ADR 0083 task 005). The screen
/// gives the command that opens each target.
pub fn render_album_name_links(
    links: Vec<AlbumNameLinkVm>,
    on_open: impl Fn(&AlbumNameLinkTarget, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Vec<AnyElement> {
    let on_open = Rc::new(on_open);
    links
        .into_iter()
        .enumerate()
        .map(|(index, link)| {
            let on_open = Rc::clone(&on_open);
            let AlbumNameLinkVm {
                label,
                a11y_label,
                note,
                target,
            } = link;
            let button = Button::styled(
                SharedString::from(format!("album-name-link-{index}")),
                ControlStyle::Ghost,
            )
            .label(label)
            .a11y_label(a11y_label)
            .on_click(move |_, window, cx| on_open(&target, window, cx));
            let Some(note) = note else {
                return button.into_any_element();
            };
            // ADR 0085 Decision 6: "Unverified artist" reads as text, not
            // through color alone.
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(Spacing::XS.scaled(cx))
                .child(button)
                .child(
                    Label::new(note)
                        .variant(LabelVariant::Caption)
                        .color(SemanticColor::TertiaryLabel),
                )
                .into_any_element()
        })
        .collect()
}

/// Renders the named feed rows of a release page (ADR 0085 Decisions 5 and
/// 6), each other than the artist feed. `None` when `rows` is empty: the
/// panel is then absent.
#[must_use]
pub fn render_named_feed_rows(
    rows: Vec<NamedFeedRowVm>,
    on_open: impl Fn(&AlbumNameLinkTarget, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Option<ReleaseSurfaceElement> {
    if rows.is_empty() {
        return None;
    }
    let on_open = Rc::new(on_open);
    let children = rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let on_open = Rc::clone(&on_open);
            let text = row.display_text();
            let NamedFeedRowVm {
                detail,
                a11y_label,
                target,
                ..
            } = row;
            let mut text_block = div()
                .flex()
                .flex_col()
                .min_w_0()
                .gap(Spacing::XXS.scaled(cx))
                .child(Label::new(text).variant(LabelVariant::Body));
            if let Some(detail) = detail {
                text_block = text_block.child(
                    Label::new(detail)
                        .variant(LabelVariant::Caption)
                        .color(SemanticColor::TertiaryLabel),
                );
            }
            ListRow::new(SharedString::from(format!("named-feed-row-{index}")))
                .a11y_label(ListRowA11yLabel {
                    label: a11y_label.into(),
                })
                .on_click(move |_event, window, cx| on_open(&target, window, cx))
                .child(div().flex_1().min_w_0().child(text_block))
                .into_any_element()
        })
        .collect::<Vec<_>>();
    Some(ReleaseSurfaceElement::from_element(
        div()
            .flex()
            .flex_col()
            .gap(Spacing::XXS.scaled(cx))
            .children(children)
            .into_any_element(),
    ))
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
                    .max_lines(usize::MAX)
                    .wrap_lines()
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

fn entity_kind(kind: EntitySurfaceKind) -> EntityKind {
    match kind {
        EntitySurfaceKind::Artist => EntityKind::Artist,
        EntitySurfaceKind::Release => EntityKind::Release,
        EntitySurfaceKind::Track => EntityKind::Track,
        EntitySurfaceKind::Contributor => EntityKind::Generic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::entity_detail::EntitySurfaceContext;

    /// F6 of the walk of 2026-10-20 (ADR 0084 Decision 5): the release
    /// page header shows the tag "release", not "feed", for each origin.
    #[test]
    fn adr_0084_walk_fix_release_page_header_kind_label_is_release() {
        let feed = crate::views::FeedView::default();
        for context in [
            EntitySurfaceContext::Library,
            EntitySurfaceContext::Discover,
        ] {
            let hero = crate::view_models::entity_detail::ReleaseDetailVm::new(&feed, context)
                .hero()
                .display();
            assert_eq!(entity_kind(hero.kind).label(), "release", "{context:?}");
        }
    }

    #[test]
    fn behavior_slots_start_empty() {
        let slots = ReleaseDetailBehaviorSlots::default();

        assert!(slots.hero_image.is_none());
        assert!(slots.cover_color.is_none());
        assert!(slots.name_links.is_empty());
        assert!(slots.primary_actions.is_empty());
        assert!(slots.identity_actions.is_empty());
        assert!(slots.action_overlays.is_empty());
        assert!(slots.track_rows.is_none());
        assert!(slots.after_section.is_empty());
    }

    /// R83-55: the album page header cover is the 200 px `XXl` size, which
    /// draws the artwork shadow.
    #[test]
    fn adr_0083_album_header_cover_is_xxl() {
        assert_eq!(RELEASE_HEADER_COVER, ThumbnailSize::XXl);
        assert_eq!(
            RELEASE_HEADER_COVER.image_size(),
            crate::ui::primitives::ImageSize::XXl
        );
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
