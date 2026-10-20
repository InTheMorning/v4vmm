//! Library feed (album) detail surface.
//!
//! Renders the right-hand pane when a Library feed is selected. Mutations and
//! selected-entity state stay on `LibraryApp`; this module only builds shell
//! slots and routes callbacks back through `cx.listener(...)`.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{div, prelude::*, AnyElement, ClipboardItem, Context, Image, SharedString, Styled};

use crate::db::{self, TrackRow};
use crate::library::{playlist_options, LibraryApp};
use crate::media::cover_color::CoverColor;
use crate::ui::composites::{
    identity_action_button, ActionRow, ActionRowDisplay, AddToPlaylistDisplay,
    AddToPlaylistPopover, DisclosureTextPanel, DisclosureTextPanelDisplay,
    IdentityActionButtonDisplay, IdentityActionKind, ReleaseSurfaceElement, StatusRole,
    TrackRow as TrackRowComposite,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{
    Button as UiButton, ContextMenu, ContextMenuItem, ContextMenuItemDisplay, ContextMenuScope,
};
use crate::ui::shells::entity::{
    render_album_name_links, render_contributor_panel, render_feed_identity_actions,
    render_named_feed_rows, render_release_detail_shell, ContributorRowSlot,
    ReleaseDetailBehaviorSlots,
};
use crate::ui::shells::library::detail::{
    with_detail_row_slot, DetailRowSlot, LibraryTrackRowPlace,
};
use crate::ui::style::{color, typography};
use crate::view_models::album_page::{
    album_page_actions, AlbumPageAction, AlbumPageActionDisplay, AlbumPageActions, AlbumPageOrigin,
    ALBUM_PAGE_MENU_A11Y_LABEL,
};
use crate::view_models::entity_detail::{
    ContributorIdentityActionDisplay, ContributorIdentityActionKind, ContributorRowVm,
    EntityActionKind, EntityActionTone, EntityActionVm, EntitySurfaceContext, ReleaseDetailVm,
};
use crate::view_models::library::{
    AlbumNode, DescriptionDisclosureDisplay, LibraryAlbumDetailVm, LibraryTrackRowDisplay,
    LibraryTrackRowVm, LibraryViewModel, MbStatusKind, MbTrackStatus,
};
use crate::view_models::track_detail::{TrackDetailSurfaceContext, TrackDetailVm, TrackRowVm};
use crate::view_models::workspace::{FrameNavigationEntry, ReleaseKey};
use crate::views::{FeedView, TrackView};

#[expect(
    clippy::too_many_lines,
    reason = "lifted legacy feed-detail surface stays intact during Task 007 decomposition"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "the cover color of the backdrop (ADR 0083 task 005) adds one already-resolved value to an existing page renderer"
)]
pub(crate) fn render_library_feed_detail(
    album: &AlbumNode,
    busy_track: Option<i64>,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    library_vm: &LibraryViewModel,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cover_color: Option<CoverColor>,
    playlists: &[db::Playlist],
    row_slots: Vec<DetailRowSlot>,
    scroll: &gpui::ScrollHandle,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let feed_row = db::FeedRow {
        id: album.feed_id.unwrap_or(0),
        feed_url: album.feed_url_for_detail(),
        feed_guid: album.feed_guid.clone(),
        title: Some(album.name.clone()),
        language: album.language.clone(),
        description: LibraryViewModel::display_description_text(album.description.as_deref())
            .map(str::to_owned),
        album_image_href: album.image_href.clone(),
        is_subscribed: false,
    };
    let track_views = album
        .tracks
        .clone()
        .into_iter()
        .map(TrackView::from_local)
        .collect();
    // ADR 0075 packet 020: the query layer projects the stored values of the
    // feed. An album without a feed row has only its columns.
    let mut feed_view = match album.stored_values.as_deref() {
        Some(values) => FeedView::from_local_with_facts(
            feed_row,
            track_views,
            album.identity_facts.clone(),
            values.clone(),
        ),
        None => {
            FeedView::from_local_with_identity(feed_row, track_views, album.identity_facts.clone())
        }
    };
    // ADR 0085 Decisions 5 and 6: the query layer already read the stored
    // named feeds of this album. The screen adds no request.
    feed_view.named_feeds.clone_from(&album.named_feeds);

    let thumb_image = album
        .image_href
        .as_ref()
        .and_then(|url| album_thumbs.get(url.as_str()))
        .and_then(Clone::clone);

    let vm = LibraryAlbumDetailVm::new(&album.tracks, mb_status);
    let feed_busy = album
        .feed_id
        .is_some_and(|feed_id| library_vm.busy_feed() == Some(feed_id));
    // ADR 0084 Decisions 4 and 8 (operator decision of 2026-10-16): the
    // release page reads no list filter. It always shows each track of
    // the release, each with its own state word.
    let track_rows: Vec<ReleaseSurfaceElement> = album
        .release_track_rows()
        .iter()
        .zip(row_slots)
        .map(|(track, slot)| {
            ReleaseSurfaceElement::from_element(render_library_track_row(
                track,
                mb_status,
                vm.track_row_busy(track, busy_track == Some(track.id), feed_busy),
                album_thumbs,
                playlists,
                LibraryTrackRowContext::ReleasePage.with_slot(slot),
                cx,
            ))
        })
        .collect();

    let album_actions = album_page_actions(
        AlbumPageOrigin::Library {
            membership: vm.membership(feed_busy),
            musicbrainz_available: !vm.has_active_musicbrainz(),
        },
        album.feed_url.as_deref(),
    );
    let action_row = render_library_album_actions(album, &album_actions, playlists, cx);

    let projection = ReleaseDetailVm::new(&feed_view, EntitySurfaceContext::Library);
    let page = projection.page();
    let entity = cx.entity();
    // ADR 0084 Decision 6: a single in a Library group links to its group.
    let links =
        library_vm.release_page_name_links(projection.name_links(), album.release_key().as_ref());
    let open_name_link = move |target: &_, _window: &mut _, cx: &mut _| {
        entity.update(cx, |this, cx| this.open_album_name_link(target, cx));
    };
    let name_links = render_album_name_links(links, open_name_link.clone(), cx);
    let named_feed_rows = render_named_feed_rows(projection.named_feed_rows(), open_name_link, cx);
    let mut slots = ReleaseDetailBehaviorSlots {
        hero_image: thumb_image,
        cover_color,
        name_links,
        named_feed_rows,
        primary_actions: vec![ReleaseSurfaceElement::from_element(action_row)],
        identity_actions: render_feed_identity_actions(&page),
        track_rows: Some(track_rows),
        scroll_handle: Some(scroll.clone()),
        ..ReleaseDetailBehaviorSlots::default()
    };
    if let Some(panel) = render_library_contributors_panel(&projection, album_thumbs) {
        slots
            .after_section
            .push(ReleaseSurfaceElement::from_element(panel));
    }
    if let (Some(feed_id), Some(description)) = (
        album.feed_id,
        LibraryViewModel::display_description_text(album.description.as_deref()),
    ) {
        let description_state = library_vm.album_description_state(feed_id, Some(description));
        let display = DescriptionDisclosureDisplay::project(description, description_state);
        slots.description_panel = Some(ReleaseSurfaceElement::from_element(
            DisclosureTextPanel::new(DisclosureTextPanelDisplay {
                id: (
                    "library-feed-description",
                    usize::try_from(feed_id).unwrap_or_default(),
                )
                    .into(),
                label: SharedString::from("Description"),
                a11y_label: SharedString::from("Toggle feed description"),
                collapsed: display.collapsed,
                body: display.body.map(SharedString::from),
                show_more: display.show_more.map(|show_more| {
                    (
                        SharedString::from(show_more.label),
                        SharedString::from(show_more.a11y_label),
                    )
                }),
            })
            .on_toggle(cx.listener(move |this, _, _, cx| {
                this.toggle_album_description(feed_id, cx);
            }))
            .into_any_element(),
        ));
    }
    render_release_detail_shell(&page, slots)
}

/// The action row of a Library album page (ADR 0083 Decision 5): one filled
/// button and the "⋯" menu.
fn render_library_album_actions(
    album: &AlbumNode,
    actions: &AlbumPageActions,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let mut controls = vec![render_library_album_button(
        actions.filled,
        album,
        playlists,
        cx,
    )];
    controls.extend(
        actions
            .plain
            .iter()
            .map(|action| render_library_album_button(*action, album, playlists, cx)),
    );
    if !actions.menu.is_empty() {
        controls.push(render_library_album_menu(&actions.menu, album, cx));
    }
    ActionRow::new(ActionRowDisplay {
        a11y_label: SharedString::from("Album actions"),
    })
    .control_group(controls)
    .into_any_element()
}

/// The filled button of a Library album page. "Add to playlist" opens the
/// shared playlist popover.
fn render_library_album_button(
    display: AlbumPageActionDisplay,
    album: &AlbumNode,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let action = display.action;
    let feed_id = album.feed_id;
    let kind = album.release_kind();
    if let (AlbumPageAction::AddToPlaylist, Some(feed_id)) = (action, feed_id) {
        return AddToPlaylistPopover::new(AddToPlaylistDisplay {
            id: SharedString::from(format!("album-feed-add:{feed_id}")),
            playlists: playlist_options(playlists),
            trigger_label: SharedString::from(action.label(kind)),
            trigger_a11y_label: SharedString::from(action.a11y_label(kind)),
            new_playlist_a11y_label: SharedString::from("Create a new playlist"),
            back_a11y_label: SharedString::from("Back to playlist choices"),
            create_a11y_label: SharedString::from("Create playlist and add album"),
        })
        .trigger_style(ControlStyle::Primary)
        .disabled(!display.available)
        .on_select(cx.listener(move |this, playlist_id: &i64, _window, cx| {
            this.add_album_to_playlist(feed_id, *playlist_id, cx);
        }))
        .on_create(cx.listener(move |this, name: &String, _window, cx| {
            this.create_playlist_and_add_album(name, feed_id, cx);
        }))
        .into_any_element();
    }
    let available = display.available && feed_id.is_some();
    let button = UiButton::styled(
        SharedString::from(format!("album-page-action:{action:?}")),
        ControlStyle::Primary,
    )
    .label(action.label(kind))
    .a11y_label(action.a11y_label(kind))
    .disabled(!available);
    let (true, Some(feed_id)) = (available, feed_id) else {
        return button.into_any_element();
    };
    let album = album.clone();
    button
        .on_click(cx.listener(move |this, _, window, cx| {
            run_library_album_action(this, action, feed_id, &album, window, cx);
        }))
        .into_any_element()
}

/// The "⋯" menu of a Library album page. The context menu draws a divider
/// before the destructive item.
fn render_library_album_menu(
    items: &[AlbumPageActionDisplay],
    album: &AlbumNode,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let entity = cx.entity();
    let feed_id = album.feed_id;
    let kind = album.release_kind();
    let menu_items = items.iter().map(|display| {
        let action = display.action;
        let available = display.available && feed_id.is_some();
        let mut item = ContextMenuItem::new(ContextMenuItemDisplay {
            id: SharedString::from(format!("album-page-menu:{action:?}")),
            label: SharedString::from(action.label(kind)),
            a11y_label: SharedString::from(action.a11y_label(kind)),
            destructive: action.is_destructive(),
            disabled: !available,
        });
        if let (true, Some(feed_id)) = (available, feed_id) {
            let entity = entity.clone();
            let album = album.clone();
            item = item.on_select(move |window, cx| {
                entity.update(cx, |this, cx| {
                    run_library_album_action(this, action, feed_id, &album, window, cx);
                });
            });
        }
        item
    });
    ContextMenu::new(
        SharedString::from(format!("album-page-menu-{}", feed_id.unwrap_or_default())),
        ContextMenuScope::TrackList,
        SharedString::from(ALBUM_PAGE_MENU_A11Y_LABEL),
    )
    .trigger_label("")
    .items(menu_items)
    .into_any_element()
}

/// Runs the command of an album page action on a Library album.
fn run_library_album_action(
    this: &mut LibraryApp,
    action: AlbumPageAction,
    feed_id: i64,
    album: &AlbumNode,
    window: &mut gpui::Window,
    cx: &mut Context<LibraryApp>,
) {
    match action {
        AlbumPageAction::DownloadAlbum => this.download_feed(feed_id, cx),
        AlbumPageAction::CopyFeedUrl => {
            if let Some(url) = album.feed_url.as_deref() {
                cx.write_to_clipboard(ClipboardItem::new_string(url.to_owned()));
            }
        }
        AlbumPageAction::MusicBrainzLookup => this.musicbrainz_feed(album.clone(), cx),
        AlbumPageAction::RemoveAlbum => this.remove_album_after_confirmation(feed_id, window, cx),
        // "Add to playlist" opens its popover.
        AlbumPageAction::AddToPlaylist => {}
    }
    cx.notify();
}

fn render_library_contributors_panel(
    projection: &ReleaseDetailVm<'_>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
) -> Option<AnyElement> {
    let display = projection.contributor_panel_display();
    render_contributor_panel(
        display.id,
        display.title,
        projection.contributors(),
        |contributor| {
            let thumbnail = contributor
                .image_url()
                .and_then(|url| album_thumbs.get(url))
                .and_then(Clone::clone);
            ContributorRowSlot {
                thumbnail,
                actions: library_contributor_identity_actions(contributor),
            }
        },
    )
}

fn library_contributor_identity_actions(
    contributor: &ContributorRowVm<'_>,
) -> Vec<ReleaseSurfaceElement> {
    contributor
        .identity_actions()
        .into_iter()
        .map(|action| {
            let ContributorIdentityActionDisplay {
                id,
                kind,
                target,
                a11y_label,
            } = action;
            let target_for_click = target;
            let a11y_label = SharedString::from(a11y_label);
            match kind {
                ContributorIdentityActionKind::Website => {
                    identity_action_button(IdentityActionButtonDisplay {
                        id: SharedString::from(id),
                        kind: IdentityActionKind::Website,
                        a11y_label,
                    })
                    .on_click(move |_, _, _| {
                        let _ = open::that(&target_for_click);
                    })
                    .into_any_element()
                }
                ContributorIdentityActionKind::Nostr => {
                    identity_action_button(IdentityActionButtonDisplay {
                        id: SharedString::from(id),
                        kind: IdentityActionKind::Nostr,
                        a11y_label,
                    })
                    .on_click(move |_, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(target_for_click.clone()));
                    })
                    .into_any_element()
                }
            }
        })
        .map(ReleaseSurfaceElement::from_element)
        .collect()
}

/// The page that shows a Library track row. It selects the page that a
/// click opens and the date text of the row.
pub(crate) enum LibraryTrackRowContext {
    /// A release page row: a click opens the track page. The row shows the
    /// date of the track.
    ReleasePage,
    /// A singles group page row (ADR 0084 task 015, operator decision of
    /// 2026-10-17): a click opens the release page of the single. The row
    /// shows `date_text`, the date of the single from the view model.
    SinglesGroupPage {
        key: ReleaseKey,
        date_text: Option<String>,
    },
}

impl LibraryTrackRowContext {
    /// Gives the page that a click on the row of `track` opens. A group
    /// page row also puts the date text of its single into `row_vm`.
    fn apply(self, track: &TrackRow, row_vm: &mut TrackRowVm) -> FrameNavigationEntry {
        match self {
            Self::ReleasePage => FrameNavigationEntry::Track(track.track_key()),
            Self::SinglesGroupPage { key, date_text } => {
                row_vm.date_text = date_text;
                FrameNavigationEntry::Release(key)
            }
        }
    }
}

/// The track row of a Library release page. The singles group page shows
/// each single with this row too.
pub(crate) fn render_library_track_row(
    track: &TrackRow,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    is_busy: bool,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    playlists: &[db::Playlist],
    place: LibraryTrackRowPlace,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let track_for_click = track.clone();
    let track_id = track.id;
    let vm = LibraryTrackRowVm::new(track, mb_status.get(&track_id));
    let EntityActionVm {
        kind,
        label,
        enabled,
        tone,
        ..
    } = vm.primary_action_vm(is_busy);
    let LibraryTrackRowDisplay {
        row_id,
        toggle_button_id,
        state_label,
    } = vm.row_display();
    let in_library = kind == EntityActionKind::Remove;
    let mb_text = vm.mb_status_text();
    let mb_kind = vm.mb_status_kind();
    let thumbnail = track
        .track_image_href
        .as_ref()
        .or(track.album_image_href.as_ref())
        .and_then(|url| album_thumbs.get(url.as_str()))
        .and_then(Clone::clone);
    let primary_style = match tone {
        EntityActionTone::DestructiveQuiet => ControlStyle::DestructiveRowAction,
        _ => ControlStyle::RowAction,
    };

    let toggle_button = UiButton::styled(SharedString::from(toggle_button_id), primary_style)
        .label(label)
        .disabled(!enabled)
        .on_click(cx.listener(move |this, _, window, cx| {
            if in_library {
                this.remove_track(track_id, window, cx);
            } else {
                this.subscribe_track(track_for_click.clone(), cx);
            }
            cx.notify();
        }));
    let mut actions = vec![toggle_button.into_any_element()];

    if let Some(text) = mb_text {
        let status_color = match mb_kind {
            Some(MbStatusKind::Success) => StatusRole::Success.color(cx),
            Some(MbStatusKind::Danger) => StatusRole::Danger.color(cx),
            Some(MbStatusKind::Warning) => StatusRole::Warning.color(cx),
            _ => color::text_muted(),
        };
        actions.push(
            div()
                .text_size(typography::SIZE_MICRO)
                .text_color(status_color)
                .child(SharedString::from(text))
                .into_any_element(),
        );
    }
    if let Some(label) = state_label {
        actions.push(
            div()
                .text_size(typography::SIZE_MICRO)
                .text_color(color::text_muted())
                .child(SharedString::from(label))
                .into_any_element(),
        );
    }

    let playlist_display = vm.playlist_display();
    actions.push(
        AddToPlaylistPopover::new(AddToPlaylistDisplay {
            id: SharedString::from(playlist_display.popover_id),
            playlists: playlist_options(playlists),
            trigger_label: SharedString::from(playlist_display.trigger_label),
            trigger_a11y_label: SharedString::from("Add track to playlist"),
            new_playlist_a11y_label: SharedString::from("Create a new playlist"),
            back_a11y_label: SharedString::from("Back to playlist choices"),
            create_a11y_label: SharedString::from("Create playlist and add track"),
        })
        .on_select(cx.listener(move |this, playlist_id: &i64, _window, cx| {
            this.add_track_to_playlist(track_id, *playlist_id, cx);
        }))
        .on_create(cx.listener(move |this, name: &String, _window, cx| {
            this.create_playlist_and_add_track(name, track_id, cx);
        }))
        .into_any_element(),
    );

    let track_view = TrackView::from_local(track.clone());
    let mut row_vm = TrackDetailVm::new(&track_view, TrackDetailSurfaceContext::Library).row();
    let open_entry = place.context.apply(track, &mut row_vm);
    let row = TrackRowComposite::from_vm(SharedString::from(row_id), &row_vm).thumbnail(thumbnail);
    let mut row = with_detail_row_slot(row, place.slot, open_entry, cx);

    for action in actions {
        row = row.trailing_child(action);
    }

    row.into_any_element()
}
