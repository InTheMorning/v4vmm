//! Publisher page dispatch (ADR 0077 packet 004).
//!
//! `TopApp` owns the fetch and the navigation entry of a publisher page,
//! reached from an album or a track in the Library or the Index route. The
//! screen in `src/ui/shells/publisher.rs` composes the view model that this
//! module loads through the ADR 0040 runtime. This module selects which
//! display state applies; `src/view_models/publisher_page.rs` decides its
//! text.

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{AnyElement, Context, Image};

use crate::application::queries::feed::FetchIndexPublisherPage;
use crate::application::queries::library::FetchLibraryPublisherPage;
use crate::application::{ApplicationCommand, CommandContext};
use crate::presentation::present_command;
use crate::ui::shells::publisher::{render_publisher_page, render_publisher_page_status};
use crate::view_models::publisher_page::{
    PublisherPageContext, PublisherPageFacts, PublisherPageLoadDisplay, PublisherPageVm,
};
use crate::view_models::workspace::FrameNavigationEntry;

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
        facts: PublisherPageFacts,
        context: PublisherPageContext,
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
            self.settings_status = format!("Failed to navigate to publisher: {error}");
        }
        self.fetch_publisher_page(publisher_feed_guid, context, cx);
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
                this.publisher_page = Some(PublisherPageState::Loaded {
                    publisher_feed_guid: success_guid,
                    facts,
                    context,
                });
                cx.notify();
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
            Some(PublisherPageState::Loaded { facts, context, .. }) => {
                let vm = PublisherPageVm::new(facts);
                // ADR 0077 packet 004, orchestrator fix 5: `image_url` on
                // each album already comes from packet 003 (Index) and this
                // packet's own Library query. Resolve it through the same
                // thumbnail path other album rows use; add no new request.
                let mut album_thumbs: BTreeMap<String, Option<Arc<Image>>> = BTreeMap::new();
                for url in vm.album_image_urls() {
                    let image = self.index_remote_detail_hero_image(&url, cx);
                    album_thumbs.insert(url, image);
                }
                render_publisher_page(&vm, context, &album_thumbs, cx)
            }
            Some(PublisherPageState::Failed { message, .. }) => {
                let display = PublisherPageLoadDisplay::failed(message);
                render_publisher_page_status(&display, cx)
            }
            None => render_publisher_page_status(&PublisherPageLoadDisplay::empty(), cx),
        }
    }

    /// The breadcrumb text of the mounted publisher page: the view model
    /// title (ADR 0077 packet 004, R4-04). `None` when the page has not
    /// loaded, so the caller falls back to the GUID.
    pub(super) fn publisher_page_breadcrumb_text(&self) -> Option<String> {
        match self.publisher_page.as_ref()? {
            PublisherPageState::Loaded { facts, .. } => {
                Some(PublisherPageVm::new(facts.clone()).title_text())
            }
            PublisherPageState::Loading { .. } | PublisherPageState::Failed { .. } => None,
        }
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
            facts: PublisherPageFacts::default(),
            context: PublisherPageContext::Index,
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
            facts: PublisherPageFacts::default(),
            context: PublisherPageContext::Index,
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
            facts: PublisherPageFacts::default(),
            context: PublisherPageContext::Library,
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
