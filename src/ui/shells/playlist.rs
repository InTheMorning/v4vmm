//! Playlist detail shell.
//!
//! The shell owns the playlist page hierarchy and row chrome. Screens provide
//! thumbnail images and command callbacks through behavior slots.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, Context, Entity, FontWeight, InteractiveElement,
    ParentElement, Pixels, Point, Render, SharedString, Styled, Window,
};
use gpui_component::input::{Input, InputState};
use gpui_component::Size;

use crate::ui::composites::{
    DetailGrid, DetailHeader, DetailHeaderDisplay, DetailRow, DetailTextRow, EntityKind,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::{Icon, IconName, IconSize};
use crate::ui::primitives::{
    Button as UiButton, ContextMenu, ContextMenuItem, ContextMenuItemDisplay, ContextMenuScope,
};
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::style::{color, spacing};
use crate::ui::{
    layouts as layout,
    tokens::{FontSize, Radius, Size as TokenSize, Spacing},
};
use crate::view_models::library::{
    PlaylistTrackControlsDisplay, PlaylistTrackMenuItemDisplay, PlaylistTrackRowDisplay,
};
use crate::view_models::playlist_detail::PlaylistDetailPageVm;

type PlaylistClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type PlaylistCommandHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type PlaylistReorderHandler = Rc<dyn Fn(&(i64, i64), &mut Window, &mut App) + 'static>;

#[derive(Default)]
pub(crate) struct PlaylistDetailBehaviorSlots {
    pub(crate) on_rename: Option<PlaylistClickHandler>,
    pub(crate) rename_input: Option<Entity<InputState>>,
    pub(crate) renaming: bool,
    pub(crate) on_submit_rename: Option<PlaylistClickHandler>,
    pub(crate) on_cancel_rename: Option<PlaylistClickHandler>,
    pub(crate) on_delete: Option<PlaylistClickHandler>,
    pub(crate) on_reorder: Option<PlaylistReorderHandler>,
    pub(crate) track_rows: Vec<PlaylistShellRow>,
}

/// One row inside the playlist detail shell.
///
/// `Pending` is emitted by paged callers when the row body has not yet
/// been fetched: the shell paints a [`SkeletonTrackRow`] sized to match
/// the real row footprint so the scroll position does not jump on
/// hydration. Eager callers always emit `Ready`.
#[cfg_attr(
    not(feature = "async-runtime"),
    expect(
        dead_code,
        reason = "Pending variant is consumed by the paged playlist screen which is gated on `async-runtime`"
    )
)]
pub(crate) enum PlaylistShellRow {
    Pending {
        position: usize,
        last_position: usize,
    },
    Ready(Box<PlaylistShellReadyRow>),
}

pub(crate) struct PlaylistShellReadyRow {
    pub(crate) display: PlaylistTrackRowDisplay,
    pub(crate) slot: PlaylistTrackRowSlot,
}

#[derive(Default)]
pub(crate) struct PlaylistTrackRowSlot {
    pub(crate) thumbnail: Option<AnyElement>,
    pub(crate) on_select: Option<PlaylistClickHandler>,
    pub(crate) on_play: Option<PlaylistClickHandler>,
    pub(crate) on_move_up: Option<PlaylistCommandHandler>,
    pub(crate) on_move_down: Option<PlaylistCommandHandler>,
    pub(crate) on_remove: Option<PlaylistCommandHandler>,
}

struct PlaylistActionSlots {
    on_rename: Option<PlaylistClickHandler>,
    rename_input: Option<Entity<InputState>>,
    renaming: bool,
    on_submit_rename: Option<PlaylistClickHandler>,
    on_cancel_rename: Option<PlaylistClickHandler>,
    on_delete: Option<PlaylistClickHandler>,
}

#[must_use]
pub(crate) fn click_slot<F>(handler: F) -> PlaylistClickHandler
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    Rc::new(handler)
}

#[must_use]
pub(crate) fn command_slot<F>(handler: F) -> PlaylistCommandHandler
where
    F: Fn(&(), &mut Window, &mut App) + 'static,
{
    Rc::new(move |window, cx| handler(&(), window, cx))
}

#[must_use]
pub(crate) fn reorder_slot<F>(handler: F) -> PlaylistReorderHandler
where
    F: Fn(&(i64, i64), &mut Window, &mut App) + 'static,
{
    Rc::new(handler)
}

#[must_use]
pub(crate) fn render_playlist_detail_shell(
    page: &PlaylistDetailPageVm<'_>,
    slots: PlaylistDetailBehaviorSlots,
    cx: &App,
) -> AnyElement {
    let header_display = page.header_display();
    let PlaylistDetailBehaviorSlots {
        on_rename,
        rename_input,
        renaming,
        on_submit_rename,
        on_cancel_rename,
        on_delete,
        on_reorder,
        track_rows: rows,
    } = slots;
    let track_rows = if rows.is_empty() {
        vec![render_empty_message(page.empty_message())]
    } else {
        render_playlist_rows_with_reorder_targets(page.playlist_id(), rows, on_reorder.as_ref(), cx)
    };

    div()
        .id(page.scroll_id())
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .p(spacing::LG)
        .flex()
        .flex_col()
        .gap(spacing::MD)
        .child(DetailHeader::new(DetailHeaderDisplay {
            kind: EntityKind::Playlist,
            title: SharedString::from(header_display.title),
            subtitle: None,
            data_rows: Vec::new(),
        }))
        .child(DetailGrid::new(
            page.detail_rows()
                .into_iter()
                .map(|(key, value)| {
                    DetailRow::text(DetailTextRow {
                        key: key.into(),
                        value,
                        max_lines: 6,
                    })
                })
                .collect::<Vec<_>>(),
        ))
        .child(render_playlist_actions(
            page,
            PlaylistActionSlots {
                on_rename,
                rename_input,
                renaming,
                on_submit_rename,
                on_cancel_rename,
                on_delete,
            },
            cx,
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(spacing::XXS)
                .children(track_rows),
        )
        .into_any_element()
}

fn render_playlist_rows_with_reorder_targets(
    playlist_id: i64,
    rows: Vec<PlaylistShellRow>,
    on_reorder: Option<&PlaylistReorderHandler>,
    cx: &App,
) -> Vec<AnyElement> {
    let mut rendered = Vec::with_capacity(rows.len());

    for row in rows {
        rendered.push(render_playlist_shell_row(
            playlist_id,
            row,
            on_reorder.cloned(),
            cx,
        ));
    }

    rendered
}

fn render_empty_message(message: &'static str) -> AnyElement {
    div()
        .text_center()
        .p(spacing::XXL)
        .text_color(color::text_muted())
        .child(message)
        .into_any_element()
}

fn render_playlist_actions(
    page: &PlaylistDetailPageVm<'_>,
    slots: PlaylistActionSlots,
    cx: &App,
) -> AnyElement {
    let actions = page.actions_display();
    if slots.renaming {
        return render_playlist_rename_editor(
            actions,
            slots.rename_input,
            slots.on_submit_rename,
            slots.on_cancel_rename,
            cx,
        );
    }

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(spacing::SM)
        .child(apply_click_handler(
            UiButton::styled(
                SharedString::from(actions.rename_button_id),
                ControlStyle::Ghost,
            )
            .label(actions.rename_label)
            .a11y_label(actions.rename_a11y_label),
            slots.on_rename,
        ))
        .child(apply_click_handler(
            UiButton::styled(
                SharedString::from(actions.delete_button_id),
                ControlStyle::Destructive,
            )
            .label(actions.delete_label)
            .a11y_label(actions.delete_a11y_label),
            slots.on_delete,
        ))
        .into_any_element()
}

fn render_playlist_rename_editor(
    actions: crate::view_models::library::PlaylistDetailActionsDisplay,
    rename_input: Option<Entity<InputState>>,
    on_submit_rename: Option<PlaylistClickHandler>,
    on_cancel_rename: Option<PlaylistClickHandler>,
    cx: &App,
) -> AnyElement {
    let input = rename_input.map_or_else(
        || div().into_any_element(),
        |rename_input| {
            div()
                .id(SharedString::from(actions.rename_input_id))
                .flex_1()
                .min_w_0()
                .child(
                    Input::new(&rename_input)
                        .cleanable(false)
                        .scaled(Size::Small, cx),
                )
                .into_any_element()
        },
    );

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(spacing::SM)
        .child(input)
        .child(apply_click_handler(
            UiButton::styled(
                SharedString::from(actions.rename_save_button_id),
                ControlStyle::Primary,
            )
            .label(actions.rename_save_label)
            .a11y_label(actions.rename_save_a11y_label),
            on_submit_rename,
        ))
        .child(apply_click_handler(
            UiButton::styled(
                SharedString::from(actions.rename_cancel_button_id),
                ControlStyle::Ghost,
            )
            .label(actions.rename_cancel_label)
            .a11y_label(actions.rename_cancel_a11y_label),
            on_cancel_rename,
        ))
        .into_any_element()
}

fn render_playlist_shell_row(
    playlist_id: i64,
    row: PlaylistShellRow,
    on_reorder: Option<PlaylistReorderHandler>,
    cx: &App,
) -> AnyElement {
    match row {
        PlaylistShellRow::Pending {
            position,
            last_position,
        } => render_pending_playlist_row(playlist_id, position, last_position, cx),
        PlaylistShellRow::Ready(ready) => {
            render_playlist_track_row(playlist_id, ready.display, ready.slot, on_reorder, cx)
        }
    }
}

fn render_pending_playlist_row(
    playlist_id: i64,
    position: usize,
    last_position: usize,
    cx: &App,
) -> AnyElement {
    let row_id = SharedString::from(format!(
        "playlist-{playlist_id}-row-{position}-of-{last_position}-pending"
    ));
    div()
        .id(row_id)
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::SM.scaled(cx))
        .px(Spacing::SM.scaled(cx))
        .py(Spacing::XS.scaled(cx))
        .rounded(Radius::SM.scaled(cx))
        .child(
            crate::ui::composites::SkeletonTrackRow::new(("playlist-skeleton-row", position))
                .show_thumbnail(true)
                .show_duration(true),
        )
        .into_any_element()
}

#[derive(Clone, Debug)]
struct PlaylistTrackDragPayload {
    playlist_id: i64,
    from_position: i64,
    title: SharedString,
}

struct PlaylistTrackDragPreview {
    title: SharedString,
}

impl Render for PlaylistTrackDragPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .rounded(Radius::MD.scaled(cx))
            .border_1()
            .border_color(color::accent())
            .bg(color::bg_surface())
            .px(Spacing::MD.scaled(cx))
            .py(Spacing::SM.scaled(cx))
            .text_size(FontSize::Micro.scaled(cx))
            .font_weight(FontWeight::BOLD)
            .text_color(color::text_primary())
            .child(self.title.clone())
    }
}

#[must_use]
const fn playlist_reorder_target(from: i64, drop_index: i64) -> Option<i64> {
    let target = if drop_index > from {
        drop_index - 1
    } else {
        drop_index
    };
    if target == from {
        None
    } else {
        Some(target)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlaylistInsertionEdge {
    Before,
    After,
}

#[must_use]
const fn playlist_row_drop_index(from: i64, row_position: i64) -> i64 {
    match playlist_row_insertion_edge(from, row_position) {
        Some(PlaylistInsertionEdge::After) => row_position + 1,
        Some(PlaylistInsertionEdge::Before) | None => row_position,
    }
}

#[must_use]
const fn playlist_row_insertion_edge(
    from: i64,
    row_position: i64,
) -> Option<PlaylistInsertionEdge> {
    if row_position > from {
        Some(PlaylistInsertionEdge::After)
    } else if row_position < from {
        Some(PlaylistInsertionEdge::Before)
    } else {
        None
    }
}

fn render_playlist_track_row(
    playlist_id: i64,
    display: PlaylistTrackRowDisplay,
    slot: PlaylistTrackRowSlot,
    on_reorder: Option<PlaylistReorderHandler>,
    cx: &App,
) -> AnyElement {
    let controls = display.controls.clone();
    let row_drop_index = display.position;
    let row_gap = Spacing::SM.scaled(cx);
    let row_pad_x = Spacing::SM.scaled(cx);
    let row_pad_y = Spacing::XS.scaled(cx);
    let row_radius = Radius::SM.scaled(cx);
    let drop_indicator = Spacing::XS.scaled(cx);
    let drag_payload = PlaylistTrackDragPayload {
        playlist_id,
        from_position: display.position,
        title: SharedString::from(display.title.clone()),
    };
    let PlaylistTrackRowSlot {
        thumbnail,
        on_select,
        on_play,
        on_move_up,
        on_move_down,
        on_remove,
    } = slot;

    let mut row = div()
        .id(SharedString::from(controls.row_id.clone()))
        .flex()
        .flex_row()
        .items_center()
        .gap(row_gap)
        .px(row_pad_x)
        .py(row_pad_y)
        .rounded(row_radius)
        .when(!display.is_available, |el| {
            el.bg(color::bg_surface_hi())
                .border_1()
                .border_color(color::border_strong())
        })
        .hover(|el| el.bg(color::bg_surface_hi()))
        .child(render_playlist_drag_handle(&controls, drag_payload, cx))
        .child(render_playlist_track_body(
            display, thumbnail, on_select, cx,
        ))
        .child(render_playlist_track_controls(
            controls,
            PlaylistTrackControlSlots {
                play: on_play,
                move_up: on_move_up,
                move_down: on_move_down,
                remove: on_remove,
            },
            cx,
        ));

    if let Some(on_reorder) = on_reorder {
        row = row
            .can_drop(move |drag, _window, _cx| {
                drag.downcast_ref::<PlaylistTrackDragPayload>().is_some()
            })
            .drag_over(
                move |el, payload: &PlaylistTrackDragPayload, _window, _cx| {
                    if payload.playlist_id != playlist_id {
                        return el
                            .opacity(0.55)
                            .border_1()
                            .border_color(color::text_muted())
                            .line_through()
                            .cursor_no_drop();
                    }
                    let drop_index = playlist_row_drop_index(payload.from_position, row_drop_index);
                    if playlist_reorder_target(payload.from_position, drop_index).is_none() {
                        return el;
                    }
                    match playlist_row_insertion_edge(payload.from_position, row_drop_index) {
                        Some(PlaylistInsertionEdge::Before) => {
                            el.border_t(drop_indicator).border_color(color::accent())
                        }
                        Some(PlaylistInsertionEdge::After) => {
                            el.border_b(drop_indicator).border_color(color::accent())
                        }
                        None => el,
                    }
                },
            )
            .on_drop(
                move |payload: &PlaylistTrackDragPayload, window: &mut Window, cx: &mut App| {
                    if payload.playlist_id != playlist_id {
                        return;
                    }
                    let drop_index = playlist_row_drop_index(payload.from_position, row_drop_index);
                    if let Some(target) = playlist_reorder_target(payload.from_position, drop_index)
                    {
                        on_reorder(&(payload.from_position, target), window, cx);
                    }
                },
            );
    }

    row.into_any_element()
}

struct PlaylistTrackControlSlots {
    play: Option<PlaylistClickHandler>,
    move_up: Option<PlaylistCommandHandler>,
    move_down: Option<PlaylistCommandHandler>,
    remove: Option<PlaylistCommandHandler>,
}

fn render_playlist_drag_handle(
    controls: &PlaylistTrackControlsDisplay,
    payload: PlaylistTrackDragPayload,
    cx: &App,
) -> AnyElement {
    div()
        .id(SharedString::from(controls.drag_handle_id.clone()))
        .min_w(TokenSize::MinHitTarget.scaled(cx))
        .min_h(TokenSize::MinHitTarget.scaled(cx))
        .flex()
        .items_center()
        .justify_center()
        .rounded(Radius::SM.scaled(cx))
        .text_color(color::text_muted())
        .cursor_move()
        .hover(|el| el.bg(color::bg_surface_hi()))
        .tooltip({
            let label = SharedString::from(controls.drag_handle_a11y_label);
            move |window, cx| crate::ui::primitives::Tooltip::new(label.clone()).build(window, cx)
        })
        .on_drag(
            payload,
            |payload: &PlaylistTrackDragPayload,
             _position: Point<Pixels>,
             _window,
             cx: &mut App| {
                cx.new(|_| PlaylistTrackDragPreview {
                    title: payload.title.clone(),
                })
            },
        )
        .child(
            Icon::new(IconName::DragHandle)
                .size(IconSize::Action)
                .color(color::text_muted()),
        )
        .into_any_element()
}

fn render_playlist_track_controls(
    controls: PlaylistTrackControlsDisplay,
    slots: PlaylistTrackControlSlots,
    cx: &App,
) -> AnyElement {
    let PlaylistTrackControlsDisplay {
        actions_menu_id,
        actions_menu_a11y_label,
        play_button_id,
        play_label,
        play_enabled,
        move_up_menu_item,
        move_down_menu_item,
        remove_menu_item,
        ..
    } = controls;
    let PlaylistTrackControlSlots {
        play,
        move_up,
        move_down,
        remove,
    } = slots;

    let play_btn = apply_click_handler(
        UiButton::styled(SharedString::from(play_button_id), ControlStyle::RowAction)
            .label(play_label)
            .disabled(!play_enabled),
        play,
    );
    let actions_menu = render_playlist_track_actions_menu(
        actions_menu_id,
        actions_menu_a11y_label,
        [
            menu_item(move_up_menu_item, move_up),
            menu_item(move_down_menu_item, move_down),
            menu_item(remove_menu_item, remove),
        ],
    );

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::XS.scaled(cx))
        .child(play_btn)
        .child(actions_menu)
        .into_any_element()
}

fn render_playlist_track_actions_menu(
    id: String,
    a11y_label: &'static str,
    items: [ContextMenuItem; 3],
) -> AnyElement {
    ContextMenu::new(
        SharedString::from(id),
        ContextMenuScope::PlaylistTrack,
        SharedString::from(a11y_label),
    )
    .trigger_label("")
    .items(items)
    .into_any_element()
}

fn menu_item(
    display: PlaylistTrackMenuItemDisplay,
    handler: Option<PlaylistCommandHandler>,
) -> ContextMenuItem {
    let disabled = display.disabled;
    let mut item = ContextMenuItem::new(ContextMenuItemDisplay {
        id: SharedString::from(display.id),
        label: SharedString::from(display.label),
        a11y_label: SharedString::from(display.a11y_label),
        destructive: display.destructive,
        disabled,
    });
    if !disabled {
        if let Some(handler) = handler {
            item = item.on_select(move |window, cx| handler(window, cx));
        }
    }
    item
}

fn render_playlist_track_body(
    display: PlaylistTrackRowDisplay,
    thumbnail: Option<AnyElement>,
    on_select: Option<PlaylistClickHandler>,
    cx: &App,
) -> AnyElement {
    let PlaylistTrackRowDisplay {
        is_available,
        position: _,
        position_label,
        title,
        artist,
        availability_label,
        duration_label,
        thumb_url: _,
        controls,
    } = display;
    let title_color = if is_available {
        color::text_primary()
    } else {
        color::text_muted()
    };
    let gap = Spacing::SM.scaled(cx);
    let thumb_slot = layout::scaled_dimension(layout::PLAYLIST_THUMB_SLOT, cx);
    let duration_width = layout::scaled_dimension(layout::PLAYLIST_TITLE_OFFSET, cx);
    let row_text = FontSize::Caption.scaled(cx);
    let mut row_body = div()
        .id(SharedString::from(controls.row_body_id))
        .flex()
        .flex_row()
        .items_center()
        .gap(gap)
        .flex_1()
        .min_w_0()
        .cursor_pointer();
    if let Some(on_select) = on_select {
        row_body = row_body.on_click(move |event, window, cx| on_select(event, window, cx));
    }

    row_body
        .child(
            div()
                .w(thumb_slot)
                .flex_shrink_0()
                .text_size(row_text)
                .text_color(color::text_muted())
                .child(SharedString::from(position_label)),
        )
        .child(
            div()
                .when(!is_available, |el| el.opacity(0.35))
                .child(thumbnail.unwrap_or_else(|| render_playlist_thumb_placeholder(cx))),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(row_text)
                        .text_color(title_color)
                        .when(!is_available, Styled::line_through)
                        .child(SharedString::from(title)),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(row_text)
                        .text_color(color::text_muted())
                        .child(SharedString::from(artist)),
                )
                .when_some(availability_label, |el, label| {
                    el.child(
                        div()
                            .mt(Spacing::XXS.scaled(cx))
                            .rounded(Radius::SM.scaled(cx))
                            .border_1()
                            .border_color(color::border_strong())
                            .px(Spacing::XS.scaled(cx))
                            .py(Spacing::XXS.scaled(cx))
                            .text_size(row_text)
                            .text_color(color::text_muted())
                            .child(label),
                    )
                }),
        )
        .child(
            div()
                .text_size(row_text)
                .text_color(color::text_muted())
                .w(duration_width)
                .flex_shrink_0()
                .child(SharedString::from(duration_label)),
        )
        .into_any_element()
}

fn render_playlist_thumb_placeholder(cx: &App) -> AnyElement {
    let thumb_slot = layout::scaled_dimension(layout::PLAYLIST_THUMB_SLOT, cx);
    div()
        .w(thumb_slot)
        .h(thumb_slot)
        .rounded(Radius::SM.scaled(cx))
        .bg(color::border_subtle())
        .flex()
        .items_center()
        .justify_center()
        .text_size(FontSize::Headline.scaled(cx))
        .flex_shrink_0()
        .child("\u{1F3B5}")
        .into_any_element()
}

fn apply_click_handler(button: UiButton, handler: Option<PlaylistClickHandler>) -> UiButton {
    if let Some(handler) = handler {
        button.on_click(move |event, window, cx| handler(event, window, cx))
    } else {
        button
    }
}

#[cfg(test)]
mod tests {
    use super::{
        playlist_reorder_target, playlist_row_drop_index, playlist_row_insertion_edge,
        PlaylistInsertionEdge,
    };

    #[test]
    fn playlist_reorder_target_ignores_original_and_adjacent_slots() {
        assert_eq!(playlist_reorder_target(1, 1), None);
        assert_eq!(playlist_reorder_target(1, 2), None);
    }

    #[test]
    fn playlist_reorder_target_adjusts_after_source_slot() {
        assert_eq!(playlist_reorder_target(1, 0), Some(0));
        assert_eq!(playlist_reorder_target(1, 3), Some(2));
    }

    #[test]
    fn playlist_row_drop_index_quantizes_by_drag_direction() {
        assert_eq!(playlist_row_drop_index(1, 3), 4);
        assert_eq!(
            playlist_row_insertion_edge(1, 3),
            Some(PlaylistInsertionEdge::After)
        );
        assert_eq!(
            playlist_reorder_target(1, playlist_row_drop_index(1, 3)),
            Some(3)
        );
        assert_eq!(playlist_row_drop_index(3, 1), 1);
        assert_eq!(
            playlist_row_insertion_edge(3, 1),
            Some(PlaylistInsertionEdge::Before)
        );
        assert_eq!(
            playlist_reorder_target(3, playlist_row_drop_index(3, 1)),
            Some(1)
        );
        assert_eq!(playlist_row_insertion_edge(2, 2), None);
        assert_eq!(
            playlist_reorder_target(2, playlist_row_drop_index(2, 2)),
            None
        );
    }
}
