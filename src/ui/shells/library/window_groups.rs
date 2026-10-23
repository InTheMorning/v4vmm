//! The window group tile and the window group row of New releases (ADR
//! 0086 Decisions 2 and 3).
//!
//! The view model gives each text, the state and the art choice
//! ([`WindowGroupTileVm`]). The app layer loads the art. This shell only
//! composes the tile and the row with the size, the colors and the text
//! places of a release tile and a release row.

#![warn(clippy::pedantic)]

use gpui::{
    div, prelude::FluentBuilder, AnyElement, App, ClickEvent, Context, FontWeight, IntoElement,
    ParentElement, SharedString, Styled,
};

use crate::library::LibraryApp;
use crate::ui::composites::collection_art::{CollectionArt, CollectionArtImage};
use crate::ui::composites::selectable_tile::SelectableTile;
use crate::ui::composites::{
    EntityKind, ListRow, ListRowA11yLabel, TagBadge, TagBadgeDisplay, ThumbnailSize,
};
use crate::ui::primitives::Label;
use crate::ui::shells::library::destination_lists::RowArts;
use crate::ui::tokens::{FontSize, SemanticColor, Size, Spacing};
use crate::view_models::collection_art::CollectionKind;
use crate::view_models::library::ContentListSelectionTarget;
use crate::view_models::window_groups::{WindowGroupStateVm, WindowGroupTileVm};

/// The loaded art of a group, by its element id. A group with no loaded
/// state shows the entity color (ADR 0084 Decision 12).
pub(crate) fn group_art(arts: &RowArts, tile: &WindowGroupTileVm) -> CollectionArtImage {
    arts.get(&tile.element_id)
        .cloned()
        .unwrap_or(CollectionArtImage::EntityColor)
}

/// Renders one group tile of New releases (ADR 0086 Decisions 2 and 3).
/// It has the size, the colors and the text places of a release tile. A
/// click opens the release page of the anchor single (packet 002 decision
/// D8). The tile shows no checkbox (decision D9).
pub(crate) fn render_window_group_tile(
    tile: &WindowGroupTileVm,
    art: CollectionArtImage,
    (target, probe): (ContentListSelectionTarget, Option<AnyElement>),
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let open_row_id = tile.open_row_id.clone();
    let artwork_size = Size::ContentTileArtwork.scaled(cx);
    let artwork =
        CollectionArt::new(art, CollectionKind::SinglesGroup, ThumbnailSize::Xl).into_any_element();

    SelectableTile::new(SharedString::from(tile.element_id.clone()), artwork)
        .checkbox(None)
        .selected(tile.selection.is_marked())
        .focused(tile.selection.focused)
        .scroll_probe(probe)
        .child(
            div().w(artwork_size).min_w_0().child(
                Label::new(tile.title.clone())
                    .size(FontSize::Caption)
                    .weight(FontWeight::MEDIUM)
                    .truncated(),
            ),
        )
        .child(
            div().w(artwork_size).min_w_0().child(
                Label::new(tile.date_span.clone())
                    .size(FontSize::Micro)
                    .color(SemanticColor::TertiaryLabel)
                    .truncated(),
            ),
        )
        .child(render_window_group_badges(tile, BadgePlace::First, cx))
        .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
            this.click_content_row(target, &open_row_id, event.modifiers(), cx);
        }))
        .into_any_element()
}

/// Renders one group row of New releases in the List mode (ADR 0086
/// Decision 3). The art, the title and the badges have the places of a
/// release row.
pub(crate) fn render_window_group_row(
    tile: &WindowGroupTileVm,
    art: CollectionArtImage,
    (target, probe): (ContentListSelectionTarget, Option<AnyElement>),
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let open_row_id = tile.open_row_id.clone();
    let text = div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .child(
            Label::new(tile.title.clone())
                .size(FontSize::Micro)
                .weight(FontWeight::MEDIUM),
        )
        .child(
            Label::new(tile.date_span.clone())
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel),
        )
        .child(render_window_group_badges(tile, BadgePlace::Last, cx));

    ListRow::new(SharedString::from(tile.element_id.clone()))
        .a11y_label(ListRowA11yLabel {
            label: SharedString::from(tile.a11y_label.clone()),
        })
        .selected(tile.selection.is_marked())
        .focused(tile.selection.focused)
        .scroll_probe(probe)
        .child(CollectionArt::new(
            art,
            CollectionKind::SinglesGroup,
            ThumbnailSize::Sm,
        ))
        .child(text)
        .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
            this.click_content_row(target, &open_row_id, event.modifiers(), cx);
        }))
        .into_any_element()
}

/// The place of the kind badge on the status line of a group. A release
/// tile shows its kind badge first, and a release row shows it last.
#[derive(Clone, Copy, Eq, PartialEq)]
enum BadgePlace {
    First,
    Last,
}

/// The status line of a group: the state word, the Library count and the
/// "n Singles" badge (ADR 0086 Decision 3). "New" has the color of the
/// "New" mark of a release. "In library" and the count have the color of
/// the Library word of a release.
fn render_window_group_badges(tile: &WindowGroupTileVm, place: BadgePlace, cx: &App) -> AnyElement {
    let word_color = match tile.state {
        WindowGroupStateVm::New { .. } => SemanticColor::Accent,
        WindowGroupStateVm::InLibrary => SemanticColor::TertiaryLabel,
    };
    let badge = || {
        TagBadge::new(TagBadgeDisplay {
            kind: EntityKind::Release,
            label: Some(SharedString::from(tile.kind_badge.clone())),
        })
    };
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(Spacing::XS.scaled(cx))
        .when(place == BadgePlace::First, |badges| badges.child(badge()))
        .child(
            Label::new(tile.state.word())
                .size(FontSize::Micro)
                .color(word_color),
        )
        .when_some(tile.state.count_text(), |badges, count_text| {
            badges.child(
                Label::new(count_text)
                    .size(FontSize::Micro)
                    .color(SemanticColor::TertiaryLabel),
            )
        })
        .when(place == BadgePlace::Last, |badges| badges.child(badge()))
        .into_any_element()
}
