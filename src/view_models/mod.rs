//! View-models — the layer between services and screens.
//!
//! A *view-model* is a thin, **GPUI-free** projection of domain data
//! (rows from `db`, hydrated `api::Feed` / `api::Track`, derived state
//! held by the screen) into the *display-ready* shape the view needs.
//! Screens then bind primitives + composites to that projection. This
//! is the same separation of concerns SwiftUI encourages with its
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

pub mod artist;
pub mod feed;
pub mod format;
pub mod track;
