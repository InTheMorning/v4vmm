//! Fixed layout geometry for reusable UI shells and legacy screen parity.
//!
//! This module is the named layout boundary from the design-system target:
//! screens and composites may use these stable dimensions while fuller layout
//! shells continue moving into `ui::composites` or future layout modules.

#![warn(clippy::pedantic)]

use gpui::{px, Pixels};

pub const WINDOW_WIDTH: Pixels = px(1120.0);
pub const WINDOW_HEIGHT: Pixels = px(760.0);
pub const TAB_BAR_HEIGHT: Pixels = px(44.0);
pub const APP_TOOLBAR_NOW_PLAYING_COMPACT_BREAKPOINT: Pixels = px(1280.0);
pub const APP_TOOLBAR_GLOBAL_SEARCH_COMPACT_BREAKPOINT: Pixels = px(1120.0);
pub const APP_TOOLBAR_SCOPE_BREAKPOINT: Pixels = px(1280.0);
pub const ROW_HEIGHT: Pixels = px(36.0);
pub const MIN_HIT_TARGET: Pixels = px(44.0);
pub const HIT_TARGET_MIN: Pixels = MIN_HIT_TARGET;
pub const INSPECTOR_WIDTH: Pixels = px(360.0);
pub const INSPECTOR_MIN_WIDTH: Pixels = px(200.0);
pub const INSPECTOR_MAX_WIDTH: Pixels = px(800.0);
pub const SPLIT_HANDLE_WIDTH: Pixels = px(5.0);
pub const APP_ICON_SIZE: Pixels = px(26.0);
pub const ACTION_ICON_SIZE: Pixels = px(18.0);
pub const ACTION_ICON_INNER_SIZE: Pixels = px(14.0);
pub const FEED_TILE_WIDTH: Pixels = px(140.0);
pub const SEARCH_TILE_WIDTH: Pixels = px(168.0);
pub const THUMBNAIL_XL: Pixels = px(152.0);
pub const COMPACT_COLUMN_WIDTH: Pixels = px(86.0);
pub const METADATA_LABEL_WIDTH: Pixels = px(136.0);
pub const METADATA_VALUE_INDENT: Pixels = px(142.0);
pub const PLAYLIST_THUMB_SLOT: Pixels = px(32.0);
pub const PLAYLIST_TITLE_OFFSET: Pixels = px(48.0);
pub const DETAIL_HEADER_TEXT_OFFSET: Pixels = px(96.0);
pub const STATUS_MESSAGE_WIDTH: Pixels = px(220.0);
pub const CONFLICT_MESSAGE_WIDTH: Pixels = px(190.0);
pub const ACTION_MESSAGE_WIDTH: Pixels = px(180.0);
pub const SETTINGS_COLUMN_WIDTH: Pixels = px(720.0);
pub const MENU_MIN_WIDTH: Pixels = px(320.0);
pub const MENU_MAX_WIDTH: Pixels = px(520.0);
pub const TRACK_NUMBER_WIDTH: Pixels = px(24.0);
