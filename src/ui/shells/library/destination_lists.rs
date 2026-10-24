//! Releases, Artists, and Playlists destination lists (ADR 0084 Decision
//! 2), with the singles group rows of the Releases list (ADR 0084
//! Decision 6).
//!
//! Each destination reads straight from the Library tree or the loaded
//! playlists; none of these owns a `ContentListPageVm`. Row labels and
//! targets come from the view model; this shell owns only layout and
//! interaction wiring.
//!
//! The Releases list and the Artists list are virtual lists (ADR 0084
//! task 037). Each one renders one item at a time, so a frame does work
//! only for the items in view.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, Context, FontWeight, Image, SharedString, Styled,
    Window,
};
use gpui_component::input::Input;
use gpui_component::Size;

use crate::library::LibraryApp;
use crate::ui::composites::music_column_scroll::{music_column_scrollbar, MusicColumnList};
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::composites::release_download_state::render_release_download_state;
use crate::ui::composites::release_row_menu::{release_row_menu, ReleaseRowMenuContext};
use crate::ui::composites::track_drag::{self, playlist_drop_target, PlaylistDrop};
use crate::ui::composites::{
    disclosure_column, CollectionArt, CollectionArtImage, DisclosureIndicator,
    DisclosureIndicatorDisplay, DisclosureSupplementDisplay, DisclosureSupplementLabel, EntityKind,
    ListRow, ListRowA11yLabel, Thumbnail, ThumbnailSize,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::primary_selection::PrimarySelectionExt as _;
use crate::ui::primitives::{Button as UiButton, Checkbox, CheckboxState, Label};
use crate::ui::shells::library::destination_list_scroll::render_destination_virtual_list;
use crate::ui::shells::library::index_list_status::{
    render_index_list_status_row, IndexListActions,
};
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::style::spacing;
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::collection_art::CollectionKind;
use crate::view_models::library::{
    ArtistsListItemVm, DestinationListRowTarget, DestinationListRowVm, PlaylistSidebarRowVm,
    PlaylistSidebarVm, ReleasesListItemVm,
};
use crate::view_models::list_selection::RowCheckState;
use crate::view_models::singles_group::SinglesGroupRowVm;
use crate::view_models::workspace::{FrameNavigationEntry, NavigationPathStart};

/// Maps the view model's typed check state to the checkbox primitive's own
/// state (Required correction item 5, 2026-10-18). Only a singles group
/// row's checkbox can read mixed.
fn checkbox_state_for(check_state: RowCheckState) -> CheckboxState {
    match check_state {
        RowCheckState::Checked => CheckboxState::Checked,
        RowCheckState::Mixed => CheckboxState::Mixed,
        RowCheckState::Clear => CheckboxState::NotChecked,
    }
}

/// The collection art of each row, by element id (ADR 0084 Decision 12).
pub(crate) type RowArts = BTreeMap<String, CollectionArtImage>;

/// The logo frame of one collection row. A row with no loaded state shows
/// the entity color.
fn row_art(arts: &RowArts, element_id: &str, kind: CollectionKind) -> CollectionArt {
    collection_art(arts.get(element_id).cloned(), kind)
}

/// The logo frame of one collection row from its loaded `image`. A row
/// with no loaded state shows the entity color.
fn collection_art(image: Option<CollectionArtImage>, kind: CollectionKind) -> CollectionArt {
    CollectionArt::new(
        image.unwrap_or(CollectionArtImage::EntityColor),
        kind,
        ThumbnailSize::Sm,
    )
}

/// The leading art of a destination row.
enum RowArt {
    /// A release cover, with its image when it loaded.
    Cover(Option<Arc<Image>>),
    /// The logo frame of a collection.
    Collection(CollectionArt),
}

/// Renders the Artists destination list (ADR 0084 Decision 2) as a
/// virtual list (ADR 0084 task 037, operator answer Q1 option B).
/// `item_count` is the count of [`ArtistsListItemVm`] items, and
/// `render_item` renders one of them through
/// [`render_artists_list_item`].
pub(crate) fn render_library_destination_rows(
    list_state: &MusicColumnList,
    item_count: usize,
    render_item: impl Fn(usize, &mut Window, &mut App) -> AnyElement + 'static,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    render_destination_virtual_list(list_state, item_count, "artists", render_item, cx)
}

/// Renders one item of the Artists list: a row with the logo frame
/// `art`, or the status row of the scope "Everything" (ADR 0084 packet
/// 016 decision D8).
pub(crate) fn render_artists_list_item(
    item: ArtistsListItemVm,
    art: Option<CollectionArtImage>,
    actions: IndexListActions,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    match item {
        ArtistsListItemVm::Row(row) => {
            let art = RowArt::Collection(collection_art(art, CollectionKind::Artist));
            // ADR 0084 task 031, decision D2: an Artists row has no menu.
            render_destination_list_row(row, art, LeadingDisclosure::None, None, cx)
        }
        ArtistsListItemVm::Status(row) => render_index_list_status_row(row, actions, cx),
    }
}

/// Renders the Releases destination list (ADR 0084 Decisions 2, 6 and
/// 10) as a virtual list (ADR 0084 task 037). `item_count` is the count of
/// [`ReleasesListItemVm`] items, and `render_item` renders one of them
/// through [`render_releases_list_item`]. `list_state` is the held state
/// of ADR 0084 Decision 10, so a keyboard focus move keeps the focused
/// row in view.
pub(crate) fn render_library_releases_list(
    list_state: &MusicColumnList,
    item_count: usize,
    render_item: impl Fn(usize, &mut Window, &mut App) -> AnyElement + 'static,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    render_destination_virtual_list(list_state, item_count, "releases", render_item, cx)
}

/// Renders one item of the Releases list: a release row, a singles group
/// row, the indented row of a single of an open group, or a status row
/// (ADR 0084 Decision 6, packet 016 decision D8). `thumbnail` is the
/// cover of a release row or of a single row, and `art` is the logo frame
/// of a group row. `menu` gives each release row its "⋯" menu and its
/// right-click menu (ADR 0084 task 031).
pub(crate) fn render_releases_list_item(
    item: ReleasesListItemVm,
    thumbnail: Option<Arc<Image>>,
    art: Option<CollectionArtImage>,
    actions: IndexListActions,
    menu: &ReleaseRowMenuContext,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    match item {
        ReleasesListItemVm::LeadingStatus(row) | ReleasesListItemVm::TrailingStatus(row) => {
            render_index_list_status_row(row, actions, cx)
        }
        ReleasesListItemVm::Release(row) => render_destination_list_row(
            row,
            RowArt::Cover(thumbnail),
            LeadingDisclosure::Empty,
            Some(menu),
            cx,
        ),
        ReleasesListItemVm::Group(group) => render_singles_group_row(group, art, cx),
        ReleasesListItemVm::Single(row) => render_destination_list_row(
            row,
            RowArt::Cover(thumbnail),
            LeadingDisclosure::Indented,
            Some(menu),
            cx,
        ),
    }
}

/// Renders one singles group row (ADR 0084 Decision 6). The chevron opens
/// and closes the row in place. A click on the row body opens the group
/// page. The singles of an open group are their own items.
fn render_singles_group_row(
    group: SinglesGroupRowVm,
    art: Option<CollectionArtImage>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let SinglesGroupRowVm {
        element_id,
        label,
        secondary_text,
        a11y_label,
        download_state,
        art: _,
        open,
        expanded,
        disclosure_element_id,
        disclosure_a11y_label,
        singles: _,
        single_keys,
        selection,
    } = group;
    let art = collection_art(art, CollectionKind::SinglesGroup);
    let open_key = open.clone();
    let toggle_key = open;

    let disclosure = disclosure_column(cx)
        .id(SharedString::from(disclosure_element_id))
        .cursor_pointer()
        .tooltip({
            let tooltip = crate::ui::primitives::Tooltip::new(disclosure_a11y_label);
            move |window, cx| tooltip.build(window, cx)
        })
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            // The chevron toggles the row in place. The row body click
            // that opens the group page must not also run.
            cx.stop_propagation();
            this.toggle_singles_group(&toggle_key, cx);
        }))
        .child(DisclosureIndicator::new(DisclosureIndicatorDisplay {
            expanded,
        }));

    let mut group_row = ListRow::new(SharedString::from(element_id.clone()))
        .a11y_label(ListRowA11yLabel {
            label: SharedString::from(a11y_label),
        })
        .selected(selection.is_marked())
        .focused(selection.focused);

    // ADR 0084 Decision 6: the group row's checkbox selects each single
    // of the group. ADR 0084 Decision 10 Amendment, 2026-10-19:
    // `checkbox_visible` is true only in selection mode, so this row
    // keeps no checkbox space in `Normal` mode. Required correction item
    // 5: the checkbox reads checked, mixed, or clear, from the group's
    // own singles.
    if selection.checkbox_visible {
        let checkbox_a11y_label = selection.checkbox_a11y_label.clone();
        let checkbox_group_key = open_key.clone();
        let checkbox_keys = single_keys.clone();
        group_row = group_row.checkbox(
            Checkbox::new(
                SharedString::from(format!("{element_id}-checkbox")),
                checkbox_state_for(selection.check_state),
            )
            .a11y_label(checkbox_a11y_label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                this.toggle_release_group_selection(&checkbox_group_key, &checkbox_keys, cx);
            }))
            .into_any_element(),
        );
    }

    group_row
        .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
            this.click_singles_group_row(&open_key, &single_keys, event.modifiers(), cx);
        }))
        .child(disclosure)
        .child(art)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    Label::new(label)
                        .size(FontSize::Micro)
                        .weight(FontWeight::MEDIUM)
                        .truncated(),
                )
                .child(
                    Label::new(secondary_text)
                        .size(FontSize::Micro)
                        .color(SemanticColor::TertiaryLabel)
                        .truncated(),
                ),
        )
        // ADR 0083 Decision 6, amended 2026-10-22; ADR 0084 task 033:
        // the group row's own download-state icon, from the count of
        // its members that are in the Library.
        .child(render_release_download_state(
            &download_state,
            format!("{element_id}-download-state"),
            cx,
        ))
        .into_any_element()
}

/// The leading disclosure column of a destination row (ADR 0084 Decision
/// 6). The Releases list keeps the column on each row, so each cover aligns
/// with the cover of a singles group row. The Artists list has no group, so
/// it keeps no column.
#[derive(Clone, Copy)]
enum LeadingDisclosure {
    None,
    Empty,
    /// A single row of an open group. The indent comes after the checkbox,
    /// so each checkbox in the list stays in one column.
    Indented,
}

/// One row of the Releases list or of the Artists list. `menu` is `Some`
/// only for a release row: the row then has the menu cell of the one
/// release row menu (ADR 0084 task 031). The view model gives `None` in
/// selection mode, so the cell is then empty.
fn render_destination_list_row(
    row: DestinationListRowVm,
    art: RowArt,
    leading_disclosure: LeadingDisclosure,
    menu: Option<&ReleaseRowMenuContext>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let entity_kind = match &row.open {
        DestinationListRowTarget::Release(_) => EntityKind::Release,
        DestinationListRowTarget::ArtistName(_)
        | DestinationListRowTarget::Publisher(_)
        | DestinationListRowTarget::IndexPublisher(_) => EntityKind::Artist,
    };
    let target = row.open.clone();
    let thumbnail_href = row.thumbnail_href.clone();
    let thumb_element_id = format!("{}-thumb", row.element_id);
    let download_state_id = format!("{}-download-state", row.element_id);
    let selection = row.selection.clone();

    let mut list_row = ListRow::new(SharedString::from(row.element_id.clone()))
        .a11y_label(ListRowA11yLabel {
            label: SharedString::from(row.a11y_label),
        })
        .selected(selection.is_marked())
        .focused(selection.focused);

    // ADR 0084 Decision 10 Amendment, 2026-10-19: a Releases row's
    // checkbox shows only in selection mode (`checkbox_visible`).
    // Required correction item 4: a single row of an open group gets
    // this same checkbox, under the same rule as any other row. The
    // Artists list gives every row `RowSelectionVm::none`, so
    // `checkbox_visible` is always false there.
    if selection.checkbox_visible {
        let checkbox_target = target.clone();
        list_row = list_row.checkbox(
            Checkbox::new(
                SharedString::from(format!("{}-checkbox", row.element_id)),
                checkbox_state_for(selection.check_state),
            )
            .a11y_label(selection.checkbox_a11y_label.clone())
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if let DestinationListRowTarget::Release(key) = &checkbox_target {
                    this.toggle_release_row_selection(key.clone(), cx);
                }
            }))
            .into_any_element(),
        );
    }

    if let Some(context) = menu {
        list_row = list_row.menu_cell(release_row_menu(row.menu.as_ref(), context));
    }

    list_row
        .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
            this.click_release_row(&target, event.modifiers(), cx);
        }))
        .when(
            matches!(leading_disclosure, LeadingDisclosure::Indented),
            |el| el.child(div().flex_none().w(spacing::LG)),
        )
        .when(
            matches!(
                leading_disclosure,
                LeadingDisclosure::Empty | LeadingDisclosure::Indented
            ),
            |el| el.child(disclosure_column(cx)),
        )
        .child(match art {
            RowArt::Cover(thumbnail) => render_hoverable_release_thumb(
                thumb_element_id,
                thumbnail_href,
                Thumbnail::new(entity_kind, ThumbnailSize::Sm).image(thumbnail),
                cx,
            ),
            RowArt::Collection(art) => art.into_any_element(),
        })
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    Label::new(row.label)
                        .size(FontSize::Micro)
                        .weight(FontWeight::MEDIUM)
                        .truncated(),
                )
                .when(!row.secondary_text.is_empty(), |el| {
                    el.child(
                        Label::new(row.secondary_text)
                            .size(FontSize::Micro)
                            .color(SemanticColor::TertiaryLabel)
                            .truncated(),
                    )
                }),
        )
        // ADR 0083 Decision 6, amended 2026-10-22; ADR 0084 task 033: the
        // download-state icon of a release row. An Artists row carries
        // none.
        .when_some(row.download_state, |el, state| {
            el.trailing(render_release_download_state(&state, download_state_id, cx))
        })
        .into_any_element()
}

/// Wraps a release thumbnail so hovering it animates the matching image
/// (ADR 0084 Decision 2), the same way the Library sidebar tree once did
/// for an album row.
pub(crate) fn render_hoverable_release_thumb(
    element_id: String,
    thumbnail_href: Option<String>,
    thumb: Thumbnail,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let Some(href) = thumbnail_href else {
        return thumb.into_any_element();
    };
    let enter_href = href.clone();
    let leave_href = href;
    div()
        .id(SharedString::from(element_id))
        .on_mouse_move(cx.listener(move |this, _, _, cx| {
            if this.hovered_thumb_url() != Some(enter_href.as_str()) {
                this.set_hovered_thumb(Some(enter_href.clone()), cx);
            }
        }))
        .on_hover(cx.listener(move |this, entered: &bool, _, cx| {
            if !*entered && this.hovered_thumb_url() == Some(leave_href.as_str()) {
                this.set_hovered_thumb(None, cx);
            }
        }))
        .child(thumb)
        .into_any_element()
}

/// Renders the Playlists destination list (ADR 0084 Decision 2): sort,
/// "+", each playlist row, and the inline new-playlist input.
#[expect(
    clippy::too_many_lines,
    reason = "moves the Library sidebar's existing playlist rows, sort, add, and inline input intact (ADR 0084 Decision 2)"
)]
pub(crate) fn render_library_playlists_list(
    sidebar: PlaylistSidebarVm,
    new_playlist_input: &gpui::Entity<gpui_component::input::InputState>,
    arts: &RowArts,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let PlaylistSidebarVm {
        sort_button_id,
        add_button_id,
        new_playlist_input_id,
        new_playlist_add_button_id,
        sort_label,
        add_label,
        new_playlist_add_label,
        creating_playlist,
        rows,
        ..
    } = sidebar;

    let mut items: Vec<AnyElement> = Vec::new();
    items.push(
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_end()
            .gap(spacing::XS)
            .px(spacing::MD)
            .pb(spacing::XS)
            .child(
                UiButton::styled(sort_button_id, ControlStyle::ToolbarIcon)
                    .label(sort_label)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.cycle_playlist_sort(cx);
                    })),
            )
            .child(
                UiButton::styled(add_button_id, ControlStyle::ToolbarIcon)
                    .label(add_label)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_creating_playlist(cx);
                    })),
            )
            .into_any_element(),
    );

    if creating_playlist {
        items.push(
            div()
                .id(new_playlist_input_id)
                .px(spacing::MD)
                .pb(spacing::XS)
                .flex()
                .flex_row()
                .gap(spacing::XS)
                .items_center()
                .child(
                    Input::new(new_playlist_input)
                        .cleanable(false)
                        .scaled(Size::Small, cx)
                        .with_primary_selection(new_playlist_input),
                )
                .child(
                    UiButton::styled(new_playlist_add_button_id, ControlStyle::Primary)
                        .label(new_playlist_add_label)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.create_playlist(cx);
                        })),
                )
                .into_any_element(),
        );
    }

    // ADR 0083 task 008, D6: a drop uses the add path of the menu item.
    let on_add =
        track_drag::playlist_drop_handler(cx.listener(|this, drop: &PlaylistDrop, _, cx| {
            this.end_track_drag();
            this.add_tracks_to_playlist(drop.track_ids.clone(), drop.playlist_id, cx);
        }));
    for row in rows {
        let PlaylistSidebarRowVm {
            id: playlist_id,
            element_id,
            name,
            track_count_label,
            selected,
            art: _,
        } = row;
        let art = row_art(arts, &element_id, CollectionKind::Playlist);
        let row_a11y_label = format!("Playlist: {name}");
        let row = ListRow::new(SharedString::from(element_id))
            .a11y_label(ListRowA11yLabel {
                label: row_a11y_label.into(),
            })
            .selected(selected)
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.open_page(
                    FrameNavigationEntry::PlaylistDetail(playlist_id),
                    NavigationPathStart::CurrentPath,
                    cx,
                );
            }))
            .child(div().pl(spacing::MD).child(art))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        Label::new(name)
                            .color(if selected {
                                SemanticColor::Accent
                            } else {
                                SemanticColor::Label
                            })
                            .truncated(),
                    )
                    .child(DisclosureSupplementLabel::new(
                        DisclosureSupplementDisplay {
                            label: track_count_label.into(),
                        },
                    )),
            )
            .into_any_element();
        items.push(playlist_drop_target(row, playlist_id, &on_add, cx));
    }

    let container = div()
        .id("music-playlists-destination-list")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .pt(spacing::MD)
        .pb(spacing::MD)
        .child(page_scroll_content(cx).gap(spacing::XXS).children(items));
    music_column_scrollbar(container, "music-playlists-destination-list-scrollbar")
}
