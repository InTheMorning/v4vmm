//! The release page shell of the Library and of the Index (ADR 0084
//! Decision 4, packet 021).
//!
//! `render_release_page` renders the release page body and its track rows
//! for both origins, from `view_models::release_page`. The host entity
//! gives the view model, the resolved images and one dispatch function.
//! This shell sends each command through that function. It holds no state.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, ClipboardItem, ElementId, Image, Modifiers,
    ScrollHandle, SharedString, Window,
};

use crate::media::cover_color::CoverColor;
use crate::ui::composites::{
    identity_action_button, ActionRow, ActionRowDisplay, AddToPlaylistDisplay,
    AddToPlaylistPopover, DisclosureTextPanel, DisclosureTextPanelDisplay,
    IdentityActionButtonDisplay, IdentityActionKind, PlaylistOption, PlaylistOptionDisplay,
    ReleaseSurfaceElement, StatusRole, TrackRow,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{
    Button as UiButton, ContextMenu, ContextMenuItem, ContextMenuItemDisplay, ContextMenuScope,
};
use crate::ui::shells::entity::{
    render_album_name_links, render_contributor_panel, render_feed_identity_actions,
    render_named_feed_rows, render_release_detail_shell, ContributorRowSlot,
    ReleaseDetailBehaviorSlots,
};
use crate::ui::style::{color, typography};
use crate::view_models::album_page::{AlbumPageAction, ALBUM_PAGE_MENU_A11Y_LABEL};
use crate::view_models::entity_detail::{
    ContributorIdentityActionDisplay, ContributorIdentityActionKind, ContributorRowVm,
};
use crate::view_models::library::MbStatusKind;
use crate::view_models::release_page::{
    ReleaseDescriptionId, ReleaseDescriptionVm, ReleasePageActionVm, ReleasePageActionsVm,
    ReleasePageCommand, ReleasePageVm, ReleaseTrackRowVm, RowClickModifiers,
};
use crate::view_models::PlaylistOptionDisplayVm;

/// The one dispatch function of a release page. The host entity builds it
/// from `cx.entity()`. It runs each command on the data that the host
/// holds at the time of the click.
pub(crate) type ReleasePageDispatch = Rc<dyn Fn(ReleasePageCommand, &mut Window, &mut App)>;

/// The values of a release page that the host resolves before the render:
/// the images, the scroll probes and the scroll handle.
#[derive(Default)]
pub(crate) struct ReleasePageImages<'a> {
    pub(crate) hero_image: Option<Arc<Image>>,
    pub(crate) cover_color: Option<CoverColor>,
    /// The thumbnail of each row, in row order.
    pub(crate) row_thumbnails: Vec<Option<Arc<Image>>>,
    /// The scroll probe of each row, in row order (ADR 0084 packet 018).
    pub(crate) row_probes: Vec<Option<AnyElement>>,
    /// The contributor thumbnails by image URL. `None` when the page shows
    /// no contributors panel.
    pub(crate) contributor_thumbnails: Option<&'a BTreeMap<String, Option<Arc<Image>>>>,
    /// The held scroll handle of the Music detail column.
    pub(crate) scroll_handle: Option<ScrollHandle>,
}

/// Renders the release page of `vm` for each origin.
pub(crate) fn render_release_page(
    vm: &ReleasePageVm<'_>,
    images: ReleasePageImages<'_>,
    dispatch: &ReleasePageDispatch,
    cx: &App,
) -> AnyElement {
    let ReleasePageImages {
        hero_image,
        cover_color,
        row_thumbnails,
        row_probes,
        contributor_thumbnails,
        scroll_handle,
    } = images;
    let open_name_link = {
        let dispatch = Rc::clone(dispatch);
        move |target: &_, window: &mut Window, cx: &mut App| {
            dispatch(
                ReleasePageCommand::OpenNameLink(Clone::clone(target)),
                window,
                cx,
            );
        }
    };
    let name_links = render_album_name_links(vm.name_links.clone(), open_name_link.clone(), cx);
    let named_feed_rows = render_named_feed_rows(vm.named_feed_rows.clone(), open_name_link, cx);
    let mut thumbnails = row_thumbnails.into_iter();
    let mut probes = row_probes.into_iter();
    let track_rows = vm
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            ReleaseSurfaceElement::from_element(render_release_page_track_row(
                row,
                index,
                thumbnails.next().flatten(),
                probes.next().flatten(),
                &vm.playlists,
                dispatch,
                cx,
            ))
        })
        .collect();
    let mut slots = ReleaseDetailBehaviorSlots {
        hero_image,
        cover_color,
        name_links,
        named_feed_rows,
        primary_actions: vec![ReleaseSurfaceElement::from_element(
            render_release_page_actions(&vm.actions, &vm.playlists, dispatch),
        )],
        identity_actions: render_feed_identity_actions(&vm.page),
        track_rows: Some(track_rows),
        scroll_handle,
        ..ReleaseDetailBehaviorSlots::default()
    };
    if vm.contributors_panel {
        if let Some(panel) = render_release_page_contributors(vm, contributor_thumbnails) {
            slots
                .after_section
                .push(ReleaseSurfaceElement::from_element(panel));
        }
    }
    if let Some(description) = vm.description.as_ref() {
        slots.description_panel = Some(ReleaseSurfaceElement::from_element(
            render_release_page_description(description, dispatch),
        ));
    }
    render_release_detail_shell(&vm.page, slots)
}

/// The action row of a release page (ADR 0083 Decision 5): one filled
/// button, the plain buttons and the "⋯" menu.
fn render_release_page_actions(
    actions: &ReleasePageActionsVm,
    playlists: &[PlaylistOptionDisplayVm],
    dispatch: &ReleasePageDispatch,
) -> AnyElement {
    let mut controls = vec![render_release_page_button(
        &actions.filled,
        actions.playlist_popover_id.as_deref(),
        playlists,
        dispatch,
    )];
    controls.extend(actions.plain.iter().map(|action| {
        render_release_page_button(
            action,
            actions.playlist_popover_id.as_deref(),
            playlists,
            dispatch,
        )
    }));
    if !actions.menu.is_empty() {
        controls.push(render_release_page_menu(
            &actions.menu,
            &actions.menu_id,
            dispatch,
        ));
    }
    ActionRow::new(ActionRowDisplay {
        a11y_label: SharedString::from("Album actions"),
    })
    .control_group(controls)
    .into_any_element()
}

/// One button of a release page. "Add to playlist" opens the shared
/// playlist popover.
fn render_release_page_button(
    action: &ReleasePageActionVm,
    playlist_popover_id: Option<&str>,
    playlists: &[PlaylistOptionDisplayVm],
    dispatch: &ReleasePageDispatch,
) -> AnyElement {
    if let (AlbumPageAction::AddToPlaylist, Some(popover_id)) = (action.action, playlist_popover_id)
    {
        let select = Rc::clone(dispatch);
        let create = Rc::clone(dispatch);
        return AddToPlaylistPopover::new(AddToPlaylistDisplay {
            id: SharedString::from(popover_id.to_owned()),
            playlists: playlist_choices(playlists),
            trigger_label: SharedString::from(action.label.clone()),
            trigger_a11y_label: SharedString::from(action.a11y_label.clone()),
            new_playlist_a11y_label: SharedString::from("Create a new playlist"),
            back_a11y_label: SharedString::from("Back to playlist choices"),
            create_a11y_label: SharedString::from("Create playlist and add album"),
        })
        .trigger_style(ControlStyle::Primary)
        .disabled(!action.available)
        .on_select(move |playlist_id: &i64, window, cx| {
            select(
                ReleasePageCommand::AddReleaseToPlaylist(*playlist_id),
                window,
                cx,
            );
        })
        .on_create(move |name: &String, window, cx| {
            create(
                ReleasePageCommand::CreatePlaylistForRelease(name.clone()),
                window,
                cx,
            );
        })
        .into_any_element();
    }
    let button = UiButton::styled(SharedString::from(action.id.clone()), ControlStyle::Primary)
        .label(action.label.clone())
        .a11y_label(action.a11y_label.clone())
        .disabled(!action.available);
    let (true, Some(command)) = (action.available, action.command.clone()) else {
        return button.into_any_element();
    };
    let dispatch = Rc::clone(dispatch);
    button
        .on_click(move |_, window, cx| dispatch(command.clone(), window, cx))
        .into_any_element()
}

/// The "⋯" menu of a release page. The context menu draws a divider before
/// the destructive item.
fn render_release_page_menu(
    items: &[ReleasePageActionVm],
    menu_id: &str,
    dispatch: &ReleasePageDispatch,
) -> AnyElement {
    let menu_items = items.iter().map(|action| {
        let mut item = ContextMenuItem::new(ContextMenuItemDisplay {
            id: SharedString::from(action.id.clone()),
            label: SharedString::from(action.label.clone()),
            a11y_label: SharedString::from(action.a11y_label.clone()),
            destructive: action.destructive,
            disabled: !action.available,
        });
        if let (true, Some(command)) = (action.available, action.command.clone()) {
            let dispatch = Rc::clone(dispatch);
            item = item.on_select(move |window, cx| dispatch(command.clone(), window, cx));
        }
        item
    });
    ContextMenu::new(
        SharedString::from(menu_id.to_owned()),
        ContextMenuScope::TrackList,
        SharedString::from(ALBUM_PAGE_MENU_A11Y_LABEL),
    )
    .trigger_label("")
    .items(menu_items)
    .into_any_element()
}

/// The description panel of a release page. The host keeps its disclosure
/// state.
fn render_release_page_description(
    description: &ReleaseDescriptionVm,
    dispatch: &ReleasePageDispatch,
) -> AnyElement {
    let id: ElementId = match &description.id {
        ReleaseDescriptionId::Library(feed_id) => ("library-feed-description", *feed_id).into(),
        ReleaseDescriptionId::Index(feed_guid) => {
            SharedString::from(format!("index-feed-description:{feed_guid}")).into()
        }
    };
    let display = description.display.clone();
    let dispatch = Rc::clone(dispatch);
    DisclosureTextPanel::new(DisclosureTextPanelDisplay {
        id,
        label: SharedString::from("Description"),
        a11y_label: SharedString::from("Toggle feed description"),
        collapsed: display.collapsed,
        body: display.body.map(SharedString::from),
        show_more: display.show_more.map(|show_more| {
            (
                SharedString::from(show_more.label),
                SharedString::from(show_more.a11y_label),
            )
        }),
    })
    .on_toggle(move |_, window, cx| dispatch(ReleasePageCommand::ToggleDescription, window, cx))
    .into_any_element()
}

/// The contributors panel of a release page.
fn render_release_page_contributors(
    vm: &ReleasePageVm<'_>,
    thumbnails: Option<&BTreeMap<String, Option<Arc<Image>>>>,
) -> Option<AnyElement> {
    let display = vm.detail.contributor_panel_display();
    render_contributor_panel(
        display.id,
        display.title,
        vm.detail.contributors(),
        |contributor| {
            let thumbnail = contributor
                .image_url()
                .and_then(|url| thumbnails?.get(url))
                .and_then(Clone::clone);
            ContributorRowSlot {
                thumbnail,
                actions: contributor_identity_actions(contributor),
            }
        },
    )
}

fn contributor_identity_actions(contributor: &ContributorRowVm<'_>) -> Vec<ReleaseSurfaceElement> {
    contributor
        .identity_actions()
        .into_iter()
        .map(|action| {
            let ContributorIdentityActionDisplay {
                id,
                kind,
                target,
                a11y_label,
            } = action;
            let target_for_click = target;
            let a11y_label = SharedString::from(a11y_label);
            match kind {
                ContributorIdentityActionKind::Website => {
                    identity_action_button(IdentityActionButtonDisplay {
                        id: SharedString::from(id),
                        kind: IdentityActionKind::Website,
                        a11y_label,
                    })
                    .on_click(move |_, _, _| {
                        let _ = open::that(&target_for_click);
                    })
                    .into_any_element()
                }
                ContributorIdentityActionKind::Nostr => {
                    identity_action_button(IdentityActionButtonDisplay {
                        id: SharedString::from(id),
                        kind: IdentityActionKind::Nostr,
                        a11y_label,
                    })
                    .on_click(move |_, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(target_for_click.clone()));
                    })
                    .into_any_element()
                }
            }
        })
        .map(ReleaseSurfaceElement::from_element)
        .collect()
}

/// The one track row of a release page and of a singles group page, for
/// each origin. `index` is the row index that each row command carries.
pub(crate) fn render_release_page_track_row(
    vm: &ReleaseTrackRowVm,
    index: usize,
    thumbnail: Option<Arc<Image>>,
    probe: Option<AnyElement>,
    playlists: &[PlaylistOptionDisplayVm],
    dispatch: &ReleasePageDispatch,
    cx: &App,
) -> AnyElement {
    let primary_style = if vm.primary.destructive {
        ControlStyle::DestructiveRowAction
    } else {
        ControlStyle::RowAction
    };
    let primary = {
        let dispatch = Rc::clone(dispatch);
        UiButton::styled(SharedString::from(vm.primary.id.clone()), primary_style)
            .label(vm.primary.label.clone())
            .disabled(!vm.primary.enabled)
            .on_click(move |_, window, cx| {
                dispatch(ReleasePageCommand::TrackPrimary(index), window, cx);
            })
    };
    let mut actions = vec![primary.into_any_element()];
    if let Some(status) = vm.status {
        let status_color = match status.kind {
            Some(MbStatusKind::Success) => StatusRole::Success.color(cx),
            Some(MbStatusKind::Danger) => StatusRole::Danger.color(cx),
            Some(MbStatusKind::Warning) => StatusRole::Warning.color(cx),
            _ => color::text_muted(),
        };
        actions.push(
            div()
                .text_size(typography::SIZE_MICRO)
                .text_color(status_color)
                .child(SharedString::from(status.text))
                .into_any_element(),
        );
    }
    if let Some(label) = vm.state_label {
        actions.push(
            div()
                .text_size(typography::SIZE_MICRO)
                .text_color(color::text_muted())
                .child(SharedString::from(label))
                .into_any_element(),
        );
    }
    let select = Rc::clone(dispatch);
    let create = Rc::clone(dispatch);
    actions.push(
        AddToPlaylistPopover::new(AddToPlaylistDisplay {
            id: SharedString::from(vm.playlist_popover_id.clone()),
            playlists: playlist_choices(playlists),
            trigger_label: SharedString::from(vm.playlist_trigger_label),
            trigger_a11y_label: SharedString::from("Add track to playlist"),
            new_playlist_a11y_label: SharedString::from("Create a new playlist"),
            back_a11y_label: SharedString::from("Back to playlist choices"),
            create_a11y_label: SharedString::from("Create playlist and add track"),
        })
        .on_select(move |playlist_id: &i64, window, cx| {
            select(
                ReleasePageCommand::AddTrackToPlaylist(index, *playlist_id),
                window,
                cx,
            );
        })
        .on_create(move |name: &String, window, cx| {
            create(
                ReleasePageCommand::CreatePlaylistForTrack(index, name.clone()),
                window,
                cx,
            );
        })
        .into_any_element(),
    );

    let mut row = TrackRow::from_vm(SharedString::from(vm.row_id.clone()), &vm.row)
        .thumbnail(thumbnail)
        .selection(vm.selection.clone())
        .scroll_probe(probe);
    // ADR 0084 Decision 4 and packet 018: a row with a key opens its page.
    // In `Selecting` mode, a click changes the selection instead.
    if vm.row_key.is_some() {
        let toggle = Rc::clone(dispatch);
        let click = Rc::clone(dispatch);
        row = row
            .on_toggle_selection(move |_: &ClickEvent, window, cx| {
                toggle(ReleasePageCommand::ToggleRow(index), window, cx);
            })
            .on_click(move |event: &ClickEvent, window, cx| {
                click(
                    ReleasePageCommand::ClickRow(index, row_click_modifiers(event.modifiers())),
                    window,
                    cx,
                );
            });
    }
    for action in actions {
        row = row.trailing_child(action);
    }
    row.into_any_element()
}

/// The playlist choices of a playlist popover.
fn playlist_choices(playlists: &[PlaylistOptionDisplayVm]) -> Vec<PlaylistOption> {
    playlists
        .iter()
        .map(|option| {
            PlaylistOption::new(PlaylistOptionDisplay {
                id: option.id,
                name: SharedString::from(option.name.clone()),
                a11y_label: SharedString::from(option.a11y_label.clone()),
            })
        })
        .collect()
}

/// Copies the modifier keys of a click event into the view model type.
#[must_use]
pub(crate) const fn row_click_modifiers(modifiers: Modifiers) -> RowClickModifiers {
    RowClickModifiers {
        control: modifiers.control,
        alt: modifiers.alt,
        shift: modifiers.shift,
        platform: modifiers.platform,
        function: modifiers.function,
    }
}

/// Copies the modifier keys of a row command back into the GPUI type, for
/// the click routes of packet 018.
#[must_use]
pub(crate) const fn click_modifiers(modifiers: RowClickModifiers) -> Modifiers {
    Modifiers {
        control: modifiers.control,
        alt: modifiers.alt,
        shift: modifiers.shift,
        platform: modifiers.platform,
        function: modifiers.function,
    }
}
