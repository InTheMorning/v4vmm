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
use crate::ui::primitives::Button;
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::workspace::{BreadcrumbDisplay, BreadcrumbSegment, FrameNavigationEntry};

type BreadcrumbSelectHandler = Rc<dyn Fn(FrameNavigationEntry, &mut Window, &mut App) + 'static>;

/// Breadcrumb trail shell.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct BreadcrumbTrail {
    display: BreadcrumbDisplay,
    on_select: Option<BreadcrumbSelectHandler>,
}

impl BreadcrumbTrail {
    /// Creates a breadcrumb trail from display data.
    pub(crate) fn new(display: BreadcrumbDisplay) -> Self {
        Self {
            display,
            on_select: None,
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
}

impl RenderOnce for BreadcrumbTrail {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = SharedString::from(self.display.id);
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
                            elements.push(separator(cx));
                        }
                        elements.push(segment_element(segment, self.on_select.clone(), cx));
                        elements
                    }),
            )
    }
}

fn segment_element(
    segment: BreadcrumbSegment,
    on_select: Option<BreadcrumbSelectHandler>,
    cx: &App,
) -> AnyElement {
    if segment.is_current {
        return div()
            .min_w_0()
            .text_size(FontSize::Micro.scaled(cx))
            .text_color(color(cx, SemanticColor::TertiaryLabel))
            .truncate()
            .child(SharedString::from(segment.label))
            .into_any_element();
    }

    let target = segment.target;
    let mut button = Button::styled(SharedString::from(segment.id), ControlStyle::Ghost)
        .label(SharedString::from(segment.label))
        .a11y_label(SharedString::from(segment.a11y_label));
    if let (Some(target), Some(handler)) = (target, on_select) {
        button = button.on_click(move |_, window, cx| {
            handler(target.clone(), window, cx);
        });
    }
    button.into_any_element()
}

fn separator(cx: &App) -> AnyElement {
    div()
        .text_size(FontSize::Micro.scaled(cx))
        .text_color(color(cx, SemanticColor::TertiaryLabel))
        .child("/")
        .into_any_element()
}
