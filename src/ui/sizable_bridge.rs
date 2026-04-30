//! Bridge between v4vmm's [`crate::ui::tokens::ScaleFactor`] and
//! [`gpui_component::Size`].
//!
//! gpui-component widgets (Button, Sidebar, Input, Table, Sheet, …) accept a
//! [`Size`](gpui_component::Size) discriminator (`XSmall`/`Small`/`Medium`/
//! `Large`) but have no awareness of the app-wide [`ScaleFactor`] global. This
//! module wraps that gap with a single pure mapping plus a
//! [`SizableScaled`] extension trait so call sites can write
//!
//! ```ignore
//! Button::new("save").primary().scaled(Size::Small, cx)
//! ```
//!
//! and the actual size shifts up or down with the user's preferred scale.
//!
//! ## Design
//!
//! `ScaleFactor` has five steps centred on `Medium`; gpui-component's `Size`
//! has four. We model each as an integer step, sum them, then clamp to the
//! gpui-component range. The custom [`Size::Size(Pixels)`](gpui_component::Size::Size)
//! variant is multiplied by [`ScaleFactor::multiplier`] instead — it
//! represents an explicit pixel value, not a discrete tier.
//!
//! See `docs/architecture/architecture-diagrams.md` § 2.4.

#![warn(clippy::pedantic)]

use crate::ui::tokens::ScaleFactor;
use gpui::App;
use gpui_component::{Sizable, Size};

/// Step offset of a [`ScaleFactor`] relative to [`ScaleFactor::Medium`].
///
/// Pure helper — exposed for tests.
#[must_use]
pub fn scale_step(scale: ScaleFactor) -> i32 {
    match scale {
        ScaleFactor::XSmall => -2,
        ScaleFactor::Small => -1,
        ScaleFactor::Medium => 0,
        ScaleFactor::Large => 1,
        ScaleFactor::XLarge => 2,
    }
}

/// Step index of a discrete [`Size`] tier (`XSmall=0` … `Large=3`).
///
/// Returns `None` for [`Size::Size`] (the custom-pixel variant has no tier).
#[must_use]
fn size_step(size: Size) -> Option<i32> {
    match size {
        Size::XSmall => Some(0),
        Size::Small => Some(1),
        Size::Medium => Some(2),
        Size::Large => Some(3),
        Size::Size(_) => None,
    }
}

/// Reconstruct a [`Size`] from a clamped step index.
fn size_from_step(step: i32) -> Size {
    match step.clamp(0, 3) {
        0 => Size::XSmall,
        1 => Size::Small,
        2 => Size::Medium,
        _ => Size::Large,
    }
}

/// Combine a base `Size` with the current `ScaleFactor`.
///
/// * Discrete tiers shift by the scale's signed step and clamp to the
///   gpui-component range.
/// * The [`Size::Size(Pixels)`](gpui_component::Size::Size) variant is
///   multiplied by [`ScaleFactor::multiplier`].
#[must_use]
pub fn scaled_for(base: Size, scale: ScaleFactor) -> Size {
    if let Size::Size(pixels) = base {
        return Size::Size(gpui::px(f32::from(pixels) * scale.multiplier()));
    }
    let Some(base_step) = size_step(base) else {
        unreachable!("non-Size variants handled above")
    };
    size_from_step(base_step + scale_step(scale))
}

/// Resolve `base` against the [`ScaleFactor`] currently installed on `cx`.
///
/// Convenience wrapper around [`scaled_for`] that reads
/// [`ScaleFactor::current`].
#[must_use]
pub fn scaled(base: Size, cx: &App) -> Size {
    scaled_for(base, ScaleFactor::current(cx))
}

/// Extension trait that adds a `.scaled(base, cx)` method to every
/// [`Sizable`] widget.
///
/// Call sites read like `SwiftUI` environment-aware modifiers:
///
/// ```ignore
/// Button::new("save").primary().scaled(Size::Small, cx)
/// ```
pub trait SizableScaled: Sizable {
    /// Apply `base` shifted by the current [`ScaleFactor`].
    #[must_use]
    fn scaled(self, base: Size, cx: &App) -> Self {
        self.with_size(scaled(base, cx))
    }
}

impl<T: Sizable> SizableScaled for T {}

#[cfg(test)]
mod tests {
    use super::{scale_step, scaled_for, size_from_step, ScaleFactor, Size};
    use gpui::px;

    #[test]
    fn scale_step_is_zero_at_medium_baseline() {
        assert_eq!(scale_step(ScaleFactor::Medium), 0);
    }

    #[test]
    fn scale_step_is_signed_around_baseline() {
        assert_eq!(scale_step(ScaleFactor::XSmall), -2);
        assert_eq!(scale_step(ScaleFactor::Small), -1);
        assert_eq!(scale_step(ScaleFactor::Large), 1);
        assert_eq!(scale_step(ScaleFactor::XLarge), 2);
    }

    #[test]
    fn medium_scale_is_a_no_op() {
        for base in [Size::XSmall, Size::Small, Size::Medium, Size::Large] {
            assert_eq!(scaled_for(base, ScaleFactor::Medium), base);
        }
    }

    #[test]
    fn larger_scale_steps_size_up() {
        assert_eq!(scaled_for(Size::Small, ScaleFactor::Large), Size::Medium);
        assert_eq!(scaled_for(Size::Small, ScaleFactor::XLarge), Size::Large);
        assert_eq!(scaled_for(Size::XSmall, ScaleFactor::Large), Size::Small);
    }

    #[test]
    fn smaller_scale_steps_size_down() {
        assert_eq!(scaled_for(Size::Medium, ScaleFactor::Small), Size::Small);
        assert_eq!(scaled_for(Size::Medium, ScaleFactor::XSmall), Size::XSmall);
        assert_eq!(scaled_for(Size::Large, ScaleFactor::XSmall), Size::Small);
    }

    #[test]
    fn size_clamps_at_the_extremes() {
        // Large + XLarge would step beyond Large; must clamp.
        assert_eq!(scaled_for(Size::Large, ScaleFactor::XLarge), Size::Large);
        // XSmall + XSmall would step below XSmall; must clamp.
        assert_eq!(scaled_for(Size::XSmall, ScaleFactor::XSmall), Size::XSmall);
    }

    #[test]
    fn custom_pixel_size_is_multiplied_not_stepped() {
        let base = Size::Size(px(20.0));
        let result = scaled_for(base, ScaleFactor::Large);
        match result {
            Size::Size(p) => assert!(
                (f32::from(p) - 20.0 * 1.12).abs() < 0.001,
                "expected 22.4, got {}",
                f32::from(p)
            ),
            other => panic!("expected Size::Size variant, got {other:?}"),
        }
    }

    #[test]
    fn size_from_step_clamps() {
        assert_eq!(size_from_step(-5), Size::XSmall);
        assert_eq!(size_from_step(0), Size::XSmall);
        assert_eq!(size_from_step(1), Size::Small);
        assert_eq!(size_from_step(2), Size::Medium);
        assert_eq!(size_from_step(3), Size::Large);
        assert_eq!(size_from_step(99), Size::Large);
    }
}
