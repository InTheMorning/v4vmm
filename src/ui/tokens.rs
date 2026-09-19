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
    ///
    /// ADR 0039: `Spacing` is a CHROME token — this resolves through
    /// [`scale_chrome_px`], never the TYPE domain.
    #[must_use]
    pub fn scaled(self, cx: &App) -> Pixels {
        scale_chrome_px(f32::from(self.px()), ScaleFactor::current(cx))
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

    /// ADR 0039: `Radius` is a CHROME token — this resolves through
    /// [`scale_chrome_px`], never the TYPE domain.
    #[must_use]
    pub fn scaled(self, cx: &App) -> Pixels {
        // The pill radius is intentionally capped — scaling 999px makes no
        // visual difference and risks integer overflow on extreme factors.
        if matches!(self, Self::Full) {
            return self.px();
        }
        scale_chrome_px(f32::from(self.px()), ScaleFactor::current(cx))
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

    /// Returns the type coefficient for this role at the specified scale
    ///
    /// ADR 0039 task 003 ratified the endpoints on 2026-09-18.
    /// Each role uses [`Self::type_endpoints`] and [`Self::interpolate`].
    /// Medium returns exactly `1.0`.
    /// This replaces task 001's uniform coefficients.
    /// Keep the endpoints separate for each role.
    /// Record new operator ratification in the ADR before changing an endpoint.
    #[must_use]
    pub const fn type_multiplier(self, scale: ScaleFactor) -> f32 {
        let (x_small, x_large) = self.type_endpoints();
        Self::interpolate(scale, x_small, x_large)
    }

    /// Returns the ratified `(x-small factor, x-large factor)` for this role
    ///
    /// ADR 0039 task 003 records the operator's decision on 2026-09-18.
    /// `Title` retains the former uniform x-small factor of 0.85.
    /// The x-small factor increases by 0.01 per role from `Title` toward `Micro`.
    /// Ratification did not change the proposed x-large factors.
    const fn type_endpoints(self) -> (f32, f32) {
        match self {
            Self::Micro => (0.91, 1.36),
            Self::Caption => (0.90, 1.32),
            Self::Body => (0.89, 1.28),
            Self::Headline => (0.88, 1.24),
            Self::Title3 => (0.87, 1.20),
            Self::Title2 => (0.86, 1.16),
            Self::Title => (0.85, 1.12),
        }
    }

    /// Interpolates the type factor between the ratified endpoints
    ///
    /// ADR 0039 task 003 uses [`Self::step_coordinate`] for the position `c`.
    /// The type resolver does not call [`ScaleFactor::chrome_multiplier`].
    /// Below medium, the factor is `1 - ((1 - c) / 0.15) * (1 - x_small)`.
    /// Medium returns exactly `1.0`.
    /// Above medium, the factor is `1 + ((c - 1) / 0.25) * (x_large - 1)`.
    /// X-small and x-large resolve to their respective endpoints.
    /// Small uses 8/15 of the downward change.
    /// Large uses 12/25 of the upward change.
    const fn interpolate(scale: ScaleFactor, x_small: f32, x_large: f32) -> f32 {
        match scale {
            ScaleFactor::Medium => 1.0,
            ScaleFactor::XSmall | ScaleFactor::Small => {
                let c = Self::step_coordinate(scale);
                1.0 - ((1.0 - c) / 0.15) * (1.0 - x_small)
            }
            ScaleFactor::Large | ScaleFactor::XLarge => {
                let c = Self::step_coordinate(scale);
                1.0 + ((c - 1.0) / 0.25) * (x_large - 1.0)
            }
        }
    }

    /// Returns the step coordinate for type interpolation
    ///
    /// ADR 0039 keeps these literals separate from [`ScaleFactor::chrome_multiplier`].
    /// Task 003 changed the type endpoints without changing the chrome resolver.
    const fn step_coordinate(scale: ScaleFactor) -> f32 {
        match scale {
            ScaleFactor::XSmall => 0.85,
            ScaleFactor::Small => 0.92,
            ScaleFactor::Medium => 1.0,
            ScaleFactor::Large => 1.12,
            ScaleFactor::XLarge => 1.25,
        }
    }

    /// Pure ADR 0039 TYPE-domain resolver — usable in tests without an
    /// `App`. [`Self::scaled`] is a thin environment-backed wrapper around
    /// this same resolver.
    #[must_use]
    pub fn scaled_px(self, scale: ScaleFactor) -> Pixels {
        px(f32::from(self.px()) * self.type_multiplier(scale))
    }

    /// ADR 0039: `FontSize` is the TYPE domain — this resolves through
    /// [`Self::scaled_px`]/[`Self::type_multiplier`], never CHROME.
    #[must_use]
    pub fn scaled(self, cx: &App) -> Pixels {
        self.scaled_px(ScaleFactor::current(cx))
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
    /// 44 px — minimum interactive hit region.
    MinHitTarget,
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
    /// 176 px — Music content tile outer width.
    ContentTileWidth,
    /// 152 px — Music content tile artwork edge.
    ContentTileArtwork,
    /// 600 px — readable measure for empty-state explanations and reports.
    NoticeWidth,
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
            Self::MinHitTarget | Self::RowLg => px(44.0),
            Self::ContentTileWidth => px(176.0),
            Self::ContentTileArtwork => px(152.0),
            Self::NoticeWidth => px(600.0),
        }
    }

    /// ADR 0039: `Size` is a CHROME token — this resolves through
    /// [`scale_chrome_px`], never the TYPE domain.
    #[must_use]
    pub fn scaled(self, cx: &App) -> Pixels {
        scale_chrome_px(f32::from(self.px()), ScaleFactor::current(cx))
    }
}

// -----------------------------------------------------------------------------
// SkeletonBlock — redacted placeholder dimensions.
// -----------------------------------------------------------------------------

/// Semantic dimensions for skeleton placeholder blocks.
///
/// These values mirror the populated controls they stand in for, keeping
/// loading surfaces stable without scattering raw block sizes through
/// composites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkeletonBlock {
    /// 220 x 22 px — inspector title line.
    InspectorTitle,
    /// 160 x 14 px — inspector subtitle line.
    InspectorSubtitle,
    /// 120 x 12 px — inspector caption line.
    InspectorCaption,
    /// 96 x 12 px — recent-feed tile subtitle line.
    FeedTileSubtitle,
    /// 12 x 16 px — compact track-number placeholder.
    TrackNumber,
    /// 32 x 16 px — compact track-duration placeholder.
    TrackDuration,
}

impl SkeletonBlock {
    #[must_use]
    pub const fn px(self) -> (Pixels, Pixels) {
        match self {
            Self::InspectorTitle => (px(220.0), px(22.0)),
            Self::InspectorSubtitle => (px(160.0), px(14.0)),
            Self::InspectorCaption => (px(120.0), px(12.0)),
            Self::FeedTileSubtitle => (px(96.0), px(12.0)),
            Self::TrackNumber => (px(12.0), px(16.0)),
            Self::TrackDuration => (px(32.0), px(16.0)),
        }
    }

    /// ADR 0039: `SkeletonBlock` is a CHROME token — this resolves through
    /// [`scale_chrome_px`], never the TYPE domain.
    #[must_use]
    pub fn scaled(self, cx: &App) -> (Pixels, Pixels) {
        let (width, height) = self.px();
        let scale = ScaleFactor::current(cx);
        (
            scale_chrome_px(f32::from(width), scale),
            scale_chrome_px(f32::from(height), scale),
        )
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
    /// ADR 0039 CHROME domain: the five geometry coefficients.
    ///
    /// Bit-identical to the pre-ADR-0039 uniform `ScaleFactor::multiplier()`
    /// values at every step. `Spacing`, `Radius`, `Size`, `SkeletonBlock`,
    /// icons, images, thumbnails and the geometry bridges listed in ADR 0039
    /// task 001 all route through this resolver; `FontSize` never does — see
    /// [`FontSize::type_multiplier`] for the TYPE domain.
    #[must_use]
    pub const fn chrome_multiplier(self) -> f32 {
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
/// appearance scheme, Dynamic-Type-style scale, and motion preference that
/// every component observes. Today it lives as a `gpui::Global` (app-scoped); per-subtree
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
    /// Mirrors the platform Reduce Motion preference when the runtime exposes
    /// it. Keep all animation decisions routed through [`Self::allows_motion`].
    pub reduce_motion: bool,
}

impl Environment {
    /// Returns the environment installed on `cx`, or the default
    /// (Dark / Medium) if none has been installed yet (early startup, tests).
    #[must_use]
    pub fn current(cx: &App) -> Self {
        cx.try_global::<Environment>().copied().unwrap_or_default()
    }

    /// Returns whether user-visible animation may run.
    #[must_use]
    pub const fn allows_motion(self) -> bool {
        !self.reduce_motion
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

/// ADR 0039 CHROME domain: multiply a base point value by the chrome
/// coefficient and return `Pixels`.
///
/// Used by `Spacing`, `Radius`, `Size` and `SkeletonBlock`. `FontSize` never
/// calls this — see [`FontSize::scaled_px`] for the TYPE domain's own pure
/// resolver.
#[inline]
fn scale_chrome_px(base: f32, scale: ScaleFactor) -> Pixels {
    px(base * scale.chrome_multiplier())
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
    fn skeleton_block_tokens_match_placeholder_footprints() {
        assert_eq!(SkeletonBlock::InspectorTitle.px(), (px(220.0), px(22.0)));
        assert_eq!(SkeletonBlock::InspectorSubtitle.px(), (px(160.0), px(14.0)));
        assert_eq!(SkeletonBlock::InspectorCaption.px(), (px(120.0), px(12.0)));
        assert_eq!(SkeletonBlock::FeedTileSubtitle.px(), (px(96.0), px(12.0)));
        assert_eq!(SkeletonBlock::TrackNumber.px(), (px(12.0), px(16.0)));
        assert_eq!(SkeletonBlock::TrackDuration.px(), (px(32.0), px(16.0)));
    }

    #[test]
    fn reduce_motion_disables_environment_motion() {
        let env = Environment {
            reduce_motion: true,
            ..Environment::default()
        };

        assert!(!env.allows_motion());
    }

    #[test]
    fn hex_decodes_correctly() {
        let c = hex(0xff_8040);
        assert!((c.r - 1.0).abs() < f32::EPSILON);
        assert!((c.g - 0.501_960_8).abs() < 1e-4);
        assert!((c.b - 0.250_980_4).abs() < 1e-4);
        assert!((c.a - 1.0).abs() < f32::EPSILON);
    }

    // -------------------------------------------------------------------
    // ADR 0039 task 001 (amended scope): CHROME/TYPE domain split.
    // -------------------------------------------------------------------

    const ADR_0039_STEPS: [(ScaleFactor, f32); 5] = [
        (ScaleFactor::XSmall, 0.85),
        (ScaleFactor::Small, 0.92),
        (ScaleFactor::Medium, 1.0),
        (ScaleFactor::Large, 1.12),
        (ScaleFactor::XLarge, 1.25),
    ];

    /// M1: the CHROME coefficients are bit-identical to the five former
    /// uniform `ScaleFactor` values — not merely close by an epsilon.
    #[test]
    fn adr_0039_chrome_multiplier_matches_former_uniform_values_bit_exact() {
        for (scale, former_value) in ADR_0039_STEPS {
            assert_eq!(
                scale.chrome_multiplier().to_bits(),
                former_value.to_bits(),
                "{scale:?} CHROME coefficient drifted from the former uniform value"
            );
        }
    }

    /// M1: every resolved `Spacing`/`Radius`/`Size`/`SkeletonBlock` value at
    /// every step matches the old uniform arithmetic bit-for-bit, including
    /// the `Radius::Full` pill exception. Exercises the environment-backed
    /// `.scaled(cx)` path, not only the pure coefficient table.
    #[gpui::test]
    fn adr_0039_chrome_resolved_tokens_match_former_arithmetic_at_every_step(
        cx: &mut gpui::TestAppContext,
    ) {
        let spacing = [
            Spacing::XXS,
            Spacing::XS,
            Spacing::SM,
            Spacing::MD,
            Spacing::LG,
            Spacing::XL,
            Spacing::XXL,
        ];
        let radius = [Radius::SM, Radius::MD, Radius::LG, Radius::XL, Radius::Full];
        let size = [
            Size::MinHitTarget,
            Size::ButtonSm,
            Size::ButtonMd,
            Size::ButtonLg,
            Size::MenuCompact,
            Size::MenuRegular,
            Size::MenuWide,
            Size::ColumnShort,
            Size::ColumnRegular,
            Size::ColumnTall,
            Size::RowMd,
            Size::RowLg,
            Size::ContentTileWidth,
            Size::ContentTileArtwork,
            Size::NoticeWidth,
        ];
        let skeleton = [
            SkeletonBlock::InspectorTitle,
            SkeletonBlock::InspectorSubtitle,
            SkeletonBlock::InspectorCaption,
            SkeletonBlock::FeedTileSubtitle,
            SkeletonBlock::TrackNumber,
            SkeletonBlock::TrackDuration,
        ];

        cx.update(|cx| {
            for (scale, former_multiplier) in ADR_0039_STEPS {
                cx.set_global(scale);

                for token in spacing {
                    let expected = f32::from(token.px()) * former_multiplier;
                    assert_eq!(
                        f32::from(token.scaled(cx)).to_bits(),
                        expected.to_bits(),
                        "Spacing::{token:?} at {scale:?} drifted from the former arithmetic"
                    );
                }

                for token in radius {
                    let expected = if matches!(token, Radius::Full) {
                        f32::from(token.px())
                    } else {
                        f32::from(token.px()) * former_multiplier
                    };
                    assert_eq!(
                        f32::from(token.scaled(cx)).to_bits(),
                        expected.to_bits(),
                        "Radius::{token:?} at {scale:?} drifted from the former arithmetic"
                    );
                }

                for token in size {
                    let expected = f32::from(token.px()) * former_multiplier;
                    assert_eq!(
                        f32::from(token.scaled(cx)).to_bits(),
                        expected.to_bits(),
                        "Size::{token:?} at {scale:?} drifted from the former arithmetic"
                    );
                }

                for token in skeleton {
                    let (base_w, base_h) = token.px();
                    let expected = (
                        f32::from(base_w) * former_multiplier,
                        f32::from(base_h) * former_multiplier,
                    );
                    let (resolved_w, resolved_h) = token.scaled(cx);
                    assert_eq!(
                        f32::from(resolved_w).to_bits(),
                        expected.0.to_bits(),
                        "SkeletonBlock::{token:?} width at {scale:?} drifted from the former arithmetic"
                    );
                    assert_eq!(
                        f32::from(resolved_h).to_bits(),
                        expected.1.to_bits(),
                        "SkeletonBlock::{token:?} height at {scale:?} drifted from the former arithmetic"
                    );
                }
            }
        });
    }

    /// ADR 0039 task 003 compares all 35 type outcomes with the ratified table.
    /// The comparison tolerance is 0.001 px.
    /// Expected values come from the table, independently of the resolver's formula.
    /// This test replaces task 001's assertion that every outcome matches uniform scaling.
    /// `adr_0039_non_medium_type_outcomes_differ_from_uniform_except_title_downward`
    /// retains the comparison with uniform scaling.
    #[test]
    fn adr_0039_type_outcomes_match_ratified_values() {
        // (role, [XSmall, Small, Medium, Large, XLarge] expected px).
        let cases: [(FontSize, [f32; 5]); 7] = [
            (FontSize::Micro, [10.01, 10.472, 11.00, 12.9008, 14.96]),
            (FontSize::Caption, [10.80, 11.36, 12.00, 13.8432, 15.84]),
            (FontSize::Body, [11.57, 12.237_333, 13.00, 14.7472, 16.64]),
            (FontSize::Headline, [13.20, 14.04, 15.00, 16.728, 18.60]),
            (FontSize::Title3, [14.79, 15.821_333, 17.00, 18.632, 20.40]),
            (FontSize::Title2, [17.20, 18.506_667, 20.00, 21.536, 23.20]),
            (FontSize::Title, [20.40, 22.08, 24.00, 25.3824, 26.88]),
        ];

        let mut checked = 0;
        for (role, expected_by_step) in cases {
            for (i, (scale, _)) in ADR_0039_STEPS.iter().enumerate() {
                let resolved = f32::from(role.scaled_px(*scale));
                let expected = expected_by_step[i];
                assert!(
                    (resolved - expected).abs() < 0.001,
                    "{role:?} at {scale:?}: resolved {resolved} != ratified {expected}"
                );
                checked += 1;
            }
        }
        assert_eq!(
            checked, 35,
            "expected all 35 ratified role/step outcomes to be checked"
        );
    }

    /// ADR 0039 task 003 M1 compares the 28 non-medium outcomes with uniform scaling.
    /// Exactly 26 outcomes differ.
    /// `Title` at `XSmall` and `Small` retains bit-identical output.
    ///
    /// The operator retained `Title`'s x-small factor of 0.85.
    /// Interpolation therefore also retains its small factor of 0.92.
    /// Do not change these endpoints to remove the two accepted exceptions.
    ///
    /// This test excludes medium.
    /// `adr_0039_medium_type_returns_each_role_base_exactly` checks its seven unchanged outcomes.
    #[test]
    fn adr_0039_non_medium_type_outcomes_differ_from_uniform_except_title_downward() {
        let roles = [
            FontSize::Micro,
            FontSize::Caption,
            FontSize::Body,
            FontSize::Headline,
            FontSize::Title3,
            FontSize::Title2,
            FontSize::Title,
        ];
        let non_medium_steps: Vec<(ScaleFactor, f32)> = ADR_0039_STEPS
            .into_iter()
            .filter(|(scale, _)| !matches!(scale, ScaleFactor::Medium))
            .collect();
        assert_eq!(non_medium_steps.len(), 4, "expected 4 non-medium steps");

        let mut differing = 0;
        let mut equal = 0;
        for role in roles {
            for (scale, former_multiplier) in non_medium_steps.iter().copied() {
                let former = f32::from(role.px()) * former_multiplier;
                let resolved = f32::from(role.scaled_px(scale));
                let is_title_downward = matches!(role, FontSize::Title)
                    && matches!(scale, ScaleFactor::XSmall | ScaleFactor::Small);

                if is_title_downward {
                    assert_eq!(
                        resolved.to_bits(),
                        former.to_bits(),
                        "documented exception: {role:?} at {scale:?} must stay bit-identical \
                         to the former uniform result"
                    );
                    equal += 1;
                } else {
                    assert_ne!(
                        resolved.to_bits(),
                        former.to_bits(),
                        "{role:?} at {scale:?} unexpectedly matches the former uniform result"
                    );
                    differing += 1;
                }
            }
        }

        assert_eq!(
            equal, 2,
            "expected exactly 2 documented Title-downward exceptions"
        );
        assert_eq!(
            differing, 26,
            "expected the other 26 non-medium cells to differ from uniform"
        );
    }

    /// ADR 0039 task 003 M1: above medium, smaller roles grow more
    /// proportionally than larger roles, at both `Large` and `XLarge`.
    #[test]
    fn adr_0039_smaller_roles_grow_more_above_medium() {
        let roles_smallest_to_largest = [
            FontSize::Micro,
            FontSize::Caption,
            FontSize::Body,
            FontSize::Headline,
            FontSize::Title3,
            FontSize::Title2,
            FontSize::Title,
        ];
        for scale in [ScaleFactor::Large, ScaleFactor::XLarge] {
            for pair in roles_smallest_to_largest.windows(2) {
                let smaller_growth = pair[0].type_multiplier(scale) - 1.0;
                let larger_growth = pair[1].type_multiplier(scale) - 1.0;
                assert!(
                    smaller_growth > larger_growth,
                    "{:?} should grow more than {:?} at {scale:?} \
                     ({smaller_growth} <= {larger_growth})",
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    /// ADR 0039 task 003 M1: below medium, smaller roles shrink less
    /// proportionally than larger roles, at both `XSmall` and `Small`.
    #[test]
    fn adr_0039_smaller_roles_shrink_less_below_medium() {
        let roles_smallest_to_largest = [
            FontSize::Micro,
            FontSize::Caption,
            FontSize::Body,
            FontSize::Headline,
            FontSize::Title3,
            FontSize::Title2,
            FontSize::Title,
        ];
        for scale in [ScaleFactor::XSmall, ScaleFactor::Small] {
            for pair in roles_smallest_to_largest.windows(2) {
                let smaller_loss = 1.0 - pair[0].type_multiplier(scale);
                let larger_loss = 1.0 - pair[1].type_multiplier(scale);
                assert!(
                    smaller_loss < larger_loss,
                    "{:?} should shrink less than {:?} at {scale:?} \
                     ({smaller_loss} >= {larger_loss})",
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    /// M2: medium returns each role's exact base size (11, 12, 13, 15, 17,
    /// 20, 24), unchanged by the domain split.
    #[test]
    fn adr_0039_medium_type_returns_each_role_base_exactly() {
        let expected = [
            (FontSize::Micro, 11.0),
            (FontSize::Caption, 12.0),
            (FontSize::Body, 13.0),
            (FontSize::Headline, 15.0),
            (FontSize::Title3, 17.0),
            (FontSize::Title2, 20.0),
            (FontSize::Title, 24.0),
        ];
        for (role, base) in expected {
            assert_eq!(role.scaled_px(ScaleFactor::Medium), px(base));
        }
    }

    /// M2: role ordering `Micro < Caption < Body < Headline < Title3 <
    /// Title2 < Title` holds at every step.
    #[test]
    fn adr_0039_type_role_ordering_holds_at_every_step() {
        let roles = [
            FontSize::Micro,
            FontSize::Caption,
            FontSize::Body,
            FontSize::Headline,
            FontSize::Title3,
            FontSize::Title2,
            FontSize::Title,
        ];
        for (scale, _) in ADR_0039_STEPS {
            for pair in roles.windows(2) {
                assert!(
                    pair[0].scaled_px(scale) < pair[1].scaled_px(scale),
                    "{:?} < {:?} failed at {scale:?}",
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    /// M2: each role grows monotonically across the five steps.
    #[test]
    fn adr_0039_type_grows_monotonically_per_role_across_steps() {
        let roles = [
            FontSize::Micro,
            FontSize::Caption,
            FontSize::Body,
            FontSize::Headline,
            FontSize::Title3,
            FontSize::Title2,
            FontSize::Title,
        ];
        for role in roles {
            let resolved: Vec<Pixels> = ADR_0039_STEPS
                .iter()
                .map(|&(scale, _)| role.scaled_px(scale))
                .collect();
            for pair in resolved.windows(2) {
                assert!(
                    pair[0] < pair[1],
                    "{role:?} did not grow monotonically across steps"
                );
            }
        }
    }
}

/// ADR 0063: all log bodies use the same compact, scaled monospace role.
pub const LOG_TEXT_SIZE: FontSize = FontSize::Caption;
pub const LOG_LINE_HEIGHT: f32 = 1.5;

#[must_use]
pub fn log_font_family(cx: &App) -> gpui::SharedString {
    cx.theme().mono_font_family.clone()
}
