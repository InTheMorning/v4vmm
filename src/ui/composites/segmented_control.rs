#![warn(clippy::pedantic)]
//! Apple-style segmented control composite.
//!
//! Mutually-exclusive horizontal selector built on the [`Button`] primitive.
//! The currently-selected segment renders with the `Filled` variant; the rest
//! render `Ghost`. Spacing between segments scales with the global UI scale.

use std::rc::Rc;

use gpui::{
    div, App, ClickEvent, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window,
};

use crate::ui::primitives::{Button, ButtonVariant, HStack};
use crate::ui::tokens::{Radius, Spacing};

type OnSelect<K> = Rc<dyn Fn(&K, &mut Window, &mut App) + 'static>;

/// One segment in a [`SegmentedControl`].
pub struct Segment<K: Clone + PartialEq + 'static> {
    pub id: ElementId,
    pub key: K,
    pub label: SharedString,
}

/// Display-ready segment fields for a [`SegmentedControl`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentDisplay<K: Clone + PartialEq + 'static> {
    pub id: ElementId,
    pub key: K,
    pub label: SharedString,
}

impl<K: Clone + PartialEq + 'static> Segment<K> {
    pub fn new(display: SegmentDisplay<K>) -> Self {
        Self {
            id: display.id,
            key: display.key,
            label: display.label,
        }
    }
}

/// Segmented control — pick exactly one of N options.
#[derive(IntoElement)]
pub struct SegmentedControl<K: Clone + PartialEq + 'static> {
    segments: Vec<Segment<K>>,
    selected: K,
    on_select: Option<OnSelect<K>>,
}

impl<K: Clone + PartialEq + 'static> SegmentedControl<K> {
    pub fn new(selected: K) -> Self {
        Self {
            segments: Vec::new(),
            selected,
            on_select: None,
        }
    }

    #[must_use]
    pub fn segment(mut self, segment: Segment<K>) -> Self {
        self.segments.push(segment);
        self
    }

    #[must_use]
    pub fn segments(mut self, segments: impl IntoIterator<Item = Segment<K>>) -> Self {
        self.segments.extend(segments);
        self
    }

    #[must_use]
    pub fn on_select(mut self, handler: impl Fn(&K, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl<K: Clone + PartialEq + 'static> RenderOnce for SegmentedControl<K> {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_select = self.on_select;
        let selected = self.selected;
        let mut row = HStack::new().spacing(Spacing::XS).center();

        for segment in self.segments {
            let key = segment.key.clone();
            let is_selected = key == selected;
            let variant = if is_selected {
                ButtonVariant::Filled
            } else {
                ButtonVariant::Plain
            };
            let mut button = Button::new(segment.id, variant).label(segment.label.clone());
            if let Some(handler) = on_select.clone() {
                let key_for_click = key.clone();
                button = button.on_click(move |event: &ClickEvent, window, cx| {
                    let _ = event;
                    handler(&key_for_click, window, cx);
                });
            }
            row = row.child(button);
        }

        div().rounded(Radius::SM.scaled(cx)).child(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_uses_display_contract() {
        let segment = Segment::new(SegmentDisplay {
            id: "scale-medium".into(),
            key: 2_u8,
            label: "M".into(),
        });

        assert_eq!(segment.key, 2);
        assert_eq!(segment.label, SharedString::from("M"));
    }
}
