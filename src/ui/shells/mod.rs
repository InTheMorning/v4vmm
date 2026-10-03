//! UI shells — the seventh layer of the design system.
//!
//! Shells sit above [`crate::ui::composites`] and below screen modules. A shell
//! is a top-level GPUI layout module that consumes view-models and composites
//! to produce a complete page or pane.
//!
//! Shells:
//! - Import view-models, composites, primitives, and tokens.
//! - Do not import screens (`src/library.rs`, `src/app/`), services, or
//!   backend modules.
//! - Carry no selected-entity state; that belongs to screens.
//! - Resolve all dimensions through `.scaled(cx)` and all colors through
//!   `SemanticColor`.
//!
//! Screen-specific shells live under `library/`. They are allowed to
//! reference their owning screen module (`crate::library::LibraryApp`)
//! because they are owned by that screen.
//!
//! See `docs/adr/archive/0038-presentation-contract-enforcement.md` for the layer
//! architecture invariant.

#![warn(clippy::pedantic)]

pub mod artist;
pub mod entity;
pub mod library;
pub mod library_removal_confirmation;
pub mod name_match_page;
pub mod playlist;
pub(crate) mod playlist_removal_confirmation;
pub mod publisher;
pub mod queue_now_playing;
pub mod search_result_rows;
pub mod search_results_inspector;
pub mod show;
pub(crate) mod tag_update_confirmation;
pub mod track;
pub mod window_layers;
pub mod workspace;
