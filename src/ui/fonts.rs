//! Figtree font embedding — ADR 0083 Decision 3.
//!
//! This module holds the four static Figtree weights as bytes. It loads
//! them into the GPUI text system at startup. This happens before the
//! first window opens, in `crate::app::bootstrap::run_app`.
//!
//! `crate::ui::theme_bridge` sets [`FIGTREE_FAMILY`] as the interface font
//! family. The monospace font stays the `gpui-component` default. See
//! [`crate::ui::tokens::log_font_family`].
//!
//! Source: `erikdkennedy/figtree`, tag `v2.0.3`.
//! Commit: `be6cb018f2f93a9b1195f3dfd077123f718c65f8`.
//! Files: `fonts/ttf/Figtree-Regular.ttf`, `Figtree-Medium.ttf`,
//! `Figtree-SemiBold.ttf`, `Figtree-Bold.ttf`.
//!
//! The OFL license text sits with the font files, named `Figtree-OFL.txt`.
//! It comes from the same release.

#![warn(clippy::pedantic)]

use std::borrow::Cow;

use anyhow::Context as _;
use gpui::App;

/// The interface font family name.
///
/// Each embedded Figtree file stores this name in its name table. Name ID
/// 16 holds it directly. Name ID 1 holds it for a reader that falls back
/// to that ID, such as `fontdb`.
pub const FIGTREE_FAMILY: &str = "Figtree";

const REGULAR: &[u8] = include_bytes!("../assets/fonts/figtree/Figtree-Regular.ttf");
const MEDIUM: &[u8] = include_bytes!("../assets/fonts/figtree/Figtree-Medium.ttf");
const SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/figtree/Figtree-SemiBold.ttf");
const BOLD: &[u8] = include_bytes!("../assets/fonts/figtree/Figtree-Bold.ttf");

/// Loads the four Figtree weights into the GPUI text system.
///
/// Use this one time, at startup, before the first window opens. An error
/// here does not stop the app. GPUI uses its own default font for a text
/// style that names a font family it could not find.
///
/// # Errors
///
/// Returns an error when the platform text system rejects the font bytes.
pub fn install(cx: &App) -> anyhow::Result<()> {
    let fonts: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(REGULAR),
        Cow::Borrowed(MEDIUM),
        Cow::Borrowed(SEMIBOLD),
        Cow::Borrowed(BOLD),
    ];
    cx.text_system()
        .add_fonts(fonts)
        .context("Figtree fonts did not load into the text system")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT_FILES: [&[u8]; 4] = [REGULAR, MEDIUM, SEMIBOLD, BOLD];

    /// R83-11, first part: the four Figtree files load into a GPUI test
    /// text system with no error.
    ///
    /// `#[gpui::test]` builds its `TestAppContext` on GPUI's
    /// `NoopTextSystem` (`gpui-pre-0.3.1/src/platform/test/platform.rs`).
    /// This test platform accepts each byte sequence. It does not read a
    /// font name table. This test proves the call raises no error. It does
    /// not prove the bytes are a font.
    ///
    /// The test `adr_0083_figtree_file_family_names_resolve` proves that
    /// part. It reads the embedded bytes directly.
    #[gpui::test]
    fn adr_0083_figtree_fonts_load_into_the_test_text_system(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            install(cx).expect("the four Figtree files must load into the text system");
        });
    }

    /// R83-11, second part: each embedded file's family name matches
    /// [`FIGTREE_FAMILY`].
    ///
    /// `Medium` and `SemiBold` carry their own family name at ID 1, for
    /// example "Figtree Medium". `Regular` and `Bold` do not. The
    /// typographic family at ID 16 carries the shared name "Figtree" for
    /// each weight.
    ///
    /// `fontdb` reads ID 16 first, then ID 1, for a loaded font. See
    /// `fontdb-0.23.0/src/lib.rs`, function `parse_face_info`. This test
    /// proves the name the deployed text system would read.
    #[test]
    fn adr_0083_figtree_file_family_names_resolve() {
        for bytes in FONT_FILES {
            let face = ttf_parser::Face::parse(bytes, 0)
                .expect("embedded Figtree file must parse as a TrueType font");
            let family = face
                .names()
                .into_iter()
                .find(|name| {
                    name.name_id == ttf_parser::name_id::TYPOGRAPHIC_FAMILY && name.is_unicode()
                })
                .and_then(|name| name.to_string())
                .or_else(|| {
                    face.names()
                        .into_iter()
                        .find(|name| {
                            name.name_id == ttf_parser::name_id::FAMILY && name.is_unicode()
                        })
                        .and_then(|name| name.to_string())
                })
                .expect("Figtree file must carry a family or typographic family name");
            assert_eq!(
                family, FIGTREE_FAMILY,
                "the resolved family name must be Figtree"
            );
        }
    }
}
