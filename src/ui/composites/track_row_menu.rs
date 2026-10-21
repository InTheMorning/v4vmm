//! The "⋯" menu and the right-click menu of a track row (ADR 0083
//! Decision 6, task 006).
//!
//! One function, [`track_row_menu_items`], builds the items of both menus
//! from [`TrackRowMenuVm`]. So both menus show the same items, in the same
//! order, with the same divider before the destructive item.
//!
//! "Add to playlist" opens a second level in the same menu surface. That
//! level reuses the list mode and the create mode of the shared playlist
//! popover (ADR 0083 task 006, decision D3).
//!
//! The menu cell is hidden until the pointer is on the row. It is visible
//! when the row has the keyboard focus or when its menu is open
//! ([`menu_cell_visibility`]). A hidden cell keeps its width.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, AnyElement, App, Div, Entity, FocusHandle, IntoElement, Pixels, Point,
    RenderOnce, SharedString, Window,
};
use gpui_component::{input::InputState, v_flex};

use crate::ui::composites::playlist_popover::{
    build_create_mode, build_list_mode, CreateHandler, PlaylistChoiceControls, PlaylistOption,
    SelectHandler,
};
use crate::ui::icons::IconName;
use crate::ui::primitives::{
    Button, ContextMenu, ContextMenuItem, ContextMenuItemDisplay, ContextMenuScope, Divider,
    MenuLevelControls, PointerContextMenu,
};
use crate::ui::tokens::Spacing;
use crate::view_models::release_page::{TrackRowMenuAction, TrackRowMenuVm};

/// The GPUI group name of a track row. The menu cell becomes visible on a
/// hover of this group.
pub(crate) const TRACK_ROW_GROUP: &str = "track-row";

/// The visibility rule of the menu cell of a track row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MenuCellVisibility {
    /// The cell is always visible.
    Always,
    /// The cell is visible only while the pointer is on the row.
    OnHover,
}

/// The visibility of the menu cell (ADR 0083 Decision 6). The cell is
/// always visible when the row has the keyboard focus or when its menu is
/// open. Otherwise it shows on a hover of the row.
#[must_use]
pub(crate) const fn menu_cell_visibility(focused: bool, open: bool) -> MenuCellVisibility {
    if focused || open {
        MenuCellVisibility::Always
    } else {
        MenuCellVisibility::OnHover
    }
}

/// Runs one menu action of the row.
pub(crate) type MenuActionHandler = Rc<dyn Fn(TrackRowMenuAction, &mut Window, &mut App) + 'static>;

/// The handlers of the row menu.
#[derive(Clone)]
pub(crate) struct TrackRowMenuHandlers {
    /// Runs "Download", "Inspect sources" and "Remove…".
    pub(crate) action: MenuActionHandler,
    /// Adds the track to the playlist with this id.
    pub(crate) select_playlist: SelectHandler,
    /// Makes a playlist with this name and adds the track.
    pub(crate) create_playlist: CreateHandler,
}

/// The keyed state of one row menu.
pub(crate) struct TrackRowMenuState {
    /// The pointer position of the open right-click menu.
    pointer: Option<Point<Pixels>>,
    /// The playlist level shows its create mode.
    creating: bool,
    name_input: Entity<InputState>,
}

impl TrackRowMenuState {
    /// The pointer position of the open right-click menu. `None` when the
    /// right-click menu is closed.
    pub(crate) const fn pointer(&self) -> Option<Point<Pixels>> {
        self.pointer
    }

    /// Opens the right-click menu at `position`.
    pub(crate) fn open_at(&mut self, position: Point<Pixels>) {
        self.pointer = Some(position);
        self.creating = false;
    }
}

/// The keyed state of the menu `menu_id`. Call it at the same element
/// position at each render.
pub(crate) fn row_menu_state(
    menu_id: &str,
    window: &mut Window,
    cx: &mut App,
) -> Entity<TrackRowMenuState> {
    window.use_keyed_state(
        SharedString::from(format!("{menu_id}-row-state")),
        cx,
        |window, cx| TrackRowMenuState {
            pointer: None,
            creating: false,
            name_input: cx.new(|cx| InputState::new(window, cx).placeholder("Playlist name")),
        },
    )
}

/// The items of the "⋯" menu and of the right-click menu (ADR 0083
/// Decision 6). Both menus use this one function.
pub(crate) fn track_row_menu_items(
    menu: &TrackRowMenuVm,
    playlists: &Rc<Vec<PlaylistOption>>,
    handlers: &TrackRowMenuHandlers,
    state: &Entity<TrackRowMenuState>,
) -> Vec<ContextMenuItem> {
    menu.items
        .iter()
        .map(|item| {
            let entry = ContextMenuItem::new(ContextMenuItemDisplay {
                id: SharedString::from(item.id.clone()),
                label: SharedString::from(item.label),
                a11y_label: SharedString::from(item.a11y_label.clone()),
                destructive: item.destructive,
                disabled: false,
            });
            if item.action == TrackRowMenuAction::AddToPlaylist {
                let menu = menu.clone();
                let playlists = Rc::clone(playlists);
                let handlers = handlers.clone();
                let state = state.clone();
                return entry.level(move |controls, _window, cx| {
                    playlist_level(&menu, &playlists, &handlers, &state, controls, cx)
                });
            }
            let action = item.action;
            let run = Rc::clone(&handlers.action);
            entry.on_select(move |window, cx| run(action, window, cx))
        })
        .collect()
}

/// The "Add to playlist" level: "Back" and the playlist choices, or the
/// create mode of the shared playlist popover.
fn playlist_level(
    menu: &TrackRowMenuVm,
    playlists: &Rc<Vec<PlaylistOption>>,
    handlers: &TrackRowMenuHandlers,
    state: &Entity<TrackRowMenuState>,
    controls: &MenuLevelControls,
    cx: &App,
) -> AnyElement {
    let labels = menu.playlist_level;
    let (creating, name_input) = {
        let state = state.read(cx);
        (state.creating, state.name_input.clone())
    };
    let set_creating = |creating: bool| {
        let state = state.clone();
        Rc::new(move |_: &mut Window, cx: &mut App| {
            state.update(cx, |state, cx| {
                state.creating = creating;
                cx.notify();
            });
        })
    };
    let choice_controls = PlaylistChoiceControls {
        close: {
            let dismiss = Rc::clone(&controls.dismiss);
            let reset = set_creating(false);
            Rc::new(move |window, cx| {
                reset(window, cx);
                dismiss(window, cx);
            })
        },
        show_create: set_creating(true),
        show_list: set_creating(false),
    };
    if creating {
        return build_create_mode(
            &choice_controls,
            name_input,
            Some(Rc::clone(&handlers.create_playlist)),
            SharedString::from(labels.back_to_choices_a11y_label),
            SharedString::from(labels.create_a11y_label),
            cx,
        )
        .into_any_element();
    }
    let back = Rc::clone(&controls.back);
    let back_button = Button::plain(SharedString::from(format!("{}-back", menu.id)))
        .full_width()
        .align_leading()
        .leading_icon(IconName::Back)
        .label(labels.back_text)
        .a11y_label(labels.back_a11y_label)
        .on_click(move |_, window, cx| back(window, cx));
    v_flex()
        .gap(Spacing::XXS.scaled(cx))
        .child(back_button)
        .child(
            div()
                .my(Spacing::XS.scaled(cx))
                .child(Divider::horizontal()),
        )
        .child(build_list_mode(
            &choice_controls,
            Rc::clone(playlists),
            Some(Rc::clone(&handlers.select_playlist)),
            true,
            SharedString::from(labels.new_playlist_a11y_label),
            cx,
        ))
        .into_any_element()
}

/// The right-click menu of a row at the recorded pointer position. A
/// dismissal closes it and gives the focus back to `return_focus`.
pub(crate) fn track_row_pointer_menu(
    menu: &TrackRowMenuVm,
    position: Point<Pixels>,
    return_focus: FocusHandle,
    items: Vec<ContextMenuItem>,
    state: &Entity<TrackRowMenuState>,
) -> PointerContextMenu {
    let state = state.clone();
    items.into_iter().fold(
        PointerContextMenu::new(
            SharedString::from(format!("{}-pointer", menu.id)),
            position,
            return_focus,
            move |_, cx| {
                state.update(cx, |state, cx| {
                    state.pointer = None;
                    state.creating = false;
                    cx.notify();
                });
            },
        ),
        PointerContextMenu::item,
    )
}

/// The menu column cell of a track row. It holds the "⋯" button and
/// applies the hover rule. It reads the open state of its menu at the
/// element position of the menu.
#[derive(IntoElement)]
pub(crate) struct TrackRowMenuCell {
    cell: Div,
    focused: bool,
    menu: Option<(
        TrackRowMenuVm,
        Vec<ContextMenuItem>,
        Entity<TrackRowMenuState>,
    )>,
}

impl TrackRowMenuCell {
    /// A cell with no menu keeps its width and shows nothing.
    pub(crate) const fn new(cell: Div, focused: bool) -> Self {
        Self {
            cell,
            focused,
            menu: None,
        }
    }

    /// The menu of the cell, its items and its keyed state.
    pub(crate) fn menu(
        mut self,
        menu: TrackRowMenuVm,
        items: Vec<ContextMenuItem>,
        state: Entity<TrackRowMenuState>,
    ) -> Self {
        self.menu = Some((menu, items, state));
        self
    }
}

impl RenderOnce for TrackRowMenuCell {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some((menu, items, state)) = self.menu else {
            return self.cell;
        };
        let id = SharedString::from(menu.id.clone());
        let open = ContextMenu::is_open(&id, window, cx);
        // A menu that closed from outside starts again at its list mode.
        let closed = !open && state.read(cx).pointer.is_none();
        if closed && state.read(cx).creating {
            state.update(cx, |state, _| state.creating = false);
        }
        let trigger = ContextMenu::new(
            id,
            ContextMenuScope::TrackList,
            SharedString::from(menu.trigger_a11y_label),
        )
        .trigger_label("")
        .trigger_tooltip(menu.trigger_tooltip)
        .items(items);
        let cell = match menu_cell_visibility(self.focused, open) {
            MenuCellVisibility::Always => self.cell,
            MenuCellVisibility::OnHover => self
                .cell
                .invisible()
                .group_hover(TRACK_ROW_GROUP, Styled::visible),
        };
        cell.child(trigger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R83-6-12 (ADR 0083 Decision 6): the menu cell is always visible for
    /// a focused row or an open menu, and shows on hover in each other
    /// case.
    #[test]
    fn adr_0083_row_actions_r83_6_12_menu_cell_visibility_rule() {
        assert_eq!(
            menu_cell_visibility(true, false),
            MenuCellVisibility::Always
        );
        assert_eq!(
            menu_cell_visibility(false, true),
            MenuCellVisibility::Always
        );
        assert_eq!(menu_cell_visibility(true, true), MenuCellVisibility::Always);
        assert_eq!(
            menu_cell_visibility(false, false),
            MenuCellVisibility::OnHover
        );
    }
}
