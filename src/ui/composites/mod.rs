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
pub mod action_row;
pub mod detail_grid;
pub mod detail_header;
pub mod disclosure_group;
pub mod file_header;
pub mod identity_action;
pub mod list_row;
pub mod musicbrainz_panel;
pub mod now_playing_bar;
pub mod playlist_popover;
pub mod release_detail_surface;
pub mod segmented_control;
pub mod split_pane;
pub mod tag_badge;
pub mod thumbnail;
pub mod track_detail_surface;
pub mod track_header;
pub mod track_metadata_grid;
pub mod track_row;

pub use action_button::{action_button, ActionButtonDisplay};
pub use action_row::{
    ActionRow, ActionRowDisplay, ActionRowMessage, ActionRowMessageDisplay, ActionRowMessageTone,
};
pub use detail_grid::{DetailElementRow, DetailGrid, DetailRow, DetailTextRow};
pub use detail_header::{DetailHeader, DetailHeaderDataRow, DetailHeaderDisplay};
pub use disclosure_group::{
    DisclosureGroup, DisclosureGroupDisplay, DisclosureIndicator, DisclosureIndicatorDisplay,
    DisclosureSupplementDisplay, DisclosureSupplementLabel,
};
pub use file_header::FileHeader;
pub use identity_action::{
    identity_action_button, IdentityActionButtonDisplay, IdentityActionKind,
};
pub use list_row::{ListRow, ListRowA11yLabel, ListRowDensity};
pub use musicbrainz_panel::MusicBrainzPanel;
pub use now_playing_bar::{NowPlayingBar, NowPlayingData, PlaybackState as NowPlayingState};
pub use playlist_popover::{
    AddToPlaylistDisplay, AddToPlaylistPopover, PlaylistOption, PlaylistOptionDisplay,
};
pub use release_detail_surface::{
    ReleaseActionGroupDisplay, ReleaseDetailSurface, ReleaseSurfaceElement,
    ReleaseTrackSectionDisplay,
};
pub use segmented_control::{Segment, SegmentDisplay, SegmentedControl, SegmentedControlStyle};
pub use split_pane::SplitPane;
pub use tag_badge::{EntityKind, ProvenanceRole, StatusRole, TagBadge, TagBadgeDisplay};
pub use thumbnail::{Thumbnail, ThumbnailSize};
pub use track_detail_surface::{TrackDetailSurface, TrackSurfaceElement};
pub use track_header::TrackHeader;
pub use track_metadata_grid::{
    TrackMetadataFieldCell, TrackMetadataFieldDisplay, TrackMetadataFrameDisplay,
    TrackMetadataGrid, TrackMetadataGroupCell, TrackMetadataGroupDisplay, TrackMetadataSourceCell,
    TrackMetadataTagCell, TrackMetadataTagDisplay, TrackMetadataTextDisplay,
    TrackMetadataTextValue,
};
pub use track_row::TrackRow;
