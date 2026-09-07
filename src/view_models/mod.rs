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
//!    screen's owned data (`&ArtistView`, `&[Feed]`, …). They're
//!    constructed fresh each render and dropped before the element
//!    tree is painted. This avoids any extra `Arc`/`clone` churn.
//! 5. **One module per screen.** `view_models::artist`,
//!    `view_models::feed`, `view_models::track`,
//!    `view_models::library`, `view_models::search`. Shared helpers
//!    live alongside in this `mod.rs` or a `common` submodule.
//!
//! ## Reference implementation
//!
//! See [`artist::ArtistVm`] for the pattern: a borrow-only struct with
//! pure projection methods (`title`, `track_count_label`,
//! `detail_rows`) and a sibling `#[cfg(test)] mod tests` pinning every
//! display invariant. New VMs should copy that shape exactly.

#![warn(clippy::pedantic)]

pub(crate) mod app_toolbar;
pub mod artist;
pub mod artist_detail;
pub mod entity_detail;
pub mod feed;
pub mod format;
pub mod library;
pub mod library_removal;
pub mod metadata;
pub mod musicbrainz_panel;
pub mod paged_feed_detail;
pub mod paged_playlist_detail;
pub(crate) mod pagination;
pub mod playlist_detail;
pub(crate) mod queue_now_playing;
pub(crate) mod recent_feeds;
pub mod search;
pub mod search_results;
pub(crate) mod text_filter;
pub mod track;
pub mod track_detail;
pub mod track_metadata_grid;
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

/// Pure resize state for a two-pane shell.
///
/// Screens own GPUI event wiring and convert framework pixel types into
/// `f32`; this state owns the clampable leading-pane width and drag lifecycle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SplitPaneState {
    leading_width: f32,
    resizing: bool,
}

impl SplitPaneState {
    #[must_use]
    pub(crate) const fn new(leading_width: f32) -> Self {
        Self {
            leading_width,
            resizing: false,
        }
    }

    #[must_use]
    pub(crate) fn leading_width(self) -> f32 {
        self.leading_width
    }

    #[must_use]
    pub(crate) fn is_resizing(self) -> bool {
        self.resizing
    }

    pub(crate) fn begin_resize(&mut self) {
        self.resizing = true;
    }

    pub(crate) fn end_resize(&mut self) {
        self.resizing = false;
    }

    pub(crate) fn resize_to(&mut self, requested_width: f32, min_width: f32, max_width: f32) {
        self.leading_width = requested_width.clamp(min_width, max_width);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        playlist_option_displays, ActionStatusMessageDisplay, ActionStatusMessageTone,
        ActionStatusMessageWidth, PlaylistOptionDisplayVm, SplitPaneState,
    };
    use crate::db;

    fn assert_width_eq(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn split_pane_state_tracks_resize_lifecycle() {
        let mut state = SplitPaneState::new(360.0);
        assert!(!state.is_resizing());

        state.begin_resize();
        assert!(state.is_resizing());

        state.end_resize();
        assert!(!state.is_resizing());
    }

    #[test]
    fn split_pane_state_clamps_width() {
        let mut state = SplitPaneState::new(360.0);

        state.resize_to(120.0, 200.0, 800.0);
        assert_width_eq(state.leading_width(), 200.0);

        state.resize_to(900.0, 200.0, 800.0);
        assert_width_eq(state.leading_width(), 800.0);

        state.resize_to(420.0, 200.0, 800.0);
        assert_width_eq(state.leading_width(), 420.0);
    }

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
