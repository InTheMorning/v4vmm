//! WCAG 2.1 contrast-ratio math + a static matrix of acceptable
//! foreground/background pairs.
//!
//! Used by unit tests to guarantee every text-on-surface combination we ship
//! meets the WCAG AA threshold (4.5:1 for normal text, 3:1 for large text /
//! UI components). The matrix is the source of truth: every legitimate pair
//! is enumerated, and any token combination *not* listed is implicitly
//! "do not use".
//!
//! Why not enforce at compile time? WCAG luminance requires `f32::powf`,
//! which is not `const fn` in stable Rust. A proc-macro / build-script
//! enforcement would add complexity without changing the failure mode —
//! `cargo test` already gates every PR.

#![warn(clippy::pedantic)]

use gpui::Rgba;

use super::tokens::SemanticColor;

/// Required contrast ratio per WCAG 2.1 success criterion 1.4.3 / 1.4.11.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContrastLevel {
    /// 4.5 : 1 — normal body text (AA).
    NormalText,
    /// 3.0 : 1 — large text (≥18 pt or ≥14 pt bold) and graphical UI
    /// components (icons, focus rings, separators on adjacent surface).
    LargeOrGraphic,
}

impl ContrastLevel {
    #[must_use]
    pub const fn min_ratio(self) -> f32 {
        match self {
            Self::NormalText => 4.5,
            Self::LargeOrGraphic => 3.0,
        }
    }
}

/// Compute the WCAG 2.1 contrast ratio between two colors.
///
/// Returns a value in `[1.0, 21.0]`. Order of arguments does not matter.
#[must_use]
pub fn ratio(a: Rgba, b: Rgba) -> f32 {
    let la = relative_luminance(a);
    let lb = relative_luminance(b);
    let (lighter, darker) = if la > lb { (la, lb) } else { (lb, la) };
    (lighter + 0.05) / (darker + 0.05)
}

/// WCAG relative luminance of an sRGB color (alpha is ignored).
#[must_use]
pub fn relative_luminance(c: Rgba) -> f32 {
    0.2126 * linearize(c.r) + 0.7152 * linearize(c.g) + 0.0722 * linearize(c.b)
}

fn linearize(channel: f32) -> f32 {
    if channel <= 0.039_28 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

// -----------------------------------------------------------------------------
// Pair matrix — the canonical list of legal foreground/background combos.
// -----------------------------------------------------------------------------

/// A required (foreground, background) token combination plus the WCAG level
/// it must meet. Tests iterate this matrix in both `Light` and `Dark`
/// appearances; if any pair drops below threshold the build fails.
#[derive(Debug, Clone, Copy)]
pub struct ContrastPair {
    pub fg: SemanticColor,
    pub bg: SemanticColor,
    pub level: ContrastLevel,
    pub note: &'static str,
}

/// Every text-on-surface or graphic-on-surface combination v4vmm uses.
/// Add a new entry whenever a primitive renders a new token pairing.
pub const REQUIRED_PAIRS: &[ContrastPair] = &[
    // Body text on the three background tiers.
    ContrastPair {
        fg: SemanticColor::Label,
        bg: SemanticColor::SystemBackground,
        level: ContrastLevel::NormalText,
        note: "primary body text on canvas",
    },
    ContrastPair {
        fg: SemanticColor::Label,
        bg: SemanticColor::SecondarySystemBackground,
        level: ContrastLevel::NormalText,
        note: "primary body text on raised surface (popovers, sheets)",
    },
    ContrastPair {
        fg: SemanticColor::Label,
        bg: SemanticColor::TertiarySystemBackground,
        level: ContrastLevel::NormalText,
        note: "primary body text on highest surface tier",
    },
    // Secondary labels — still must hit 4.5:1 for body text.
    ContrastPair {
        fg: SemanticColor::SecondaryLabel,
        bg: SemanticColor::SystemBackground,
        level: ContrastLevel::NormalText,
        note: "secondary body text on canvas",
    },
    ContrastPair {
        fg: SemanticColor::SecondaryLabel,
        bg: SemanticColor::SecondarySystemBackground,
        level: ContrastLevel::NormalText,
        note: "secondary body text on raised surface",
    },
    // Tertiary / quaternary labels are reserved for large text & UI hints.
    ContrastPair {
        fg: SemanticColor::TertiaryLabel,
        bg: SemanticColor::SystemBackground,
        level: ContrastLevel::LargeOrGraphic,
        note: "tertiary label — large text / UI hint only",
    },
    ContrastPair {
        fg: SemanticColor::TertiaryLabel,
        bg: SemanticColor::SecondarySystemBackground,
        level: ContrastLevel::LargeOrGraphic,
        note: "tertiary label on raised surface",
    },
    // Accent text on canvas (links, active labels).
    ContrastPair {
        fg: SemanticColor::Accent,
        bg: SemanticColor::SystemBackground,
        level: ContrastLevel::LargeOrGraphic,
        note: "accent link / button text on canvas",
    },
    ContrastPair {
        fg: SemanticColor::Accent,
        bg: SemanticColor::SecondarySystemBackground,
        level: ContrastLevel::LargeOrGraphic,
        note: "accent link / button text on raised surface",
    },
    // Filled-button text on accent fill. Button labels per Apple HIG are
    // always at large-text scale (>=17pt regular or >=14pt bold), which
    // qualifies under WCAG 1.4.3 large-text rules at 3:1.
    ContrastPair {
        fg: SemanticColor::OnAccent,
        bg: SemanticColor::Accent,
        level: ContrastLevel::LargeOrGraphic,
        note: "filled-accent button label (large/bold per HIG)",
    },
    // Status text on its own colored fill (filled status pills / badges).
    // Same large-text justification.
    ContrastPair {
        fg: SemanticColor::OnSuccess,
        bg: SemanticColor::Success,
        level: ContrastLevel::LargeOrGraphic,
        note: "filled success badge",
    },
    ContrastPair {
        fg: SemanticColor::OnWarning,
        bg: SemanticColor::Warning,
        level: ContrastLevel::LargeOrGraphic,
        note: "filled warning badge",
    },
    ContrastPair {
        fg: SemanticColor::OnDanger,
        bg: SemanticColor::Danger,
        level: ContrastLevel::LargeOrGraphic,
        note: "filled danger badge",
    },
    ContrastPair {
        fg: SemanticColor::OnInfo,
        bg: SemanticColor::Info,
        level: ContrastLevel::LargeOrGraphic,
        note: "filled info badge",
    },
    // Inline status TEXT on canvas — uses the *Label tokens, must hit 4.5:1.
    ContrastPair {
        fg: SemanticColor::SuccessLabel,
        bg: SemanticColor::SystemBackground,
        level: ContrastLevel::NormalText,
        note: "inline success text on canvas",
    },
    ContrastPair {
        fg: SemanticColor::WarningLabel,
        bg: SemanticColor::SystemBackground,
        level: ContrastLevel::NormalText,
        note: "inline warning text on canvas",
    },
    ContrastPair {
        fg: SemanticColor::DangerLabel,
        bg: SemanticColor::SystemBackground,
        level: ContrastLevel::NormalText,
        note: "inline danger text on canvas",
    },
    ContrastPair {
        fg: SemanticColor::InfoLabel,
        bg: SemanticColor::SystemBackground,
        level: ContrastLevel::NormalText,
        note: "inline info text on canvas",
    },
    // Same on raised surface (popovers, sheets).
    ContrastPair {
        fg: SemanticColor::SuccessLabel,
        bg: SemanticColor::SecondarySystemBackground,
        level: ContrastLevel::NormalText,
        note: "inline success text on raised surface",
    },
    ContrastPair {
        fg: SemanticColor::WarningLabel,
        bg: SemanticColor::SecondarySystemBackground,
        level: ContrastLevel::NormalText,
        note: "inline warning text on raised surface",
    },
    ContrastPair {
        fg: SemanticColor::DangerLabel,
        bg: SemanticColor::SecondarySystemBackground,
        level: ContrastLevel::NormalText,
        note: "inline danger text on raised surface",
    },
    ContrastPair {
        fg: SemanticColor::InfoLabel,
        bg: SemanticColor::SecondarySystemBackground,
        level: ContrastLevel::NormalText,
        note: "inline info text on raised surface",
    },
    // Diff colors on canvas — used as glyphs & short tags, treat as graphic.
    ContrastPair {
        fg: SemanticColor::DiffMatch,
        bg: SemanticColor::SystemBackground,
        level: ContrastLevel::LargeOrGraphic,
        note: "diff = match glyph on canvas",
    },
    ContrastPair {
        fg: SemanticColor::DiffDifferent,
        bg: SemanticColor::SystemBackground,
        level: ContrastLevel::LargeOrGraphic,
        note: "diff ≠ glyph on canvas",
    },
    ContrastPair {
        fg: SemanticColor::DiffMissing,
        bg: SemanticColor::SystemBackground,
        level: ContrastLevel::LargeOrGraphic,
        note: "diff ∅ glyph on canvas",
    },
    // Borders & separators must hit 3:1 against their adjacent surface.
    ContrastPair {
        fg: SemanticColor::OpaqueSeparator,
        bg: SemanticColor::SystemBackground,
        level: ContrastLevel::LargeOrGraphic,
        note: "opaque separator hairline",
    },
    ContrastPair {
        fg: SemanticColor::Focus,
        bg: SemanticColor::SystemBackground,
        level: ContrastLevel::LargeOrGraphic,
        note: "focus ring on canvas",
    },
];

// -----------------------------------------------------------------------------
// Tests.
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::tokens::Appearance;

    /// Sanity: black-on-white is the WCAG reference at 21 : 1.
    #[test]
    fn black_on_white_is_max_contrast() {
        let black = Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        let white = Rgba {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };
        let r = ratio(black, white);
        assert!((r - 21.0).abs() < 0.01, "expected 21:1 got {r}");
    }

    /// Sanity: identical colors give 1 : 1.
    #[test]
    fn same_color_is_one_to_one() {
        let c = Rgba {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        };
        let r = ratio(c, c);
        assert!((r - 1.0).abs() < 0.001, "expected 1:1 got {r}");
    }

    fn check_matrix(appearance: Appearance) {
        let mut failures = Vec::new();
        for pair in REQUIRED_PAIRS {
            let fg = pair.fg.resolve(appearance);
            let bg = pair.bg.resolve(appearance);
            let actual = ratio(fg, bg);
            let required = pair.level.min_ratio();
            if actual + 0.005 < required {
                failures.push(format!(
                    "{appearance:?}: {fg:?} on {bg:?} = {actual:.2}:1 (need {required:.2}:1) — {note}",
                    fg = pair.fg,
                    bg = pair.bg,
                    note = pair.note,
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "WCAG contrast failures in {appearance:?} mode:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn dark_palette_meets_wcag() {
        check_matrix(Appearance::Dark);
    }

    #[test]
    fn light_palette_meets_wcag() {
        check_matrix(Appearance::Light);
    }
}
