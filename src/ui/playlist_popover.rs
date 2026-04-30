//! Re-export shim — `AddToPlaylistPopover` now lives in
//! [`crate::ui::composites::playlist_popover`]. This module keeps the
//! old import path working while call sites are migrated.
//!
//! New code should import from `crate::ui::composites::AddToPlaylistPopover`.

pub use crate::ui::composites::AddToPlaylistPopover;
