//! Button primitive — Apple HIG-aligned variants and sizes.
//!
//! Implemented natively (no `gpui_component::Button` dependency) so the
//! token system fully owns the visual contract: every color, padding, and
//! radius resolves through [`crate::ui::tokens`]. Click handlers are plain
//! callbacks — the primitive owns no state.
//!
//! Size selection follows the HIG button rule: every variant ships with
//! ≥ 14pt **semibold** label text so filled / tinted variants qualify as
//! "large text" and meet WCAG AA at the 3:1 threshold (which our
//! `OnAccent` / `OnSuccess` / etc. tokens are designed to clear).

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, App, ClickEvent, ElementId, FontWeight, IntoElement, MouseButton, Pixels,
    RenderOnce, SharedString, Window,
};

use crate::ui::tokens::{Appearance, FontSize, Radius, SemanticColor, Spacing};

/// HIG button styles.
///
/// * **Filled** — high-emphasis primary action. Solid accent fill.
/// * **Tinted** — medium-emphasis. Translucent accent background, accent
///   text.
/// * **Plain** — low-emphasis / inline action. Transparent background,
///   accent text. Equivalent to a "ghost" button, but the label is *always*
///   accent-colored so it stays readable on any surface our tokens emit.
/// * **Destructive** — high-emphasis dangerous action. Solid danger fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Filled,
    Tinted,
    Plain,
    Destructive,
}

/// Button height tier. All sizes meet Apple's minimum 28pt hit target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    /// 28pt height — toolbar, inline list actions.
    Sm,
    /// 32pt height — default.
    Md,
    /// 40pt height — primary CTAs in dialogs / sheets.
    Lg,
}

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
#[must_use]
pub struct Button {
    id: ElementId,
    variant: ButtonVariant,
    size: ButtonSize,
    label: Option<SharedString>,
    leading_glyph: Option<SharedString>,
    on_click: Option<ClickHandler>,
    appearance: Appearance,
    full_width: bool,
    disabled: bool,
    selected: bool,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, variant: ButtonVariant) -> Self {
        Self {
            id: id.into(),
            variant,
            size: ButtonSize::Md,
            label: None,
            leading_glyph: None,
            on_click: None,
            appearance: Appearance::Dark,
            full_width: false,
            disabled: false,
            selected: false,
        }
    }

    pub fn filled(id: impl Into<ElementId>) -> Self {
        Self::new(id, ButtonVariant::Filled)
    }
    pub fn tinted(id: impl Into<ElementId>) -> Self {
        Self::new(id, ButtonVariant::Tinted)
    }
    pub fn plain(id: impl Into<ElementId>) -> Self {
        Self::new(id, ButtonVariant::Plain)
    }
    pub fn destructive(id: impl Into<ElementId>) -> Self {
        Self::new(id, ButtonVariant::Destructive)
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn leading_glyph(mut self, glyph: impl Into<SharedString>) -> Self {
        self.leading_glyph = Some(glyph.into());
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = appearance;
        self
    }

    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl gpui_component::Selectable for Button {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Button {
    fn height(&self) -> Pixels {
        match self.size {
            ButtonSize::Sm => px(28.0),
            ButtonSize::Md => px(32.0),
            ButtonSize::Lg => px(40.0),
        }
    }

    fn px_inset(&self) -> Spacing {
        match self.size {
            ButtonSize::Sm => Spacing::SM,
            ButtonSize::Md | ButtonSize::Lg => Spacing::MD,
        }
    }

    fn font_size(&self) -> FontSize {
        match self.size {
            ButtonSize::Sm => FontSize::Caption,
            ButtonSize::Md => FontSize::Body,
            ButtonSize::Lg => FontSize::Headline,
        }
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let appearance = self.appearance;
        let height = self.height();
        let pad = self.px_inset().px();
        let font = self.font_size().px();
        let radius = Radius::MD.px();

        // Resolve the variant's color triple: (bg, fg, hover_bg).
        let (bg, fg, hover_bg) = match self.variant {
            ButtonVariant::Filled => (
                SemanticColor::Accent.resolve(appearance),
                SemanticColor::OnAccent.resolve(appearance),
                SemanticColor::AccentHover.resolve(appearance),
            ),
            ButtonVariant::Tinted => {
                let mut tint = SemanticColor::Accent.resolve(appearance);
                tint.a = 0.15;
                let mut hover = tint;
                hover.a = 0.25;
                (tint, SemanticColor::Accent.resolve(appearance), hover)
            }
            ButtonVariant::Plain => {
                let mut hover = SemanticColor::SecondarySystemBackground.resolve(appearance);
                hover.a = 0.6;
                (
                    gpui::transparent_black().into(),
                    SemanticColor::Accent.resolve(appearance),
                    hover,
                )
            }
            ButtonVariant::Destructive => (
                SemanticColor::Danger.resolve(appearance),
                SemanticColor::OnDanger.resolve(appearance),
                {
                    let mut hover = SemanticColor::Danger.resolve(appearance);
                    hover.r = (hover.r + 0.05).min(1.0);
                    hover.g = (hover.g + 0.05).min(1.0);
                    hover.b = (hover.b + 0.05).min(1.0);
                    hover
                },
            ),
        };

        let label = self.label.clone().unwrap_or_default();
        let glyph = self.leading_glyph.clone();
        let on_click = self.on_click.clone();
        let disabled = self.disabled;
        let full_width = self.full_width;

        let mut row = div()
            .id(self.id)
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(Spacing::XS.px())
            .h(height)
            .px(pad)
            .rounded(radius)
            .bg(bg)
            .text_color(fg)
            .text_size(font)
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer();

        if full_width {
            row = row.w_full();
        }

        if disabled {
            row = row.opacity(0.4).cursor_default();
        } else {
            row = row.hover(move |s| s.bg(hover_bg));
            if let Some(handler) = on_click {
                row = row.on_mouse_down(MouseButton::Left, move |_, _, _| {});
                row = row.on_click(move |event, window, cx| {
                    handler(event, window, cx);
                });
            }
        }

        if let Some(g) = glyph {
            row = row.child(div().child(g));
        }
        row.child(label)
    }
}
