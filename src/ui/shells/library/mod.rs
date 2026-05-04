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

pub mod feed_list;
pub mod playlist_detail;
