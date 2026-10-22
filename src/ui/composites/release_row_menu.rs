//! The row menu of a release row (ADR 0083 Decision 6, ADR 0084 task 031).
//!
//! The Releases list, the feed page and the name page give their release
//! rows the "⋯" menu and the right-click menu through this one function.
//! [`release_row_menu`] makes the [`RowMenu`] of task 007. The row then
//! builds its menu cell and its right-click root through `row_menu_parts`.
//! Thus the items come from the one item function of task 006.
//!
//! Each item goes to the handler of the host as a [`ReleaseRowCommand`],
//! with the release of the row.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{App, FocusHandle, Window};

use crate::ui::composites::list_row::RowMenu;
use crate::ui::composites::playlist_popover::PlaylistOption;
use crate::ui::composites::track_row_menu::TrackRowMenuHandlers;
use crate::view_models::release_row_actions::{
    ReleaseRowCommand, ReleaseRowMenuVm, ReleaseRowTarget,
};

/// Runs one command of a release row menu on the release of the row.
pub(crate) type ReleaseRowCommandHandler =
    Rc<dyn Fn(&ReleaseRowTarget, ReleaseRowCommand, &mut Window, &mut App) + 'static>;

/// The playlists, the return focus and the command handler of the release
/// row menus of one list.
#[derive(Clone)]
pub(crate) struct ReleaseRowMenuContext {
    /// The choices of the "Add to playlist" level.
    pub(crate) playlists: Rc<Vec<PlaylistOption>>,
    /// The focus that the right-click menu gives back.
    pub(crate) return_focus: FocusHandle,
    pub(crate) on_command: ReleaseRowCommandHandler,
}

/// The row menu of one release row, or `None` when the view model gives
/// no menu.
#[must_use]
pub(crate) fn release_row_menu(
    menu: Option<&ReleaseRowMenuVm>,
    context: &ReleaseRowMenuContext,
) -> Option<RowMenu> {
    let menu = menu?;
    let target = Rc::new(menu.target.clone());
    let action = {
        let target = Rc::clone(&target);
        let on_command = Rc::clone(&context.on_command);
        Rc::new(move |action, window: &mut Window, cx: &mut App| {
            if let Some(command) = ReleaseRowCommand::of_menu_action(action) {
                on_command(&target, command, window, cx);
            }
        })
    };
    let select_playlist = {
        let target = Rc::clone(&target);
        let on_command = Rc::clone(&context.on_command);
        Rc::new(
            move |playlist_id: &i64, window: &mut Window, cx: &mut App| {
                on_command(
                    &target,
                    ReleaseRowCommand::AddToPlaylist(*playlist_id),
                    window,
                    cx,
                );
            },
        )
    };
    let create_playlist = {
        let on_command = Rc::clone(&context.on_command);
        Rc::new(move |name: &String, window: &mut Window, cx: &mut App| {
            on_command(
                &target,
                ReleaseRowCommand::CreatePlaylist(name.clone()),
                window,
                cx,
            );
        })
    };
    Some(RowMenu {
        menu: menu.menu.clone(),
        playlists: Rc::clone(&context.playlists),
        handlers: TrackRowMenuHandlers {
            action,
            select_playlist,
            create_playlist,
        },
        return_focus: context.return_focus.clone(),
    })
}
