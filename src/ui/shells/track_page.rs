//! The track page shell of the Library and of the Index (ADR 0084
//! Decision 4, packet 023).
//!
//! `render_track_page` renders the whole track page for both origins, from
//! `view_models::track_page`. The host entity gives the view model, the
//! resolved images, one dispatch function and, for a Library page, the
//! "Inspect sources" disclosure. This shell sends each command through the
//! dispatch function. It holds no state.

#![warn(clippy::pedantic)]

use std::rc::Rc;
use std::sync::Arc;

use gpui::{div, prelude::*, AnyElement, App, Image, ScrollHandle, SharedString, Window};

use crate::ui::composites::music_column_scroll::music_column_tracked_scrollbar;
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::composites::{
    ActionRow, ActionRowDisplay, ActionRowMessage, AddToPlaylistDisplay, AddToPlaylistPopover,
    DisclosureTextPanel, DisclosureTextPanelDisplay, PlaylistOption, PlaylistOptionDisplay,
    TrackSurfaceElement,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{
    Button, ContextMenu, ContextMenuItem, ContextMenuItemDisplay, ContextMenuScope,
};
use crate::ui::shells::track::{
    build_track_detail_surface, render_track_feed_identity_section, render_track_name_links,
    render_track_page_identity_actions, TrackDetailBehaviorSlots,
};
use crate::ui::tokens::Spacing;
use crate::view_models::track_detail::TrackPageAction;
use crate::view_models::track_page::{
    TrackPageActionRowVm, TrackPageActionVm, TrackPageCommand, TrackPageDescriptionVm, TrackPageVm,
};

/// The one dispatch function of a track page. The host entity builds it
/// from `cx.entity()`. It runs each command on the data that the host
/// holds at the time of the click.
pub(crate) type TrackPageDispatch = Rc<dyn Fn(TrackPageCommand, &mut Window, &mut App)>;

/// The values of a track page that the host resolves before the render.
pub(crate) struct TrackPageImages {
    /// The cover of the track.
    pub(crate) hero_image: Option<Arc<Image>>,
    /// The held scroll handle of the Music detail column (ADR 0084
    /// packet 018).
    pub(crate) scroll_handle: ScrollHandle,
}

/// Renders the track page of `vm` for each origin. `inspect_sources` shows
/// below the surface only when `vm.inspect_sources` is true.
pub(crate) fn render_track_page(
    vm: &TrackPageVm<'_>,
    images: TrackPageImages,
    dispatch: &TrackPageDispatch,
    inspect_sources: Option<AnyElement>,
    cx: &mut App,
) -> AnyElement {
    let TrackPageImages {
        hero_image,
        scroll_handle,
    } = images;
    let open_name_link = Rc::clone(dispatch);
    let name_links = render_track_name_links(
        &vm.page,
        move |target, window, cx| {
            open_name_link(TrackPageCommand::OpenNameLink(target.clone()), window, cx);
        },
        cx,
    );
    let mut slots = TrackDetailBehaviorSlots {
        hero_image,
        name_links,
        primary_actions: vec![TrackSurfaceElement::from_element(
            render_track_page_actions(&vm.actions, dispatch),
        )],
        external_links: render_track_page_identity_actions(&vm.page),
        description_panel: vm.description.as_ref().map(|description| {
            TrackSurfaceElement::from_element(render_track_page_description(description, dispatch))
        }),
        ..TrackDetailBehaviorSlots::default()
    };
    if let Some(feed_identity) = render_track_feed_identity_section(&vm.page, cx) {
        slots.section_elements.push(feed_identity);
    }
    let surface = build_track_detail_surface(&vm.page, slots).into_any_element();
    let content = match inspect_sources.filter(|_| vm.inspect_sources) {
        Some(inspect_sources) => div()
            .flex()
            .flex_col()
            .gap(Spacing::LG.scaled(cx))
            .child(surface)
            .child(inspect_sources)
            .into_any_element(),
        None => surface,
    };
    // ADR 0084 packet 023, decision 14: one padding for both origins,
    // scaled with the interface scale.
    let container = div()
        .id(SharedString::from(vm.container_id))
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .p(Spacing::LG.scaled(cx))
        .child(page_scroll_content(cx).child(content));
    // ADR 0084 packet 018: the keys of the detail column scroll the page
    // through its held scroll handle.
    music_column_tracked_scrollbar(container, &scroll_handle, None)
}

/// The action row of a track page (ADR 0083 Decision 5): one filled
/// button, the plain buttons and the "⋯" menu. The status message shows
/// below the row when a message exists.
fn render_track_page_actions(
    actions: &TrackPageActionRowVm,
    dispatch: &TrackPageDispatch,
) -> AnyElement {
    let mut controls = vec![render_track_page_button(
        &actions.filled,
        ControlStyle::Primary,
        actions,
        dispatch,
    )];
    controls.extend(actions.plain.iter().map(|action| {
        render_track_page_button(action, ControlStyle::Secondary, actions, dispatch)
    }));
    if !actions.menu.is_empty() {
        controls.push(render_track_page_menu(actions, dispatch));
    }
    let mut row = ActionRow::new(ActionRowDisplay {
        a11y_label: SharedString::from(actions.a11y_label),
    })
    .control_group(controls);
    if let Some(message) = actions.message.clone() {
        row = row.message(ActionRowMessage::from_status_display(message));
    }
    row.into_any_element()
}

/// One button of the track page action row. "Add to playlist" opens the
/// shared playlist popover.
fn render_track_page_button(
    action: &TrackPageActionVm,
    style: ControlStyle,
    actions: &TrackPageActionRowVm,
    dispatch: &TrackPageDispatch,
) -> AnyElement {
    if let (TrackPageAction::AddToPlaylist, Some(popover_id)) =
        (action.action, actions.playlist_popover_id.as_ref())
    {
        let select = Rc::clone(dispatch);
        let create = Rc::clone(dispatch);
        return AddToPlaylistPopover::new(AddToPlaylistDisplay {
            id: SharedString::from(popover_id.clone()),
            playlists: actions
                .playlists
                .iter()
                .map(|option| {
                    PlaylistOption::new(PlaylistOptionDisplay {
                        id: option.id,
                        name: SharedString::from(option.name.clone()),
                        a11y_label: SharedString::from(option.a11y_label.clone()),
                    })
                })
                .collect(),
            trigger_label: SharedString::from(action.label.clone()),
            trigger_a11y_label: SharedString::from(action.a11y_label.clone()),
            new_playlist_a11y_label: SharedString::from("Create a new playlist"),
            back_a11y_label: SharedString::from("Back to playlist choices"),
            create_a11y_label: SharedString::from("Create playlist and add track"),
        })
        .trigger_style(style)
        .disabled(!action.available)
        .on_select(move |playlist_id: &i64, window, cx| {
            select(TrackPageCommand::AddToPlaylist(*playlist_id), window, cx);
        })
        .on_create(move |name: &String, window, cx| {
            create(TrackPageCommand::CreatePlaylist(name.clone()), window, cx);
        })
        .into_any_element();
    }
    let button = Button::styled(SharedString::from(action.id.clone()), style)
        .label(action.label.clone())
        .a11y_label(action.a11y_label.clone())
        .disabled(!action.available);
    if !action.available {
        return button.into_any_element();
    }
    let dispatch = Rc::clone(dispatch);
    let command = action.action;
    button
        .on_click(move |_, window, cx| dispatch(TrackPageCommand::Action(command), window, cx))
        .into_any_element()
}

/// The "⋯" menu of a track page. The context menu draws a divider before
/// the destructive item.
fn render_track_page_menu(
    actions: &TrackPageActionRowVm,
    dispatch: &TrackPageDispatch,
) -> AnyElement {
    let menu_items = actions.menu.iter().map(|action| {
        let mut item = ContextMenuItem::new(ContextMenuItemDisplay {
            id: SharedString::from(action.id.clone()),
            label: SharedString::from(action.label.clone()),
            a11y_label: SharedString::from(action.a11y_label.clone()),
            destructive: action.destructive,
            disabled: !action.available,
        });
        if let Some(reason) = action.reason {
            item = item.description(reason);
        }
        if action.available {
            let dispatch = Rc::clone(dispatch);
            let command = action.action;
            item = item.on_select(move |window, cx| {
                dispatch(TrackPageCommand::Action(command), window, cx);
            });
        }
        item
    });
    ContextMenu::new(
        SharedString::from(actions.menu_id.clone()),
        ContextMenuScope::TrackList,
        SharedString::from(actions.menu_a11y_label),
    )
    .trigger_label("")
    .items(menu_items)
    .into_any_element()
}

/// The description panel of a track page. The host keeps its disclosure
/// state.
fn render_track_page_description(
    description: &TrackPageDescriptionVm,
    dispatch: &TrackPageDispatch,
) -> AnyElement {
    let display = description.display.clone();
    let dispatch = Rc::clone(dispatch);
    DisclosureTextPanel::new(DisclosureTextPanelDisplay {
        id: SharedString::from(description.id.clone()).into(),
        label: SharedString::from(description.label),
        a11y_label: SharedString::from(description.a11y_label),
        collapsed: display.collapsed,
        body: display.body.map(SharedString::from),
        show_more: display.show_more.map(|show_more| {
            (
                SharedString::from(show_more.label),
                SharedString::from(show_more.a11y_label),
            )
        }),
    })
    .on_toggle(move |_, window, cx| dispatch(TrackPageCommand::ToggleDescription, window, cx))
    .into_any_element()
}
