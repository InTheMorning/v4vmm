//! Fixed layout geometry for reusable UI shells and legacy screen parity.
//!
//! This module is the named layout boundary from the design-system target:
//! screens and composites may use these stable dimensions while fuller layout
//! shells continue moving into `ui::composites` or future layout modules.

#![warn(clippy::pedantic)]

use gpui::{px, AbsoluteLength, App, Pixels, TextStyle};

use crate::ui::tokens::{FontSize, ScaleFactor, Size, Spacing};

pub const WINDOW_WIDTH: Pixels = px(1120.0);
pub const WINDOW_HEIGHT: Pixels = px(760.0);
pub const TAB_BAR_HEIGHT: Pixels = px(44.0);
pub const APP_TOOLBAR_GLOBAL_SEARCH_COMPACT_BREAKPOINT: Pixels = px(1120.0);
pub const FILTER_CHIP_STRIP_NARROW_COLLAPSE_BREAKPOINT: Pixels = px(640.0);
pub const WORKSPACE_QUEUE_COLLAPSE_BREAKPOINT: Pixels = px(1024.0);
pub const WORKSPACE_SECONDARY_DETAIL_COLLAPSE_BREAKPOINT: Pixels = px(840.0);
pub const ROW_HEIGHT: Pixels = px(36.0);
pub const MIN_HIT_TARGET: Pixels = px(44.0);
pub const HIT_TARGET_MIN: Pixels = MIN_HIT_TARGET;
pub const CONTROL_FOCUS_RING_WIDTH: Pixels = px(2.0);
pub const INSPECTOR_WIDTH: Pixels = px(360.0);
pub const INSPECTOR_MIN_WIDTH: Pixels = px(200.0);
pub const INSPECTOR_MAX_WIDTH: Pixels = px(800.0);
pub const SPLIT_HANDLE_WIDTH: Pixels = px(5.0);
/// ADR 0046: stacked source navigation leaves two thirds for the current content.
pub(crate) const SPLIT_STACKED_LEADING_FRACTION: f32 = 1.0 / 3.0;
/// ADR 0046: leave usable navigation and detail viewports during vertical resizing.
pub(crate) const SPLIT_STACKED_MIN_HEIGHT: Pixels = px(120.0);
/// ADR 0066: keep normal-shell recovery notices from consuming the workspace.
pub(crate) const CAPABILITY_NOTICE_MAX_HEIGHT: Pixels = px(160.0);
pub const CONTENT_PANE_DEFAULT_WIDTH: Pixels = px(1024.0);
pub const CONTENT_PANE_MIN_WIDTH: Pixels = px(320.0);
pub const CONTENT_PANE_MAX_WIDTH: Pixels = px(1600.0);
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
/// Scrollable viewport for configuration correction text (ADR 0066).
pub(crate) const CONFIGURATION_EDITOR_HEIGHT: Pixels = px(160.0);
/// ADR 0074: actions remain separate from the active instructions or report.
pub(crate) const MAINTENANCE_ACTION_COLUMN_WIDTH: Pixels = px(256.0);
pub(crate) const MAINTENANCE_ACTION_BAND_FRACTION: f32 = 0.25;
pub const MENU_MIN_WIDTH: Pixels = px(320.0);
pub const MENU_MAX_WIDTH: Pixels = px(520.0);
pub const TRACK_NUMBER_WIDTH: Pixels = px(24.0);

#[must_use]
pub fn scaled_dimension(base: Pixels, cx: &App) -> Pixels {
    scaled_f32(f32::from(base), cx)
}

#[must_use]
pub fn scaled_f32(base: f32, cx: &App) -> Pixels {
    // ADR 0039: pure geometry, so this resolves through the CHROME domain.
    px(base * ScaleFactor::current(cx).chrome_multiplier())
}

/// ADR 0063: shared embedded log allocation and scrollbar clearance.
pub const LOG_FRAME_HEIGHT: Pixels = px(200.0);
pub const LOG_FRAME_BORDER: Pixels = px(1.0);
pub const LOG_SCROLLBAR_GUTTER: Pixels = px(16.0);

/// Default gpui-base overlay scrollbar track width.
/// ADR 0063: reserve this width plus scaled separation for nested controls.
pub(crate) const OVERLAY_SCROLLBAR_WIDTH: Pixels = gpui_base::Scrollbar::width();

// -----------------------------------------------------------------------------
// ADR 0039 task 002 — fixed-height reservation.
// -----------------------------------------------------------------------------
//
// This is the shared geometry owner for the fixed-height reservation ADR 0039
// requires: a fixed-height row/card reserves space for its largest permitted
// type step so text readable at medium does not acquire vertical clipping at
// either scale extreme, x-small or x-large. Two things live here, and they
// stay deliberately distinct:
//
// - `line_box_height` is the *measured* line-box height GPUI resolves for a
//   bare `.text_size(f)` with no `.line_height()` override, derived through
//   GPUI's own `TextStyle`/`phi()`/`DefiniteLength` API rather than a
//   hand-copied ratio, so a future gpui-pre upgrade that changes the default
//   line-height policy fails the pinned test below instead of silently
//   drifting every reservation.
// - `reservation_endpoints`, `reservation_ceiling_factor` and
//   `reservation_line_box` use the endpoints ratified in ADR 0039 task 003.
//   The decision date is 2026-09-18. See docs/adr/0039-dynamic-type-ramp.md.
//   Task 003 reduced the proposed downward endpoints and retained the upward endpoints.
//   These helpers calculate reservation bounds. They do not resolve rendered font sizes.
//   `FontSize::type_multiplier` in src/ui/tokens.rs maintains the same values separately.
//   It must not read this reservation table.
//   The review records checks at all five steps on 2026-09-18.
//   See docs/reviews/adr-0039-review-checklist.md.

/// ADR 0039: the line-box height GPUI renders for `div().text_size(font_size)`
/// with no `.line_height()` override.
///
/// Mirrors `TextStyle::line_height_in_pixels` exactly: `TextStyle::default()`
/// already carries GPUI's golden-ratio line height (`phi()`,
/// gpui-pre-0.3.1/src/geometry.rs:3708-3711), and the `Fraction` branch of
/// `DefiniteLength::to_pixels` (geometry.rs:3496-3504) multiplies the text
/// style's own `font_size` field — never `rem_size` — so the `rem_size`
/// argument below is inert for this call and passed as zero. No ancestor of
/// the surfaces this module sizes overrides `line_height` (ADR 0039 Audited
/// Findings, 2026-09-18).
#[must_use]
pub(crate) fn line_box_height(font_size: Pixels) -> Pixels {
    let style = TextStyle {
        font_size: AbsoluteLength::Pixels(font_size),
        ..TextStyle::default()
    };
    style.line_height_in_pixels(px(0.0))
}

/// Returns the reservation endpoints for this role
///
/// ADR 0039 task 003 ratified these `(x-small factor, x-large factor)` values on 2026-09-18.
/// `FontSize::type_multiplier` maintains the same values in src/ui/tokens.rs.
/// That resolver must not read this table.
const fn reservation_endpoints(role: FontSize) -> (f32, f32) {
    match role {
        FontSize::Micro => (0.91, 1.36),
        FontSize::Caption => (0.90, 1.32),
        FontSize::Body => (0.89, 1.28),
        FontSize::Headline => (0.88, 1.24),
        FontSize::Title3 => (0.87, 1.20),
        FontSize::Title2 => (0.86, 1.16),
        FontSize::Title => (0.85, 1.12),
    }
}

/// ADR 0039 sizing ceiling: the ratified interpolation factor at `scale` for
/// `role` (docs/adr/0039-dynamic-type-ramp.md, ratified 2026-09-18), reusing
/// `ScaleFactor::chrome_multiplier`'s value as the step coordinate `c`. Below
/// medium: small uses 8/15 of the downward change, x-small the full `d`
/// endpoint. Above medium: large uses 12/25 of the upward change, x-large the
/// full `u` endpoint.
fn reservation_ceiling_factor(role: FontSize, scale: ScaleFactor) -> f32 {
    let (d, u) = reservation_endpoints(role);
    match scale {
        ScaleFactor::Medium => 1.0,
        ScaleFactor::XSmall | ScaleFactor::Small => {
            let c = scale.chrome_multiplier();
            1.0 - ((1.0 - c) / 0.15) * (1.0 - d)
        }
        ScaleFactor::Large | ScaleFactor::XLarge => {
            let c = scale.chrome_multiplier();
            1.0 + ((c - 1.0) / 0.25) * (u - 1.0)
        }
    }
}

/// ADR 0039 sizing ceiling: `role`'s maximum permitted line-box height at
/// `scale`, sized against the ratified numeric decision (see module docs
/// above). Takes no text — the reservation cannot depend on string length.
#[must_use]
pub(crate) fn reservation_line_box(role: FontSize, scale: ScaleFactor) -> Pixels {
    let ceiling_px = px(f32::from(role.px()) * reservation_ceiling_factor(role, scale));
    line_box_height(ceiling_px)
}

/// ADR 0039: inner height left for content once vertical padding and border
/// subtract from a border-box `.h()` (box-sizing is border-box:
/// taffy-0.13.0/src/style/mod.rs:598-605; gpui-pre-0.3.1/src/taffy.rs:479-513
/// never overrides it). Shared by every capped fixed-height consumer so no
/// call site re-derives its own subtraction.
#[must_use]
pub(crate) fn available_inner_height(
    box_height: Pixels,
    vertical_padding: Pixels,
    border_width: Pixels,
) -> Pixels {
    box_height - vertical_padding * 2.0 - border_width * 2.0
}

/// ADR 0039 task 002: `ShowCard`'s reserved header+summary block — the taller
/// of the title line or the state badge, the header/body gap, two body lines
/// and the inter-line gap. Mirrors `src/ui/composites/show_card.rs`'s actual
/// layout (`render_header`/`render_summary_lines`); a shape change there must
/// update this function to stay accurate.
#[must_use]
pub(crate) fn show_card_summary_reservation(scale: ScaleFactor) -> Pixels {
    let headline_line = reservation_line_box(FontSize::Headline, scale);
    let badge_padding = px(f32::from(Spacing::XXS.px()) * scale.chrome_multiplier());
    let badge_height = reservation_line_box(FontSize::Micro, scale) + badge_padding * 2.0;
    let header_height = if headline_line > badge_height {
        headline_line
    } else {
        badge_height
    };
    let header_gap = px(f32::from(Spacing::SM.px()) * scale.chrome_multiplier());
    let body_line = reservation_line_box(FontSize::Body, scale);
    let body_gap = px(f32::from(Spacing::XS.px()) * scale.chrome_multiplier());
    header_height + header_gap + body_line * 2.0 + body_gap
}

/// ADR 0039 task 002: the inner height `Size::MenuCompact` (`ShowCard`'s fixed
/// `.h()`, pinned by `adr_0063_show_card_grid_shell_uses_vm_contract`) leaves
/// once its own `Spacing::MD` padding and 1px border subtract, at `scale`.
#[must_use]
pub(crate) fn show_card_available_inner_height(scale: ScaleFactor) -> Pixels {
    let card_height = px(f32::from(Size::MenuCompact.px()) * scale.chrome_multiplier());
    let padding = px(f32::from(Spacing::MD.px()) * scale.chrome_multiplier());
    available_inner_height(card_height, padding, px(1.0))
}

/// ADR 0039 task 002: a button's reserved single label line at `scale`. A
/// button label is always one line — see `src/ui/primitives/button.rs`.
#[must_use]
pub(crate) fn button_label_reservation(role: FontSize, scale: ScaleFactor) -> Pixels {
    reservation_line_box(role, scale)
}

#[cfg(test)]
mod tests {
    use super::{
        available_inner_height, button_label_reservation, line_box_height,
        show_card_available_inner_height, show_card_summary_reservation,
    };
    use crate::ui::tokens::{FontSize, ScaleFactor};
    use gpui::px;

    /// ADR 0039: pins the GPUI line-height ratio this module's reservation
    /// arithmetic depends on. If a gpui-pre upgrade changes `phi()` or the
    /// `Fraction` branch of `DefiniteLength::to_pixels`, this fails loudly
    /// instead of silently shifting every reservation in this module.
    #[test]
    fn adr_0039_line_box_height_matches_gpui_phi_rounding() {
        // round(100 * 1.618_034) = round(161.8034) = 162.
        assert_eq!(line_box_height(px(100.0)), px(162.0));
        // The medium ShowCard headline case below depends on this exact value:
        // round(15 * 1.618_034) = round(24.270_51) = 24.
        assert_eq!(line_box_height(px(15.0)), px(24.0));
    }

    /// ADR 0039 task 003 rechecks ShowCard capacity at all five steps.
    /// The reserved header and summary must fit within `Size::MenuCompact`.
    /// Ratification reduced the reserved height at `XSmall` and `Small`.
    /// It retained the `Medium`, `Large` and `XLarge` figures.
    /// See docs/reviews/adr-0039-review-checklist.md for the original figures.
    /// The assertions detect changes to chrome, padding, gaps or endpoints.
    #[test]
    fn adr_0039_show_card_reservation_fits_available_at_all_steps() {
        let cases = [
            (ScaleFactor::XSmall, 113.60, 69.20),
            (ScaleFactor::Small, 123.12, 74.04),
            (ScaleFactor::Medium, 134.0, 78.0),
            (ScaleFactor::Large, 150.32, 88.44),
            (ScaleFactor::XLarge, 168.0, 99.0),
        ];
        for (scale, expected_available, expected_reserved) in cases {
            let available = show_card_available_inner_height(scale);
            let reserved = show_card_summary_reservation(scale);
            assert!(
                (f32::from(available) - expected_available).abs() < 0.05,
                "{scale:?}: available {available:?} != {expected_available}"
            );
            assert!(
                (f32::from(reserved) - expected_reserved).abs() < 0.05,
                "{scale:?}: reserved {reserved:?} != {expected_reserved}"
            );
            assert!(
                reserved <= available,
                "ADR 0039 task 002: ShowCard reservation {reserved:?} exceeds \
                 available inner height {available:?} at {scale:?}"
            );
        }
    }

    /// ADR 0039 task 003 rechecks each button size and role at all five steps.
    /// Each label reservation must fit within the button height.
    /// The tightest case remains Sm/Caption at x-small.
    /// Its reservation is 17 px, compared with the proposed 19 px.
    /// Available height is 23.80 px, or 21.80 px with the optional border.
    #[test]
    fn adr_0039_button_reservation_fits_available_at_all_steps() {
        let box_heights = [
            (px(28.0), FontSize::Caption),
            (px(32.0), FontSize::Body),
            (px(40.0), FontSize::Headline),
        ];
        for (base_height, role) in box_heights {
            for scale in [
                ScaleFactor::XSmall,
                ScaleFactor::Small,
                ScaleFactor::Medium,
                ScaleFactor::Large,
                ScaleFactor::XLarge,
            ] {
                let box_height = px(f32::from(base_height) * scale.chrome_multiplier());
                let reserved = button_label_reservation(role, scale);
                for border_width in [px(0.0), px(1.0)] {
                    let available = available_inner_height(box_height, px(0.0), border_width);
                    assert!(
                        reserved <= available,
                        "ADR 0039 task 002: Button reservation {reserved:?} exceeds \
                         available {available:?} for {role:?} at {scale:?} \
                         (border {border_width:?})"
                    );
                }
            }
        }

        // Pin the named tightest case exactly.
        let reserved = button_label_reservation(FontSize::Caption, ScaleFactor::XSmall);
        assert_eq!(reserved, px(17.0));
        let available = available_inner_height(px(23.8), px(0.0), px(0.0));
        assert!((f32::from(available) - 23.8).abs() < 0.05);
        assert!(reserved <= available);
    }

    /// ADR 0039 task 002 M1: the reservation is a pure function of role/size
    /// and scale step — its signature carries no text, so it structurally
    /// cannot depend on string length. Calling it twice with the same inputs
    /// must return the same value.
    #[test]
    fn adr_0039_reservation_does_not_take_text_and_is_deterministic() {
        for scale in [
            ScaleFactor::XSmall,
            ScaleFactor::Small,
            ScaleFactor::Medium,
            ScaleFactor::Large,
            ScaleFactor::XLarge,
        ] {
            assert_eq!(
                show_card_summary_reservation(scale),
                show_card_summary_reservation(scale)
            );
            assert_eq!(
                button_label_reservation(FontSize::Body, scale),
                button_label_reservation(FontSize::Body, scale)
            );
        }
    }
}
