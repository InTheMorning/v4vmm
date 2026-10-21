//! The singles group page (ADR 0084 Decision 6).
//!
//! The view model gives each text, the main action and its availability.
//! This shell owns only the layout and the callbacks into `LibraryApp`. A
//! Library single shows as the shared Library track row, and a `MusicIndex`
//! single shows as a release row (packet 026 decision D13).

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, Context, FocusHandle, FontWeight, Image,
    ScrollHandle, SharedString, Styled,
};

use crate::ui::composites::release_section_list::checkbox_state_for;
use crate::ui::composites::{EntityKind, ListRow, ListRowA11yLabel, Thumbnail};
use crate::ui::primitives::{Checkbox, LabelVariant};
use crate::ui::shells::library::index_list_status::{
    render_index_list_status_row, IndexListActions,
};
use crate::view_models::index_name_list::IndexListStatusRowVm;
use crate::view_models::singles_group::{SinglesGroupIndexRowVm, SinglesGroupPageEntryVm};
use crate::view_models::workspace::FrameNavigationEntry;

use crate::db;
use crate::library::{playlist_options, LibraryApp};
use crate::ui::composites::music_column_scroll::music_column_tracked_scrollbar;
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::composites::{
    ActionRow, ActionRowDisplay, AddToPlaylistDisplay, AddToPlaylistPopover, CollectionArt,
    CollectionArtImage, ThumbnailSize,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button as UiButton, Label};
use crate::ui::shells::library::detail::DetailRowSlot;
use crate::ui::shells::library::feed_detail::library_release_page_dispatch;
use crate::ui::shells::release_page::{render_release_page_track_row, ReleasePageDispatch};
use crate::ui::style::spacing;
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::collection_art::CollectionKind;
use crate::view_models::library::MbTrackStatus;
use crate::view_models::release_page::ReleaseTrackRowVm;
use crate::view_models::singles_group::{
    SinglesGroupAction, SinglesGroupActionVm, SinglesGroupPageRowVm, SinglesGroupPageVm,
};
use crate::view_models::{playlist_option_displays, PlaylistOptionDisplayVm};

/// The element id of the main action of a group page.
const MAIN_ACTION_ID: &str = "singles-group-main-action";

/// Renders the page of one singles group (ADR 0084 Decision 6).
#[expect(
    clippy::too_many_arguments,
    reason = "the page composes the view model, the header art, the Library state and the row slots"
)]
pub(crate) fn render_library_singles_group_page(
    page: SinglesGroupPageVm,
    header_art: Option<CollectionArtImage>,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    playlists: &[db::Playlist],
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    row_slots: Vec<DetailRowSlot>,
    scroll: &ScrollHandle,
    detail_focus: &FocusHandle,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let SinglesGroupPageVm {
        title,
        subtitle,
        identity_note,
        rows,
        index_status,
        more_note,
        main_action,
        empty_message,
        art: _,
    } = page;

    let heading = render_heading(title, subtitle, identity_note);
    // ADR 0084 Decision 12: the header art of the group in the logo frame.
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
        // The shared action row of a page header, as on a release page.
        let row_a11y_label = SharedString::from(action.row_a11y_label);
        content = content.child(
            ActionRow::new(ActionRowDisplay {
                a11y_label: row_a11y_label,
            })
            .control_group(vec![render_main_action(action, playlists, cx)]),
        );
    }
    let dispatch = library_release_page_dispatch(cx);
    let playlist_displays = playlist_option_displays(playlists);
    let row_elements: Vec<AnyElement> = rows
        .iter()
        .zip(row_slots)
        .enumerate()
        .map(|(index, (row, slot))| match row {
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
                render_index_single_row(row, album_thumbs, slot, cx)
            }
        })
        .collect();
    content = content.child(
        div()
            .flex()
            .flex_col()
            .gap(spacing::XXS)
            .children(row_elements),
    );
    content = content.children(render_index_part(index_status, more_note, cx));

    let container = div()
        .id("singles-group-page")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .p(spacing::LG)
        .child(content);
    // ADR 0084 packet 018: the page holds the scroll handle of the detail
    // column, so a key can keep the focused row in view.
    music_column_tracked_scrollbar(container, scroll, None)
}

/// The title, the subtitle and the identity note of a group page.
fn render_heading(
    title: String,
    subtitle: Option<String>,
    identity_note: Option<&'static str>,
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
    if let Some(subtitle) = subtitle {
        heading = heading.child(
            Label::new(subtitle)
                .size(FontSize::Caption)
                .color(SemanticColor::SecondaryLabel),
        );
    }
    if let Some(note) = identity_note {
        heading = heading.child(
            Label::new(note)
                .size(FontSize::Caption)
                .color(SemanticColor::TertiaryLabel),
        );
    }
    heading
}

/// The rows after the single rows of a group page (packet 026 Part B): the
/// loading row or the error row of the request for the `MusicIndex`
/// singles of the feed, and the note that more releases exist (decision
/// D11). The Library rows stay above them.
fn render_index_part(
    index_status: Option<IndexListStatusRowVm>,
    more_note: Option<&'static str>,
    cx: &mut Context<LibraryApp>,
) -> Vec<AnyElement> {
    let mut elements = Vec::new();
    if let Some(status) = index_status {
        elements.push(render_index_list_status_row(
            status,
            IndexListActions {
                retry: LibraryApp::retry_singles_group_feed,
                // The page shows no "Load more" row (decision D11).
                load_more: None,
            },
            cx,
        ));
    }
    if let Some(note) = more_note {
        elements.push(
            Label::new(note)
                .size(FontSize::Caption)
                .color(SemanticColor::TertiaryLabel)
                .into_any_element(),
        );
    }
    elements
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
        SinglesGroupAction::DownloadSingles(plan) => {
            UiButton::styled(MAIN_ACTION_ID, ControlStyle::Primary)
                .label(label)
                .a11y_label(a11y_label)
                .disabled(!available)
                .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                    this.download_release_plan(plan.clone(), cx);
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
/// 2026-10-17): the shared track row of a release page, with the track of
/// the single. A click opens the release page of the single. The row shows
/// the date of the single from the view model.
#[expect(
    clippy::too_many_arguments,
    reason = "the row takes each already-resolved value of the Library host"
)]
fn render_single_row(
    row: &SinglesGroupPageRowVm,
    index: usize,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    playlists: &[PlaylistOptionDisplayVm],
    slot: DetailRowSlot,
    detail_focus: &FocusHandle,
    dispatch: &ReleasePageDispatch,
    cx: &App,
) -> AnyElement {
    let vm = ReleaseTrackRowVm::library_single(row, mb_status.get(&row.track.id), slot.selection);
    let thumbnail = vm
        .thumbnail_url
        .as_deref()
        .and_then(|url| album_thumbs.get(url))
        .and_then(Clone::clone);
    render_release_page_track_row(
        &vm,
        index,
        thumbnail,
        slot.probe,
        playlists,
        Some(detail_focus),
        dispatch,
        cx,
    )
}

/// One `MusicIndex` single row of a group page (packet 026 decision D13):
/// the release image, the title and the date, with no state word. A click
/// opens the release page of the single, or changes the selection in
/// selection mode, as a release row of the feed page does.
fn render_index_single_row(
    row: &SinglesGroupIndexRowVm,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    slot: DetailRowSlot,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let thumbnail = row
        .image_url
        .as_deref()
        .and_then(|url| album_thumbs.get(url))
        .and_then(Clone::clone);
    // Column text does not call `truncate()` (ADR 0063,
    // docs/troubleshooting/column-text-truncation.md). `overflow_hidden()`
    // is the required mitigation for text stacked in a flex column.
    let mut text_block = div()
        .flex()
        .flex_col()
        .min_w_0()
        .overflow_hidden()
        .gap(spacing::XXS)
        .child(Label::new(row.title.clone()).variant(LabelVariant::Headline));
    if let Some(date) = &row.date_text {
        text_block = text_block.child(
            Label::new(date.clone())
                .variant(LabelVariant::Caption)
                .color(SemanticColor::SecondaryLabel),
        );
    }
    let DetailRowSlot { selection, probe } = slot;
    let open_entry = FrameNavigationEntry::Release(row.key.clone());
    let mut list_row = ListRow::new(SharedString::from(row.element_id.clone()))
        .selected(selection.is_marked())
        .focused(selection.focused)
        .scroll_probe(probe);
    if selection.checkbox_visible {
        let toggle_entry = open_entry.clone();
        list_row = list_row.checkbox(
            Checkbox::new(
                SharedString::from(row.checkbox_id.clone()),
                checkbox_state_for(selection.check_state),
            )
            .a11y_label(selection.checkbox_a11y_label.clone())
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                this.toggle_library_detail_row(&toggle_entry, cx);
            }))
            .into_any_element(),
        );
    }
    list_row
        .a11y_label(ListRowA11yLabel {
            label: SharedString::from(row.a11y_label.clone()),
        })
        .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
            this.click_library_detail_row(&open_entry, event.modifiers(), cx);
        }))
        .child(Thumbnail::new(EntityKind::Release, ThumbnailSize::Sm).image(thumbnail))
        .child(div().flex_1().min_w_0().child(text_block))
        .into_any_element()
}
