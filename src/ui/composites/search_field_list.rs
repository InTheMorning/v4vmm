//! The field list under the toolbar search field (ADR 0087 Decision 3).
//!
//! The list draws over the other content with `deferred` and `anchored`. It
//! has no focus handle, so the keyboard focus stays in the search field.
//! The search field view captures the list keys. This composite only draws
//! the surface, the notices and the rows of [`FieldListVm`]. Each id, text
//! and label comes from the view model. No text calls `truncate()` (ADR
//! 0063). Long text wraps.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    anchored, deferred, div, prelude::*, AnyElement, App, MouseButton, ScrollHandle, SharedString,
    Window,
};

use crate::ui::primitives::status_badge::render_row_badge;
use crate::ui::primitives::surface::{Surface, SurfaceElevation};
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor, Size, Spacing, Weight};
use crate::view_models::search_field_list::{
    FieldListNoticeKind, FieldListNoticeVm, FieldListRowVm, FieldListVm,
};

/// The handler of a mouse down on a row. It gets the row index.
type RowHandler = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// The field list under the toolbar search field.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct SearchFieldList {
    vm: FieldListVm,
    scroll: ScrollHandle,
    on_row: Option<RowHandler>,
}

impl SearchFieldList {
    /// Makes the list for the view model. `scroll` scrolls the rows.
    pub(crate) fn new(vm: FieldListVm, scroll: ScrollHandle) -> Self {
        Self {
            vm,
            scroll,
            on_row: None,
        }
    }

    /// Sets the handler of a left mouse down on a row.
    pub(crate) fn on_row(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_row = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SearchFieldList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { vm, scroll, on_row } = self;
        let notices: Vec<AnyElement> = vm
            .notices
            .into_iter()
            .map(|notice| render_notice(notice, cx))
            .collect();
        let rows: Vec<AnyElement> = vm
            .rows
            .into_iter()
            .enumerate()
            .map(|(index, row)| render_row(index, row, on_row.clone(), cx))
            .collect();
        let row_list = div()
            .id(FieldListVm::LIST_ID)
            .flex()
            .flex_col()
            .max_h(Size::ColumnRegular.scaled(cx))
            .overflow_y_scroll()
            .track_scroll(&scroll)
            .children(rows);
        let surface = div()
            .flex()
            .flex_col()
            .occlude()
            .w(Size::ColumnTall.scaled(cx))
            .mt(Spacing::XS.scaled(cx))
            .child(
                Surface::new(SurfaceElevation::Raised)
                    .padding(Spacing::XS)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .min_w_0()
                            .children(notices)
                            .child(row_list),
                    ),
            );
        div()
            .absolute()
            .left_0()
            .bottom_0()
            .size_0()
            .child(deferred(
                anchored()
                    .snap_to_window_with_margin(Spacing::SM.scaled(cx))
                    .child(surface),
            ))
    }
}

/// Draws one notice. The text names the problem, so the color is not the
/// only cue.
fn render_notice(notice: FieldListNoticeVm, cx: &App) -> AnyElement {
    let text_color = match notice.kind {
        FieldListNoticeKind::FieldError => SemanticColor::WarningLabel,
        FieldListNoticeKind::ScopeConflict => SemanticColor::SecondaryLabel,
    };
    div()
        .min_w_0()
        .px(Spacing::SM.scaled(cx))
        .py(Spacing::XS.scaled(cx))
        .text_size(FontSize::Caption.scaled(cx))
        .text_color(color(cx, text_color))
        .child(SharedString::from(notice.text))
        .into_any_element()
}

/// Draws one row. The selected row has a fill and a border. An unavailable
/// row draws its label in a tertiary color, and its note gives the reason.
fn render_row(
    index: usize,
    row: FieldListRowVm,
    on_row: Option<RowHandler>,
    cx: &App,
) -> AnyElement {
    let available = row.availability.is_available();
    let label_color = if available {
        SemanticColor::Label
    } else {
        SemanticColor::TertiaryLabel
    };
    let border_color = if row.selected {
        SemanticColor::Focus
    } else {
        SemanticColor::SecondarySystemBackground
    };
    let title_line = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(Spacing::XS.scaled(cx))
        .min_w_0()
        .child(
            div()
                .min_w_0()
                .text_size(FontSize::Body.scaled(cx))
                .font_weight(Weight::Semibold.into())
                .text_color(color(cx, label_color))
                .child(SharedString::from(row.label)),
        )
        .when_some(row.mark, |line, mark| {
            line.child(render_row_badge(mark.label(), cx))
        });
    let detail_line = div()
        .flex()
        .flex_wrap()
        .gap(Spacing::MD.scaled(cx))
        .min_w_0()
        .when_some(row.values_text, |line, text| {
            line.child(caption(text, SemanticColor::SecondaryLabel, cx))
        })
        .when_some(row.example_text, |line, text| {
            line.child(caption(text, SemanticColor::TertiaryLabel, cx))
        })
        .when_some(row.note, |line, text| {
            line.child(caption(text, SemanticColor::SecondaryLabel, cx))
        });
    div()
        .id(SharedString::from(row.element_id))
        .flex()
        .flex_col()
        .justify_center()
        .gap(Spacing::XXS.scaled(cx))
        .min_w_0()
        .min_h(Size::RowMd.scaled(cx))
        .px(Spacing::SM.scaled(cx))
        .py(Spacing::XS.scaled(cx))
        .rounded(Radius::MD.scaled(cx))
        .border_1()
        .border_color(color(cx, border_color))
        .when(row.selected, |el| {
            el.bg(color(cx, SemanticColor::SelectedContent))
        })
        .when(available, gpui::Styled::cursor_pointer)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            if let Some(handler) = &on_row {
                handler(index, window, cx);
            }
        })
        .child(title_line)
        .child(detail_line)
        .into_any_element()
}

/// One caption text of a row.
fn caption(text: String, text_color: SemanticColor, cx: &App) -> AnyElement {
    div()
        .min_w_0()
        .text_size(FontSize::Caption.scaled(cx))
        .text_color(color(cx, text_color))
        .child(SharedString::from(text))
        .into_any_element()
}
