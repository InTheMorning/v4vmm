//! Name-match track page dispatch (ADR 0077 packet 006, Accepted Refinement
//! "Index artist page by name").
//!
//! `TopApp` owns the fetch and the navigation entry of the Index name-match
//! track page, reached from an Index name candidate on the search results.
//! The screen in `src/ui/shells/name_match_page.rs` composes the view model
//! that this module loads through the ADR 0040 runtime. This page shows a
//! search result, never an artist: it has no artist identity, no role, and
//! no page type (ADR 0077 Decision 1).

use std::rc::Rc;

use gpui::{AnyElement, Context};

use crate::application::queries::search::FetchNameMatchTracks;
use crate::application::CommandContext;
use crate::presentation::present_command;
use crate::ui::shells::name_match_page::{render_name_match_page, render_name_match_page_status};
use crate::ui::shells::search_result_rows::SearchResultSelectHandler;
use crate::view_models::name_match_page::{
    NameMatchPageFacts, NameMatchPageLoadDisplay, NameMatchPageVm,
};
use crate::view_models::search_results::TrackResultDisplay;
use crate::view_models::workspace::{FrameNavigationEntry, WorkspaceFrameId};

use super::TopApp;

/// Loading lifecycle of the mounted name-match track page.
#[derive(Clone, Debug)]
pub(super) enum NameMatchPageState {
    Loading {
        name: String,
    },
    Loaded {
        name: String,
        facts: NameMatchPageFacts,
    },
    Failed {
        name: String,
        message: String,
    },
}

impl NameMatchPageState {
    /// The name this state belongs to, regardless of its lifecycle step
    /// (packet 006: the stale-result fix keys on this value, the same
    /// pattern `publisher_page_result_is_current` follows).
    fn name(&self) -> &str {
        match self {
            Self::Loading { name } | Self::Loaded { name, .. } | Self::Failed { name, .. } => name,
        }
    }
}

impl TopApp {
    /// Opens the name-match track page for `name`: pushes its navigation
    /// entry, and starts its fetch through the ADR 0040 runtime.
    pub(super) fn open_name_match_page(
        &mut self,
        name: String,
        content_frame_id: WorkspaceFrameId,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.workspace_layout.push_nav(
            content_frame_id,
            FrameNavigationEntry::IndexNameMatches(name.clone()),
        ) {
            self.settings_status =
                format!("Failed to navigate to tracks matching this name: {error}");
        }
        self.fetch_name_match_page(name, cx);
    }

    /// Restores the name-match track page fetch for a navigation entry that
    /// a breadcrumb or a history move (back or forward) just restored.
    /// Starts no fetch when `entry` is not a name-match page, or when the
    /// mounted page already matches its name.
    pub(super) fn restore_name_match_page_for_nav(
        &mut self,
        entry: &FrameNavigationEntry,
        cx: &mut Context<Self>,
    ) {
        let FrameNavigationEntry::IndexNameMatches(name) = entry else {
            return;
        };
        if name_match_page_result_is_current(self.name_match_page.as_ref(), name) {
            return;
        }
        self.fetch_name_match_page(name.clone(), cx);
    }

    fn fetch_name_match_page(&mut self, name: String, cx: &mut Context<Self>) {
        self.name_match_page = Some(NameMatchPageState::Loading { name: name.clone() });
        cx.notify();

        let command = FetchNameMatchTracks::new(self.musicindex_endpoint.clone(), name.clone());
        let success_name = name.clone();
        let failure_name = name;
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, facts, cx| {
                // Packet 006: an older fetch's result must not overwrite a
                // page the operator already navigated away from. This is
                // the same stale-result race `publisher_page_result_is_current`
                // guards.
                if !name_match_page_result_is_current(this.name_match_page.as_ref(), &success_name)
                {
                    return;
                }
                this.name_match_page = Some(NameMatchPageState::Loaded {
                    name: success_name,
                    facts,
                });
                cx.notify();
            },
            move |this, error, cx| {
                if !name_match_page_result_is_current(this.name_match_page.as_ref(), &failure_name)
                {
                    return;
                }
                this.name_match_page = Some(NameMatchPageState::Failed {
                    name: failure_name,
                    message: format!("{error}"),
                });
                cx.notify();
            },
        );
    }

    /// Renders the mounted name-match page, from its loading lifecycle.
    /// This method only selects the display state; the view model decides
    /// its text, and the screen composes it.
    pub(super) fn render_name_match_page_content(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match self.name_match_page.clone() {
            Some(NameMatchPageState::Loading { name }) => {
                render_name_match_page_status(&NameMatchPageLoadDisplay::loading(&name), cx)
            }
            Some(NameMatchPageState::Loaded { facts, .. }) => {
                let vm = NameMatchPageVm::new(facts);
                let thumbnail_hrefs: Vec<String> = vm
                    .rows()
                    .iter()
                    .filter_map(|row| row.thumbnail_href.clone())
                    .collect();
                let thumbnails = self.resolve_search_result_thumbnails(thumbnail_hrefs, cx);
                let entity = cx.entity();
                let on_result_select: SearchResultSelectHandler =
                    Rc::new(move |tab, result_id, _window, cx| {
                        entity.update(cx, |this, cx| {
                            this.handle_search_result_selected(tab, &result_id, cx);
                        });
                    });
                render_name_match_page(&vm, &thumbnails, &on_result_select, cx)
            }
            Some(NameMatchPageState::Failed { message, .. }) => {
                render_name_match_page_status(&NameMatchPageLoadDisplay::failed(message), cx)
            }
            None => render_name_match_page_status(&NameMatchPageLoadDisplay::empty(), cx),
        }
    }

    /// The cached track row for `activation_id` on the mounted name-match
    /// page (Required Change 3). The Index track detail route falls back
    /// to this cache when the operator reached the track from this page,
    /// not from a search flow.
    pub(super) fn name_match_page_track_row(
        &self,
        activation_id: &str,
    ) -> Option<TrackResultDisplay> {
        match self.name_match_page.as_ref()? {
            NameMatchPageState::Loaded { facts, .. } => NameMatchPageVm::new(facts.clone())
                .rows()
                .into_iter()
                .find(|row| row.id == activation_id),
            NameMatchPageState::Loading { .. } | NameMatchPageState::Failed { .. } => None,
        }
    }
}

/// `true` when a fetch result for `name` still belongs to the mounted
/// name-match page (packet 006), following the pattern of
/// `publisher_page_result_is_current`. A newer name-match activation, or a
/// navigation restore to a different name, already replaces `current` by
/// the time an older fetch's result arrives. That call gives `false`, so
/// the caller applies no result.
fn name_match_page_result_is_current(current: Option<&NameMatchPageState>, name: &str) -> bool {
    current.is_some_and(|state| state.name() == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R6-06: an older fetch's result for a name the operator already
    /// navigated away from must not apply.
    #[test]
    fn adr_0077_name_matches_stale_result_for_replaced_name_is_ignored() {
        let current = Some(NameMatchPageState::Loading {
            name: "Survival Guide".into(),
        });

        assert!(!name_match_page_result_is_current(
            current.as_ref(),
            "DETOX"
        ));
        assert!(name_match_page_result_is_current(
            current.as_ref(),
            "Survival Guide"
        ));
    }

    /// A result also applies when the mounted page already reached
    /// `Loaded` or `Failed` for that same name, not only `Loading`.
    #[test]
    fn adr_0077_name_matches_result_is_current_for_any_lifecycle_step() {
        let loaded = Some(NameMatchPageState::Loaded {
            name: "DETOX".into(),
            facts: NameMatchPageFacts::default(),
        });
        assert!(name_match_page_result_is_current(loaded.as_ref(), "DETOX"));

        let failed = Some(NameMatchPageState::Failed {
            name: "DETOX".into(),
            message: "error".into(),
        });
        assert!(name_match_page_result_is_current(failed.as_ref(), "DETOX"));

        assert!(!name_match_page_result_is_current(None, "DETOX"));
    }
}
