//! Publisher page dispatch (ADR 0077 packet 004).
//!
//! `TopApp` owns the fetch and the navigation entry of a publisher page,
//! reached from an album or a track in the Library or the Index route. The
//! screen in `src/ui/shells/publisher.rs` composes the view model that this
//! module loads through the ADR 0040 runtime. This module selects which
//! display state applies; `src/view_models/publisher_page.rs` decides its
//! text.
//!
//! In the Library context, the page also lists the releases of each name
//! that matches exactly this feed, in the section "Matched by name" (ADR
//! 0084 Decision 7, packet 017). The Library rows of that section come from
//! `LibraryApp::name_level_members` at each render. This module sends one
//! `MusicIndex` name level request for each matched key.

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{AnyElement, App, Context, Image, Modifiers, Window};

use crate::application::commands::feed::StorePublisherFeedImage;
use crate::application::queries::feed::FetchIndexPublisherPage;
use crate::application::queries::library::FetchLibraryPublisherPage;
use crate::application::queries::search::FetchNameLevelReleases;
use crate::application::{ApplicationCommand, CommandContext};
use crate::presentation::collection_art::collection_art_image;
use crate::presentation::present_command;
use crate::ui::composites::release_section_list::{
    ClickReleaseCallback, ReleaseSectionRows, ToggleReleaseCallback,
};
use crate::ui::composites::ThumbnailSize;
use crate::ui::shells::publisher::{render_publisher_page, render_publisher_page_status};
use crate::view_models::detail_selection::DetailRowKey;
use crate::view_models::name_level_page::NameLevelIndexState;
use crate::view_models::publisher_page::{
    MatchedByNameFacts, PublisherPageContext, PublisherPageFacts, PublisherPageLoadDisplay,
    PublisherPageSectionVm, PublisherPageVm, TitleDisplay,
};
use crate::view_models::workspace::{FrameNavigationEntry, NavigationPathStart, ReleaseKey};

use super::TopApp;

/// Loading lifecycle of the mounted publisher page.
#[derive(Clone, Debug)]
pub(super) enum PublisherPageState {
    Loading {
        publisher_feed_guid: String,
        context: PublisherPageContext,
    },
    Loaded {
        publisher_feed_guid: String,
        // `Box`ed: `PublisherPageFacts` grew past clippy's large-enum-variant
        // threshold once it gained `feed_image_url` and each album's named
        // rank and track count (ADR 0085 Decision 7 design).
        facts: Box<PublisherPageFacts>,
        context: PublisherPageContext,
        /// The `MusicIndex` part of each matched name key, by key (ADR
        /// 0084 Decision 7, packet 017). Only the Library context sends
        /// these requests.
        matched_by_name: BTreeMap<String, NameLevelIndexState>,
    },
    Failed {
        publisher_feed_guid: String,
        message: String,
    },
}

impl PublisherPageState {
    /// The publisher feed GUID this state belongs to, regardless of its
    /// lifecycle step (ADR 0077 packet 004: the stale-result and the
    /// navigation-restore fixes both key on this value).
    fn publisher_feed_guid(&self) -> &str {
        match self {
            Self::Loading {
                publisher_feed_guid,
                ..
            }
            | Self::Loaded {
                publisher_feed_guid,
                ..
            }
            | Self::Failed {
                publisher_feed_guid,
                ..
            } => publisher_feed_guid,
        }
    }
}

impl TopApp {
    /// Opens the publisher page for `publisher_feed_guid`: pushes its
    /// navigation entry, and starts its fetch through the ADR 0040 runtime.
    /// `context` selects the query and the page shape: the Library query
    /// for a Library entry point, the Index query for an Index entry point.
    pub(super) fn open_publisher_page(
        &mut self,
        publisher_feed_guid: String,
        context: PublisherPageContext,
        cx: &mut Context<Self>,
    ) {
        let Some(content_frame_id) = self.content_list_frame_id() else {
            self.settings_status = "ContentList frame not found".to_string();
            cx.notify();
            return;
        };
        if let Err(error) = self.workspace_layout.push_nav(
            content_frame_id,
            FrameNavigationEntry::PublisherDetail(publisher_feed_guid.clone()),
        ) {
            self.settings_status = format!("The app could not open the feed page: {error}");
        }
        self.fetch_publisher_page(publisher_feed_guid, context, cx);
    }

    /// Opens the feed page of a `MusicIndex` feed row of the Artists list
    /// directly under its destination (ADR 0084 Decision 9, packet 016
    /// decision D14). The page uses the `MusicIndex` query, because the
    /// Library holds no release of the feed.
    pub(super) fn open_index_publisher_page_from_list_row(
        &mut self,
        publisher_feed_guid: String,
        cx: &mut Context<Self>,
    ) {
        self.publisher_page_routes
            .insert(publisher_feed_guid.clone(), PublisherPageContext::Index);
        self.open_music_page(
            &FrameNavigationEntry::PublisherDetail(publisher_feed_guid),
            NavigationPathStart::ListRow,
            cx,
        );
    }

    /// Restores the publisher page fetch for a navigation entry that a
    /// breadcrumb or a history move (back or forward) just restored (ADR
    /// 0077 packet 004). Starts no fetch when `entry` is not a publisher
    /// page, or when the mounted page already matches its GUID. Otherwise
    /// fetches with the context this GUID last opened with, so the restore
    /// selects the same query it originally used.
    pub(super) fn restore_publisher_page_for_nav(
        &mut self,
        entry: &FrameNavigationEntry,
        cx: &mut Context<Self>,
    ) {
        let FrameNavigationEntry::PublisherDetail(publisher_feed_guid) = entry else {
            return;
        };
        let known_context = self.publisher_page_routes.get(publisher_feed_guid).copied();
        let Some(context) = publisher_page_restore_context(
            self.publisher_page.as_ref(),
            publisher_feed_guid,
            known_context,
        ) else {
            return;
        };
        self.fetch_publisher_page(publisher_feed_guid.clone(), context, cx);
    }

    /// Starts the publisher page fetch, and records which context this GUID
    /// used, for a later navigation restore.
    fn fetch_publisher_page(
        &mut self,
        publisher_feed_guid: String,
        context: PublisherPageContext,
        cx: &mut Context<Self>,
    ) {
        self.publisher_page_routes
            .insert(publisher_feed_guid.clone(), context);
        self.publisher_page = Some(PublisherPageState::Loading {
            publisher_feed_guid: publisher_feed_guid.clone(),
            context,
        });
        cx.notify();

        match context {
            PublisherPageContext::Library => {
                let command = FetchLibraryPublisherPage::new(
                    Arc::clone(&self.conn),
                    self.musicindex_endpoint.clone(),
                    publisher_feed_guid.clone(),
                );
                self.dispatch_publisher_page_fetch(command, publisher_feed_guid, context, cx);
            }
            PublisherPageContext::Index => {
                let command = FetchIndexPublisherPage::new(
                    Arc::clone(&self.conn),
                    self.musicindex_endpoint.clone(),
                    publisher_feed_guid.clone(),
                );
                self.dispatch_publisher_page_fetch(command, publisher_feed_guid, context, cx);
            }
        }
    }

    fn dispatch_publisher_page_fetch<C>(
        &mut self,
        command: C,
        publisher_feed_guid: String,
        context: PublisherPageContext,
        cx: &mut Context<Self>,
    ) where
        C: ApplicationCommand<Output = PublisherPageFacts>,
    {
        let success_guid = publisher_feed_guid.clone();
        let failure_guid = publisher_feed_guid;
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, facts, cx| {
                // ADR 0077 packet 004: a newer "open publisher" click, or a
                // navigation restore to a different GUID, already replaced
                // the mounted page. This older fetch's result must not
                // overwrite it (the stale-result race).
                if !publisher_page_result_is_current(this.publisher_page.as_ref(), &success_guid) {
                    return;
                }
                let feed_image_url = facts.feed_image_url.clone();
                this.publisher_page = Some(PublisherPageState::Loaded {
                    publisher_feed_guid: success_guid.clone(),
                    facts: Box::new(facts),
                    context,
                    matched_by_name: BTreeMap::new(),
                });
                cx.notify();
                // Packet 017 decision D14: the match reads the Library tree
                // only, so the Index context sends no request.
                if context == PublisherPageContext::Library {
                    this.fetch_matched_by_name(&success_guid, cx);
                }
                if let Some(image_url) = feed_image_url {
                    this.store_publisher_feed_image(success_guid, image_url, cx);
                }
            },
            move |this, error, cx| {
                if !publisher_page_result_is_current(this.publisher_page.as_ref(), &failure_guid) {
                    return;
                }
                this.publisher_page = Some(PublisherPageState::Failed {
                    publisher_feed_guid: failure_guid,
                    message: format!("{error}"),
                });
                cx.notify();
            },
        );
    }

    /// Sends one `MusicIndex` name level request for each name key that
    /// matches exactly the feed `publisher_feed_guid`, in key order (ADR
    /// 0084 Decision 7, packet 017). Each result changes the section
    /// "Matched by name" in place.
    fn fetch_matched_by_name(&mut self, publisher_feed_guid: &str, cx: &mut Context<Self>) {
        let keys = self.library.read(cx).matched_name_keys(publisher_feed_guid);
        for key in keys {
            let Some(PublisherPageState::Loaded {
                matched_by_name, ..
            }) = self.publisher_page.as_mut()
            else {
                return;
            };
            matched_by_name.insert(key.key.clone(), NameLevelIndexState::Loading);
            let command =
                FetchNameLevelReleases::new(self.musicindex_endpoint.clone(), key.key.clone());
            let success = (publisher_feed_guid.to_owned(), key.key.clone());
            let failure = success.clone();
            present_command(
                &self.command_runner,
                command,
                CommandContext::next(),
                cx,
                move |this, index, cx| {
                    if apply_matched_by_name_result(
                        &mut this.publisher_page,
                        &success.0,
                        &success.1,
                        index,
                    ) {
                        cx.notify();
                    }
                },
                move |this, error, cx| {
                    let failed = NameLevelIndexState::Failed {
                        detail: format!("{error}"),
                    };
                    if apply_matched_by_name_result(
                        &mut this.publisher_page,
                        &failure.0,
                        &failure.1,
                        failed,
                    ) {
                        cx.notify();
                    }
                },
            );
        }
        cx.notify();
    }

    /// The view model of the loaded feed page, with the section "Matched by
    /// name" in the Library context (ADR 0084 Decision 7, packet 017). The
    /// facts of the section come from the current Library tree, so a
    /// download or a removal changes the section in place.
    pub(super) fn publisher_page_vm(
        &self,
        cx: &App,
    ) -> Option<(PublisherPageVm, PublisherPageContext)> {
        let Some(PublisherPageState::Loaded {
            publisher_feed_guid,
            facts,
            context,
            matched_by_name,
        }) = self.publisher_page.as_ref()
        else {
            return None;
        };
        let vm = PublisherPageVm::new(facts.as_ref().clone());
        if *context != PublisherPageContext::Library {
            return Some((vm, *context));
        }
        let library = self.library.read(cx);
        let keys = library.matched_name_keys(publisher_feed_guid);
        let section = MatchedByNameFacts {
            library: keys
                .iter()
                .flat_map(|key| library.name_level_members(&key.key))
                .collect(),
            index: NameLevelIndexState::combine(
                keys.iter().filter_map(|key| matched_by_name.get(&key.key)),
            ),
            other_identity_feed_guids: library.other_identity_feed_guids(&keys),
            keys,
        };
        Some((vm.with_matched_by_name(section), *context))
    }

    /// Stores the channel image of a loaded feed page in the relationship
    /// rows of that feed (ADR 0084 Decision 12). The command runs on the
    /// ADR 0040 runtime and sends no request. When a row changes, the
    /// Library reloads, so the Artists row and the singles groups show the
    /// image in place.
    fn store_publisher_feed_image(
        &mut self,
        publisher_feed_guid: String,
        image_url: String,
        cx: &mut Context<Self>,
    ) {
        let command =
            StorePublisherFeedImage::new(Arc::clone(&self.conn), publisher_feed_guid, image_url);
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, changed, cx| {
                if changed > 0 {
                    this.library.update(cx, crate::library::LibraryApp::refresh);
                }
            },
            |this, error, cx| {
                this.settings_status =
                    format!("The app could not store the image of the artist feed: {error}");
                cx.notify();
            },
        );
    }

    /// Renders the mounted publisher page, from its loading lifecycle. This
    /// method only selects the display state; the view model decides its
    /// text, and the screen composes it.
    pub(super) fn render_publisher_page_content(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match self.publisher_page.clone() {
            Some(PublisherPageState::Loading {
                publisher_feed_guid,
                context,
            }) => {
                let display = PublisherPageLoadDisplay::loading(context, &publisher_feed_guid);
                render_publisher_page_status(&display, cx)
            }
            Some(PublisherPageState::Loaded { .. }) => {
                let Some((vm, context)) = self.publisher_page_vm(cx) else {
                    return render_publisher_page_status(&PublisherPageLoadDisplay::empty(), cx);
                };
                let sections = vm.sections();
                // ADR 0077 packet 004, orchestrator fix 5: `image_url` on
                // each album already comes from packet 003 (Index) and this
                // packet's own Library query. Resolve it through the same
                // thumbnail path other album rows use; add no new request.
                let mut album_thumbs: BTreeMap<String, Option<Arc<Image>>> = BTreeMap::new();
                // Packet 017: the rows of the section "Matched by name"
                // need their covers too.
                let mut urls = vm.album_image_urls();
                urls.extend(
                    sections
                        .iter()
                        .flat_map(|section| section.rows.iter())
                        .filter_map(|row| row.image_url.clone()),
                );
                urls.sort_unstable();
                urls.dedup();
                for url in urls {
                    let image = self.index_remote_detail_hero_image(&url, cx);
                    album_thumbs.insert(url, image);
                }
                // ADR 0084 Decision 12: the feed's own channel image as a
                // square, else the logo mosaic of its album images.
                let header_art =
                    collection_art_image(self, &vm.collection_art(), ThumbnailSize::XXl, cx);
                // ADR 0082 Decision 5: a "Shares albums with" entry opens
                // the Index shape of its own feed page. The data behind
                // every entry comes from the one Index request
                // (`fetch_index_publisher_page_albums`), on the Library
                // page and the Index page alike.
                let entity = cx.entity();
                let on_open_publisher = move |guid: &str, _window: &mut Window, cx: &mut App| {
                    let guid = guid.to_owned();
                    entity.update(cx, |this, cx| {
                        this.open_publisher_page(guid, PublisherPageContext::Index, cx);
                    });
                };
                // Required Change 7: a release row opens the one release
                // route, in the current frame, the same as a release row
                // of a list does. ADR 0084 packet 018: in selection mode,
                // a click changes the selection of the row instead.
                let rows = self.release_section_rows(sections, cx);
                render_publisher_page(
                    &vm,
                    context,
                    &album_thumbs,
                    header_art,
                    on_open_publisher,
                    &rows,
                    cx,
                )
            }
            Some(PublisherPageState::Failed { message, .. }) => {
                let display = PublisherPageLoadDisplay::failed(message);
                render_publisher_page_status(&display, cx)
            }
            None => render_publisher_page_status(&PublisherPageLoadDisplay::empty(), cx),
        }
    }

    /// The release rows of the mounted feed page or name level page, with
    /// the facts of the detail selection on each row (ADR 0084 Decision 10,
    /// packet 018).
    pub(super) fn release_section_rows(
        &self,
        mut sections: Vec<PublisherPageSectionVm>,
        cx: &mut Context<Self>,
    ) -> ReleaseSectionRows {
        let library = self.library.read(cx);
        for row in sections
            .iter_mut()
            .flat_map(|section| section.rows.iter_mut())
        {
            let key = row
                .open
                .as_ref()
                .map(|open| DetailRowKey::Release(open.key.clone()));
            let title = match &row.title {
                TitleDisplay::Stated(text) | TitleDisplay::Missing(text) => text.as_str(),
            };
            row.selection = library
                .detail_selection()
                .row_selection(key.as_ref(), title);
        }
        let scroll = library.detail_scroll_handle();
        let reveal = library.detail_scroll_reveal().clone();
        let entity = cx.entity();
        let on_click_release: ClickReleaseCallback = Rc::new(
            move |key: &ReleaseKey, label: String, modifiers: Modifiers, _window, cx: &mut App| {
                let key = DetailRowKey::Release(key.clone());
                entity.update(cx, |this, cx| {
                    this.click_detail_row(&key, label, modifiers, cx);
                });
            },
        );
        let entity = cx.entity();
        let on_toggle_release: ToggleReleaseCallback =
            Rc::new(move |key: &ReleaseKey, _window, cx: &mut App| {
                let key = DetailRowKey::Release(key.clone());
                entity.update(cx, |this, cx| this.toggle_detail_row(&key, cx));
            });
        ReleaseSectionRows {
            sections,
            scroll,
            reveal,
            on_click_release,
            on_toggle_release,
        }
    }

    /// The breadcrumb text of the mounted publisher page: the view model
    /// title (ADR 0077 packet 004, R4-04). `None` when the page has not
    /// loaded, so the caller falls back to the GUID.
    pub(super) fn publisher_page_breadcrumb_text(&self) -> Option<String> {
        match self.publisher_page.as_ref()? {
            PublisherPageState::Loaded { facts, .. } => {
                Some(PublisherPageVm::new(facts.as_ref().clone()).title_text())
            }
            PublisherPageState::Loading { .. } | PublisherPageState::Failed { .. } => None,
        }
    }

    /// The frame title of the mounted feed page, from its loaded facts
    /// (ADR 0085 Decision 7). `None` while the page is loading or failed:
    /// `content_list_frame_title`'s default title "Feed" then applies.
    pub(super) fn publisher_page_frame_title(&self) -> Option<String> {
        match self.publisher_page.as_ref()? {
            PublisherPageState::Loaded { facts, .. } => {
                Some(PublisherPageVm::new(facts.as_ref().clone()).frame_title())
            }
            PublisherPageState::Loading { .. } | PublisherPageState::Failed { .. } => None,
        }
    }
}

/// Applies the `MusicIndex` result of the matched name `key` to the loaded
/// feed page `publisher_feed_guid`. A result for another page is dropped,
/// and this gives `false`.
fn apply_matched_by_name_result(
    current: &mut Option<PublisherPageState>,
    publisher_feed_guid: &str,
    key: &str,
    index: NameLevelIndexState,
) -> bool {
    match current {
        Some(PublisherPageState::Loaded {
            publisher_feed_guid: loaded_guid,
            matched_by_name,
            ..
        }) if loaded_guid == publisher_feed_guid => {
            matched_by_name.insert(key.to_owned(), index);
            true
        }
        _ => false,
    }
}

/// `true` when a fetch result for `publisher_feed_guid` still belongs to
/// the mounted publisher page (ADR 0077 packet 004). A newer "open
/// publisher" click, or a navigation restore to a different publisher feed
/// GUID, already replaces `current` by the time an older fetch's result
/// arrives. That call gives `false`, so the caller applies no result.
fn publisher_page_result_is_current(
    current: Option<&PublisherPageState>,
    publisher_feed_guid: &str,
) -> bool {
    current.is_some_and(|state| state.publisher_feed_guid() == publisher_feed_guid)
}

/// Decides whether restoring a `PublisherDetail` navigation entry needs a
/// fresh fetch, and which context (Library or Index) to fetch with (ADR
/// 0077 packet 004). Gives `None` when the mounted page already matches
/// `publisher_feed_guid`, so the restore starts no fetch. `known_context`
/// is the context recorded the last time this GUID was opened; a missing
/// record falls back to the Library query.
fn publisher_page_restore_context(
    current: Option<&PublisherPageState>,
    publisher_feed_guid: &str,
    known_context: Option<PublisherPageContext>,
) -> Option<PublisherPageContext> {
    if publisher_page_result_is_current(current, publisher_feed_guid) {
        return None;
    }
    Some(known_context.unwrap_or(PublisherPageContext::Library))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R4 stale-result fix: an older fetch's result for a GUID the app
    /// already navigated away from must not apply.
    #[test]
    fn adr_0077_publisher_navigation_stale_result_for_replaced_guid_is_ignored() {
        let current = Some(PublisherPageState::Loading {
            publisher_feed_guid: "guid-b".into(),
            context: PublisherPageContext::Library,
        });

        assert!(!publisher_page_result_is_current(
            current.as_ref(),
            "guid-a"
        ));
        assert!(publisher_page_result_is_current(current.as_ref(), "guid-b"));
    }

    /// A result for a GUID also applies when the mounted page already
    /// reached `Loaded` or `Failed` for that same GUID, not only `Loading`.
    #[test]
    fn adr_0077_publisher_navigation_result_is_current_for_any_lifecycle_step() {
        let loaded = Some(PublisherPageState::Loaded {
            publisher_feed_guid: "guid-a".into(),
            facts: Box::new(PublisherPageFacts::default()),
            context: PublisherPageContext::Index,
            matched_by_name: BTreeMap::new(),
        });
        assert!(publisher_page_result_is_current(loaded.as_ref(), "guid-a"));

        let failed = Some(PublisherPageState::Failed {
            publisher_feed_guid: "guid-a".into(),
            message: "error".into(),
        });
        assert!(publisher_page_result_is_current(failed.as_ref(), "guid-a"));

        assert!(!publisher_page_result_is_current(None, "guid-a"));
    }

    /// R4 navigation-restore fix: restoring the GUID that is already
    /// mounted starts no fetch.
    #[test]
    fn adr_0077_publisher_navigation_restore_skips_fetch_for_the_mounted_guid() {
        let current = Some(PublisherPageState::Loaded {
            publisher_feed_guid: "guid-a".into(),
            facts: Box::new(PublisherPageFacts::default()),
            context: PublisherPageContext::Index,
            matched_by_name: BTreeMap::new(),
        });

        assert_eq!(
            publisher_page_restore_context(
                current.as_ref(),
                "guid-a",
                Some(PublisherPageContext::Library)
            ),
            None,
            "the mounted page already matches; the restore starts no fetch"
        );
    }

    /// Restoring a different GUID fetches with the context recorded for
    /// that GUID, not the context of the page that is currently mounted.
    #[test]
    fn adr_0077_publisher_navigation_restore_uses_the_recorded_context_for_a_different_guid() {
        let current = Some(PublisherPageState::Loaded {
            publisher_feed_guid: "guid-b".into(),
            facts: Box::new(PublisherPageFacts::default()),
            context: PublisherPageContext::Library,
            matched_by_name: BTreeMap::new(),
        });

        assert_eq!(
            publisher_page_restore_context(
                current.as_ref(),
                "guid-a",
                Some(PublisherPageContext::Index)
            ),
            Some(PublisherPageContext::Index),
            "restoring guid-a must use guid-a's own recorded context"
        );
    }

    /// With no recorded context for the restored GUID, the restore falls
    /// back to the Library query.
    #[test]
    fn adr_0077_publisher_navigation_restore_falls_back_to_library_when_context_is_unknown() {
        assert_eq!(
            publisher_page_restore_context(None, "guid-a", None),
            Some(PublisherPageContext::Library)
        );
    }
}
