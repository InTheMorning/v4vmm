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
use crate::ui::style::{color, radius, spacing, typography};
use crate::ui::{layouts as layout, tokens::Radius};
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
        render_playlist_rows_with_drop_zones(page.playlist_id(), rows, on_reorder)
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

fn render_playlist_rows_with_drop_zones(
    playlist_id: i64,
    rows: Vec<PlaylistShellRow>,
    on_reorder: Option<PlaylistReorderHandler>,
) -> Vec<AnyElement> {
    let mut rendered = Vec::with_capacity(rows.len().saturating_mul(2).saturating_add(1));

    for row in rows {
        let drop_index = row.position();
        rendered.push(render_playlist_drop_zone(
            playlist_id,
            drop_index,
            on_reorder.clone(),
        ));
        rendered.push(render_playlist_shell_row(playlist_id, row));
    }

    let final_drop_index = i64::try_from(rendered.len() / 2).unwrap_or(i64::MAX);
    rendered.push(render_playlist_drop_zone(
        playlist_id,
        final_drop_index,
        on_reorder,
    ));
    rendered
}

impl PlaylistShellRow {
    fn position(&self) -> i64 {
        match self {
            Self::Pending { position, .. } => i64::try_from(*position).unwrap_or(i64::MAX),
            Self::Ready(row) => row.display.position,
        }
    }
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

fn render_playlist_shell_row(playlist_id: i64, row: PlaylistShellRow) -> AnyElement {
    match row {
        PlaylistShellRow::Pending {
            position,
            last_position,
        } => render_pending_playlist_row(playlist_id, position, last_position),
        PlaylistShellRow::Ready(ready) => {
            render_playlist_track_row(playlist_id, ready.display, ready.slot)
        }
    }
}

fn render_pending_playlist_row(
    playlist_id: i64,
    position: usize,
    last_position: usize,
) -> AnyElement {
    let row_id = SharedString::from(format!(
        "playlist-{playlist_id}-row-{position}-of-{last_position}-pending"
    ));
    div()
        .id(row_id)
        .flex()
        .flex_row()
        .items_center()
        .gap(spacing::SM)
        .px(spacing::SM)
        .py(spacing::XS)
        .rounded(radius::SM)
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
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .rounded(radius::MD)
            .border_1()
            .border_color(color::accent())
            .bg(color::bg_surface())
            .px(spacing::MD)
            .py(spacing::SM)
            .text_size(typography::SIZE_MICRO)
            .font_weight(FontWeight::BOLD)
            .text_color(color::text_primary())
            .child(self.title.clone())
    }
}

fn render_playlist_drop_zone(
    playlist_id: i64,
    drop_index: i64,
    on_reorder: Option<PlaylistReorderHandler>,
) -> AnyElement {
    let mut zone = div()
        .id(SharedString::from(format!(
            "playlist-drop-{playlist_id}-{drop_index}"
        )))
        .h(spacing::XS)
        .rounded(radius::SM);

    if let Some(on_reorder) = on_reorder {
        zone = zone
            .can_drop(move |drag, _window, _cx| {
                drag.downcast_ref::<PlaylistTrackDragPayload>()
                    .is_some_and(|payload| payload.playlist_id == playlist_id)
            })
            .drag_over(
                move |el, payload: &PlaylistTrackDragPayload, _window, _cx| {
                    if payload.playlist_id == playlist_id {
                        el.bg(color::accent())
                    } else {
                        el
                    }
                },
            )
            .on_drop(
                move |payload: &PlaylistTrackDragPayload, window: &mut Window, cx: &mut App| {
                    if payload.playlist_id != playlist_id {
                        return;
                    }
                    if let Some(target) = playlist_reorder_target(payload.from_position, drop_index)
                    {
                        on_reorder(&(payload.from_position, target), window, cx);
                    }
                },
            );
    }

    zone.into_any_element()
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

fn render_playlist_track_row(
    playlist_id: i64,
    display: PlaylistTrackRowDisplay,
    slot: PlaylistTrackRowSlot,
) -> AnyElement {
    let controls = display.controls.clone();
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

    div()
        .id(SharedString::from(controls.row_id.clone()))
        .flex()
        .flex_row()
        .items_center()
        .gap(spacing::SM)
        .px(spacing::SM)
        .py(spacing::XS)
        .rounded(radius::SM)
        .when(!display.is_available, |el| el.opacity(0.55))
        .hover(|el| el.bg(color::bg_surface_hi()))
        .child(render_playlist_drag_handle(&controls, drag_payload))
        .child(render_playlist_track_body(display, thumbnail, on_select))
        .child(render_playlist_track_controls(
            controls,
            PlaylistTrackControlSlots {
                play: on_play,
                move_up: on_move_up,
                move_down: on_move_down,
                remove: on_remove,
            },
        ))
        .into_any_element()
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
) -> AnyElement {
    div()
        .id(SharedString::from(controls.drag_handle_id.clone()))
        .min_w(layout::MIN_HIT_TARGET)
        .min_h(layout::MIN_HIT_TARGET)
        .flex()
        .items_center()
        .justify_center()
        .rounded(radius::SM)
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
        .gap(spacing::XS)
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
    let mut row_body = div()
        .id(SharedString::from(controls.row_body_id))
        .flex()
        .flex_row()
        .items_center()
        .gap(spacing::SM)
        .flex_1()
        .cursor_pointer();
    if let Some(on_select) = on_select {
        row_body = row_body.on_click(move |event, window, cx| on_select(event, window, cx));
    }

    row_body
        .child(
            div()
                .w(layout::PLAYLIST_THUMB_SLOT)
                .text_xs()
                .text_color(color::text_muted())
                .child(SharedString::from(position_label)),
        )
        .child(thumbnail.unwrap_or_else(render_playlist_thumb_placeholder))
        .child(
            div()
                .flex_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(title_color)
                        .child(SharedString::from(title)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(color::text_muted())
                        .child(SharedString::from(artist)),
                )
                .when_some(availability_label, |el, label| {
                    el.child(div().text_xs().text_color(color::text_muted()).child(label))
                }),
        )
        .child(
            div()
                .text_xs()
                .text_color(color::text_muted())
                .w(layout::PLAYLIST_TITLE_OFFSET)
                .child(SharedString::from(duration_label)),
        )
        .into_any_element()
}

fn render_playlist_thumb_placeholder() -> AnyElement {
    div()
        .w(layout::PLAYLIST_THUMB_SLOT)
        .h(layout::PLAYLIST_THUMB_SLOT)
        .rounded(Radius::SM.px())
        .bg(color::border_subtle())
        .flex()
        .items_center()
        .justify_center()
        .text_size(layout::ACTION_ICON_INNER_SIZE)
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
    use super::playlist_reorder_target;

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
}
