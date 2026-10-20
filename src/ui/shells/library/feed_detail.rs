//! Library feed (album) detail surface.
//!
//! `LibraryApp` is the host of the Library release page (ADR 0084 Decision
//! 4, packet 021). This module gives the facts, the resolved images and one
//! dispatch function to the shared release page shell. Mutations and
//! selected-entity state stay on `LibraryApp`.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{AnyElement, Context, Image};

use crate::library::LibraryApp;
use crate::media::cover_color::CoverColor;
use crate::ui::shells::library::detail::DetailRowSlot;
use crate::ui::shells::release_page::{
    render_release_page, ReleasePageDispatch, ReleasePageImages,
};
use crate::view_models::library::{AlbumNode, LibraryViewModel, MbTrackStatus};
use crate::view_models::release_page::{release_page_vm, ReleasePageFacts};

/// Renders the Library release page of `album`.
#[expect(
    clippy::too_many_arguments,
    reason = "the page renderer takes each already-resolved value of the Library host"
)]
pub(crate) fn render_library_feed_detail(
    album: &AlbumNode,
    busy_track: Option<i64>,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    library_vm: &LibraryViewModel,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cover_color: Option<CoverColor>,
    row_slots: Vec<DetailRowSlot>,
    scroll: &gpui::ScrollHandle,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let (selections, row_probes): (Vec<_>, Vec<_>) = row_slots
        .into_iter()
        .map(|slot| (slot.selection, slot.probe))
        .unzip();
    let facts = ReleasePageFacts::from_library_album(album, mb_status, busy_track, library_vm)
        .with_row_selections(selections);
    let vm = release_page_vm(&facts);
    let thumbnail = |url: Option<&str>| {
        url.and_then(|url| album_thumbs.get(url))
            .and_then(Clone::clone)
    };
    let images = ReleasePageImages {
        hero_image: thumbnail(album.image_href.as_deref()),
        cover_color,
        row_thumbnails: vm
            .rows
            .iter()
            .map(|row| thumbnail(row.thumbnail_url.as_deref()))
            .collect(),
        row_probes,
        contributor_thumbnails: Some(album_thumbs),
        scroll_handle: Some(scroll.clone()),
    };
    render_release_page(&vm, images, &library_release_page_dispatch(cx), cx)
}

/// The dispatch function of a Library release page and of a singles group
/// page. It holds only the entity handle.
pub(crate) fn library_release_page_dispatch(cx: &Context<LibraryApp>) -> ReleasePageDispatch {
    let entity = cx.entity();
    Rc::new(move |command, window, cx| {
        entity.update(cx, |this, cx| {
            this.run_release_page_command(command, window, cx);
        });
    })
}
