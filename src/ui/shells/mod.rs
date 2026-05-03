//! UI shells — the seventh layer of the design system.
//!
//! Shells sit above [`crate::ui::composites`] and below screen modules. A shell
//! is a top-level GPUI layout module that consumes view-models and composites
//! to produce a complete page or pane.
//!
//! Shells:
//! - Import view-models, composites, primitives, and tokens.
//! - Do not import screens (`src/library.rs`, `src/search.rs`, `src/app/`),
//!   services, or backend modules.
//! - Carry no selected-entity state; that belongs to screens.
//! - Resolve all dimensions through `.scaled(cx)` and all colors through
//!   `SemanticColor`.
//!
//! See `docs/adr/0038-presentation-contract-enforcement.md` for the layer
//! architecture invariant.

#![warn(clippy::pedantic)]

pub mod artist;
pub mod entity;
pub mod feed;
pub mod track;
