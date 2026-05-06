//! Discover screen-specific shells.
//!
//! Each child module owns one Discover surface (search input, result list,
//! recent feeds tiles, feed inspector, track inspector). Surfaces accept
//! `&mut Context<SearchApp>` directly and dispatch mutations via
//! `cx.listener(...)` calls into screen-side mutator methods.
//!
//! Selected-entity state stays in `crate::search::SearchApp.inspector_stack`.
//! Surfaces are render-only after their callbacks return; they do not retain
//! state.
//!
//! See `docs/adr/0038-presentation-contract-enforcement.md` and
//! `docs/tasks/adr-0038-task-007-screen-decomposition.md`.

#![warn(clippy::pedantic)]

pub mod actions;
pub mod feed_inspector;
pub mod feed_lists;
pub mod recent;
pub mod result_list;
pub mod search_input;
pub mod track_inspector;
pub mod track_inspector_metadata;
pub mod track_inspector_metadata_grid;
pub mod track_rows;
