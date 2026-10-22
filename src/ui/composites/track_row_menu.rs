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
    Button, ContextMenu, ContextMenuHandle, ContextMenuItem, ContextMenuItemDisplay,
    ContextMenuScope, Divider, MenuLevelControls, PointerContextMenu,
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
/// applies the hover rule. It holds the open state of its menu and gives
/// it to the menu.
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
        // The cell holds the open state and gives it to the menu. Thus the
        // reset rule below reads the state of the open menu (ADR 0084 task
        // 030, P1).
        let handle = ContextMenuHandle::new(&id, window, cx);
        let open = handle.is_open(cx);
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
        .handle(handle)
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

// ---------------------------------------------------------------------------
// ADR 0084 task 030 P1: "New Playlist" in the row menu shows the create mode.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod create_mode_tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gpui::{
        div, prelude::*, px, FocusHandle, Modifiers, MouseButton, TestAppContext, VisualTestContext,
    };

    use super::TrackRowMenuHandlers;
    use crate::ui::composites::playlist_popover::{PlaylistOption, PlaylistOptionDisplay};
    use crate::ui::composites::track_row::TrackRow;
    use crate::view_models::release_page::{TrackRowMenuFacts, TrackRowMenuVm};
    use crate::view_models::track_detail::TrackRowVm;

    struct MenuRowTest {
        focus: FocusHandle,
        created: Rc<RefCell<Vec<String>>>,
    }

    impl gpui::Render for MenuRowTest {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            let mut facts = TrackRowMenuFacts::new("row", "Song", false);
            facts.add_to_playlist = true;
            let created = Rc::clone(&self.created);
            let handlers = TrackRowMenuHandlers {
                action: Rc::new(|_, _, _| {}),
                select_playlist: Rc::new(|_, _, _| {}),
                create_playlist: Rc::new(move |name: &String, _, _| {
                    created.borrow_mut().push(name.clone());
                }),
            };
            let playlists = vec![PlaylistOption::new(PlaylistOptionDisplay {
                id: 7,
                name: "Mix".into(),
                a11y_label: "Add to Mix".into(),
            })];
            let row = TrackRow::from_vm(
                "row",
                &TrackRowVm {
                    element_key: "track:1".to_owned(),
                    number: Some("1".to_owned()),
                    title: "Song".to_owned(),
                    subtitle: None,
                    duration: Some("3:00".to_owned()),
                    date_text: None,
                    a11y_label: "Song".to_owned(),
                },
            )
            .menu(TrackRowMenuVm::project(&facts), playlists)
            .on_menu_action(handlers, self.focus.clone());
            div().w(px(900.0)).h(px(800.0)).p(px(200.0)).child(row)
        }
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
    }

    fn click(cx: &mut VisualTestContext, selector: &'static str) {
        draw(cx);
        let bounds = cx
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} must have bounds"));
        cx.simulate_click(bounds.center(), Modifiers::default());
        draw(cx);
    }

    fn open_test(cx: &mut TestAppContext) -> (Rc<RefCell<Vec<String>>>, &mut VisualTestContext) {
        cx.update(gpui_component::init);
        cx.update(crate::ui::primitives::context_menu::init);
        let created = Rc::new(RefCell::new(Vec::new()));
        let shared = Rc::clone(&created);
        let (_view, cx) = cx.add_window_view(move |_, cx| MenuRowTest {
            focus: cx.focus_handle(),
            created: shared,
        });
        (created, cx)
    }

    /// The level "Add to playlist", then "New Playlist", and the proof
    /// that the create mode shows.
    fn show_create_mode(cx: &mut VisualTestContext) {
        click(cx, "row-menu:AddToPlaylist");
        click(cx, "pl-new");
        assert!(
            cx.debug_bounds("pl-name-field").is_some(),
            "after \"New Playlist\", the name field must show"
        );
        assert!(
            cx.debug_bounds("pl-create-confirm").is_some(),
            "after \"New Playlist\", \"Create & Add\" must show"
        );
    }

    /// R30-01 (ADR 0083 Decision 6): "⋯", "Add to playlist" and "New
    /// Playlist" show the create mode.
    #[gpui::test]
    fn adr_0084_music_open_defects_r30_01_menu_new_playlist_shows_create_mode(
        cx: &mut TestAppContext,
    ) {
        let (_, cx) = open_test(cx);
        draw(cx);
        // The "⋯" cell shows on a hover of the row.
        let row = cx
            .debug_bounds("track-row")
            .expect("the row must have bounds");
        cx.simulate_mouse_move(row.center(), None, Modifiers::default());
        click(cx, "row-menu-trigger");
        show_create_mode(cx);
    }

    /// R30-03 (ADR 0083 Decision 6). In the create mode, a name and a click
    /// on "Create & Add" call the create handler one time. The handler gets
    /// that name. The menu then closes.
    #[gpui::test]
    fn adr_0084_music_open_defects_r30_03_create_and_add_calls_handler_once(
        cx: &mut TestAppContext,
    ) {
        let (created, cx) = open_test(cx);
        draw(cx);
        let row = cx
            .debug_bounds("track-row")
            .expect("the row must have bounds");
        cx.simulate_mouse_move(row.center(), None, Modifiers::default());
        click(cx, "row-menu-trigger");
        show_create_mode(cx);
        click(cx, "pl-name-field");
        cx.simulate_input("Road mix");
        click(cx, "pl-create-confirm");
        assert_eq!(*created.borrow(), vec!["Road mix".to_owned()]);
        assert!(
            cx.debug_bounds("pl-create-confirm").is_none(),
            "the menu must close after \"Create & Add\""
        );
        assert!(cx.debug_bounds("context-menu-items").is_none());
    }

    /// R30-02 (ADR 0083 Decision 6): the same steps through the
    /// right-click menu of the row.
    #[gpui::test]
    fn adr_0084_music_open_defects_r30_02_right_click_new_playlist_shows_create_mode(
        cx: &mut TestAppContext,
    ) {
        let (_, cx) = open_test(cx);
        draw(cx);
        let row = cx
            .debug_bounds("track-row")
            .expect("the row must have bounds");
        cx.simulate_mouse_down(row.center(), MouseButton::Right, Modifiers::default());
        cx.simulate_mouse_up(row.center(), MouseButton::Right, Modifiers::default());
        show_create_mode(cx);
    }
}
