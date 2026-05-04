//! Design tokens — single source of truth for color, spacing, radius, and typography.
//!
//! Inspired by Apple Human Interface Guidelines: every value has a semantic name
//! (`Label`, `SecondaryLabel`, `SystemBackground`, `SystemFill`, `Separator`,
//! `Accent`, …) rather than a raw hex literal. Base color tokens resolve
//! against an [`Appearance`] (`Light` or `Dark`); app rendering resolves them
//! through the active [`crate::theme_profile::ThemeProfile`].
//!
//! Call sites should never use raw `rgb(0x…)` literals. Use:
//!
//! ```ignore
//! use crate::ui::tokens::{color, SemanticColor};
//! let label = color(cx, SemanticColor::Label);
//! ```
//!
//! Dark is the default appearance for v4vmm. Light values follow HIG so the
//! system can be flipped at runtime once a settings UI exists.

#![warn(clippy::pedantic)]

use gpui::{px, App, FontWeight, Pixels, Rgba, WindowAppearance};

use gpui_component::ActiveTheme;

use crate::theme_profile::ThemeProfile;

/// Visual appearance scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Appearance {
    Light,
    #[default]
    Dark,
}

impl Appearance {
    /// Returns the appearance currently bound to the rendering environment.
    ///
    /// Prefers the typed [`Environment`] global (the bundled SwiftUI-style
    /// accessor); falls back to the gpui-component theme mode for any path
    /// that hasn't installed the env yet.
    #[must_use]
    pub fn current(cx: &App) -> Self {
        if let Some(env) = cx.try_global::<Environment>() {
            return env.appearance;
        }
        if cx.theme().mode.is_dark() {
            Self::Dark
        } else {
            Self::Light
        }
    }
}

impl From<WindowAppearance> for Appearance {
    fn from(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::Light,
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::Dark,
        }
    }
}

// -----------------------------------------------------------------------------
// SemanticColor — Apple HIG-style palette.
// -----------------------------------------------------------------------------

/// Semantic color token. Always prefer this enum over hardcoded hex literals.
///
/// The naming follows Apple's HIG so designers can reason about role
/// (`Label`, `Fill`, `Background`) rather than appearance (`Gray`, `Blue`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::module_name_repetitions)]
pub enum SemanticColor {
    // Backgrounds — used for the window canvas and broad surfaces.
    SystemBackground,
    SecondarySystemBackground,
    TertiarySystemBackground,

    // Labels — text colors in descending prominence.
    Label,
    SecondaryLabel,
    TertiaryLabel,
    QuaternaryLabel,

    // Fills — chip / pill / inline button backgrounds.
    SystemFill,
    SecondaryFill,
    TertiaryFill,

    // Selection.
    SelectedContent,

    // Separators.
    Separator,
    OpaqueSeparator,

    // Tint / Accent.
    Accent,
    AccentHover,
    AccentPressed,
    OnAccent,
    Focus,

    // Status — system colors (HIG fidelity, used as fills / large text).
    Success,
    Warning,
    Danger,
    Info,

    // Status labels — darker variants tuned for body text on canvas.
    SuccessLabel,
    WarningLabel,
    DangerLabel,
    InfoLabel,

    // Text colors that pair with each filled status surface above.
    // Choice depends on the saturation of the fill in each appearance —
    // bright greens/oranges want near-black, dark reds/blues want white.
    OnSuccess,
    OnWarning,
    OnDanger,
    OnInfo,

    // v4vmm-specific provenance diff colors.
    DiffMatch,
    DiffDifferent,
    DiffMissing,

    // v4vmm-specific ID3 frame version chips. Used as label and frame-tag
    // color on metadata rows so the user can spot frame-version provenance
    // at a glance.
    Id3FrameV22,
    Id3FrameV23Only,
    Id3FrameV24Only,
    Id3FrameUnknown,
}

impl SemanticColor {
    /// Resolve the token to a concrete color for the given appearance.
    #[must_use]
    pub fn resolve(self, appearance: Appearance) -> Rgba {
        match appearance {
            Appearance::Dark => Self::dark_palette(self),
            Appearance::Light => Self::light_palette(self),
        }
    }

    fn dark_palette(token: Self) -> Rgba {
        // Dark palette mirrors v4vmm's existing aesthetic (cool, slightly
        // tinted) while satisfying WCAG AA contrast ratios for label-on-bg
        // pairs (label/SystemBackground = 14.4 : 1).
        //
        // Several semantic tokens intentionally resolve to the same hex —
        // e.g. `SystemFill` and `Separator` both want a subtle hairline at
        // `#2a2d3a`. Keep the match exhaustive so the symmetry with
        // `light_palette` stays obvious.
        #[expect(clippy::match_same_arms, reason = "different semantics, shared value")]
        match token {
            Self::SystemBackground => hex(0x0f_1117),
            Self::SecondarySystemBackground => hex(0x1a_1d27),
            Self::TertiarySystemBackground => hex(0x23_2735),

            Self::Label => hex(0xec_eef5),
            Self::SecondaryLabel => hex(0xb4_bacb),
            Self::TertiaryLabel => hex(0x8a_90a4),
            Self::QuaternaryLabel => hex(0x5f_6577),

            Self::SystemFill => hex(0x2a_2d3a),
            Self::SecondaryFill => hex(0x23_2735),
            Self::TertiaryFill => hex(0x1a_1d27),

            Self::SelectedContent => hex(0x2a_3352),

            Self::Separator => hex(0x2a_2d3a),
            Self::OpaqueSeparator => hex(0x6a_708a),

            Self::Accent => hex(0x8b_9bff),
            Self::AccentHover => hex(0xa5_b2ff),
            Self::AccentPressed => hex(0x74_86f5),
            Self::OnAccent => hex(0x0b_0d13),
            Self::Focus => hex(0xa8_b6ff),

            Self::Success => hex(0x7d_d67d),
            Self::Warning => hex(0xff_d666),
            Self::Danger => hex(0xff_8585),
            Self::Info => hex(0x8b_9bff),

            // In dark, the system colors already meet body-text contrast
            // against the canvas — labels can match the system value.
            Self::SuccessLabel => hex(0x7d_d67d),
            Self::WarningLabel => hex(0xff_d666),
            Self::DangerLabel => hex(0xff_8585),
            Self::InfoLabel => hex(0x8b_9bff),

            // Dark-mode status fills are all bright/saturated — near-black
            // text reads cleanly on every one of them.
            Self::OnSuccess => hex(0x0b_0d13),
            Self::OnWarning => hex(0x0b_0d13),
            Self::OnDanger => hex(0x0b_0d13),
            Self::OnInfo => hex(0x0b_0d13),

            Self::DiffMatch => hex(0x6f_d4a3),
            Self::DiffDifferent => hex(0xff_d27a),
            Self::DiffMissing => hex(0xff_a07f),

            Self::Id3FrameV22 => hex(0xb0_6cf4),
            Self::Id3FrameV23Only => hex(0xff_c857),
            Self::Id3FrameV24Only => hex(0x3a_c4c4),
            Self::Id3FrameUnknown => hex(0xff_8a65),
        }
    }

    fn light_palette(token: Self) -> Rgba {
        // Light palette follows Apple HIG iOS 17 / macOS Sonoma defaults.
        #[expect(clippy::match_same_arms, reason = "different semantics, shared value")]
        match token {
            Self::SystemBackground => hex(0xff_ffff),
            Self::SecondarySystemBackground => hex(0xf2_f2f7),
            Self::TertiarySystemBackground => hex(0xff_ffff),

            Self::Label => hex(0x00_0000),
            Self::SecondaryLabel => hex(0x3c_3c43),
            Self::TertiaryLabel => hex(0x6c_6c70),
            Self::QuaternaryLabel => hex(0xa9_a9ad),

            Self::SystemFill => hex(0xe5_e5ea),
            Self::SecondaryFill => hex(0xee_eef0),
            Self::TertiaryFill => hex(0xf2_f2f7),

            Self::SelectedContent => hex(0xd1_e0ff),

            Self::Separator => hex(0xc6_c6c8),
            Self::OpaqueSeparator => hex(0x8e_8e93),

            // iOS systemBlue and friends — Apple's iconic system colors.
            // These are intended as fills and large-text accents; for body
            // text use the *Label tokens below, which are darker.
            Self::Accent => hex(0x00_7aff),
            Self::AccentHover => hex(0x33_94ff),
            Self::AccentPressed => hex(0x00_64d1),
            Self::OnAccent => hex(0xff_ffff),
            Self::Focus => hex(0x33_94ff),

            Self::Success => hex(0x34_c759),
            Self::Warning => hex(0xff_9500),
            Self::Danger => hex(0xff_3b30),
            Self::Info => hex(0x00_7aff),

            // Label variants — tuned to satisfy 4.5:1 against SystemBackground
            // (white). Same intent as Apple's `.systemRed`/`.systemGreen`
            // *label* shades that ship in `UIColor` but are slightly darker
            // than the fills.
            Self::SuccessLabel => hex(0x1f_7a3a),
            Self::WarningLabel => hex(0xa0_5400),
            Self::DangerLabel => hex(0xc2_271c),
            Self::InfoLabel => hex(0x00_5fcc),

            // iOS systemGreen / systemOrange are too bright to take white
            // text — Apple's own HIG examples render black on them. Red and
            // blue fills are dark enough to take white.
            Self::OnSuccess => hex(0x00_0000),
            Self::OnWarning => hex(0x00_0000),
            Self::OnDanger => hex(0xff_ffff),
            Self::OnInfo => hex(0xff_ffff),

            Self::DiffMatch => hex(0x1f_7a3a),
            Self::DiffDifferent => hex(0x96_5a00),
            Self::DiffMissing => hex(0xb1_3c20),

            // Light-mode frame chips: darker, lower-saturation variants of
            // the dark-mode hues, tuned to read as labels on white.
            Self::Id3FrameV22 => hex(0x5b_3b9e),
            Self::Id3FrameV23Only => hex(0x8b_5a00),
            Self::Id3FrameV24Only => hex(0x00_6d77),
            Self::Id3FrameUnknown => hex(0xb1_3c20),
        }
    }
}

/// Resolve a semantic color against the current appearance.
#[must_use]
pub fn color(cx: &App, token: SemanticColor) -> Rgba {
    let env = Environment::current(cx);
    crate::ui::theme_profiles::resolve_profile_color_for_appearance(
        env.profile,
        env.appearance,
        token,
    )
}

/// Resolve a semantic color for a render context, honoring an explicit
/// light/dark appearance override when one is provided.
#[must_use]
pub fn resolve_color(cx: &App, token: SemanticColor, appearance: Option<Appearance>) -> Rgba {
    appearance.map_or_else(|| color(cx, token), |appearance| token.resolve(appearance))
}

// -----------------------------------------------------------------------------
// Spacing — 4-pt grid.
// -----------------------------------------------------------------------------

/// 4-pt spacing scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Spacing {
    /// 2 px.
    XXS,
    /// 4 px.
    XS,
    /// 8 px.
    SM,
    /// 12 px.
    MD,
    /// 16 px.
    LG,
    /// 24 px.
    XL,
    /// 32 px.
    XXL,
}

impl Spacing {
    /// Base (1.0×) value. Use [`Self::scaled`] when an `App` is in scope so
    /// the user's UI scale is honored.
    #[must_use]
    pub const fn px(self) -> Pixels {
        match self {
            Self::XXS => px(2.0),
            Self::XS => px(4.0),
            Self::SM => px(8.0),
            Self::MD => px(12.0),
            Self::LG => px(16.0),
            Self::XL => px(24.0),
            Self::XXL => px(32.0),
        }
    }

    /// Same as [`Self::px`] but multiplied by the active [`ScaleFactor`].
    #[must_use]
    pub fn scaled(self, cx: &App) -> Pixels {
        scale_px(f32::from(self.px()), ScaleFactor::current(cx))
    }
}

// -----------------------------------------------------------------------------
// Radius.
// -----------------------------------------------------------------------------

/// Corner radius scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Radius {
    /// 4 px — chips, badges.
    SM,
    /// 6 px — buttons, inputs.
    MD,
    /// 10 px — popovers, sheets.
    LG,
    /// 14 px — large overlays.
    XL,
    /// Pill shape (999 px).
    Full,
}

impl Radius {
    #[must_use]
    pub const fn px(self) -> Pixels {
        match self {
            Self::SM => px(4.0),
            Self::MD => px(6.0),
            Self::LG => px(10.0),
            Self::XL => px(14.0),
            Self::Full => px(999.0),
        }
    }

    #[must_use]
    pub fn scaled(self, cx: &App) -> Pixels {
        // The pill radius is intentionally capped — scaling 999px makes no
        // visual difference and risks integer overflow on extreme factors.
        if matches!(self, Self::Full) {
            return self.px();
        }
        scale_px(f32::from(self.px()), ScaleFactor::current(cx))
    }
}

// -----------------------------------------------------------------------------
// Typography.
// -----------------------------------------------------------------------------

/// Type-scale token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontSize {
    /// 11 px — caption / micro labels.
    Micro,
    /// 12 px — captions.
    Caption,
    /// 13 px — body.
    Body,
    /// 15 px — headline.
    Headline,
    /// 17 px — large title row.
    Title3,
    /// 20 px — section title.
    Title2,
    /// 24 px — page title.
    Title,
}

impl FontSize {
    #[must_use]
    pub const fn px(self) -> Pixels {
        match self {
            Self::Micro => px(11.0),
            Self::Caption => px(12.0),
            Self::Body => px(13.0),
            Self::Headline => px(15.0),
            Self::Title3 => px(17.0),
            Self::Title2 => px(20.0),
            Self::Title => px(24.0),
        }
    }

    #[must_use]
    pub fn scaled(self, cx: &App) -> Pixels {
        scale_px(f32::from(self.px()), ScaleFactor::current(cx))
    }
}

/// Type weight token (mirrors HIG weights).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weight {
    Regular,
    Medium,
    Semibold,
    Bold,
}

impl From<Weight> for FontWeight {
    fn from(w: Weight) -> Self {
        match w {
            Weight::Regular => Self::NORMAL,
            Weight::Medium => Self::MEDIUM,
            Weight::Semibold => Self::SEMIBOLD,
            Weight::Bold => Self::BOLD,
        }
    }
}

// -----------------------------------------------------------------------------
// Size — semantic widths/heights for menus, popovers, scrollable columns, etc.
// -----------------------------------------------------------------------------

/// Semantic size token for recurring container widths and heights.
///
/// These values are HIG-aligned (`NSPopover` menus typically run 160–280pt
/// wide; scrollable lists use 240/320/480pt depending on density). Always
/// prefer this enum over a hard-coded `px(220.)` so the entire UI re-scales
/// when a future `ScaleFactor` token is introduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Size {
    /// 28 px — compact button / toolbar affordance.
    ButtonSm,
    /// 32 px — default button.
    ButtonMd,
    /// 40 px — prominent dialog / sheet button.
    ButtonLg,
    /// 160 px — compact menu / dropdown.
    MenuCompact,
    /// 220 px — regular menu / popover (default).
    MenuRegular,
    /// 280 px — wide menu.
    MenuWide,
    /// 240 px — short scrollable column.
    ColumnShort,
    /// 320 px — regular scrollable column.
    ColumnRegular,
    /// 480 px — tall scrollable column.
    ColumnTall,
    /// 36 px — HIG menu row height (regular).
    RowMd,
    /// 44 px — HIG menu/list row height (touch-friendly).
    RowLg,
}

impl Size {
    #[must_use]
    pub const fn px(self) -> Pixels {
        match self {
            Self::ButtonSm => px(28.0),
            Self::ButtonMd => px(32.0),
            Self::ButtonLg => px(40.0),
            Self::MenuCompact => px(160.0),
            Self::MenuRegular => px(220.0),
            Self::MenuWide => px(280.0),
            Self::ColumnShort => px(240.0),
            Self::ColumnRegular => px(320.0),
            Self::ColumnTall => px(480.0),
            Self::RowMd => px(36.0),
            Self::RowLg => px(44.0),
        }
    }

    #[must_use]
    pub fn scaled(self, cx: &App) -> Pixels {
        scale_px(f32::from(self.px()), ScaleFactor::current(cx))
    }
}

// -----------------------------------------------------------------------------
// ScaleFactor — Apple Dynamic Type-style runtime UI scale.
// -----------------------------------------------------------------------------

/// Global UI scale, mirroring the named steps of iOS Dynamic Type.
///
/// Multiplied into every dimension token's `.scaled(cx)` accessor so the
/// entire UI can shrink or grow without any per-screen code change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ScaleFactor {
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
    XLarge,
}

impl ScaleFactor {
    #[must_use]
    pub const fn multiplier(self) -> f32 {
        match self {
            Self::XSmall => 0.85,
            Self::Small => 0.92,
            Self::Medium => 1.0,
            Self::Large => 1.12,
            Self::XLarge => 1.25,
        }
    }

    /// Returns the scale stored on `cx`, or [`ScaleFactor::Medium`] if none
    /// has been installed yet (e.g. early-startup paths or tests).
    ///
    /// Prefers the bundled [`Environment`] global; falls back to the legacy
    /// stand-alone `ScaleFactor` global, then to the default.
    #[must_use]
    pub fn current(cx: &App) -> Self {
        if let Some(env) = cx.try_global::<Environment>() {
            return env.scale;
        }
        cx.try_global::<ScaleFactor>().copied().unwrap_or_default()
    }
}

impl gpui::Global for ScaleFactor {}

// -----------------------------------------------------------------------------
// Environment — SwiftUI-style bundle of every value the UI tree reads at render.
// -----------------------------------------------------------------------------

/// Bundled rendering context that primitives and composites consult.
///
/// Modeled after `SwiftUI`'s `Environment`: a single typed value carrying the
/// appearance scheme and Dynamic-Type-style scale that every component
/// observes. Today it lives as a `gpui::Global` (app-scoped); per-subtree
/// override would be a wrapper element that swaps the global for the duration
/// of its render.
///
/// # Examples
///
/// ```ignore
/// use crate::ui::tokens::Environment;
/// let env = Environment::current(cx);
/// let bg = crate::ui::theme_profiles::resolve_profile_color(
///     env.profile,
///     SemanticColor::SystemBackground,
/// );
/// let pad = Spacing::Md.scaled(cx);   // reads env.scale internally
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Environment {
    pub profile: ThemeProfile,
    pub appearance: Appearance,
    pub scale: ScaleFactor,
}

impl Environment {
    /// Returns the environment installed on `cx`, or the default
    /// (Dark / Medium) if none has been installed yet (early startup, tests).
    #[must_use]
    pub fn current(cx: &App) -> Self {
        cx.try_global::<Environment>().copied().unwrap_or_default()
    }

    /// Installs the environment as a `gpui::Global` and mirrors the scale
    /// into the legacy `ScaleFactor` global so any code still reading that
    /// directly keeps working during the migration.
    pub fn install(self, cx: &mut App) {
        cx.set_global(self);
        cx.set_global(self.scale);
    }
}

impl gpui::Global for Environment {}

// -----------------------------------------------------------------------------
// Helpers.
// -----------------------------------------------------------------------------

/// Multiply a base point value by a scale and return `Pixels`.
#[inline]
fn scale_px(base: f32, scale: ScaleFactor) -> Pixels {
    px(base * scale.multiplier())
}

#[inline]
const fn hex(rgb: u32) -> Rgba {
    // Decode 0xRRGGBB into a fully opaque sRGB color. The integer-to-float
    // casts are bounded to [0, 255] so precision loss is irrelevant.
    #[expect(
        clippy::cast_precision_loss,
        reason = "byte values 0..=255 fit exactly in f32"
    )]
    Rgba {
        r: ((rgb >> 16) & 0xff) as f32 / 255.0,
        g: ((rgb >> 8) & 0xff) as f32 / 255.0,
        b: (rgb & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

// -----------------------------------------------------------------------------
// Tests.
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_label_contrast_is_high() {
        // Sanity check: label-on-systemBackground in dark must be nearly white
        // on near-black so contrast is well above WCAG AA.
        let label = SemanticColor::Label.resolve(Appearance::Dark);
        let bg = SemanticColor::SystemBackground.resolve(Appearance::Dark);
        assert!(label.r > 0.85 && bg.r < 0.15);
    }

    #[test]
    fn light_label_contrast_is_high() {
        let label = SemanticColor::Label.resolve(Appearance::Light);
        let bg = SemanticColor::SystemBackground.resolve(Appearance::Light);
        assert!(label.r < 0.05 && bg.r > 0.95);
    }

    #[test]
    fn spacing_is_monotonic() {
        let sizes = [
            Spacing::XXS,
            Spacing::XS,
            Spacing::SM,
            Spacing::MD,
            Spacing::LG,
            Spacing::XL,
            Spacing::XXL,
        ];
        for w in sizes.windows(2) {
            assert!(w[0].px() < w[1].px());
        }
    }

    #[test]
    fn radius_is_monotonic_until_full() {
        assert!(Radius::SM.px() < Radius::MD.px());
        assert!(Radius::MD.px() < Radius::LG.px());
        assert!(Radius::LG.px() < Radius::XL.px());
        assert!(Radius::XL.px() < Radius::Full.px());
    }

    #[test]
    fn hex_decodes_correctly() {
        let c = hex(0xff_8040);
        assert!((c.r - 1.0).abs() < f32::EPSILON);
        assert!((c.g - 0.501_960_8).abs() < 1e-4);
        assert!((c.b - 0.250_980_4).abs() < 1e-4);
        assert!((c.a - 1.0).abs() < f32::EPSILON);
    }
}
