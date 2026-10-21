//! Name level page dispatch (ADR 0084 Decision 7, packet 017).
//!
//! `TopApp` owns the `MusicIndex` request and the navigation entry of the
//! name level page. The Library releases of the page come from
//! `LibraryApp::name_level_members` at each render, so a download or a
//! removal changes the page in place. The screen in
//! `src/ui/shells/name_level_page.rs` composes the view model. This module
//! selects which state applies. `src/view_models/name_level_page.rs`
//! decides its text.

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{AnyElement, App, Context, Image, Window};

use crate::application::queries::search::FetchNameLevelReleases;
use crate::application::CommandContext;
use crate::presentation::collection_art::collection_art_image;
use crate::presentation::present_command;
use crate::ui::composites::ThumbnailSize;
use crate::ui::shells::name_level_page::render_name_level_page;
use crate::view_models::artist_identity::{
    ArtistNameDestination, ArtistNameKey, NameFeedMatch, UNKNOWN_ARTIST_TEXT,
};
use crate::view_models::index_name_list::name_sort_key;
use crate::view_models::name_level_page::{
    MatchedFeed, NameLevelIndexState, NameLevelPageFacts, NameLevelPageVm,
};
use crate::view_models::publisher_page::PublisherPageContext;
use crate::view_models::workspace::{FrameNavigationEntry, NavigationPathStart};

use super::TopApp;

/// The `MusicIndex` part of the mounted name level page.
#[derive(Clone, Debug)]
pub(super) struct NameLevelPageState {
    key: ArtistNameKey,
    index: NameLevelIndexState,
}

impl TopApp {
    /// Opens the page of an artist name (ADR 0084 Decision 7). A name that
    /// matches exactly one Library artist feed opens that feed page with
    /// the Library context. Each other name opens its name level page
    /// through the one navigation stack.
    pub(super) fn open_artist_name(
        &mut self,
        key: ArtistNameKey,
        start: NavigationPathStart,
        cx: &mut Context<Self>,
    ) {
        let matches = self.library.read(cx).name_feed_matches();
        match matches.destination(key) {
            ArtistNameDestination::FeedPage(feed_guid) => {
                self.open_publisher_page(feed_guid, PublisherPageContext::Library, cx);
            }
            ArtistNameDestination::NameLevel(key) => {
                self.open_music_page(&FrameNavigationEntry::ArtistName(key), start, cx);
            }
        }
    }

    /// Restores the `MusicIndex` request of a name level entry that a
    /// breadcrumb, a row or a history move just restored. It sends no
    /// request when `entry` is not a name level entry, or when the mounted
    /// page already has its key.
    pub(super) fn restore_name_level_page_for_nav(
        &mut self,
        entry: &FrameNavigationEntry,
        cx: &mut Context<Self>,
    ) {
        let FrameNavigationEntry::ArtistName(key) = entry else {
            return;
        };
        if !name_level_restore_needs_request(self.name_level_page.as_ref(), key) {
            return;
        }
        self.fetch_name_level_page(key.clone(), cx);
    }

    fn fetch_name_level_page(&mut self, key: ArtistNameKey, cx: &mut Context<Self>) {
        // Packet 017: the key of the fallback artist text sends no request.
        if key.key == name_sort_key(UNKNOWN_ARTIST_TEXT) {
            self.name_level_page = Some(NameLevelPageState {
                key,
                index: NameLevelIndexState::NotRequested,
            });
            cx.notify();
            return;
        }
        self.name_level_page = Some(NameLevelPageState {
            key: key.clone(),
            index: NameLevelIndexState::Loading,
        });
        cx.notify();

        let command = FetchNameLevelReleases::new(self.musicindex_endpoint.clone(), &key.key);
        let failure_key = key.key.clone();
        let success_key = key.key;
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, index, cx| {
                if apply_name_level_result(this.name_level_page.as_mut(), &success_key, index) {
                    cx.notify();
                }
            },
            move |this, error, cx| {
                let failed = NameLevelIndexState::Failed {
                    detail: format!("{error}"),
                };
                if apply_name_level_result(this.name_level_page.as_mut(), &failure_key, failed) {
                    cx.notify();
                }
            },
        );
    }

    /// The view model of the mounted name level page, from the current
    /// Library tree and the `MusicIndex` state (packet 017). A Library
    /// change that makes the key match one feed gives the notice of
    /// decision D16.
    pub(super) fn name_level_page_vm(&self, cx: &App) -> Option<NameLevelPageVm> {
        let state = self.name_level_page.as_ref()?;
        let FrameNavigationEntry::ArtistName(current_key) = self.current_music_entry()? else {
            return None;
        };
        if current_key != state.key {
            return None;
        }
        let library = self.library.read(cx);
        let matches = library.name_feed_matches();
        let matched_feed = match matches.get(&state.key.key) {
            NameFeedMatch::MatchedFeed(feed_guid) => Some(MatchedFeed {
                title: matches.feed_title(&feed_guid),
                feed_guid,
            }),
            NameFeedMatch::Ambiguous | NameFeedMatch::NoFeed => None,
        };
        Some(NameLevelPageVm::new(NameLevelPageFacts {
            key: current_key,
            library: library.name_level_members(&state.key.key),
            index: state.index.clone(),
            other_identity_feed_guids: library
                .other_identity_feed_guids(std::slice::from_ref(&state.key)),
            matched_feed,
        }))
    }

    /// Renders the mounted name level page. This method only selects the
    /// state. The view model decides its text, and the screen composes it.
    pub(super) fn render_name_level_page_content(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(vm) = self.name_level_page_vm(cx) else {
            return crate::ui::shells::publisher::render_publisher_page_status(
                &crate::view_models::publisher_page::PublisherPageLoadDisplay::empty(),
                cx,
            );
        };
        let mut album_thumbs: BTreeMap<String, Option<Arc<Image>>> = BTreeMap::new();
        for url in vm.album_image_urls() {
            let image = self.index_remote_detail_hero_image(&url, cx);
            album_thumbs.insert(url, image);
        }
        // ADR 0084 Decision 12: the name level gives no image. Its art is
        // the logo mosaic, or the entity color.
        let header_art = collection_art_image(self, &vm.collection_art(), ThumbnailSize::XXl, cx);
        let rows = self.release_section_rows(vm.sections(), cx);
        let entity = cx.entity();
        let on_open_feed = move |feed_guid: &str, _window: &mut Window, cx: &mut App| {
            let feed_guid = feed_guid.to_owned();
            entity.update(cx, |this, cx| {
                this.open_publisher_page(feed_guid, PublisherPageContext::Library, cx);
            });
        };
        render_name_level_page(&vm, &album_thumbs, header_art, &rows, on_open_feed, cx)
    }
}

/// Applies the `MusicIndex` result of `key` to the mounted page. A result
/// for a key that is not the current key is dropped, and this gives
/// `false`.
fn apply_name_level_result(
    current: Option<&mut NameLevelPageState>,
    key: &str,
    index: NameLevelIndexState,
) -> bool {
    match current {
        Some(state) if state.key.key == key => {
            state.index = index;
            true
        }
        _ => false,
    }
}

/// `true` when the restore of `key` needs a new `MusicIndex` request (packet
/// 017). The mounted page with the same key keeps its state, so the
/// restore sends no request.
fn name_level_restore_needs_request(
    current: Option<&NameLevelPageState>,
    key: &ArtistNameKey,
) -> bool {
    current.is_none_or(|state| &state.key != key)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R17-12: a result for a key that is not the current key is dropped.
    #[test]
    fn adr_0084_name_level_r17_12_result_of_another_key_is_dropped() {
        let mut mounted = NameLevelPageState {
            key: ArtistNameKey::new("haleen", "Haleen"),
            index: NameLevelIndexState::Loading,
        };
        let loaded = NameLevelIndexState::Loaded {
            releases: Vec::new(),
            more_available: false,
        };
        assert!(!apply_name_level_result(
            Some(&mut mounted),
            "survival guide",
            loaded.clone()
        ));
        assert_eq!(mounted.index, NameLevelIndexState::Loading);
        assert!(apply_name_level_result(
            Some(&mut mounted),
            "haleen",
            loaded.clone()
        ));
        assert_eq!(mounted.index, loaded);
        assert!(!apply_name_level_result(
            None,
            "haleen",
            NameLevelIndexState::Loading
        ));
    }

    /// R17-12: a restore for the mounted key sends no request. A restore
    /// for another key, or with no mounted page, sends one. Two labels of
    /// one key are the mounted key.
    #[test]
    fn adr_0084_name_level_r17_12_restore_of_the_mounted_key_sends_no_request() {
        let mounted = NameLevelPageState {
            key: ArtistNameKey::new("haleen", "Haleen"),
            index: NameLevelIndexState::Loading,
        };
        assert!(!name_level_restore_needs_request(
            Some(&mounted),
            &ArtistNameKey::new("haleen", "HALEEN")
        ));
        assert!(name_level_restore_needs_request(
            Some(&mounted),
            &ArtistNameKey::from_text("Survival Guide")
        ));
        assert!(name_level_restore_needs_request(
            None,
            &ArtistNameKey::from_text("Haleen")
        ));
    }
}
