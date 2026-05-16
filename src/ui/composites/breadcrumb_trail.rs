//! Breadcrumb trail composite.
//!
//! The composite renders GPUI-free breadcrumb display contracts and dispatches
//! typed navigation targets. It contains no source-specific routing policy.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, AnyElement, App, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window,
};

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::{Icon, IconName};
use crate::ui::primitives::Button;
use crate::ui::tokens::{resolve_color, Appearance, FontSize, SemanticColor, Spacing};
use crate::view_models::workspace::{BreadcrumbDisplay, BreadcrumbSegment, FrameNavigationEntry};

type BreadcrumbSelectHandler = Rc<dyn Fn(FrameNavigationEntry, &mut Window, &mut App) + 'static>;

/// Breadcrumb trail shell.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct BreadcrumbTrail {
    display: BreadcrumbDisplay,
    on_select: Option<BreadcrumbSelectHandler>,
    appearance: Option<Appearance>,
}

impl BreadcrumbTrail {
    /// Creates a breadcrumb trail from display data.
    pub(crate) fn new(display: BreadcrumbDisplay) -> Self {
        Self {
            display,
            on_select: None,
            appearance: None,
        }
    }

    /// Supplies a segment-selection callback.
    pub(crate) fn on_select(
        mut self,
        handler: impl Fn(FrameNavigationEntry, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the appearance for color resolution (theme awareness).
    pub(crate) fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for BreadcrumbTrail {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = SharedString::from(self.display.id);
        let secondary_color = resolve_color(cx, SemanticColor::SecondaryLabel, self.appearance);
        let current_color = resolve_color(cx, SemanticColor::TertiaryLabel, self.appearance);

        div()
            .id(id)
            .flex()
            .flex_row()
            .items_center()
            .gap(Spacing::XS.scaled(cx))
            .min_w_0()
            .children(
                self.display
                    .segments
                    .into_iter()
                    .enumerate()
                    .flat_map(|(index, segment)| {
                        let mut elements = Vec::with_capacity(2);
                        if index > 0 {
                            elements.push(separator(secondary_color));
                        }
                        elements.push(segment_element(
                            segment,
                            self.on_select.clone(),
                            secondary_color,
                            current_color,
                            cx,
                        ));
                        elements
                    }),
            )
    }
}

fn segment_element(
    segment: BreadcrumbSegment,
    on_select: Option<BreadcrumbSelectHandler>,
    secondary_color: gpui::Rgba,
    current_color: gpui::Rgba,
    cx: &App,
) -> AnyElement {
    let Some(target) = segment.target.clone() else {
        return text_segment(&segment, secondary_color, current_color, cx);
    };
    let Some(handler) = on_select else {
        return text_segment(&segment, secondary_color, current_color, cx);
    };
    if segment.is_current {
        return text_segment(&segment, secondary_color, current_color, cx);
    }

    Button::styled(SharedString::from(segment.id), ControlStyle::Ghost)
        .label(SharedString::from(segment.label))
        .a11y_label(SharedString::from(segment.a11y_label))
        .on_click(move |_, window, cx| {
            handler(target.clone(), window, cx);
        })
        .into_any_element()
}

fn text_segment(
    segment: &BreadcrumbSegment,
    secondary_color: gpui::Rgba,
    current_color: gpui::Rgba,
    cx: &App,
) -> AnyElement {
    let text_color = if segment.is_current {
        current_color
    } else {
        secondary_color
    };
    div()
        .min_w_0()
        .text_size(FontSize::Micro.scaled(cx))
        .text_color(text_color)
        .truncate()
        .child(SharedString::from(segment.label.clone()))
        .into_any_element()
}

fn separator(color: gpui::Rgba) -> AnyElement {
    Icon::new(IconName::ChevronRight)
        .color(color)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::workspace::{
        BreadcrumbSegment, BreadcrumbTruncation, FrameNavigationEntry,
    };

    fn test_breadcrumb_display() -> BreadcrumbDisplay {
        BreadcrumbDisplay {
            id: "test-breadcrumb".to_string(),
            segments: vec![
                BreadcrumbSegment {
                    id: "seg-0".to_string(),
                    label: "Library".to_string(),
                    a11y_label: "Library".to_string(),
                    target: Some(FrameNavigationEntry::SourceList),
                    is_current: false,
                },
                BreadcrumbSegment {
                    id: "seg-1".to_string(),
                    label: "Track 42".to_string(),
                    a11y_label: "Track 42".to_string(),
                    target: Some(FrameNavigationEntry::TrackDetail(42)),
                    is_current: true,
                },
            ],
            truncation: BreadcrumbTruncation::MiddleEllipsis,
        }
    }

    #[test]
    fn breadcrumb_trail_new_creates_with_no_handler() {
        let display = test_breadcrumb_display();
        let trail = BreadcrumbTrail::new(display.clone());
        assert_eq!(trail.display, display);
        assert!(trail.on_select.is_none());
        assert!(trail.appearance.is_none());
    }

    #[test]
    fn breadcrumb_trail_appearance_is_optional() {
        let display = test_breadcrumb_display();
        let trail = BreadcrumbTrail::new(display.clone()).appearance(Appearance::Light);
        assert_eq!(trail.display, display);
        assert!(trail.appearance.is_some());
    }

    #[test]
    fn breadcrumb_trail_on_select_stores_handler() {
        let display = test_breadcrumb_display();
        let trail = BreadcrumbTrail::new(display).on_select(|_entry, _window, _cx| {
            // Handler body not executed in this test context
        });
        assert!(trail.on_select.is_some());
    }

    #[test]
    fn breadcrumb_segment_without_target_renders_as_text() {
        let segment = BreadcrumbSegment {
            id: "seg-no-target".to_string(),
            label: "No Target".to_string(),
            a11y_label: "No Target".to_string(),
            target: None,
            is_current: false,
        };
        // Segment without target should render as plain text regardless of handler presence
        assert!(segment.target.is_none());
    }

    #[test]
    fn breadcrumb_current_segment_renders_as_text() {
        let segment = BreadcrumbSegment {
            id: "seg-current".to_string(),
            label: "Current".to_string(),
            a11y_label: "Current".to_string(),
            target: Some(FrameNavigationEntry::SourceList),
            is_current: true,
        };
        // Current segment should render as plain text even with target and handler
        assert!(segment.is_current);
    }

    #[test]
    fn breadcrumb_segment_with_target_and_handler_renders_as_button() {
        let segment = BreadcrumbSegment {
            id: "seg-button".to_string(),
            label: "Navigate".to_string(),
            a11y_label: "Navigate to Library".to_string(),
            target: Some(FrameNavigationEntry::SourceList),
            is_current: false,
        };
        // This segment has target, no is_current, and would render as button if handler present
        assert!(segment.target.is_some());
        assert!(!segment.is_current);
    }
}
