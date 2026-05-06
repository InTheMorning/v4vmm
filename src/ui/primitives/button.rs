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
    div, prelude::*, App, ClickEvent, ElementId, FontWeight, IntoElement, MouseButton, RenderOnce,
    Rgba, Window,
};

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::{Icon, IconName, IconSize};
use crate::ui::tokens::{
    resolve_color, Appearance, FontSize, Radius, SemanticColor, Size, Spacing,
};

use super::tooltip::Tooltip;

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

/// Button visual-height tier. The primitive wraps every size in the shared
/// 44pt minimum hit-target contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    /// 28pt height — toolbar, inline list actions.
    Sm,
    /// 32pt height — default.
    Md,
    /// 40pt height — primary CTAs in dialogs / sheets.
    Lg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonContentAlignment {
    Center,
    Leading,
}

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

pub(crate) const TINTED_BUTTON_BG_ALPHA: f32 = 0.08;
pub(crate) const TINTED_BUTTON_HOVER_BG_ALPHA: f32 = 0.12;

#[derive(IntoElement)]
#[must_use]
pub struct Button {
    id: ElementId,
    variant: ButtonVariant,
    size: ButtonSize,
    label: Option<gpui::SharedString>,
    a11y_label: Option<gpui::SharedString>,
    leading_icon: Option<IconName>,
    on_click: Option<ClickHandler>,
    appearance: Option<Appearance>,
    radius: Option<Radius>,
    font_size: Option<FontSize>,
    foreground: Option<SemanticColor>,
    border: Option<SemanticColor>,
    control_style: Option<ControlStyle>,
    tooltip: Option<Tooltip>,
    full_width: bool,
    content_alignment: ButtonContentAlignment,
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
            a11y_label: None,
            leading_icon: None,
            on_click: None,
            appearance: None,
            radius: None,
            font_size: None,
            foreground: None,
            border: None,
            control_style: None,
            tooltip: None,
            full_width: false,
            content_alignment: ButtonContentAlignment::Center,
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

    pub fn label(mut self, label: impl Into<gpui::SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn a11y_label(mut self, label: impl Into<gpui::SharedString>) -> Self {
        self.a11y_label = Some(label.into());
        self
    }

    pub fn tooltip(mut self, label: impl Into<gpui::SharedString>) -> Self {
        self.tooltip = Tooltip::non_empty(label);
        self
    }

    pub const fn leading_icon(mut self, icon: IconName) -> Self {
        self.leading_icon = Some(icon);
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn control_style(mut self, style: ControlStyle) -> Self {
        let spec = style.spec();
        self.variant = spec.variant;
        self.size = spec.size;
        self.font_size = Some(spec.font_size);
        self.radius = Some(spec.radius);
        self.foreground = spec.foreground;
        self.border = spec.border;
        self.control_style = Some(style);
        self
    }

    pub fn styled(id: impl Into<ElementId>, style: ControlStyle) -> Self {
        Self::new(id, style.spec().variant).control_style(style)
    }

    pub fn danger(self) -> Self {
        self.control_style(ControlStyle::Destructive)
    }

    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    pub fn align_leading(mut self) -> Self {
        self.content_alignment = ButtonContentAlignment::Leading;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
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
    fn height(&self, cx: &App) -> gpui::Pixels {
        match self.size {
            ButtonSize::Sm => Size::ButtonSm.scaled(cx),
            ButtonSize::Md => Size::ButtonMd.scaled(cx),
            ButtonSize::Lg => Size::ButtonLg.scaled(cx),
        }
    }

    fn min_hit_target(cx: &App) -> gpui::Pixels {
        Size::MinHitTarget.scaled(cx)
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

    fn effective_tooltip(&self) -> Option<Tooltip> {
        if let Some(tooltip) = self.tooltip.clone() {
            return Some(tooltip);
        }

        if !self
            .control_style
            .is_some_and(ControlStyle::prefers_tooltip)
        {
            return None;
        }

        self.a11y_label
            .clone()
            .or_else(|| self.label.clone())
            .and_then(Tooltip::non_empty)
    }

    fn resolved_colors(&self, cx: &App, appearance: Option<Appearance>) -> (Rgba, Rgba, Rgba) {
        let (bg, mut fg, hover_bg) = match self.variant {
            ButtonVariant::Filled => (
                resolve_color(cx, SemanticColor::Accent, appearance),
                resolve_color(cx, SemanticColor::OnAccent, appearance),
                resolve_color(cx, SemanticColor::AccentHover, appearance),
            ),
            ButtonVariant::Tinted => {
                let mut tint = resolve_color(cx, SemanticColor::Accent, appearance);
                tint.a = TINTED_BUTTON_BG_ALPHA;
                let mut hover = tint;
                hover.a = TINTED_BUTTON_HOVER_BG_ALPHA;
                (
                    tint,
                    resolve_color(cx, SemanticColor::Accent, appearance),
                    hover,
                )
            }
            ButtonVariant::Plain => {
                let mut hover =
                    resolve_color(cx, SemanticColor::SecondarySystemBackground, appearance);
                hover.a = 0.6;
                (
                    gpui::transparent_black().into(),
                    resolve_color(cx, SemanticColor::Accent, appearance),
                    hover,
                )
            }
            ButtonVariant::Destructive => (
                resolve_color(cx, SemanticColor::Danger, appearance),
                resolve_color(cx, SemanticColor::OnDanger, appearance),
                {
                    let mut hover = resolve_color(cx, SemanticColor::Danger, appearance);
                    hover.r = (hover.r + 0.05).min(1.0);
                    hover.g = (hover.g + 0.05).min(1.0);
                    hover.b = (hover.b + 0.05).min(1.0);
                    hover
                },
            ),
        };
        if let Some(foreground) = self.foreground {
            fg = resolve_color(cx, foreground, appearance);
        }
        (bg, fg, hover_bg)
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let height = self.height(cx);
        let pad = self.px_inset().scaled(cx);
        let min_hit_target = Self::min_hit_target(cx);
        let font = self
            .font_size
            .unwrap_or_else(|| self.font_size())
            .scaled(cx);
        let radius = self.radius.unwrap_or(Radius::MD).scaled(cx);
        let appearance = self.appearance;

        let (bg, fg, hover_bg) = self.resolved_colors(cx, appearance);

        let label = self.label.clone().unwrap_or_default();
        let leading_icon = self.leading_icon;
        let on_click = self.on_click.clone();
        let disabled = self.disabled;
        let full_width = self.full_width;
        let content_alignment = self.content_alignment;
        let tooltip = self.effective_tooltip();

        let mut visual = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(Spacing::XS.scaled(cx))
            .h(height)
            .px(pad)
            .rounded(radius)
            .bg(bg)
            .text_color(fg)
            .text_size(font)
            .font_weight(FontWeight::SEMIBOLD);

        visual = match content_alignment {
            ButtonContentAlignment::Center => visual.justify_center(),
            ButtonContentAlignment::Leading => visual.justify_start(),
        };

        if let Some(border) = self.border {
            visual = visual
                .border_1()
                .border_color(resolve_color(cx, border, appearance));
        }

        if full_width {
            visual = visual.w_full();
        }

        let mut hit_target = div()
            .id(self.id)
            .min_w(min_hit_target)
            .min_h(min_hit_target)
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer();

        if full_width {
            hit_target = hit_target.w_full();
        }

        if let Some(tooltip) = tooltip {
            hit_target = hit_target.tooltip(move |window, cx| tooltip.build(window, cx));
        }

        if disabled {
            hit_target = hit_target.opacity(0.4).cursor_default();
        } else {
            visual = visual.hover(move |s| s.bg(hover_bg));
            if let Some(handler) = on_click {
                hit_target = hit_target.on_mouse_down(MouseButton::Left, move |_, _, _| {});
                hit_target = hit_target.on_click(move |event, window, cx| {
                    handler(event, window, cx);
                });
            }
        }

        if let Some(icon) = leading_icon {
            visual = visual.child(Icon::new(icon).size(IconSize::Transport).color(fg));
        }
        hit_target.child(visual.child(label))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_icon_uses_semantic_icon_role() {
        let button = Button::plain("new-playlist").leading_icon(IconName::Add);

        assert_eq!(button.leading_icon, Some(IconName::Add));
    }

    #[test]
    fn menu_buttons_can_align_content_to_leading_edge() {
        let button = Button::plain("playlist-choice")
            .full_width()
            .align_leading();

        assert!(button.full_width);
        assert_eq!(button.content_alignment, ButtonContentAlignment::Leading);
    }

    #[test]
    fn button_carries_contract_accessibility_label() {
        let button = Button::plain("remove").a11y_label("Remove feed from library");

        assert_eq!(
            button.a11y_label,
            Some(gpui::SharedString::from("Remove feed from library"))
        );
    }

    #[test]
    fn row_action_tooltip_uses_accessibility_label() {
        let button = Button::styled("remove", ControlStyle::RowAction)
            .label("Remove")
            .a11y_label("Remove feed from library");

        assert_eq!(
            button
                .effective_tooltip()
                .expect("row action has tooltip")
                .label(),
            gpui::SharedString::from("Remove feed from library")
        );
    }

    #[test]
    fn toolbar_icon_tooltip_falls_back_to_visible_label() {
        let button = Button::styled("sort", ControlStyle::ToolbarIcon).label("Sort A-Z");

        assert_eq!(
            button
                .effective_tooltip()
                .expect("toolbar action has tooltip")
                .label(),
            gpui::SharedString::from("Sort A-Z")
        );
    }

    #[test]
    fn non_compact_button_does_not_invent_tooltip() {
        let button = Button::styled("search", ControlStyle::Primary).label("Search");

        assert!(button.effective_tooltip().is_none());
    }
}
