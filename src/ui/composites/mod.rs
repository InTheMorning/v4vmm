//! Composite components — the **third** layer of the design system, sitting
//! above [`crate::ui::primitives`] and below screen views.
//!
//! Composites combine multiple primitives into a complete domain-agnostic
//! widget (a thumbnail, a key/value detail row, a tag badge, …). Like
//! primitives they:
//!
//! * Resolve every dimension through `.scaled(cx)` so the global
//!   [`crate::ui::tokens::ScaleFactor`] re-flows them.
//! * Resolve every color through [`crate::ui::tokens::SemanticColor`].
//! * Carry no domain logic — they accept already-prepared display data.
//!
//! Unlike primitives they may make opinionated layout choices (e.g. "the
//! detail header always shows a thumbnail to the left of the title block").
//!
//! See `docs/architecture-diagrams.md` § 2.3.

#![warn(clippy::pedantic)]

pub mod detail_grid;
pub mod detail_header;
pub mod list_row;
pub mod segmented_control;
pub mod tag_badge;
pub mod thumbnail;

pub use detail_grid::{DetailGrid, DetailRow};
pub use detail_header::DetailHeader;
pub use list_row::{ListRow, ListRowDensity};
pub use segmented_control::{Segment, SegmentedControl};
pub use tag_badge::{EntityKind, TagBadge};
pub use thumbnail::{Thumbnail, ThumbnailSize};
