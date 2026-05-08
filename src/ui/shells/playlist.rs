//! Playlist detail shell.
//!
//! The shell owns the playlist page hierarchy and row chrome. Screens provide
//! thumbnail images and command callbacks through behavior slots.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, InteractiveElement, ParentElement, SharedString,
    Styled, Window,
};

use crate::ui::composites::{
    DetailGrid, DetailHeader, DetailHeaderDisplay, DetailRow, DetailTextRow, EntityKind,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button as UiButton;
use crate::ui::style::{color, radius, spacing};
use crate::ui::{layouts as layout, tokens::Radius};
use crate::view_models::library::{PlaylistTrackControlsDisplay, PlaylistTrackRowDisplay};
use crate::view_models::playlist_detail::PlaylistDetailPageVm;

type PlaylistClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(Default)]
pub(crate) struct PlaylistDetailBehaviorSlots {
    pub(crate) on_rename: Option<PlaylistClickHandler>,
    pub(crate) on_delete: Option<PlaylistClickHandler>,
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
    pub(crate) on_move_up: Option<PlaylistClickHandler>,
    pub(crate) on_move_down: Option<PlaylistClickHandler>,
    pub(crate) on_remove: Option<PlaylistClickHandler>,
}

#[must_use]
pub(crate) fn click_slot<F>(handler: F) -> PlaylistClickHandler
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    Rc::new(handler)
}

#[must_use]
pub(crate) fn render_playlist_detail_shell(
    page: &PlaylistDetailPageVm<'_>,
    slots: PlaylistDetailBehaviorSlots,
) -> AnyElement {
    let header_display = page.header_display();
    let track_rows = if slots.track_rows.is_empty() {
        vec![render_empty_message(page.empty_message())]
    } else {
        slots
            .track_rows
            .into_iter()
            .map(|row| render_playlist_shell_row(page.playlist_id(), row))
            .collect()
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
            slots.on_rename,
            slots.on_delete,
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
    on_rename: Option<PlaylistClickHandler>,
    on_delete: Option<PlaylistClickHandler>,
) -> AnyElement {
    let actions = page.actions_display();
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
            .label(actions.rename_label),
            on_rename,
        ))
        .child(apply_click_handler(
            UiButton::styled(
                SharedString::from(actions.delete_button_id),
                ControlStyle::Destructive,
            )
            .label(actions.delete_label),
            on_delete,
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

fn render_playlist_track_row(
    playlist_id: i64,
    display: PlaylistTrackRowDisplay,
    slot: PlaylistTrackRowSlot,
) -> AnyElement {
    let _ = playlist_id;
    let controls = display.controls.clone();
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
    move_up: Option<PlaylistClickHandler>,
    move_down: Option<PlaylistClickHandler>,
    remove: Option<PlaylistClickHandler>,
}

fn render_playlist_track_controls(
    controls: PlaylistTrackControlsDisplay,
    slots: PlaylistTrackControlSlots,
) -> AnyElement {
    let PlaylistTrackControlsDisplay {
        play_button_id,
        play_label,
        play_enabled,
        move_up_button_id,
        move_up_label,
        move_up_enabled,
        move_down_button_id,
        move_down_label,
        move_down_enabled,
        remove_button_id,
        remove_label,
        ..
    } = controls;

    let play_btn = apply_click_handler(
        UiButton::styled(SharedString::from(play_button_id), ControlStyle::RowAction)
            .label(play_label)
            .disabled(!play_enabled),
        slots.play,
    );
    let up_btn = apply_click_handler(
        UiButton::styled(
            SharedString::from(move_up_button_id),
            ControlStyle::RowAction,
        )
        .label(move_up_label)
        .disabled(!move_up_enabled),
        slots.move_up,
    );
    let down_btn = apply_click_handler(
        UiButton::styled(
            SharedString::from(move_down_button_id),
            ControlStyle::RowAction,
        )
        .label(move_down_label)
        .disabled(!move_down_enabled),
        slots.move_down,
    );
    let remove_btn = apply_click_handler(
        UiButton::styled(
            SharedString::from(remove_button_id),
            ControlStyle::Destructive,
        )
        .label(remove_label),
        slots.remove,
    );

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(spacing::XS)
        .child(play_btn)
        .child(up_btn)
        .child(down_btn)
        .child(remove_btn)
        .into_any_element()
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
