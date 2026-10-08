//! Cover backdrop composite — a page header over a gradient in the main
//! color of its cover (ADR 0083 Decision 4, task 005).
//!
//! The gradient starts at the top of the header and fades to the page
//! background over the `CoverBackdrop` height token. With no color, the
//! composite draws no gradient. The padding stays the same, so the header
//! does not move when the color arrives.

#![warn(clippy::pedantic)]

use gpui::{div, AnyElement, App, IntoElement, ParentElement, RenderOnce, Styled, Window};

use crate::media::cover_color::CoverColor;
use crate::ui::tokens::{CoverBackdrop as CoverBackdropToken, Radius, Spacing};

#[derive(IntoElement)]
#[must_use]
pub struct CoverBackdrop {
    color: Option<CoverColor>,
    header: AnyElement,
}

impl CoverBackdrop {
    pub fn new(color: Option<CoverColor>, header: AnyElement) -> Self {
        Self { color, header }
    }
}

impl RenderOnce for CoverBackdrop {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let backdrop = self.color.map(|color| {
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .h(CoverBackdropToken::height(cx))
                .rounded(Radius::LG.scaled(cx))
                .bg(CoverBackdropToken::background(color, cx))
        });
        div()
            .relative()
            .p(Spacing::MD.scaled(cx))
            .children(backdrop)
            .child(self.header)
    }
}
