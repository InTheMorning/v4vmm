//! The Library singles group page (ADR 0084 Decision 6).
//!
//! The view model gives each text, the main action and its availability.
//! This shell owns only the layout and the callbacks into `LibraryApp`.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, ClickEvent, Context, FontWeight, Image, SharedString, Styled,
};

use crate::db;
use crate::library::{playlist_options, LibraryApp};
use crate::ui::composites::music_column_scroll::music_column_scrollbar;
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::composites::{
    ActionRow, ActionRowDisplay, AddToPlaylistDisplay, AddToPlaylistPopover,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button as UiButton, Label};
use crate::ui::shells::library::feed_detail::{render_library_track_row, LibraryTrackRowContext};
use crate::ui::style::spacing;
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::library::MbTrackStatus;
use crate::view_models::singles_group::{
    SinglesGroupAction, SinglesGroupActionVm, SinglesGroupPageRowVm, SinglesGroupPageVm,
};

/// The element id of the main action of a group page.
const MAIN_ACTION_ID: &str = "singles-group-main-action";

/// Renders the page of one singles group (ADR 0084 Decision 6).
pub(crate) fn render_library_singles_group_page(
    page: SinglesGroupPageVm,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    playlists: &[db::Playlist],
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let SinglesGroupPageVm {
        title,
        subtitle,
        identity_note,
        rows,
        main_action,
        empty_message,
    } = page;

    let mut content = page_scroll_content(cx).gap(spacing::SM).child(
        Label::new(title)
            .size(FontSize::Title2)
            .weight(FontWeight::SEMIBOLD),
    );
    if let Some(subtitle) = subtitle {
        content = content.child(
            Label::new(subtitle)
                .size(FontSize::Caption)
                .color(SemanticColor::SecondaryLabel),
        );
    }
    if let Some(note) = identity_note {
        content = content.child(
            Label::new(note)
                .size(FontSize::Caption)
                .color(SemanticColor::TertiaryLabel),
        );
    }
    if let Some(message) = empty_message {
        content = content.child(
            Label::new(message)
                .size(FontSize::Body)
                .color(SemanticColor::TertiaryLabel),
        );
    }
    if let Some(action) = main_action {
        // The shared action row of a page header, as on a release page.
        let row_a11y_label = SharedString::from(action.row_a11y_label);
        content = content.child(
            ActionRow::new(ActionRowDisplay {
                a11y_label: row_a11y_label,
            })
            .control_group(vec![render_main_action(action, playlists, cx)]),
        );
    }
    content = content.child(
        div().flex().flex_col().gap(spacing::XXS).children(
            rows.into_iter()
                .map(|row| render_single_row(row, mb_status, album_thumbs, playlists, cx)),
        ),
    );

    let container = div()
        .id("singles-group-page")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .p(spacing::LG)
        .child(content);
    music_column_scrollbar(container, "singles-group-page-scrollbar")
}

/// The one main action of a group page: "Download n singles" or "Add to
/// playlist". "Add to playlist" opens the shared playlist popover.
fn render_main_action(
    action: SinglesGroupActionVm,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let SinglesGroupActionVm {
        action,
        label,
        a11y_label,
        available,
        new_playlist_a11y_label,
        back_a11y_label,
        create_a11y_label,
        row_a11y_label: _,
    } = action;
    match action {
        SinglesGroupAction::DownloadSingles(feed_ids) => {
            UiButton::styled(MAIN_ACTION_ID, ControlStyle::Primary)
                .label(label)
                .a11y_label(a11y_label)
                .disabled(!available)
                .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                    this.download_singles(&feed_ids, cx);
                }))
                .into_any_element()
        }
        SinglesGroupAction::AddToPlaylist(feed_ids) => {
            let create_feed_ids = feed_ids.clone();
            AddToPlaylistPopover::new(AddToPlaylistDisplay {
                id: SharedString::from(MAIN_ACTION_ID),
                playlists: playlist_options(playlists),
                trigger_label: SharedString::from(label),
                trigger_a11y_label: SharedString::from(a11y_label),
                new_playlist_a11y_label: SharedString::from(new_playlist_a11y_label),
                back_a11y_label: SharedString::from(back_a11y_label),
                create_a11y_label: SharedString::from(create_a11y_label),
            })
            .trigger_style(ControlStyle::Primary)
            .disabled(!available)
            .on_select(cx.listener(move |this, playlist_id: &i64, _window, cx| {
                this.add_albums_to_playlist(&feed_ids, *playlist_id, cx);
            }))
            .on_create(cx.listener(move |this, name: &String, _window, cx| {
                this.create_playlist_and_add_albums(name, create_feed_ids.clone(), cx);
            }))
            .into_any_element()
        }
    }
}

/// One single row of a group page (ADR 0084 task 015, operator decision of
/// 2026-10-17): the shared Library track row of a release page, with the
/// track of the single. A click opens the release page of the single. The
/// row shows the date of the single from the view model.
fn render_single_row(
    row: SinglesGroupPageRowVm,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let SinglesGroupPageRowVm {
        track,
        date_text,
        busy,
        key,
    } = row;
    render_library_track_row(
        &track,
        mb_status,
        busy,
        album_thumbs,
        playlists,
        LibraryTrackRowContext::SinglesGroupPage { key, date_text },
        cx,
    )
}
