//! The window group page of New releases (ADR 0086 Decision 4).
//!
//! The view model gives each text, the rows, the main action and its
//! availability, and the link to the group page of the artist. This shell
//! owns only the layout and the callbacks into `LibraryApp`. It uses the
//! row renderers and the main action of the singles group page, so the two
//! pages show one row style (packet 004 decision D2).

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, Context, FocusHandle, FontWeight, Image, ScrollHandle,
    SharedString, Styled,
};

use crate::db;
use crate::library::LibraryApp;
use crate::ui::composites::music_column_scroll::music_column_tracked_scrollbar;
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::composites::{
    ActionRow, ActionRowDisplay, CollectionArt, CollectionArtImage, ThumbnailSize,
};
use crate::ui::primitives::Label;
use crate::ui::shells::entity::render_album_name_links;
use crate::ui::shells::library::detail::DetailRowSlot;
use crate::ui::shells::library::feed_detail::library_release_page_dispatch;
use crate::ui::shells::library::singles_group::{
    render_index_single_row, render_main_action, render_matched_by_name_heading, render_single_row,
};
use crate::ui::style::spacing;
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::album_page::AlbumNameLinkVm;
use crate::view_models::collection_art::CollectionKind;
use crate::view_models::library::MbTrackStatus;
use crate::view_models::playlist_option_displays;
use crate::view_models::singles_group::SinglesGroupPageEntryVm;
use crate::view_models::window_group_page::WindowGroupPageVm;

/// The element id of the main action of a window group page.
const MAIN_ACTION_ID: &str = "window-group-main-action";

/// Renders the page of one window group (ADR 0086 Decision 4).
#[expect(
    clippy::too_many_arguments,
    reason = "the page composes the view model, the header art, the Library state and the row slots"
)]
pub(crate) fn render_library_window_group_page(
    page: WindowGroupPageVm,
    header_art: Option<CollectionArtImage>,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    playlists: &[db::Playlist],
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    row_slots: Vec<DetailRowSlot>,
    scroll: &ScrollHandle,
    detail_focus: &FocusHandle,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let WindowGroupPageVm {
        title,
        date_span,
        rows,
        matched_by_name,
        main_action,
        all_singles_link,
        art: _,
        empty_message,
    } = page;

    let heading = render_window_heading(title, date_span, all_singles_link, cx);
    // ADR 0084 Decision 12: the header art in the logo frame.
    let art = header_art
        .map(|image| CollectionArt::new(image, CollectionKind::SinglesGroup, ThumbnailSize::XXl));
    let mut content = page_scroll_content(cx).gap(spacing::SM).child(
        div()
            .flex()
            .flex_row()
            .items_start()
            .gap(spacing::LG)
            .children(art)
            .child(heading),
    );
    if let Some(message) = empty_message {
        content = content.child(
            Label::new(message)
                .size(FontSize::Body)
                .color(SemanticColor::TertiaryLabel),
        );
    }
    if let Some(action) = main_action {
        let row_a11y_label = SharedString::from(action.row_a11y_label);
        content = content.child(
            ActionRow::new(ActionRowDisplay {
                a11y_label: row_a11y_label,
            })
            .control_group(vec![render_main_action(
                MAIN_ACTION_ID,
                action,
                playlists,
                cx,
            )]),
        );
    }
    let dispatch = library_release_page_dispatch(cx);
    let playlist_displays = playlist_option_displays(playlists);
    // The trailing "Matched by name" rows get the heading of the singles
    // group page before them (packet 004 decision D6).
    let heading_at = matched_by_name.as_ref().map_or(rows.len(), |heading| {
        rows.len().saturating_sub(heading.count)
    });
    let mut row_elements: Vec<AnyElement> = Vec::with_capacity(rows.len() + 1);
    for (index, (row, slot)) in rows.iter().zip(row_slots).enumerate() {
        if index == heading_at {
            if let Some(heading) = &matched_by_name {
                row_elements.push(render_matched_by_name_heading(heading));
            }
        }
        row_elements.push(match row {
            SinglesGroupPageEntryVm::Library(row) => render_single_row(
                row,
                index,
                mb_status,
                album_thumbs,
                &playlist_displays,
                slot,
                detail_focus,
                &dispatch,
                cx,
            ),
            SinglesGroupPageEntryVm::Index(row) => {
                render_index_single_row(row, album_thumbs, slot, detail_focus, cx)
            }
        });
    }
    content = content.child(
        div()
            .flex()
            .flex_col()
            .gap(spacing::XXS)
            .children(row_elements),
    );

    let container = div()
        .id("window-group-page")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .p(spacing::LG)
        .child(content);
    // The page holds the scroll handle of the detail column, so a key can
    // keep the focused row in view (ADR 0084 packet 018).
    music_column_tracked_scrollbar(container, scroll, None)
}

/// The title, the date span and the link to the singles group page of a
/// window group page. The link opens the group page in the current path.
fn render_window_heading(
    title: String,
    date_span: Option<String>,
    all_singles_link: Option<AlbumNameLinkVm>,
    cx: &mut Context<LibraryApp>,
) -> gpui::Div {
    let mut heading = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .gap(spacing::SM)
        .child(
            Label::new(title)
                .size(FontSize::Title2)
                .weight(FontWeight::SEMIBOLD),
        );
    if let Some(date_span) = date_span {
        heading = heading.child(
            Label::new(date_span)
                .size(FontSize::Caption)
                .color(SemanticColor::SecondaryLabel),
        );
    }
    let entity = cx.entity();
    heading.children(render_album_name_links(
        all_singles_link.into_iter().collect(),
        move |target, _window, cx| {
            entity.update(cx, |this, cx| this.open_album_name_link(target, cx));
        },
        cx,
    ))
}
