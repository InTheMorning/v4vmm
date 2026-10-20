//! The action bar of a Releases or Tracks list selection (ADR 0084
//! Decision 10, amended 2026-10-19 for the selection mode).
//!
//! The bar shows only in selection mode: the screen calls
//! [`SelectionActionBar::new`] only when [`SelectionActionBarVm::new`]
//! gave `Some`. With no selected row, the bar carries only its count
//! label text, "Select items", and no action button. Each button
//! renders from the typed action the view model already decided; this
//! composite invents no availability of its own.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, IntoElement, RenderOnce, SharedString, Window};

use crate::ui::composites::playlist_popover::{
    AddToPlaylistDisplay, AddToPlaylistPopover, PlaylistOption,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button as UiButton, Label};
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::selection_action_bar::{SelectionActionBarVm, SelectionActionKind};

type RunHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type SelectHandler = Rc<dyn Fn(&i64, &mut Window, &mut App) + 'static>;
type CreateHandler = Rc<dyn Fn(&String, &mut Window, &mut App) + 'static>;

/// Display-ready input for [`SelectionActionBar`]'s "Add to playlist"
/// popover.
pub(crate) struct SelectionActionBarPlaylists {
    pub(crate) playlists: Vec<PlaylistOption>,
    pub(crate) new_playlist_a11y_label: SharedString,
    pub(crate) back_a11y_label: SharedString,
    pub(crate) create_a11y_label: SharedString,
}

/// The action bar at the bottom of a Releases or Tracks list with a
/// selection (ADR 0084 Decision 10).
#[derive(IntoElement)]
#[must_use]
pub(crate) struct SelectionActionBar {
    bar: SelectionActionBarVm,
    playlists: SelectionActionBarPlaylists,
    on_download: Option<RunHandler>,
    on_remove: Option<RunHandler>,
    on_clear: Option<RunHandler>,
    on_add_to_playlist: Option<SelectHandler>,
    on_create_playlist: Option<CreateHandler>,
    on_remove_from_playlist: Option<RunHandler>,
    on_move_to_top: Option<RunHandler>,
    on_move_to_bottom: Option<RunHandler>,
}

impl SelectionActionBar {
    pub(crate) fn new(bar: SelectionActionBarVm, playlists: SelectionActionBarPlaylists) -> Self {
        Self {
            bar,
            playlists,
            on_download: None,
            on_remove: None,
            on_clear: None,
            on_add_to_playlist: None,
            on_create_playlist: None,
            on_remove_from_playlist: None,
            on_move_to_top: None,
            on_move_to_bottom: None,
        }
    }

    /// "Remove n from playlist" of the playlist page (packet 025).
    pub(crate) fn on_remove_from_playlist(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_remove_from_playlist = Some(Rc::new(handler));
        self
    }

    /// "Move to top" of the playlist page (packet 025).
    pub(crate) fn on_move_to_top(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_move_to_top = Some(Rc::new(handler));
        self
    }

    /// "Move to bottom" of the playlist page (packet 025).
    pub(crate) fn on_move_to_bottom(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_move_to_bottom = Some(Rc::new(handler));
        self
    }

    pub(crate) fn on_download(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_download = Some(Rc::new(handler));
        self
    }

    pub(crate) fn on_remove(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }

    pub(crate) fn on_clear(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_clear = Some(Rc::new(handler));
        self
    }

    pub(crate) fn on_add_to_playlist(
        mut self,
        handler: impl Fn(&i64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_add_to_playlist = Some(Rc::new(handler));
        self
    }

    pub(crate) fn on_create_playlist(
        mut self,
        handler: impl Fn(&String, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_create_playlist = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SelectionActionBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            bar,
            playlists,
            on_download,
            on_remove,
            on_clear,
            on_add_to_playlist,
            on_create_playlist,
            on_remove_from_playlist,
            on_move_to_top,
            on_move_to_bottom,
        } = self;

        let mut children: Vec<AnyElement> = vec![Label::new(bar.count_label)
            .size(FontSize::Caption)
            .color(SemanticColor::SecondaryLabel)
            .into_any_element()];

        for action in bar.actions {
            children.push(match action.kind {
                SelectionActionKind::AddToPlaylist => {
                    let mut popover = AddToPlaylistPopover::new(AddToPlaylistDisplay {
                        id: SharedString::from(action.element_id),
                        playlists: playlists.playlists.clone(),
                        trigger_label: SharedString::from(action.label),
                        trigger_a11y_label: SharedString::from(action.a11y_label),
                        new_playlist_a11y_label: playlists.new_playlist_a11y_label.clone(),
                        back_a11y_label: playlists.back_a11y_label.clone(),
                        create_a11y_label: playlists.create_a11y_label.clone(),
                    });
                    if let Some(handler) = on_add_to_playlist.clone() {
                        popover = popover.on_select(move |playlist_id, window, cx| {
                            handler(playlist_id, window, cx);
                        });
                    }
                    if let Some(handler) = on_create_playlist.clone() {
                        popover =
                            popover.on_create(move |name, window, cx| handler(name, window, cx));
                    }
                    popover.into_any_element()
                }
                SelectionActionKind::Download => action_button(
                    action.element_id,
                    action.label,
                    action.a11y_label,
                    ControlStyle::Secondary,
                    on_download.clone(),
                ),
                SelectionActionKind::Remove => action_button(
                    action.element_id,
                    action.label,
                    action.a11y_label,
                    ControlStyle::Destructive,
                    on_remove.clone(),
                ),
                SelectionActionKind::RemoveFromPlaylist => action_button(
                    action.element_id,
                    action.label,
                    action.a11y_label,
                    ControlStyle::Secondary,
                    on_remove_from_playlist.clone(),
                ),
                SelectionActionKind::MoveToTop => action_button(
                    action.element_id,
                    action.label,
                    action.a11y_label,
                    ControlStyle::Secondary,
                    on_move_to_top.clone(),
                ),
                SelectionActionKind::MoveToBottom => action_button(
                    action.element_id,
                    action.label,
                    action.a11y_label,
                    ControlStyle::Secondary,
                    on_move_to_bottom.clone(),
                ),
                SelectionActionKind::Clear => action_button(
                    action.element_id,
                    action.label,
                    action.a11y_label,
                    ControlStyle::Ghost,
                    on_clear.clone(),
                ),
            });
        }

        div()
            .id("music-selection-action-bar")
            .flex()
            .flex_row()
            .flex_shrink_0()
            .flex_wrap()
            .items_center()
            .gap(Spacing::SM.scaled(cx))
            .px(Spacing::MD.scaled(cx))
            .py(Spacing::SM.scaled(cx))
            .border_t_1()
            .border_color(color(cx, SemanticColor::Separator))
            .bg(color(cx, SemanticColor::SecondarySystemBackground))
            .children(children)
    }
}

fn action_button(
    element_id: &'static str,
    label: String,
    a11y_label: String,
    style: ControlStyle,
    handler: Option<RunHandler>,
) -> AnyElement {
    UiButton::styled(element_id, style)
        .label(label)
        .a11y_label(a11y_label)
        .on_click(move |_, window, cx| {
            if let Some(handler) = &handler {
                handler(window, cx);
            }
        })
        .into_any_element()
}
