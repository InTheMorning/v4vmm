//! The singles group page (ADR 0084 Decision 6).
//!
//! The view model gives each text, the main action and its availability.
//! This shell owns only the layout and the callbacks into `LibraryApp`. A
//! Library single and a `MusicIndex` single both show as the track row
//! of the Index release page (operator decision of 2026-10-22, replacing
//! packet 026 decision D13).

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, Context, FocusHandle, FontWeight, Image,
    ScrollHandle, SharedString, Styled,
};

use crate::ui::primitives::{LabelVariant, SectionHeader};
use crate::ui::shells::library::index_list_status::{
    render_index_list_status_row, IndexListActions,
};
use crate::view_models::index_name_list::IndexListStatusRowVm;
use crate::view_models::singles_group::{
    MatchedByNameHeadingVm, SinglesGroupIndexRowVm, SinglesGroupPageEntryVm,
};
use crate::view_models::workspace::ReleaseKey;

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
        matched_by_name,
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
    // The trailing "Matched by name" rows get a heading before them
    // (operator decision of 2026-10-22, packet 032). `rows` stays one
    // flat list; the heading's own row count gives its position.
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

/// The heading and the note of the trailing "Matched by name" rows
/// (operator decision of 2026-10-22, packet 032), styled the same as the
/// feed page's section of the same name: a `SectionHeader` and a
/// `Caption` note.
pub(super) fn render_matched_by_name_heading(heading: &MatchedByNameHeadingVm) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(spacing::XS)
        .pt(spacing::SM)
        .child(SectionHeader::new(heading.heading))
        .child(
            Label::new(heading.note)
                .variant(LabelVariant::Caption)
                .color(SemanticColor::SecondaryLabel),
        )
        .into_any_element()
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
/// playlist". "Add to playlist" opens the shared playlist popover. `id` is
/// the element id of the action. The window group page uses its own id
/// (ADR 0086 packet 004).
pub(super) fn render_main_action(
    id: &'static str,
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
        SinglesGroupAction::DownloadSingles(plan) => UiButton::styled(id, ControlStyle::Primary)
            .label(label)
            .a11y_label(a11y_label)
            .disabled(!available)
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.download_release_plan(plan.clone(), cx);
            }))
            .into_any_element(),
        SinglesGroupAction::AddToPlaylist(feed_ids) => {
            let create_feed_ids = feed_ids.clone();
            AddToPlaylistPopover::new(AddToPlaylistDisplay {
                id: SharedString::from(id),
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
pub(super) fn render_single_row(
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

/// One `MusicIndex` single row of a group page: the track row of the
/// Index release page (operator decision of 2026-10-22, replacing packet
/// 026 decision D13). A click opens the release page of the single: a
/// `MusicIndex` single has no Library track and no track page of its own.
pub(super) fn render_index_single_row(
    row: &SinglesGroupIndexRowVm,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    slot: DetailRowSlot,
    detail_focus: &FocusHandle,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let vm = ReleaseTrackRowVm::index_single(
        row.title.clone(),
        row.duration.clone(),
        row.date_text.clone(),
        row.image_url.clone(),
        row.key.clone(),
        row.element_id.clone(),
        slot.selection,
        row.work,
    );
    let thumbnail = vm
        .thumbnail_url
        .as_deref()
        .and_then(|url| album_thumbs.get(url))
        .and_then(Clone::clone);
    let dispatch = index_single_dispatch(row.key.clone(), row.title.clone(), cx);
    render_release_page_track_row(
        &vm,
        0,
        thumbnail,
        slot.probe,
        &[],
        Some(detail_focus),
        &dispatch,
        cx,
    )
}

/// The dispatch of one `MusicIndex` single's row (operator decision of
/// 2026-10-22). The row has no Library track, so a shared, index-based
/// dispatch cannot look it up: each row keeps its own key and title, and
/// sends its commands to `LibraryApp::run_singles_group_index_row_command`.
fn index_single_dispatch(
    key: ReleaseKey,
    title: String,
    cx: &Context<LibraryApp>,
) -> ReleasePageDispatch {
    let entity = cx.entity();
    Rc::new(move |command, window, cx| {
        entity.update(cx, |this, cx| {
            this.run_singles_group_index_row_command(&key, &title, &command, window, cx);
        });
    })
}
