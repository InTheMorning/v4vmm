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
//! See `docs/architecture/architecture-diagrams.md` § 2.3.

#![warn(clippy::pedantic)]

pub mod action_button;
pub mod detail_grid;
pub mod detail_header;
pub mod disclosure_group;
pub mod list_row;
pub mod now_playing_bar;
pub mod playlist_popover;
pub mod release_detail_surface;
pub mod segmented_control;
pub mod split_pane;
pub mod tag_badge;
pub mod thumbnail;
pub mod track_row;

pub use action_button::action_button;
pub use detail_grid::{DetailGrid, DetailRow};
pub use detail_header::DetailHeader;
pub use disclosure_group::DisclosureGroup;
pub use list_row::{ListRow, ListRowDensity};
pub use now_playing_bar::{NowPlayingBar, NowPlayingData, PlaybackState as NowPlayingState};
pub use playlist_popover::AddToPlaylistPopover;
pub use release_detail_surface::ReleaseDetailSurface;
pub use segmented_control::{Segment, SegmentedControl};
pub use split_pane::SplitPane;
pub use tag_badge::{EntityKind, TagBadge};
pub use thumbnail::{Thumbnail, ThumbnailSize};
pub use track_row::TrackRow;
