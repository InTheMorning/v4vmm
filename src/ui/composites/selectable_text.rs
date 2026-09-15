//! Selectable read-only log text for ADRs 0063 and 0071.
//!
//! This composite owns focus, pointer selection, and clipboard interaction.
//! Source text is never parsed or edited; equal updates preserve the selection.

#![warn(clippy::pedantic)]

use gpui::{
    div, prelude::*, App, BorderStyle, Bounds, ClipboardItem, Context, Corners, Edges, Element,
    ElementId, FocusHandle, GlobalElementId, HitboxBehavior, InspectorElementId, IntoElement,
    KeyDownEvent, LayoutId, MouseButton, PaintQuad, Pixels, Render, RenderOnce, SharedString,
    StyledText, Subscription, TextLayout, WeakEntity, Window,
};

use gpui_base::{
    TextSelection as WindowSelection, TextSelectionEndpoint, TextSelectionEvent,
    TextSelectionHandle, TextSelectionRegistration, TextSelectionRun,
};
use gpui_component::input::Copy;

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
        let state = window.use_keyed_state(self.id, cx, |_, cx| SelectableTextState::new(cx));
        state.update(cx, |state, cx| {
            if state.update_value(self.value, window, cx) {
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
    handle: TextSelectionHandle,
    pending_projection: bool,
    endpoints: Option<(TextSelectionEndpoint, TextSelectionEndpoint)>,
    _subscription: Subscription,
    menu_position: Option<gpui::Point<gpui::Pixels>>,
}

impl SelectableTextState {
    fn new(cx: &mut Context<Self>) -> Self {
        let handle = TextSelectionHandle::new(String::new(), cx);
        let focus = cx.focus_handle();
        let selection_focus = focus.clone();
        handle.focus_with(move |window, cx| selection_focus.focus(window, cx), cx);
        let owner = cx.weak_entity();
        handle.copy_with(
            move |cx| {
                owner
                    .upgrade()
                    .and_then(|owner| {
                        let state = owner.read(cx);
                        state
                            .selection
                            .selected_text(&state.value)
                            .map(str::to_owned)
                    })
                    .unwrap_or_default()
            },
            cx,
        );
        let owner = cx.weak_entity();
        let subscription = handle.subscribe(
            move |event, cx| {
                let _ = owner.update(cx, |this, cx| {
                    if let TextSelectionEvent::SelectionChanged(snapshot) = event {
                        let endpoints =
                            snapshot.map(|snapshot| (snapshot.anchor(), snapshot.cursor()));
                        if endpoints != this.endpoints {
                            this.endpoints = endpoints;
                            this.pending_projection = endpoints.is_some();
                        }
                        if snapshot.is_none() && !this.handle.has_local_selection(cx) {
                            if this.menu_position.is_some() {
                                // Root clears during mouse-down capture, before the menu's
                                // Copy activates on release. Retain its exact range (ADR 0071).
                                this.handle
                                    .set_local_selection(!this.selection.range().is_empty(), cx);
                            } else {
                                this.selection = TextSelection::default();
                            }
                        }
                        cx.notify();
                    }
                });
            },
            cx,
        );
        Self {
            focus,
            value: SharedString::default(),
            selection: TextSelection::default(),
            handle,
            pending_projection: false,
            endpoints: None,
            _subscription: subscription,
            menu_position: None,
        }
    }

    fn update_value(
        &mut self,
        value: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.value != value {
            if !value.starts_with(self.value.as_ref()) {
                if self.handle.snapshot(cx).is_some() || self.handle.has_local_selection(cx) {
                    WindowSelection::clear(window, cx);
                }
                self.pending_projection = false;
            }
            self.selection.update_text(&self.value, &value);
            self.value = value;
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

    fn publish_primary(&self, cx: &mut App) {
        #[cfg(not(target_os = "linux"))]
        let _ = cx;
        #[cfg(target_os = "linux")]
        if let Some(text) = self.selection.selected_text(&self.value) {
            cx.write_to_primary(ClipboardItem::new_string(text.to_owned()));
        }
    }

    fn on_copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        // Handle Root's bound action before its whitespace-trimming fallback.
        self.copy_selection(cx);
        cx.stop_propagation();
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if !modifiers.control || modifiers.alt || modifiers.platform {
            return;
        }
        if event.keystroke.key == "a" {
            WindowSelection::clear(window, cx);
            self.handle.set_local_selection(true, cx);
            self.pending_projection = false;
            self.selection = TextSelection {
                anchor: 0,
                head: self.value.len(),
            };
            self.publish_primary(cx);
            cx.stop_propagation();
            cx.notify();
        }
    }
}

impl Render for SelectableTextState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text = LogText {
            text: StyledText::new(self.value.clone()),
            value: self.value.clone(),
            handle: self.handle.clone(),
            owner: cx.weak_entity(),
            selection_color: color(cx, SemanticColor::SelectedContent).into(),
        };
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
                    this.menu_position = Some(event.position);
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .on_key_down(cx.listener(Self::key_down))
            .on_action(cx.listener(Self::on_copy))
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

// The upstream handle owns pointer boundaries and gesture projection. This
// element only paints that projection and retains exact Copy ranges (ADR 0071).
struct LogText {
    text: StyledText,
    value: SharedString,
    handle: TextSelectionHandle,
    owner: WeakEntity<SelectableTextState>,
    selection_color: gpui::Hsla,
}

impl IntoElement for LogText {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for LogText {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some("log-selection-run".into())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        self.text.request_layout(id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.text.prepaint(id, inspector, bounds, state, window, cx);
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        self.handle.register(
            TextSelectionRegistration::new(hitbox, bounds).with_text_bounds(vec![bounds]),
            window,
            cx,
        );
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut (),
        paint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let layout = self.text.layout().clone();
        let projection = self.handle.update_runs(
            &[TextSelectionRun::new(
                self.value.clone(),
                layout.clone(),
                bounds,
            )],
            cx,
        );
        let range = self
            .owner
            .update(cx, |owner, cx| {
                if owner.pending_projection {
                    let range = projection
                        .ranges()
                        .first()
                        .cloned()
                        .flatten()
                        .unwrap_or(0..0);
                    owner.selection = TextSelection {
                        anchor: range.start,
                        head: range.end,
                    };
                    owner.pending_projection = false;
                    owner.publish_primary(cx);
                    cx.notify();
                }
                owner.selection.range()
            })
            .unwrap_or(0..0);
        paint_selection(&layout, range, self.selection_color, window);
        self.text
            .paint(id, inspector, bounds, state, paint, window, cx);
    }
}

fn paint_selection(
    layout: &TextLayout,
    range: std::ops::Range<usize>,
    color: gpui::Hsla,
    window: &mut Window,
) {
    if range.is_empty() {
        return;
    }
    let (Some(start), Some(end)) = (
        layout.position_for_index(range.start),
        layout.position_for_index(range.end),
    ) else {
        return;
    };
    let bounds = layout.bounds();
    let height = layout.line_height();
    let mut y = start.y;
    while y <= end.y {
        let left = if y == start.y { start.x } else { bounds.left() };
        let right = if y == end.y { end.x } else { bounds.right() };
        window.paint_quad(PaintQuad {
            bounds: Bounds::from_corners(gpui::point(left, y), gpui::point(right, y + height)),
            background: color.into(),
            corner_radii: Corners::default(),
            border_widths: Edges::default(),
            border_color: gpui::transparent_black(),
            border_style: BorderStyle::default(),
        });
        y += height;
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, Entity, Modifiers, MouseDownEvent, TestAppContext};

    use super::*;

    struct LogTest(Entity<SelectableTextState>, Entity<gpui_component::Root>);
    impl Render for LogTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(self.1.clone())
        }
    }

    #[gpui::test]
    fn adr_0071_log_handle_selects_words_and_lines_without_rewriting_copy(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let state = cx.new(|cx| {
                let mut state = SelectableTextState::new(cx);
                state.value = "  café /tmp/music.flac\nlast line".into();
                state
            });
            LogTest(
                state.clone(),
                cx.new(|cx| gpui_component::Root::new(state, window, cx)),
            )
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        let position = gpui::point(gpui::px(35.), gpui::px(12.));
        for (count, expected) in [(2, "café"), (3, "  café /tmp/music.flac")] {
            cx.simulate_event(MouseDownEvent {
                position,
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: count,
                first_mouse: false,
            });
            cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            cx.update(|_, cx| {
                let owner = root.read(cx).0.clone();
                let state = owner.read(cx);
                assert_eq!(state.selection.selected_text(&state.value), Some(expected));
                owner.update(cx, |state, cx| state.copy_selection(cx));
                assert_eq!(
                    cx.read_from_clipboard().unwrap().text().as_deref(),
                    Some(expected)
                );
                #[cfg(target_os = "linux")]
                assert_eq!(
                    cx.read_from_primary().unwrap().text().as_deref(),
                    Some(expected)
                );
            });
        }
        cx.update(|window, cx| {
            let owner = root.read(cx).0.clone();
            owner.update(cx, |state, cx| {
                let original = state.selection.range();
                state.update_value(
                    "  café /tmp/music.flac\nlast line\nappended".into(),
                    window,
                    cx,
                );
                assert_eq!(state.selection.range(), original);
            });
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.update(|window, cx| {
            let owner = root.read(cx).0.clone();
            owner.update(cx, |state, cx| {
                assert_eq!(
                    state.selection.selected_text(&state.value),
                    Some("  café /tmp/music.flac")
                );
                state.update_value("replacement".into(), window, cx);
                assert!(state.selection.range().is_empty());
            });
        });
    }

    #[gpui::test]
    fn adr_0071_log_pointer_copy_retains_selection_until_activation(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let expected = "  café /tmp/👩‍💻.flac  ";
        let (root, cx) = cx.add_window_view(|window, cx| {
            let state = cx.new(|cx| {
                let mut state = SelectableTextState::new(cx);
                state.value = format!("{expected}\nlast line").into();
                state
            });
            LogTest(
                state.clone(),
                cx.new(|cx| gpui_component::Root::new(state, window, cx)),
            )
        });
        cx.update(|window, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string("CLIPBOARD-KEEP".into()));
            let _ = window.draw(cx);
        });
        let position = gpui::point(gpui::px(35.), gpui::px(12.));
        cx.simulate_event(MouseDownEvent {
            position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 3,
            first_mouse: false,
        });
        cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_down(position, MouseButton::Right, Modifiers::default());
        cx.simulate_mouse_up(position, MouseButton::Right, Modifiers::default());
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        let menu_position = cx.debug_bounds("context-menu-items").unwrap().center();
        cx.simulate_mouse_down(menu_position, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.update(|_, cx| {
            let owner = root.read(cx).0.read(cx);
            assert_eq!(owner.selection.selected_text(&owner.value), Some(expected));
            assert!(owner.menu_position.is_some());
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some("CLIPBOARD-KEEP")
            );
        });
        cx.simulate_mouse_up(menu_position, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.update(|window, cx| {
            let owner = root.read(cx).0.read(cx);
            assert!(owner.menu_position.is_none());
            assert!(owner.focus.is_focused(window));
            assert_eq!(owner.selection.selected_text(&owner.value), Some(expected));
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some(expected)
            );
            #[cfg(target_os = "linux")]
            assert_eq!(
                cx.read_from_primary().unwrap().text().as_deref(),
                Some(expected)
            );
        });
        cx.simulate_click(
            gpui::point(gpui::px(600.), gpui::px(100.)),
            Modifiers::default(),
        );
        cx.update(|_, cx| {
            let owner = root.read(cx).0.read(cx);
            assert!(owner.selection.range().is_empty());
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some(expected)
            );
        });
    }

    #[gpui::test]
    fn adr_0071_log_menu_dismissal_and_replacement_do_not_copy(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        cx.update(crate::ui::primitives::context_menu::init);
        let expected = "  café /tmp/👩‍💻.flac  ";
        for dismissal in ["escape", "outside", "replacement"] {
            let (root, cx) = cx.add_window_view(|window, cx| {
                let state = cx.new(|cx| {
                    let mut state = SelectableTextState::new(cx);
                    state.value = expected.into();
                    state
                });
                LogTest(
                    state.clone(),
                    cx.new(|cx| gpui_component::Root::new(state, window, cx)),
                )
            });
            cx.update(|window, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string("CLIPBOARD-KEEP".into()));
                root.read(cx).0.read(cx).focus.clone().focus(window, cx);
                let _ = window.draw(cx);
            });
            cx.simulate_keystrokes("ctrl-a");
            let position = gpui::point(gpui::px(35.), gpui::px(12.));
            let outside = gpui::point(gpui::px(600.), gpui::px(100.));
            cx.simulate_mouse_down(position, MouseButton::Right, Modifiers::default());
            cx.simulate_mouse_up(position, MouseButton::Right, Modifiers::default());
            cx.update(|window, cx| {
                let _ = window.draw(cx);
                let owner = root.read(cx).0.read(cx);
                assert!(owner.menu_position.is_some());
                assert_eq!(
                    owner.selection.selected_text(&owner.value),
                    Some(expected),
                    "before {dismissal}"
                );
            });
            match dismissal {
                "escape" => cx.simulate_keystrokes("escape"),
                "outside" => cx.simulate_click(outside, Modifiers::default()),
                _ => {
                    let menu_position = cx.debug_bounds("context-menu-items").unwrap().center();
                    cx.simulate_mouse_down(menu_position, MouseButton::Left, Modifiers::default());
                    cx.update(|window, cx| {
                        let _ = window.draw(cx);
                        root.read(cx).0.clone().update(cx, |state, cx| {
                            state.update_value("replacement".into(), window, cx);
                        });
                        let _ = window.draw(cx);
                    });
                    cx.simulate_mouse_up(menu_position, MouseButton::Left, Modifiers::default());
                }
            }
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            cx.update(|window, cx| {
                let owner = root.read(cx).0.read(cx);
                assert!(owner.menu_position.is_none(), "{dismissal}");
                if dismissal == "escape" {
                    assert_eq!(
                        owner.selection.selected_text(&owner.value),
                        Some(expected),
                        "after {dismissal}"
                    );
                } else {
                    assert!(owner.selection.range().is_empty(), "{dismissal}");
                }
                if dismissal != "replacement" {
                    assert!(owner.focus.is_focused(window));
                }
                assert_eq!(
                    cx.read_from_clipboard().unwrap().text().as_deref(),
                    Some("CLIPBOARD-KEEP")
                );
                #[cfg(target_os = "linux")]
                assert_eq!(
                    cx.read_from_primary().unwrap().text().as_deref(),
                    Some(expected)
                );
            });
            cx.simulate_click(outside, Modifiers::default());
            cx.update(|_, cx| {
                assert!(root.read(cx).0.read(cx).selection.range().is_empty());
            });
        }
    }

    #[gpui::test]
    fn adr_0071_log_select_all_retains_whitespace_and_clears_on_outside_selection(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let state = cx.new(|cx| {
                let mut state = SelectableTextState::new(cx);
                state.value = " \t\n".into();
                state
            });
            LogTest(
                state.clone(),
                cx.new(|cx| gpui_component::Root::new(state, window, cx)),
            )
        });
        cx.update(|window, cx| {
            let owner = root.read(cx).0.clone();
            owner.read(cx).focus.clone().focus(window, cx);
        });
        cx.simulate_keystrokes("ctrl-a ctrl-c");
        cx.update(|window, cx| {
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some(" \t\n")
            );
            WindowSelection::clear(window, cx);
        });
        cx.update(|_, cx| {
            let owner = root.read(cx).0.clone();
            assert!(owner.read(cx).selection.range().is_empty());
        });
    }
}
