//! View-models — the layer between services and screens.
//!
//! A *view-model* is a thin, **GPUI-free** projection of domain data
//! (rows from `db`, hydrated `api::Feed` / `api::Track`, derived state
//! held by the screen) into the *display-ready* shape the view needs.
//! Screens then bind primitives + composites to that projection. This
//! is the same separation of concerns `SwiftUI` encourages with its
//! `@Observable` view-models, except enforced at the module-import
//! level rather than at runtime.
//!
//! ## Layered architecture (in force)
//!
//! ```text
//! db / *_service / api  (domain — no GPUI)
//!         ▲ read / write
//!         │
//! view_models/                                      ← THIS LAYER
//!   - own UI state (selection, filters, "what's showing")
//!   - project domain data into display-ready shapes
//!   - expose commands callers can dispatch on a service
//!         ▲ observe / dispatch
//!         │
//! ui/primitives/  ui/composites/                    ← shipped
//!         ▲ bind
//!         │
//! screens/  (ui_artist, ui_feed, ui_track,           ← thin
//!            library, search, app)
//! ```
//!
//! ## Rules for any module under `view_models/`
//!
//! 1. **No GPUI imports.** No `gpui::*`, no `gpui_component::*`. The
//!    only allowed deps are the domain crates (`api`, `db`, `views`,
//!    `metadata`, `track_compare`, …) and `std`. This is enforced by
//!    review — if you reach for `SharedString` or `AnyElement` here,
//!    the abstraction is wrong; expose a plain `String` and let the
//!    screen wrap it.
//! 2. **No service mutation inside the VM constructor or its
//!    accessors.** Construction is pure projection over already-loaded
//!    data; mutating commands (downloads, subscribes, playlist edits)
//!    are exposed as `Command` values the screen dispatches on the
//!    appropriate `*_service` module.
//! 3. **Every public projection is unit-testable without a `Window`
//!    or `App`.** If a method needs a `cx`, it doesn't belong here.
//! 4. **Borrow, don't clone.** VMs hold short-lived borrows of the
//!    screen's owned data (`&str`, `&[Feed]`, …). They're
//!    constructed fresh each render and dropped before the element
//!    tree is painted. This avoids any extra `Arc`/`clone` churn.
//! 5. **One module per screen.** `view_models::feed`, `view_models::track`,
//!    `view_models::library`, `view_models::search_results`. Shared
//!    helpers live alongside in this `mod.rs` or a `common` submodule.
//!
//! ## Reference implementation
//!
//! See `library::PlaylistDetailVm` for the pattern: a borrow-only
//! struct with pure projection methods (`detail_rows`, `page`) and a
//! sibling `#[cfg(test)] mod tests` that pins each display invariant.
//! Copy that shape for a new view model.

#![warn(clippy::pedantic)]

pub(crate) mod app_toolbar;
pub(crate) mod cached_files;
pub(crate) mod collection_art;
pub(crate) mod releases_everything;
pub mod startup;

pub mod album_page;
pub mod artist_identity;
pub(crate) mod artists_everything;
pub(crate) mod detail_selection;
pub(crate) mod download_progress;
pub mod entity_detail;
pub mod feed;
pub mod format;
pub(crate) mod index_feed_singles;
pub(crate) mod index_name_list;
pub mod library;
pub mod library_removal;
pub(crate) mod list_selection;
pub(crate) mod live_status;
pub mod metadata;
pub mod musicbrainz_panel;
pub(crate) mod name_level_page;
pub(crate) mod named_feeds;
pub mod paged_playlist_detail;
pub(crate) mod pagination;
pub mod playlist_detail;
pub(crate) mod playlist_rss_check;
pub(crate) mod publisher_page;
pub(crate) mod queue_now_playing;
pub(crate) mod recent_feeds;
pub(crate) mod release_download_state;
pub(crate) mod release_kind;
pub(crate) mod release_page;
pub(crate) mod release_row_actions;
pub(crate) mod role_words;
pub mod search_results;
pub(crate) mod selection_action_bar;
pub(crate) mod show;
pub mod singles_group;
pub(crate) mod tag_update;
pub(crate) mod text_filter;
pub(crate) mod text_selection;
pub mod track;
pub mod track_detail;
pub mod track_metadata_grid;
pub(crate) mod track_page;
pub(crate) mod workspace;

use crate::db;

/// Semantic tone for action-row status messages.
///
/// Screens map this to UI tokens, but the VM owns which messages are neutral
/// status and which are danger/error status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionStatusMessageTone {
    Neutral,
    Danger,
}

/// Width policy for action-row status messages.
///
/// The VM names the presentation intent; GPUI code maps it to concrete token
/// widths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionStatusMessageWidth {
    Status,
    Action,
    Conflict,
}

/// Display-ready status message for action rows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionStatusMessageDisplay {
    pub(crate) text: String,
    pub(crate) tone: ActionStatusMessageTone,
    pub(crate) width: ActionStatusMessageWidth,
}

/// Display-ready playlist option facts shared by playlist popovers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistOptionDisplayVm {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) a11y_label: String,
}

#[must_use]
pub(crate) fn playlist_option_displays(playlists: &[db::Playlist]) -> Vec<PlaylistOptionDisplayVm> {
    playlists
        .iter()
        .map(|playlist| {
            let a11y_label = if playlist.name.trim().is_empty() {
                "Add to unnamed playlist".to_string()
            } else {
                format!("Add to playlist {}", playlist.name)
            };
            PlaylistOptionDisplayVm {
                id: playlist.id,
                name: playlist.name.clone(),
                a11y_label,
            }
        })
        .collect()
}

impl ActionStatusMessageDisplay {
    #[must_use]
    pub(crate) fn neutral(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tone: ActionStatusMessageTone::Neutral,
            width: ActionStatusMessageWidth::Status,
        }
    }

    #[must_use]
    pub(crate) fn danger(text: impl Into<String>, width: ActionStatusMessageWidth) -> Self {
        Self {
            text: text.into(),
            tone: ActionStatusMessageTone::Danger,
            width,
        }
    }

    #[must_use]
    pub(crate) fn subscription(message: Option<&str>) -> Option<Self> {
        let message = message?;
        if message.to_lowercase().contains("error") {
            Some(Self::danger(message, ActionStatusMessageWidth::Status))
        } else {
            Some(Self::neutral(message))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        playlist_option_displays, ActionStatusMessageDisplay, ActionStatusMessageTone,
        ActionStatusMessageWidth, PlaylistOptionDisplayVm,
    };
    use crate::db;

    #[test]
    fn action_status_message_display_classifies_subscription_messages() {
        assert_eq!(
            ActionStatusMessageDisplay::subscription(Some("Downloaded")),
            Some(ActionStatusMessageDisplay {
                text: "Downloaded".into(),
                tone: ActionStatusMessageTone::Neutral,
                width: ActionStatusMessageWidth::Status,
            })
        );
        assert_eq!(
            ActionStatusMessageDisplay::subscription(Some("Download error: offline")),
            Some(ActionStatusMessageDisplay {
                text: "Download error: offline".into(),
                tone: ActionStatusMessageTone::Danger,
                width: ActionStatusMessageWidth::Status,
            })
        );
        assert_eq!(ActionStatusMessageDisplay::subscription(None), None);
    }

    #[test]
    fn playlist_option_displays_project_ids_and_names() {
        let playlists = vec![
            db::Playlist {
                id: 7,
                name: "Focus".into(),
                description: None,
                track_count: 0,
                created_at: 0,
                updated_at: 0,
            },
            db::Playlist {
                id: 9,
                name: String::new(),
                description: Some("blank names are preserved".into()),
                track_count: 1,
                created_at: 1,
                updated_at: 2,
            },
        ];

        assert_eq!(
            playlist_option_displays(&playlists),
            vec![
                PlaylistOptionDisplayVm {
                    id: 7,
                    name: "Focus".into(),
                    a11y_label: "Add to playlist Focus".into(),
                },
                PlaylistOptionDisplayVm {
                    id: 9,
                    name: String::new(),
                    a11y_label: "Add to unnamed playlist".into(),
                },
            ]
        );
    }
}

pub(crate) mod settings;

pub(crate) mod log_view;
pub(crate) mod maintenance;
