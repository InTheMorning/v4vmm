//! Library screen-specific shells.
//!
//! Each child module owns one Library surface (sidebar, feed list, feed detail,
//! track detail, playlist detail). Surfaces accept `&mut Context<LibraryApp>`
//! directly and dispatch mutations via `cx.listener(...)` calls into
//! screen-side mutator methods.
//!
//! Selected-entity state stays in `crate::library::LibraryApp.detail`. Surfaces
//! are render-only after their callbacks return; they do not retain state.
//!
//! See `docs/adr/0038-presentation-contract-enforcement.md` and
//! `docs/tasks/adr-0038-task-007-screen-decomposition.md`.

#![warn(clippy::pedantic)]

pub mod detail;
pub mod feed_detail;
pub mod feed_list;
pub mod playlist_detail;
pub mod sidebar;
pub mod thumbnail;
pub mod track_detail;
pub mod track_detail_metadata;
pub mod track_detail_metadata_cells;
pub mod track_detail_metadata_grid;
pub mod track_detail_metadata_values;
