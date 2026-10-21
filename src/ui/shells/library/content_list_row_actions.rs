//! The row actions of a Tracks row and of a Needs attention row (ADR 0083
//! Decision 6, task 007): the one main fix button and the row menu.
//!
//! The fix button renders the typed action of the view model (ADR 0065).
//! Each menu item goes to `LibraryApp` with the id of the row. The id of a
//! Library track row is the id of its track. The right-click menu gives the
//! focus back to the list column (decision D15).

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{AnyElement, App, ClickEvent, Context, FocusHandle, IntoElement, SharedString, Window};

use crate::library::LibraryApp;
use crate::ui::composites::list_row::RowMenu;
use crate::ui::composites::{PlaylistOption, TrackRowMenuHandlers};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button as UiButton;
use crate::view_models::library::{ContentListRowActionDisplay, ContentListRowDisplay};

/// The playlists and the return focus of the row menus of the Tracks and
/// Needs attention lists (ADR 0083 task 007). The right-click menu gives
/// the focus back to the list column (decision D15).
pub(crate) struct ContentListMenuContext {
    pub(crate) playlists: Rc<Vec<PlaylistOption>>,
    pub(crate) return_focus: FocusHandle,
}

/// The row menu of a Tracks or Needs attention row, or `None` when the
/// view model gives no menu. Each item goes to `LibraryApp` with the id of
/// the row (ADR 0083 task 007, "The commands").
pub(crate) fn content_list_row_menu(
    row: &ContentListRowDisplay,
    context: &ContentListMenuContext,
    cx: &mut Context<LibraryApp>,
) -> Option<RowMenu> {
    let menu = row.menu.clone()?;
    let row_id = row.id.as_str();
    let entity = cx.entity().downgrade();
    let action = {
        let entity = entity.clone();
        let row_id = row_id.to_owned();
        Rc::new(move |item, window: &mut Window, cx: &mut App| {
            if let Some(entity) = entity.upgrade() {
                entity.update(cx, |this, cx| {
                    this.run_content_list_menu_action(&row_id, item, window, cx);
                });
            }
        })
    };
    let select_playlist = {
        let entity = entity.clone();
        let row_id = row_id.to_owned();
        Rc::new(move |playlist_id: &i64, _: &mut Window, cx: &mut App| {
            if let Some(entity) = entity.upgrade() {
                entity.update(cx, |this, cx| {
                    this.add_content_row_to_playlist(&row_id, *playlist_id, cx);
                });
            }
        })
    };
    let create_playlist = {
        let row_id = row_id.to_owned();
        Rc::new(move |name: &String, _: &mut Window, cx: &mut App| {
            if let Some(entity) = entity.upgrade() {
                entity.update(cx, |this, cx| {
                    this.create_playlist_for_content_row(&row_id, name, cx);
                });
            }
        })
    };
    Some(RowMenu {
        menu,
        playlists: Rc::clone(&context.playlists),
        handlers: TrackRowMenuHandlers {
            action,
            select_playlist,
            create_playlist,
        },
        return_focus: context.return_focus.clone(),
    })
}

/// The one main fix button of a Needs attention row (ADR 0083 task 007,
/// decision D10). The label, the accessibility label and the availability
/// come from the view model.
pub(crate) fn render_content_list_row_action(
    action: &ContentListRowActionDisplay,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let kind = action.kind;
    UiButton::styled(
        SharedString::from(action.id.clone()),
        ControlStyle::Secondary,
    )
    .label(action.label.clone())
    .a11y_label(action.a11y_label.clone())
    .disabled(action.disabled())
    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
        this.run_content_list_row_action(kind, cx);
    }))
    .into_any_element()
}
