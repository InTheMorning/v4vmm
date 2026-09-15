//! Linux PRIMARY integration at shared input owners (ADR 0071).

#![warn(clippy::pedantic)]

use gpui::{
    AnyElement, App, Bounds, Context, Element, ElementId, Entity, EntityInputHandler,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId,
    MouseButton, MouseDownEvent, Pixels, Window,
};
use gpui_base::input::{InputBaseState, InputMode, InputModeKind, RopeExt as _, TextareaMode};

/// Install once, before creating input states, in every application window path.
pub(crate) fn init(cx: &mut App) {
    observe_inputs::<InputMode>(cx);
    observe_inputs::<TextareaMode>(cx);
}

fn observe_inputs<M: InputModeKind>(cx: &mut App) {
    cx.observe_new::<InputBaseState<M>>(|input, _, cx| {
        let mut previous_text = input.text().clone();
        let mut previous_range = input.selected_range();
        cx.observe_self(move |input, cx| {
            let range = input.selected_range();
            if range != previous_range
                && !range.is_empty()
                && *input.text() == previous_text
                && !input.presentation().is_masked()
            {
                #[cfg(target_os = "linux")]
                cx.write_to_primary(gpui::ClipboardItem::new_string(
                    input.selected_text().to_string(),
                ));
            }
            previous_range = range;
            previous_text = input.text().clone();
        })
        .detach();
    })
    .detach();
}

/// Adds PRIMARY paste without introducing a layout box or an editing engine.
pub(crate) trait PrimarySelectionExt: IntoElement + Sized {
    fn with_primary_selection<M: InputModeKind>(
        self,
        state: &Entity<InputBaseState<M>>,
    ) -> PrimaryInput<M> {
        PrimaryInput {
            element: self.into_any_element(),
            state: state.clone(),
        }
    }
}
impl PrimarySelectionExt for gpui_component::input::Input {}
impl PrimarySelectionExt for gpui_component::input::Textarea {}

pub(crate) struct PrimaryInput<M: InputModeKind> {
    element: AnyElement,
    state: Entity<InputBaseState<M>>,
}

impl<M: InputModeKind> IntoElement for PrimaryInput<M> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl<M: InputModeKind> Element for PrimaryInput<M> {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.element.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Hitbox {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        self.element.prepaint(window, cx);
        hitbox
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut (),
        hitbox: &mut Hitbox,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.paint(window, cx);
        let state = self.state.clone();
        let hitbox = hitbox.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase.bubble() && event.button == MouseButton::Middle && hitbox.is_hovered(window) {
                state.update(cx, |input, cx| {
                    #[cfg(target_os = "linux")]
                    if let Some(offset) = primary_offset_at(input, event.position, window, cx) {
                        if paste_primary_at(input, offset, window, cx) {
                            cx.stop_propagation();
                        }
                    }
                });
            }
        });
    }
}

// The IME point query only hits glyphs in 0.6.1. Use public caret geometry
// for empty lines, trailing space and later visible rows; never guess an offset
// from a font width. This work runs only on a middle click (ADR 0071).
#[cfg(target_os = "linux")]
fn primary_offset_at<M: InputModeKind>(
    input: &mut InputBaseState<M>,
    position: gpui::Point<Pixels>,
    window: &mut Window,
    cx: &mut Context<InputBaseState<M>>,
) -> Option<usize> {
    // Text bounds move with scrolling; the input viewport still accepts clicks
    // on visible trailing rows (ADR 0071).
    if !input.input_bounds().contains(&position) {
        return None;
    }
    if input.text().len() == 0 {
        return Some(0);
    }
    if let Some(offset) = input.character_index_for_point(position, window, cx) {
        let byte = input.text().offset_utf16_to_offset(offset);
        if input
            .range_to_bounds(&(byte..byte))
            .is_some_and(|caret| position.y >= caret.top() && position.y < caret.bottom())
        {
            return Some(offset);
        }
    }
    let rows = input.visible_row_range()?;
    let start = input.text().line_start_offset(rows.start);
    let end = input.text().line_end_offset(rows.end.saturating_sub(1));
    let mut offset = start;
    let mut closest: Option<(f32, f32, usize)> = None;
    for width in input
        .text()
        .slice(start..end)
        .chars()
        .map(char::len_utf8)
        .chain(std::iter::once(0))
    {
        if let Some(caret) = input.range_to_bounds(&(offset..offset)) {
            let dy = f32::from(position.y - caret.center().y).abs();
            let dx = f32::from(position.x - caret.left()).abs();
            if closest.is_none_or(|(old_y, old_x, _)| (dy, dx) < (old_y, old_x)) {
                closest = Some((dy, dx, offset));
            }
        }
        offset += width;
    }
    closest.map(|(_, _, offset)| input.text().offset_to_offset_utf16(offset))
}

#[cfg(target_os = "linux")]
fn paste_primary_at<M: InputModeKind>(
    input: &mut InputBaseState<M>,
    utf16_offset: usize,
    window: &mut Window,
    cx: &mut Context<InputBaseState<M>>,
) -> bool {
    if !input.is_editable() {
        return false;
    }
    let Some(text) = cx
        .read_from_primary()
        .and_then(|item| item.text())
        .filter(|text| !text.is_empty())
    else {
        return false;
    };
    let offset = input.text().offset_utf16_to_offset(utf16_offset);
    input.unmark_text(window, cx);
    input.set_selected_range(offset..offset, cx);
    input.focus(window, cx);
    input.insert(text, window, cx);
    true
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{ClipboardItem, TestAppContext};
    use gpui_base::input::{InputEvent, InputState, TextareaState};

    use super::*;

    #[gpui::test]
    fn adr_0071_primary_tracks_selection_without_claiming_edits_or_masked_text(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_component::init(cx);
            init(cx);
        });
        let (input, cx) = cx.add_window_view(|window, cx| {
            InputState::new(window, cx).default_value("café /tmp/🦀")
        });
        cx.update(|_, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string("clipboard".into()));
            cx.write_to_primary(ClipboardItem::new_string("primary-sentinel".into()));
            input.update(cx, |input, cx| input.set_selected_range(0..5, cx));
        });
        cx.update(|_, cx| {
            assert_eq!(
                cx.read_from_primary().unwrap().text().as_deref(),
                Some("café")
            );
        });
        cx.update(|window, cx| {
            input.update(cx, |input, cx| input.insert("!", window, cx));
        });
        cx.update(|window, cx| {
            assert_eq!(
                cx.read_from_primary().unwrap().text().as_deref(),
                Some("café")
            );
            input.update(cx, |input, cx| {
                input.set_masked(true, window, cx);
                input.set_selected_range(0..2, cx);
            });
        });
        cx.update(|_, cx| {
            assert_eq!(
                cx.read_from_primary().unwrap().text().as_deref(),
                Some("café")
            );
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some("clipboard")
            );
        });
    }

    #[gpui::test]
    fn adr_0071_primary_paste_uses_unicode_hit_offset_and_atomic_undo(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            init(cx);
        });
        let (input, cx) =
            cx.add_window_view(|window, cx| InputState::new(window, cx).default_value("a🦀b"));
        let changes = Rc::new(Cell::new(0));
        cx.update(|_, cx| {
            let changes = changes.clone();
            cx.subscribe(&input, move |_, event: &InputEvent, _| {
                if matches!(event, InputEvent::Change) {
                    changes.set(changes.get() + 1);
                }
            })
            .detach();
        });
        cx.update(|window, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string("clipboard".into()));
            input.update(cx, |input, cx| {
                input.focus(window, cx);
                input.set_selected_range(6..6, cx);
                input.replace_text_in_range(None, "!", window, cx);
            });
        });
        cx.update(|_, cx| input.update(cx, |input, cx| input.set_selected_range(0..1, cx)));
        cx.update(|window, cx| {
            cx.write_to_primary(ClipboardItem::new_string("\ncafé\r\n".into()));
            input.update(cx, |input, cx| {
                assert!(paste_primary_at(input, 3, window, cx));
            });
        });
        cx.update(|_, cx| {
            assert_eq!(input.read(cx).value().as_ref(), "a🦀caféb!");
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some("clipboard")
            );
            assert_eq!(changes.get(), 2);
        });
        cx.dispatch_action(gpui_base::input::Undo);
        cx.update(|_, cx| assert_eq!(input.read(cx).value().as_ref(), "a🦀b!"));
        cx.dispatch_action(gpui_base::input::Redo);
        cx.update(|_, cx| assert_eq!(input.read(cx).value().as_ref(), "a🦀caféb!"));
        cx.dispatch_action(gpui_base::input::Undo);
        cx.dispatch_action(gpui_base::input::Undo);
        cx.update(|_, cx| assert_eq!(input.read(cx).value().as_ref(), "a🦀b"));
    }

    #[gpui::test]
    fn adr_0071_primary_paste_respects_editability_empty_selection_and_multiline(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_component::init(cx);
            init(cx);
        });
        let (input, cx) = cx.add_window_view(|window, cx| {
            TextareaState::new(window, cx).default_value("first\nlast")
        });
        cx.update(|window, cx| {
            cx.write_to_primary(ClipboardItem::new_string("\ncafé\n".into()));
            input.update(cx, |input, cx| {
                input.set_readonly(true, cx);
                assert!(!paste_primary_at(input, 5, window, cx));
                input.set_readonly(false, cx);
                input.set_disabled(true, cx);
                assert!(!paste_primary_at(input, 5, window, cx));
                input.set_disabled(false, cx);
                assert!(paste_primary_at(input, 5, window, cx));
                assert_eq!(input.value().as_ref(), "first\ncafé\n\nlast");
                let range = input.selected_range();
                cx.write_to_primary(ClipboardItem::new_string(String::new()));
                assert!(!paste_primary_at(input, 0, window, cx));
                assert_eq!(input.selected_range(), range);
            });
        });
    }

    struct InputTest(Entity<InputState>);
    impl gpui::Render for InputTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            use gpui::{ParentElement as _, Styled as _};
            gpui::div().size_full().child(
                gpui::div().w(gpui::px(400.)).child(
                    gpui_component::input::Input::new(&self.0).with_primary_selection(&self.0),
                ),
            )
        }
    }

    #[gpui::test]
    fn adr_0071_middle_click_reaches_empty_fields_and_line_ends(cx: &mut TestAppContext) {
        use gpui::AppContext as _;
        cx.update(|cx| {
            gpui_component::init(cx);
            init(cx);
        });
        let (root, cx) =
            cx.add_window_view(|window, cx| InputTest(cx.new(|cx| InputState::new(window, cx))));
        for (before, after) in [("", "P"), ("word", "wordP")] {
            cx.update(|window, cx| {
                let input = root.read(cx).0.clone();
                input.update(cx, |input, cx| input.set_value(before, window, cx));
                cx.write_to_primary(ClipboardItem::new_string("P".into()));
                let _ = window.draw(cx);
            });
            cx.simulate_mouse_down(
                gpui::point(gpui::px(280.), gpui::px(15.)),
                MouseButton::Middle,
                gpui::Modifiers::default(),
            );
            cx.simulate_mouse_up(
                gpui::point(gpui::px(280.), gpui::px(15.)),
                MouseButton::Middle,
                gpui::Modifiers::default(),
            );
            cx.update(|_, cx| assert_eq!(root.read(cx).0.read(cx).value().as_ref(), after));
        }
    }

    struct TextareaTest(Entity<TextareaState>);
    impl gpui::Render for TextareaTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            use gpui::{ParentElement as _, Styled as _};
            gpui::div().size_full().child(
                gpui::div().w(gpui::px(220.)).h(gpui::px(200.)).child(
                    gpui_component::input::Textarea::new(&self.0)
                        .h(gpui::px(200.))
                        .with_primary_selection(&self.0),
                ),
            )
        }
    }

    #[gpui::test]
    fn adr_0071_middle_click_uses_later_and_wrapped_unicode_rows(cx: &mut TestAppContext) {
        use gpui::AppContext as _;
        cx.update(|cx| {
            gpui_component::init(cx);
            init(cx);
        });
        let text = "first\ncafé 🦀\n\n/tmp/music_dir/café/long/path/with/more/words.flac";
        let (root, cx) = cx.add_window_view(|window, cx| {
            TextareaTest(cx.new(|cx| TextareaState::new(window, cx)))
        });
        for offset in [
            "first\ncafé".len(),
            "first\ncafé 🦀\n".len(),
            text.find("with").unwrap(),
        ] {
            let position = cx.update(|window, cx| {
                let input = root.read(cx).0.clone();
                input.update(cx, |input, cx| input.set_value(text, window, cx));
                cx.write_to_primary(ClipboardItem::new_string("P".into()));
                let _ = window.draw(cx);
                input
                    .read(cx)
                    .range_to_bounds(&(offset..offset))
                    .unwrap()
                    .center()
            });
            cx.simulate_mouse_down(position, MouseButton::Middle, gpui::Modifiers::default());
            cx.simulate_mouse_up(position, MouseButton::Middle, gpui::Modifiers::default());
            cx.update(|_, cx| {
                assert_eq!(
                    root.read(cx).0.read(cx).value().as_ref(),
                    format!("{}P{}", &text[..offset], &text[offset..])
                );
            });
        }
    }

    #[gpui::test]
    fn adr_0071_middle_click_reaches_trailing_blank_rows_after_scrolling(cx: &mut TestAppContext) {
        use gpui::AppContext as _;

        cx.update(|cx| {
            gpui_component::init(cx);
            init(cx);
        });
        let (root, cx) = cx.add_window_view(|window, cx| {
            TextareaTest(cx.new(|cx| TextareaState::new(window, cx)))
        });
        let primary = "PRIMARY-RECOVERY";
        for lines in [1, 20] {
            let prefix = "café 👩‍💻\n".repeat(lines);
            let text = format!("{prefix}\n\n");
            for offset in prefix.len()..=text.len() {
                cx.update(|window, cx| {
                    let input = root.read(cx).0.clone();
                    input.update(cx, |input, cx| {
                        input.set_value(&text, window, cx);
                        input.focus(window, cx);
                    });
                    window.draw(cx).clear(cx);
                });
                cx.simulate_keystrokes("ctrl-end");
                let position = cx.update(|window, cx| {
                    window.draw(cx).clear(cx);
                    let input = root.read(cx).0.read(cx);
                    let caret = input.range_to_bounds(&(offset..offset)).unwrap();
                    let position = gpui::point(caret.left() + gpui::px(40.), caret.center().y);
                    assert!(input.input_bounds().contains(&position));
                    cx.write_to_clipboard(ClipboardItem::new_string("RECOVERY-KEEP".into()));
                    cx.write_to_primary(ClipboardItem::new_string(primary.into()));
                    position
                });
                cx.simulate_mouse_down(position, MouseButton::Middle, gpui::Modifiers::default());
                cx.simulate_mouse_up(position, MouseButton::Middle, gpui::Modifiers::default());
                let expected = format!("{}{primary}{}", &text[..offset], &text[offset..]);
                cx.update(|_, cx| {
                    assert_eq!(
                        root.read(cx).0.read(cx).value().as_ref(),
                        expected,
                        "middle-click on blank row at byte {offset}, after {lines} text lines"
                    );
                    assert_eq!(
                        cx.read_from_clipboard().unwrap().text().as_deref(),
                        Some("RECOVERY-KEEP")
                    );
                    assert_eq!(
                        cx.read_from_primary().unwrap().text().as_deref(),
                        Some(primary)
                    );
                });
                for (key, value) in [
                    ("ctrl-z", text.as_str()),
                    ("ctrl-y", expected.as_str()),
                    ("ctrl-z", text.as_str()),
                ] {
                    cx.simulate_keystrokes(key);
                    cx.update(|_, cx| assert_eq!(root.read(cx).0.read(cx).value().as_ref(), value));
                }
            }
        }
    }

    #[gpui::test]
    fn adr_0071_input_clicks_publish_words_and_whole_values(cx: &mut TestAppContext) {
        use gpui::AppContext as _;
        cx.update(|cx| {
            gpui_component::init(cx);
            init(cx);
        });
        let text = "/tmp/café/cafe\u{301}/music_dir/file.flac";
        let (root, cx) = cx.add_window_view(|window, cx| {
            InputTest(cx.new(|cx| InputState::new(window, cx).default_value(text)))
        });
        for (word, count, expected) in [
            ("café", 2, "café"),
            ("cafe\u{301}", 2, "cafe\u{301}"),
            ("music_dir", 2, "music_dir"),
            ("café", 3, text),
        ] {
            let position = cx.update(|window, cx| {
                let _ = window.draw(cx);
                let offset = text.find(word).unwrap();
                root.read(cx)
                    .0
                    .read(cx)
                    .range_to_bounds(&(offset..offset))
                    .unwrap()
                    .center()
            });
            cx.simulate_event(MouseDownEvent {
                position,
                button: MouseButton::Left,
                click_count: count,
                first_mouse: false,
                modifiers: gpui::Modifiers::default(),
            });
            cx.simulate_mouse_up(position, MouseButton::Left, gpui::Modifiers::default());
            cx.update(|_, cx| {
                assert_eq!(
                    root.read(cx).0.read(cx).selected_text().to_string(),
                    expected
                );
                assert_eq!(
                    cx.read_from_primary().unwrap().text().as_deref(),
                    Some(expected)
                );
            });
        }
    }

    #[gpui::test]
    fn adr_0071_validation_and_clipboard_replacement_share_the_upstream_edit_path(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_component::init(cx);
            init(cx);
        });
        let (input, cx) = cx.add_window_view(|window, cx| {
            InputState::new(window, cx)
                .validate(|text, _| !text.contains('!'))
                .default_value("keep")
        });
        cx.update(|window, cx| {
            cx.write_to_primary(ClipboardItem::new_string("!".into()));
            input.update(cx, |input, cx| {
                assert!(paste_primary_at(input, 2, window, cx));
                assert_eq!(input.value().as_ref(), "keep");
                input.set_selected_range(1..3, cx);
            });
            cx.write_to_clipboard(ClipboardItem::new_string("C".into()));
        });
        cx.dispatch_action(gpui_base::input::Paste);
        cx.update(|_, cx| assert_eq!(input.read(cx).value().as_ref(), "kCp"));
        cx.dispatch_action(gpui_base::input::Undo);
        cx.update(|_, cx| assert_eq!(input.read(cx).value().as_ref(), "keep"));
    }
}
