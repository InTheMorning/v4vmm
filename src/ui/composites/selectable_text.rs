//! Selectable read-only plain text for the ADR 0063 log pane.
//!
//! This composite owns focus, pointer selection, and clipboard interaction.
//! Source text is never parsed or edited; equal updates preserve the selection.

#![warn(clippy::pedantic)]

use gpui::{
    div, prelude::*, App, ClipboardItem, Context, ElementId, FocusHandle, HighlightStyle,
    IntoElement, KeyDownEvent, MouseButton, Render, RenderOnce, SharedString, StyledText,
    TextLayout, Window,
};

use crate::ui::primitives::{ContextMenuItem, ContextMenuItemDisplay, PointerContextMenu};
use crate::ui::tokens::{color, SemanticColor};
use crate::view_models::text_selection::TextSelection;

/// Selectable plain-text content with selection retained under its stable element ID.
#[derive(IntoElement)]
pub(crate) struct SelectableText {
    id: ElementId,
    value: SharedString,
}

impl SelectableText {
    pub(crate) fn new(id: impl Into<ElementId>, value: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
        }
    }
}

impl RenderOnce for SelectableText {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id, cx, |_, cx| SelectableTextState {
            focus: cx.focus_handle(),
            value: SharedString::default(),
            selection: TextSelection::default(),
            dragging: false,
            menu_position: None,
        });
        state.update(cx, |state, cx| {
            if state.update_value(self.value) {
                cx.notify();
            }
        });
        state
    }
}

struct SelectableTextState {
    focus: FocusHandle,
    value: SharedString,
    selection: TextSelection,
    dragging: bool,
    menu_position: Option<gpui::Point<gpui::Pixels>>,
}

impl SelectableTextState {
    fn update_value(&mut self, value: SharedString) -> bool {
        if self.value != value {
            self.value = value;
            self.selection = TextSelection::default();
            self.dragging = false;
            self.menu_position = None;
            return true;
        }
        false
    }

    fn copy_selection(&self, cx: &mut App) {
        if let Some(text) = self.selection.selected_text(&self.value) {
            cx.write_to_clipboard(ClipboardItem::new_string(text.to_owned()));
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if !modifiers.control || modifiers.alt || modifiers.platform {
            return;
        }
        match event.keystroke.key.as_str() {
            "a" => {
                self.selection = TextSelection {
                    anchor: 0,
                    head: self.value.len(),
                };
                cx.stop_propagation();
                cx.notify();
            }
            "c" => {
                self.copy_selection(cx);
                cx.stop_propagation();
            }
            _ => {}
        }
    }
}

impl Render for SelectableTextState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let range = self.selection.range();
        let text =
            StyledText::new(self.value.clone()).with_highlights((!range.is_empty()).then(|| {
                (
                    range,
                    HighlightStyle {
                        background_color: Some(color(cx, SemanticColor::SelectedContent).into()),
                        ..HighlightStyle::default()
                    },
                )
            }));
        let start_layout = text.layout().clone();
        let move_layout = text.layout().clone();
        let end_layout = text.layout().clone();
        let copy = self.selection.copy_action(&self.value);

        div()
            .id("selectable-text-content")
            .track_focus(&self.focus)
            .cursor_text()
            .whitespace_nowrap()
            .flex_shrink_0()
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                    this.dragging = false;
                    this.menu_position = Some(event.position);
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                    this.focus.focus(window);
                    let index = index_at(&start_layout, event.position);
                    this.selection = TextSelection::at(&this.value, index);
                    this.dragging = true;
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .on_mouse_move(
                cx.listener(move |this, event: &gpui::MouseMoveEvent, _, cx| {
                    if !event.dragging() {
                        this.dragging = false;
                    } else if this.dragging {
                        this.selection
                            .extend(&this.value, index_at(&move_layout, event.position));
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, event: &gpui::MouseUpEvent, _, cx| {
                    if this.dragging {
                        this.selection
                            .extend(&this.value, index_at(&end_layout, event.position));
                        this.dragging = false;
                        cx.notify();
                    }
                }),
            )
            .on_key_down(cx.listener(|this, event, _, cx| this.key_down(event, cx)))
            .child(text)
            .when_some(self.menu_position, |content, position| {
                let owner = cx.weak_entity();
                let copy_owner = owner.clone();
                content.child(
                    PointerContextMenu::new(
                        "selection-context-menu",
                        position,
                        self.focus.clone(),
                        move |_, cx| {
                            let _ = owner.update(cx, |this, cx| {
                                this.menu_position = None;
                                cx.notify();
                            });
                        },
                    )
                    .item(
                        ContextMenuItem::new(ContextMenuItemDisplay {
                            id: copy.id.into(),
                            label: copy.label.into(),
                            a11y_label: copy.a11y_label.into(),
                            destructive: false,
                            disabled: copy.availability.disabled(),
                        })
                        .on_select(move |_, cx| {
                            let _ = copy_owner.update(cx, |this, cx| this.copy_selection(cx));
                        }),
                    ),
                )
            })
    }
}

fn index_at(layout: &TextLayout, position: gpui::Point<gpui::Pixels>) -> usize {
    match layout.index_for_position(position) {
        Ok(index) | Err(index) => index,
    }
}
