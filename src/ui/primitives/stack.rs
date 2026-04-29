//! SwiftUI-style stack primitives: [`VStack`], [`HStack`], [`ZStack`] and
//! [`Spacer`].
//!
//! These wrap GPUI's flex layout in a small, declarative API that's
//! token-aware out of the box: `spacing`, `padding`, and arrangement all
//! resolve through [`crate::ui::tokens::Spacing`] so they re-flow with the
//! global UI scale.
//!
//! ```ignore
//! use crate::ui::primitives::{VStack, HStack, Spacer};
//! use crate::ui::tokens::Spacing;
//!
//! VStack::new()
//!     .spacing(Spacing::LG)
//!     .leading()
//!     .child(header)
//!     .child(
//!         HStack::new()
//!             .spacing(Spacing::SM)
//!             .child(left)
//!             .child(Spacer::new())
//!             .child(right),
//!     )
//! ```

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, App, Div, IntoElement, ParentElement, RenderOnce, Window};

use crate::ui::tokens::Spacing;

/// Cross-axis alignment for the children of a stack.
///
/// In a [`VStack`] this maps to horizontal alignment (`leading`,
/// `center`, `trailing`). In an [`HStack`] this maps to vertical
/// alignment (`top`, `center`, `bottom`). The shared name keeps the API
/// terse and SwiftUI-familiar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackAlignment {
    /// `leading` for `VStack`, `top` for `HStack`.
    Start,
    #[default]
    Center,
    /// `trailing` for `VStack`, `bottom` for `HStack`.
    End,
    /// Children stretch to fill the cross-axis.
    Stretch,
}

// -----------------------------------------------------------------------------
// VStack
// -----------------------------------------------------------------------------

/// Vertical stack — children are laid out top-to-bottom with token-aware
/// spacing between them.
#[derive(IntoElement)]
#[must_use]
pub struct VStack {
    spacing: Spacing,
    padding: Option<Spacing>,
    alignment: StackAlignment,
    fill: bool,
    children: Vec<AnyElement>,
}

impl VStack {
    pub fn new() -> Self {
        Self {
            spacing: Spacing::SM,
            padding: None,
            alignment: StackAlignment::default(),
            fill: false,
            children: Vec::new(),
        }
    }

    pub fn spacing(mut self, spacing: Spacing) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn padding(mut self, padding: Spacing) -> Self {
        self.padding = Some(padding);
        self
    }

    pub fn alignment(mut self, alignment: StackAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// SwiftUI-flavor shorthand: `.leading()` aligns children to the
    /// stack's leading edge.
    pub fn leading(self) -> Self {
        self.alignment(StackAlignment::Start)
    }

    pub fn center(self) -> Self {
        self.alignment(StackAlignment::Center)
    }

    pub fn trailing(self) -> Self {
        self.alignment(StackAlignment::End)
    }

    /// Stretch children across the cross-axis (`align_items: stretch`).
    pub fn stretch(self) -> Self {
        self.alignment(StackAlignment::Stretch)
    }

    /// Make the stack itself expand to fill its parent on both axes.
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }
}

impl Default for VStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for VStack {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for VStack {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut el: Div = div().flex().flex_col().gap(self.spacing.scaled(cx));
        el = match self.alignment {
            StackAlignment::Start => el.items_start(),
            StackAlignment::Center => el.items_center(),
            StackAlignment::End => el.items_end(),
            StackAlignment::Stretch => el,
        };
        if let Some(p) = self.padding {
            el = el.p(p.scaled(cx));
        }
        if self.fill {
            el = el.size_full();
        }
        el.extend(self.children);
        el
    }
}

// -----------------------------------------------------------------------------
// HStack
// -----------------------------------------------------------------------------

/// Horizontal stack — children laid out leading-to-trailing.
#[derive(IntoElement)]
#[must_use]
pub struct HStack {
    spacing: Spacing,
    padding: Option<Spacing>,
    alignment: StackAlignment,
    fill: bool,
    wrap: bool,
    children: Vec<AnyElement>,
}

impl HStack {
    pub fn new() -> Self {
        Self {
            spacing: Spacing::SM,
            padding: None,
            alignment: StackAlignment::Center,
            fill: false,
            wrap: false,
            children: Vec::new(),
        }
    }

    pub fn spacing(mut self, spacing: Spacing) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn padding(mut self, padding: Spacing) -> Self {
        self.padding = Some(padding);
        self
    }

    pub fn alignment(mut self, alignment: StackAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// `.top()` — align children to the top of the row.
    pub fn top(self) -> Self {
        self.alignment(StackAlignment::Start)
    }

    pub fn center(self) -> Self {
        self.alignment(StackAlignment::Center)
    }

    pub fn bottom(self) -> Self {
        self.alignment(StackAlignment::End)
    }

    pub fn stretch(self) -> Self {
        self.alignment(StackAlignment::Stretch)
    }

    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    /// Wrap children onto multiple lines when overflow occurs.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }
}

impl Default for HStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for HStack {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for HStack {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut el: Div = div().flex().flex_row().gap(self.spacing.scaled(cx));
        el = match self.alignment {
            StackAlignment::Start => el.items_start(),
            StackAlignment::Center => el.items_center(),
            StackAlignment::End => el.items_end(),
            StackAlignment::Stretch => el,
        };
        if self.wrap {
            el = el.flex_wrap();
        }
        if let Some(p) = self.padding {
            el = el.p(p.scaled(cx));
        }
        if self.fill {
            el = el.size_full();
        }
        el.extend(self.children);
        el
    }
}

// -----------------------------------------------------------------------------
// ZStack
// -----------------------------------------------------------------------------

/// Z-axis stack — children share the same rect and are painted in declaration
/// order. The first child establishes the size; subsequent children are
/// positioned absolutely on top.
#[derive(IntoElement)]
#[must_use]
pub struct ZStack {
    alignment: StackAlignment,
    fill: bool,
    children: Vec<AnyElement>,
}

impl ZStack {
    pub fn new() -> Self {
        Self {
            alignment: StackAlignment::Center,
            fill: false,
            children: Vec::new(),
        }
    }

    pub fn alignment(mut self, alignment: StackAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }
}

impl Default for ZStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for ZStack {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ZStack {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut el: Div = div().relative().flex();
        el = match self.alignment {
            StackAlignment::Start => el.items_start().justify_start(),
            StackAlignment::Center => el.items_center().justify_center(),
            StackAlignment::End => el.items_end().justify_end(),
            StackAlignment::Stretch => el,
        };
        if self.fill {
            el = el.size_full();
        }
        el.extend(self.children);
        el
    }
}

// -----------------------------------------------------------------------------
// Spacer
// -----------------------------------------------------------------------------

/// Flexible filler — expands along the parent stack's main axis to push
/// surrounding children apart. Equivalent to `SwiftUI`'s `Spacer`.
#[derive(IntoElement, Default)]
#[must_use]
pub struct Spacer;

impl Spacer {
    pub fn new() -> Self {
        Self
    }
}

impl RenderOnce for Spacer {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().flex_1()
    }
}
