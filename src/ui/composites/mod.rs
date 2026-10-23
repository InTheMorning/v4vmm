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

pub(crate) mod startup_report;

pub mod action_button;
pub mod action_row;
pub mod breadcrumb_trail;
pub mod collection_art;
pub mod confirmation_dialog;
pub mod cover_backdrop;
pub mod detail_grid;
pub mod detail_header;
pub mod disclosure_group;
pub mod download_status_item;
pub mod file_header;
pub mod frame_shell;
pub mod identity_action;
pub mod list_row;
pub mod live_status_strip;
pub(crate) mod music_column_scroll;
pub mod musicbrainz_panel;
pub(crate) mod page_scroll_content;
pub mod playlist_popover;
pub mod release_detail_surface;
pub(crate) mod release_download_state;
pub(crate) mod release_row_menu;
pub(crate) mod release_section_list;
pub(crate) mod search_field_list;
pub mod segmented_control;
pub mod selectable_text;
pub(crate) mod selectable_tile;
pub(crate) mod selection_action_bar;
pub mod show_card;
pub mod show_detail_panel;
pub mod show_log_pane;
pub mod skeleton_inspector;
pub mod skeleton_track_row;
pub mod split_pane;
pub mod tag_badge;
pub mod thumbnail;
pub mod track_detail_surface;
pub(crate) mod track_drag;
pub mod track_header;
pub mod track_metadata_grid;
pub mod track_row;
pub(crate) mod track_row_menu;
pub mod view_mode_control;

pub use action_button::{action_button, ActionButtonDisplay};
pub use action_row::{
    ActionRow, ActionRowDisplay, ActionRowMessage, ActionRowMessageDisplay, ActionRowMessageTone,
};
pub(crate) use breadcrumb_trail::BreadcrumbTrail;
pub use collection_art::{CollectionArt, CollectionArtImage};
pub use confirmation_dialog::{
    confirmation_dialog, ConfirmationDialogDisplay, ConfirmationDialogHandlers,
    ConfirmationDialogItem,
};
pub use cover_backdrop::CoverBackdrop;
pub use detail_grid::{DetailElementRow, DetailGrid, DetailRow, DetailTextRow};
pub use detail_header::{DetailHeader, DetailHeaderDataRow, DetailHeaderDisplay};
pub use disclosure_group::{
    disclosure_column, DisclosureGroup, DisclosureGroupDisplay, DisclosureIndicator,
    DisclosureIndicatorDisplay, DisclosureSupplementDisplay, DisclosureSupplementLabel,
    DisclosureTextPanel, DisclosureTextPanelDisplay,
};
pub(crate) use download_status_item::download_status_item;
pub use file_header::FileHeader;
#[expect(
    unused_imports,
    reason = "ADR 0046 Task 006 exports the frame shell before Task 007 wires it"
)]
pub(crate) use frame_shell::{frame_shell, FrameShell, FrameShellSlots};
pub use identity_action::{
    identity_action_button, IdentityActionButtonDisplay, IdentityActionKind,
};
pub use list_row::{ListRow, ListRowA11yLabel, ListRowDensity};
pub(crate) use live_status_strip::{live_status_strip, LiveStatusStrip, LiveStatusStripSlots};
pub use musicbrainz_panel::MusicBrainzPanel;
pub use playlist_popover::{
    AddToPlaylistDisplay, AddToPlaylistPopover, PlaylistOption, PlaylistOptionDisplay,
};
pub use release_detail_surface::{
    ReleaseActionGroupDisplay, ReleaseDetailSurface, ReleaseSurfaceElement,
    ReleaseTrackSectionDisplay,
};
pub use segmented_control::{Segment, SegmentDisplay, SegmentedControl, SegmentedControlStyle};
pub(crate) use selectable_text::SelectableText;
pub(crate) use selection_action_bar::{SelectionActionBar, SelectionActionBarPlaylists};
pub(crate) use show_card::ShowCard;
pub(crate) use show_detail_panel::{ShowDetailPanel, ShowDetailPanelDisplay, ShowDetailPanelSlots};
pub(crate) use show_log_pane::{ShowLogPane, ShowLogPaneSlots};
pub use skeleton_inspector::SkeletonInspector;
pub use skeleton_track_row::SkeletonTrackRow;
pub use split_pane::SplitPane;
pub use tag_badge::{EntityKind, ProvenanceRole, StatusRole, TagBadge, TagBadgeDisplay};
pub use thumbnail::{Thumbnail, ThumbnailSize};
pub(crate) use track_detail_surface::render_feed_identity_panel;
pub use track_detail_surface::{TrackDetailSurface, TrackSurfaceElement};
pub use track_header::TrackHeader;
pub use track_metadata_grid::{
    TrackMetadataFieldCell, TrackMetadataFieldDisplay, TrackMetadataFrameDisplay,
    TrackMetadataGrid, TrackMetadataGroupCell, TrackMetadataGroupDisplay, TrackMetadataSourceCell,
    TrackMetadataTagCell, TrackMetadataTagDisplay, TrackMetadataTextDisplay,
    TrackMetadataTextValue,
};
pub use track_row::TrackRow;
pub(crate) use track_row_menu::TrackRowMenuHandlers;
pub(crate) use view_mode_control::{view_mode_control, ViewModeControlSlots};

pub(crate) mod maintenance_forms;
pub(crate) mod maintenance_page;
pub(crate) mod settings;

pub(crate) mod log_frame;
