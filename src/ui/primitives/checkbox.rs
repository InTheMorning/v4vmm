//! Checkbox primitive — a typed checked state for row selection (ADR 0084
//! Decision 10).
//!
//! The check mark is a named icon ([`IconName::Check`]), never a glyph
//! string (AGENTS.md: token discipline). The checked state also changes
//! the fill and the border color, so it never relies on color alone.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window,
};

use crate::ui::icons::{Icon, IconName, IconSize};
use crate::ui::tokens::{color, Radius, SemanticColor, Size};

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// The checked state of a [`Checkbox`] (ADR 0084 Decision 10). A row's
/// checkbox has no third, indeterminate value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckboxState {
    Checked,
    NotChecked,
}

impl CheckboxState {
    /// Builds the state from a plain `bool`, for a caller that already
    /// holds one.
    #[must_use]
    pub const fn from_checked(checked: bool) -> Self {
        if checked {
            Self::Checked
        } else {
            Self::NotChecked
        }
    }

    #[must_use]
    const fn is_checked(self) -> bool {
        matches!(self, Self::Checked)
    }
}

/// A row-selection checkbox (ADR 0084 Decision 10). The primitive owns no
/// state: the caller's view model gives the checked state, and the
/// caller's click handler toggles it.
#[derive(IntoElement)]
#[must_use]
pub struct Checkbox {
    id: ElementId,
    state: CheckboxState,
    a11y_label: SharedString,
    on_click: Option<ClickHandler>,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>, state: CheckboxState) -> Self {
        Self {
            id: id.into(),
            state,
            a11y_label: SharedString::default(),
            on_click: None,
        }
    }

    /// The accessibility label, for example "Select Face Pale". Retained
    /// as contract data: GPUI 0.2.x gives no row-level label sink for a
    /// `div`, the same note as `ListRow` (ADR 0038 task 005).
    pub fn a11y_label(mut self, label: impl Into<SharedString>) -> Self {
        self.a11y_label = label.into();
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

impl RenderOnce for Checkbox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.state.is_checked();
        let box_size = Size::CheckboxIndicator.scaled(cx);
        let hit_target = Size::MinHitTarget.scaled(cx);
        let radius = Radius::SM.scaled(cx);
        let border = color(cx, SemanticColor::Separator);
        let accent = color(cx, SemanticColor::Accent);
        let on_accent = color(cx, SemanticColor::OnAccent);

        let indicator = div()
            .w(box_size)
            .h(box_size)
            .flex()
            .items_center()
            .justify_center()
            .rounded(radius)
            .border_1()
            .border_color(if checked { accent } else { border })
            .when(checked, |el| el.bg(accent))
            .when(checked, |el| {
                el.child(
                    Icon::new(IconName::Check)
                        .size(IconSize::Action)
                        .color(on_accent),
                )
            });

        let mut hit_area = div()
            .id(self.id)
            .min_w(hit_target)
            .min_h(hit_target)
            .flex()
            .items_center()
            .justify_center()
            .child(indicator);

        if let Some(handler) = self.on_click {
            hit_area = hit_area
                .cursor_pointer()
                .on_click(move |event, window, cx| handler(event, window, cx));
        }

        hit_area
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_state_builds_from_a_bool() {
        assert_eq!(CheckboxState::from_checked(true), CheckboxState::Checked);
        assert_eq!(
            CheckboxState::from_checked(false),
            CheckboxState::NotChecked
        );
        assert!(CheckboxState::Checked.is_checked());
        assert!(!CheckboxState::NotChecked.is_checked());
    }

    #[test]
    fn checkbox_carries_its_accessibility_label() {
        let checkbox = Checkbox::new("selection-checkbox:1", CheckboxState::Checked)
            .a11y_label("Select Face Pale");

        assert_eq!(checkbox.a11y_label, SharedString::from("Select Face Pale"));
    }
}
