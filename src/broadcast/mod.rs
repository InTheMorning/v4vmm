//! Broadcast control helpers for ADR 0059.
//!
//! The broadcast chain must keep running when the desktop app is closed. This
//! module owns GPUI-free helpers that support the app's control-surface state.

pub mod control;
pub mod registry;
pub mod tokens;
