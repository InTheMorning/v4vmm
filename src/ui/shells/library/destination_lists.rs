//! Releases, Artists, and Playlists destination lists (ADR 0084 Decision
//! 2), with the singles group rows of the Releases list (ADR 0084
//! Decision 6).
//!
//! Each destination reads straight from the Library tree or the loaded
//! playlists; none of these owns a `ContentListPageVm`. Row labels and
//! targets come from the view model; this shell owns only layout and
//! interaction wiring.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, ClickEvent, Context, FontWeight, Image, ScrollHandle,
    SharedString, Styled,
};
use gpui_component::input::Input;
use gpui_component::Size;

use crate::library::LibraryApp;
use crate::ui::composites::music_column_scroll::music_column_scrollbar;
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::composites::{
    DisclosureIndicator, DisclosureIndicatorDisplay, DisclosureSupplementDisplay,
    DisclosureSupplementLabel, EntityKind, ListRow, ListRowA11yLabel, Thumbnail, ThumbnailSize,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::primary_selection::PrimarySelectionExt as _;
use crate::ui::primitives::{Button as UiButton, Checkbox, CheckboxState, Label};
use crate::ui::shells::library::destination_list_scroll::{
    render_destination_list_container, render_destination_list_container_tracked,
};
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::style::spacing;
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::library::{
    DestinationListRowTarget, DestinationListRowVm, DiscoverViewRowVm, PlaylistSidebarRowVm,
    PlaylistSidebarVm, ReleasesListEntryVm,
};
use crate::view_models::singles_group::SinglesGroupRowVm;
use crate::view_models::workspace::{FrameNavigationEntry, NavigationPathStart};

/// Renders the Discover views list column (ADR 0084 Decision 3). "New
/// releases" is the only view today.
pub(crate) fn render_library_discover_views_list(
    rows: &[DiscoverViewRowVm],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let container = div()
        .id("music-discover-views-list")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .px(spacing::MD)
        .pb(spacing::MD)
        .child(
            page_scroll_content(cx)
                .gap(spacing::XXS)
                .children(rows.iter().map(|row| render_discover_view_row(row, cx))),
        );
    music_column_scrollbar(container, "music-discover-views-list-scrollbar")
}

fn render_discover_view_row(row: &DiscoverViewRowVm, cx: &mut Context<LibraryApp>) -> AnyElement {
    let view = row.view;

    ListRow::new(SharedString::from(row.element_id.clone()))
        .a11y_label(ListRowA11yLabel {
            label: SharedString::from(row.a11y_label.clone()),
        })
        .selected(row.selected)
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.open_discover_view(view, cx);
        }))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .pl(spacing::MD)
                .child(
                    Label::new(row.label)
                        .color(if row.selected {
                            SemanticColor::Accent
                        } else {
                            SemanticColor::Label
                        })
                        .weight(FontWeight::MEDIUM)
                        .truncated(),
                )
                .child(
                    Label::new(row.subtitle)
                        .size(FontSize::Micro)
                        .color(SemanticColor::TertiaryLabel)
                        .truncated(),
                ),
        )
        .into_any_element()
}

/// Renders the Releases or Artists destination list (ADR 0084 Decision 2).
/// `empty_label` names the row kind for the empty-list message, for
/// example "releases".
pub(crate) fn render_library_destination_rows(
    rows: Vec<DestinationListRowVm>,
    empty_label: &'static str,
    thumbnails: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let children: Vec<AnyElement> = rows
        .into_iter()
        .map(|row| {
            let thumbnail = thumbnails.get(&row.element_id).cloned().flatten();
            render_destination_list_row(row, thumbnail, cx)
        })
        .collect();
    render_destination_list_container(children, empty_label, cx)
}

/// Renders the Releases destination list (ADR 0084 Decisions 2, 6 and
/// 10): a release row for each release that is not in a group, and a
/// group row for each Library singles group. An open group row shows
/// each of its singles below it, indented. `scroll_handle` is the held
/// handle of ADR 0084 Decision 10, so a keyboard focus move keeps the
/// focused row in view.
pub(crate) fn render_library_releases_list(
    entries: Vec<ReleasesListEntryVm>,
    thumbnails: &BTreeMap<String, Option<Arc<Image>>>,
    scroll_handle: &ScrollHandle,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let mut children: Vec<AnyElement> = Vec::new();
    for entry in entries {
        match entry {
            ReleasesListEntryVm::Release(row) => {
                let thumbnail = thumbnails.get(&row.element_id).cloned().flatten();
                children.push(render_destination_list_row(row, thumbnail, cx));
            }
            ReleasesListEntryVm::SinglesGroup(group) => {
                render_singles_group_rows(group, thumbnails, cx, &mut children);
            }
        }
    }
    render_destination_list_container_tracked(children, "releases", scroll_handle, cx)
}

/// Renders one singles group row and, when it is open, the indented row of
/// each single (ADR 0084 Decision 6). The chevron opens and closes the row
/// in place. A click on the row body opens the group page.
fn render_singles_group_rows(
    group: SinglesGroupRowVm,
    thumbnails: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &mut Context<LibraryApp>,
    children: &mut Vec<AnyElement>,
) {
    let SinglesGroupRowVm {
        element_id,
        label,
        secondary_text,
        a11y_label,
        thumbnail_href,
        open,
        expanded,
        disclosure_element_id,
        disclosure_a11y_label,
        singles,
        single_keys,
        selection,
    } = group;
    let thumbnail = thumbnails.get(&element_id).cloned().flatten();
    let thumb_element_id = format!("{element_id}-thumb");
    let open_key = open.clone();
    let toggle_key = open;

    let disclosure = div()
        .id(SharedString::from(disclosure_element_id))
        .flex()
        .items_center()
        .justify_center()
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
        .selected(selection.checked)
        .focused(selection.focused);

    // ADR 0084 Decision 6: the group row's checkbox selects each single
    // of the group. ADR 0084 Decision 10 Design: "The row" — `ListRow`
    // shows it while a selection exists or the row has the keyboard
    // focus (`checkbox_visible`), and reveals it on hover otherwise.
    {
        let checkbox_a11y_label = selection.checkbox_a11y_label.clone();
        let checkbox_keys = single_keys.clone();
        group_row = group_row.checkbox(
            Checkbox::new(
                SharedString::from(format!("{element_id}-checkbox")),
                CheckboxState::from_checked(selection.checked),
            )
            .a11y_label(checkbox_a11y_label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                this.toggle_release_group_selection(&checkbox_keys, cx);
            }))
            .into_any_element(),
            selection.checkbox_visible,
        );
    }

    children.push(
        group_row
            .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
                this.click_singles_group_row(&open_key, &single_keys, event.modifiers(), cx);
            }))
            .child(disclosure)
            .child(render_hoverable_release_thumb(
                thumb_element_id,
                thumbnail_href,
                Thumbnail::new(EntityKind::Release, ThumbnailSize::Sm).image(thumbnail),
                cx,
            ))
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
            .into_any_element(),
    );

    for row in singles {
        let thumbnail = thumbnails.get(&row.element_id).cloned().flatten();
        children.push(
            div()
                .pl(spacing::LG)
                .child(render_destination_list_row(row, thumbnail, cx))
                .into_any_element(),
        );
    }
}

fn render_destination_list_row(
    row: DestinationListRowVm,
    thumbnail: Option<Arc<Image>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let entity_kind = match &row.open {
        DestinationListRowTarget::Release(_) => EntityKind::Release,
        DestinationListRowTarget::Artist(_) | DestinationListRowTarget::Publisher(_) => {
            EntityKind::Artist
        }
    };
    let target = row.open.clone();
    let thumbnail_href = row.thumbnail_href.clone();
    let thumb_element_id = format!("{}-thumb", row.element_id);
    let selection = row.selection.clone();

    let mut list_row = ListRow::new(SharedString::from(row.element_id.clone()))
        .a11y_label(ListRowA11yLabel {
            label: SharedString::from(row.a11y_label),
        })
        .selected(selection.checked)
        .focused(selection.focused);

    // ADR 0084 Decision 10: a Releases row's checkbox. The Artists list,
    // and an expanded single row inside a group, give every row
    // `RowSelectionVm::none`, so an empty `checkbox_a11y_label` means
    // this row never shows one, even on hover.
    if !selection.checkbox_a11y_label.is_empty() {
        let checkbox_target = target.clone();
        list_row = list_row.checkbox(
            Checkbox::new(
                SharedString::from(format!("{}-checkbox", row.element_id)),
                CheckboxState::from_checked(selection.checked),
            )
            .a11y_label(selection.checkbox_a11y_label.clone())
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if let DestinationListRowTarget::Release(key) = &checkbox_target {
                    this.toggle_release_row_selection(key.clone(), cx);
                }
            }))
            .into_any_element(),
            selection.checkbox_visible,
        );
    }

    list_row
        .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
            this.click_release_row(&target, event.modifiers(), cx);
        }))
        .child(render_hoverable_release_thumb(
            thumb_element_id,
            thumbnail_href,
            Thumbnail::new(entity_kind, ThumbnailSize::Sm).image(thumbnail),
            cx,
        ))
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

    for row in rows {
        let PlaylistSidebarRowVm {
            id: playlist_id,
            element_id,
            name,
            track_count_label,
            selected,
        } = row;
        let row_a11y_label = format!("Playlist: {name}");
        items.push(
            ListRow::new(SharedString::from(element_id))
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
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .pl(spacing::MD)
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
                .into_any_element(),
        );
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
