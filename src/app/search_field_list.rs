//! The field list view of the toolbar search field (ADR 0087 Decision 3).
//!
//! `SearchFieldListView` reads the text and the selection of the search
//! field, keeps the open state of the list, and composes the shared
//! composite. The keyboard focus stays in the search field. The field
//! wrapper captures the list keys before the input gets them, and stops a
//! key only when the list used it.
//!
//! The view is its own entity. The input notifies on each caret blink, so
//! the view compares the text and the selection, and draws again only on a
//! change.
//!
//! In an `artist:` value, the view also keeps the state of the one
//! `MusicIndex` name request (ADR 0087 Decision 3, packet 005). After each
//! change, a new key arms the pause of `Motion::SearchPause` with a new run
//! generation. When the pause ends, the view emits [`FieldValueRequest`].
//! `TopApp` sends the request through `present_command`, so no code here
//! calls `cx.spawn` (ADR 0040). An answer of an older generation or of
//! another key changes nothing.

#![warn(clippy::pedantic)]

use std::ops::Range;

use gpui::Div;
use gpui::{
    prelude::*, App, Context, Empty, Entity, EventEmitter, MouseButton, ScrollHandle, SharedString,
    Stateful, Subscription, WeakEntity, Window,
};
use gpui_component::input::{
    Enter, Escape, IndentInline, InputEvent, InputState, MoveDown, MoveUp,
};

use crate::api::ArtistNameRow;
use crate::application::queries::name_lists::FetchArtistNameValues;
use crate::application::CommandContext;
use crate::library::LibraryApp;
use crate::presentation::delayed_presenter::present_after;
use crate::presentation::present_command;
use crate::ui::composites::search_field_list::SearchFieldList;
use crate::ui::tokens::Motion;
use crate::view_models::live_run::{LiveRunGate, RunGeneration};
use crate::view_models::search_field_list::{
    FieldListEdit, FieldListEvent, FieldListKey, FieldListKeyOutcome, FieldListState, FieldListVm,
};
use crate::view_models::search_field_values::{
    artist_value_request_key, FieldValueRows, IndexArtistValues,
};

use super::TopApp;

/// The request for the `MusicIndex` names of one `artist:` key. The view
/// emits it when the pause of that key ends.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FieldValueRequest {
    /// The sort key of the typed value.
    pub(crate) key: String,
    /// The run generation of the request.
    pub(crate) generation: RunGeneration,
}

/// The field list under the toolbar search field.
pub(crate) struct SearchFieldListView {
    input: Entity<InputState>,
    /// The Library. The view reads its field values at each draw.
    library: Entity<LibraryApp>,
    state: FieldListState,
    /// The state of the `MusicIndex` name request.
    index_values: IndexArtistValues,
    /// The pause and the run generation of the name request.
    gate: LiveRunGate,
    scroll: ScrollHandle,
    last_text: SharedString,
    last_selection: Range<usize>,
    /// The text at the last Enter that ran the search, until the next
    /// input event.
    submitted_text: Option<SharedString>,
    _subscriptions: [Subscription; 3],
}

impl EventEmitter<FieldValueRequest> for SearchFieldListView {}

impl SearchFieldListView {
    /// Makes the view for the search field `input`. `library` gives the
    /// Library values of the fields.
    pub(crate) fn new(
        input: Entity<InputState>,
        library: Entity<LibraryApp>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let events = cx.subscribe_in(&input, window, |this, input, event: &InputEvent, _, cx| {
            let submitted_text = this.submitted_text.take();
            let event = match event {
                InputEvent::Focus => FieldListEvent::Focus,
                InputEvent::Blur => FieldListEvent::Blur,
                InputEvent::Change => {
                    // After Enter runs the search, the input can send a
                    // `Change` with the same text. That is not typing, so
                    // the list stays closed.
                    if submitted_text.is_some_and(|text| text == input.read(cx).value()) {
                        return;
                    }
                    FieldListEvent::Change
                }
                InputEvent::PressEnter { .. } => {
                    this.submitted_text = Some(input.read(cx).value());
                    FieldListEvent::Submit
                }
            };
            this.state.apply(event);
            this.refresh_index_request(cx);
            cx.notify();
        });
        let observer = cx.observe(&input, |this, input, cx| {
            let (text, selection) = read_input(&input, cx);
            if text != this.last_text || selection != this.last_selection {
                this.last_text = text;
                this.last_selection = selection;
                this.state.apply(FieldListEvent::CursorMoved);
                this.refresh_index_request(cx);
                cx.notify();
            }
        });
        // A change of the Library values updates an open list in place.
        let library_observer = cx.observe(&library, |_, _, cx| cx.notify());
        let (last_text, last_selection) = read_input(&input, cx);
        Self {
            input,
            library,
            state: FieldListState::default(),
            index_values: IndexArtistValues::NotRequested,
            gate: LiveRunGate::default(),
            scroll: ScrollHandle::new(),
            last_text,
            last_selection,
            submitted_text: None,
            _subscriptions: [events, observer, library_observer],
        }
    }

    /// Updates the `MusicIndex` name request for the text and the cursor
    /// (packet 005 "The request").
    ///
    /// With no key, no request applies, and a new generation drops the
    /// answer of a running request. The same key changes nothing. A new
    /// key starts a new generation and the pause. When the pause ends with
    /// the same generation, the view emits [`FieldValueRequest`].
    fn refresh_index_request(&mut self, cx: &mut Context<Self>) {
        let (text, selection) = read_input(&self.input, cx);
        match artist_value_request_key(&text, selection.start) {
            None => {
                self.index_values = IndexArtistValues::NotRequested;
                self.gate.next_run();
            }
            Some(key) if self.index_values.key() == Some(key.as_str()) => {}
            Some(key) => {
                let generation = self.gate.next_run();
                self.index_values = IndexArtistValues::Loading { key: key.clone() };
                let ticket = self.gate.text_changed();
                present_after(Motion::SearchPause.duration(), cx, move |this, cx| {
                    if this.gate.pause_ended(ticket) && this.gate.is_current(generation) {
                        cx.emit(FieldValueRequest { key, generation });
                    }
                });
            }
        }
    }

    /// Puts the answer rows of the request for `key` into the list. An
    /// answer of an older generation or of another key changes nothing.
    pub(crate) fn apply_index_names(
        &mut self,
        generation: RunGeneration,
        key: &str,
        rows: &[ArtistNameRow],
        cx: &mut Context<Self>,
    ) {
        if self.answer_is_current(generation, key) {
            self.index_values = IndexArtistValues::loaded(key.to_owned(), rows);
        }
        cx.notify();
    }

    /// Shows the error row for the failed request for `key`. A failure of
    /// an older generation or of another key changes nothing. The error
    /// text does not go into the list, because it can hold endpoint
    /// details.
    pub(crate) fn apply_index_failure(
        &mut self,
        generation: RunGeneration,
        key: &str,
        cx: &mut Context<Self>,
    ) {
        if self.answer_is_current(generation, key) {
            self.index_values = IndexArtistValues::Failed {
                key: key.to_owned(),
            };
        }
        cx.notify();
    }

    /// `true` when `generation` is current and `key` is the key of the
    /// request state (packet 005 decision D10).
    fn answer_is_current(&self, generation: RunGeneration, key: &str) -> bool {
        self.gate.is_current(generation) && self.index_values.key() == Some(key)
    }

    /// Acts on one list key. Returns `true` when the list used the key.
    pub(crate) fn handle_key(
        &mut self,
        key: FieldListKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(vm) = self.view_model(cx) else {
            return false;
        };
        match vm.key(key) {
            FieldListKeyOutcome::Select(index) => {
                self.state.select(Some(index));
                self.scroll.scroll_to_item(index);
                cx.notify();
                true
            }
            FieldListKeyOutcome::Apply(edit) => {
                self.apply_edit(&edit, window, cx);
                true
            }
            FieldListKeyOutcome::Close => {
                self.state.apply(FieldListEvent::Escape);
                cx.notify();
                true
            }
            FieldListKeyOutcome::Pass => false,
        }
    }

    /// Puts the edit of the row `index` into the text. An unavailable row
    /// does nothing.
    pub(crate) fn click_row(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(vm) = self.view_model(cx) else {
            return;
        };
        let Some(row) = vm.rows.get(index) else {
            return;
        };
        if row.availability.is_available() {
            self.apply_edit(&row.edit, window, cx);
        }
    }

    /// Sends one open state event, for example a click in the field.
    fn send(&mut self, event: FieldListEvent, cx: &mut Context<Self>) {
        self.state.apply(event);
        cx.notify();
    }

    fn view_model(&self, cx: &App) -> Option<FieldListVm> {
        let (text, selection) = read_input(&self.input, cx);
        let library = self.library.read(cx).search_field_values();
        let values = FieldValueRows {
            library: &library,
            index: &self.index_values,
        };
        FieldListVm::build(&text, selection, &self.state, &values)
    }

    fn apply_edit(&mut self, edit: &FieldListEdit, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| {
            input.set_selected_range(edit.range.clone(), cx);
            input.replace(edit.text.clone(), window, cx);
            input.set_selected_range(edit.cursor..edit.cursor, cx);
            input.focus(window, cx);
        });
        self.state.apply(FieldListEvent::Inserted);
        cx.notify();
    }
}

impl Render for SearchFieldListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(vm) = self.view_model(cx) else {
            return Empty.into_any_element();
        };
        let view = cx.entity().downgrade();
        SearchFieldList::new(vm, self.scroll.clone())
            .on_row(move |index, window, cx| {
                // The view is gone only when the window closes.
                let _ = view.update(cx, |view, cx| view.click_row(index, window, cx));
            })
            .into_any_element()
    }
}

impl TopApp {
    /// Sends the `MusicIndex` name request of the field list through the
    /// command runner, with the current endpoint (packet 005 decision D15).
    /// The answer goes back to the field list view with its generation and
    /// its key.
    pub(crate) fn dispatch_field_value_request(
        &mut self,
        request: &FieldValueRequest,
        cx: &mut Context<Self>,
    ) {
        let command =
            FetchArtistNameValues::new(self.musicindex_endpoint.clone(), request.key.clone());
        let generation = request.generation;
        let names_list = self.search_field_list.downgrade();
        let failure_list = names_list.clone();
        let names_key = request.key.clone();
        let failure_key = request.key.clone();
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |_, rows, cx| {
                // The view is gone only when the window closes.
                let _ = names_list.update(cx, |view, cx| {
                    view.apply_index_names(generation, &names_key, &rows, cx);
                });
            },
            move |_, _error, cx| {
                // The view is gone only when the window closes.
                let _ = failure_list.update(cx, |view, cx| {
                    view.apply_index_failure(generation, &failure_key, cx);
                });
            },
        );
    }
}

/// Reads the text and the selection of the search field.
fn read_input(input: &Entity<InputState>, cx: &App) -> (SharedString, Range<usize>) {
    let input = input.read(cx);
    (input.value(), input.selected_range())
}

/// Adds the list keys and the click event to the field wrapper.
///
/// The listeners run in the capture phase, before the input. A listener
/// stops a key only when the list used it. Otherwise the input gets the key.
pub(crate) fn install_field_list_keys(
    wrapper: Stateful<Div>,
    list: &Entity<SearchFieldListView>,
) -> Stateful<Div> {
    let up = list.downgrade();
    let down = list.downgrade();
    let tab = list.downgrade();
    let enter = list.downgrade();
    let escape = list.downgrade();
    let click = list.downgrade();
    wrapper
        .capture_action(move |_: &MoveUp, window, cx| {
            capture_key(&up, FieldListKey::Up, window, cx);
        })
        .capture_action(move |_: &MoveDown, window, cx| {
            capture_key(&down, FieldListKey::Down, window, cx);
        })
        .capture_action(move |_: &IndentInline, window, cx| {
            capture_key(&tab, FieldListKey::Tab, window, cx);
        })
        .capture_action(move |action: &Enter, window, cx| {
            if !action.secondary && !action.shift {
                capture_key(&enter, FieldListKey::Enter, window, cx);
            }
        })
        .capture_action(move |_: &Escape, window, cx| {
            capture_key(&escape, FieldListKey::Escape, window, cx);
        })
        .capture_any_mouse_down(move |event, _, cx| {
            if event.button == MouseButton::Left {
                // The view is gone only when the window closes.
                let _ = click.update(cx, |view, cx| view.send(FieldListEvent::Click, cx));
            }
        })
}

/// Gives one key to the list, and stops the key when the list used it.
fn capture_key(
    list: &WeakEntity<SearchFieldListView>,
    key: FieldListKey,
    window: &mut Window,
    cx: &mut App,
) {
    let handled = list
        .update(cx, |view, cx| view.handle_key(key, window, cx))
        .unwrap_or(false);
    if handled {
        cx.stop_propagation();
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{div, Focusable as _, TestAppContext, VisualTestContext};
    use gpui_component::input::Input;

    use super::*;

    struct FieldRoot {
        input: Entity<InputState>,
        list: Entity<SearchFieldListView>,
    }

    impl Render for FieldRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            install_field_list_keys(div().id("field-wrapper").relative(), &self.list)
                .child(Input::new(&self.input))
                .child(self.list.clone())
        }
    }

    fn setup(
        cx: &mut TestAppContext,
    ) -> (
        Entity<InputState>,
        Entity<SearchFieldListView>,
        Rc<Cell<usize>>,
        &mut VisualTestContext,
    ) {
        cx.update(gpui_component::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx));
            let library = cx.new(|cx| test_library(window, cx));
            let list = cx.new(|cx| SearchFieldListView::new(input.clone(), library, window, cx));
            FieldRoot { input, list }
        });
        let (input, list) = cx.update(|_, cx| {
            let root = root.read(cx);
            (root.input.clone(), root.list.clone())
        });
        let enters = Rc::new(Cell::new(0));
        cx.update(|_, cx| {
            let enters = enters.clone();
            cx.subscribe(&input, move |_, event: &InputEvent, _| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    enters.set(enters.get() + 1);
                }
            })
            .detach();
        });
        cx.update(|window, cx| {
            window.activate_window();
            input.update(cx, |input, cx| input.focus(window, cx));
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        (input, list, enters, cx)
    }

    /// A Library over an empty database in memory, with no runtime.
    fn test_library(window: &mut Window, cx: &mut Context<LibraryApp>) -> LibraryApp {
        let conn = rusqlite::Connection::open_in_memory().expect("an in-memory database");
        crate::db::init_schema(&conn).expect("the schema");
        crate::db::migrate_schema(&conn).expect("the migrations");
        let dir = std::env::temp_dir().join("v4vmm-search-field-list-test");
        let services = crate::application::ApplicationServices::local_with_service_adapters()
            .expect("the local services");
        LibraryApp::new_with_content_view_mode(
            std::sync::Arc::new(std::sync::Mutex::new(conn)),
            crate::media::ImageCache::new(dir.join("cache")),
            "http://127.0.0.1:9".into(),
            dir,
            std::sync::Arc::new(services),
            None,
            crate::view_models::workspace::ContentViewMode::default(),
            window,
            cx,
        )
    }

    fn set_text(input: &Entity<InputState>, text: &str, cx: &mut VisualTestContext) {
        cx.update(|window, cx| {
            input.update(cx, |input, cx| {
                input.set_value(text.to_owned(), window, cx);
                input.set_selected_range(text.len()..text.len(), cx);
            });
        });
        cx.run_until_parked();
    }

    fn text_and_cursor(
        input: &Entity<InputState>,
        cx: &mut VisualTestContext,
    ) -> (String, Range<usize>) {
        cx.update(|_, cx| {
            let input = input.read(cx);
            (input.value().to_string(), input.selected_range())
        })
    }

    fn input_focused(input: &Entity<InputState>, cx: &mut VisualTestContext) -> bool {
        cx.update(|window, cx| input.read(cx).focus_handle(cx).is_focused(window))
    }

    fn list_shows(list: &Entity<SearchFieldListView>, cx: &mut VisualTestContext) -> bool {
        cx.update(|_, cx| list.read(cx).view_model(cx).is_some())
    }

    #[gpui::test]
    fn adr_0087_field_list_r87_4_17_keys_reach_the_list_before_the_input(cx: &mut TestAppContext) {
        let (input, list, enters, cx) = setup(cx);
        cx.simulate_keystrokes("a r");
        assert_eq!(text_and_cursor(&input, cx).0, "ar");
        assert!(list_shows(&list, cx));

        cx.simulate_keystrokes("down");
        assert_eq!(cx.update(|_, cx| list.read(cx).state.selected), Some(0));
        cx.simulate_keystrokes("tab");
        assert_eq!(text_and_cursor(&input, cx), ("artist:".to_owned(), 7..7));
        assert!(input_focused(&input, cx));

        cx.simulate_keystrokes("space k");
        assert_eq!(text_and_cursor(&input, cx).0, "artist: k");
        assert!(list_shows(&list, cx));
        cx.simulate_keystrokes("escape");
        assert!(!list_shows(&list, cx));
        assert_eq!(text_and_cursor(&input, cx).0, "artist: k");

        // Enter with a selected row puts the row into the text. The input
        // does not get the key, so it emits no `PressEnter`.
        cx.simulate_keystrokes("i");
        assert!(list_shows(&list, cx));
        cx.simulate_keystrokes("down enter");
        assert_eq!(
            text_and_cursor(&input, cx),
            ("artist: kind:".to_owned(), 13..13)
        );
        assert_eq!(enters.get(), 0);

        // Enter with no selected row runs the search, as before.
        assert!(list_shows(&list, cx));
        cx.simulate_keystrokes("enter");
        assert_eq!(enters.get(), 1);
        assert_eq!(text_and_cursor(&input, cx).0, "artist: kind:");
        assert!(!list_shows(&list, cx));
        assert!(input_focused(&input, cx));
    }

    #[gpui::test]
    fn adr_0087_field_list_r87_4_18_click_row_puts_the_row_into_the_text(cx: &mut TestAppContext) {
        let (input, list, _, cx) = setup(cx);
        cx.simulate_keystrokes("a r");
        cx.update(|window, cx| {
            list.update(cx, |view, cx| view.click_row(0, window, cx));
        });
        assert_eq!(text_and_cursor(&input, cx), ("artist:".to_owned(), 7..7));
        assert!(input_focused(&input, cx));

        cx.update(|window, cx| {
            input.update(cx, |input, cx| {
                input.set_value("in:index ", window, cx);
                input.set_selected_range(9..9, cx);
            });
            list.update(cx, |view, cx| view.send(FieldListEvent::Click, cx));
        });
        // Row 8 is `is:`. `in:index` makes it unavailable.
        let unavailable = cx.update(|_, cx| {
            let vm = list.read(cx).view_model(cx).expect("the list shows");
            !vm.rows[8].availability.is_available()
        });
        assert!(unavailable);
        cx.update(|window, cx| {
            list.update(cx, |view, cx| view.click_row(8, window, cx));
        });
        assert_eq!(text_and_cursor(&input, cx).0, "in:index ");
        assert!(input_focused(&input, cx));
    }

    /// R87-5-18 (ADR 0087 packet 005): an answer of an older generation
    /// changes nothing, also for the same key. An answer of the current
    /// generation gives `Loaded`.
    #[gpui::test]
    fn adr_0087_field_values_r87_5_18_old_generation_answer_is_dropped(cx: &mut TestAppContext) {
        let (input, list, _, cx) = setup(cx);
        let generation = |list: &Entity<SearchFieldListView>, cx: &mut VisualTestContext| {
            cx.update(|_, cx| list.read(cx).gate.current_run())
        };
        let index_values = |list: &Entity<SearchFieldListView>, cx: &mut VisualTestContext| {
            cx.update(|_, cx| list.read(cx).index_values.clone())
        };
        set_text(&input, "artist:ha", cx);
        let first = generation(&list, cx);
        assert_eq!(
            index_values(&list, cx),
            IndexArtistValues::Loading {
                key: "ha".to_owned()
            }
        );
        set_text(&input, "artist:hal", cx);
        set_text(&input, "artist:ha", cx);
        let current = generation(&list, cx);
        assert!(first < current);

        let rows = [ArtistNameRow {
            kind: crate::api::ArtistNameKind::Name,
            name: "Haleen".to_owned(),
            ..ArtistNameRow::default()
        }];
        cx.update(|_, cx| {
            list.update(cx, |view, cx| {
                view.apply_index_names(first, "ha", &rows, cx);
            });
        });
        assert_eq!(
            index_values(&list, cx),
            IndexArtistValues::Loading {
                key: "ha".to_owned()
            }
        );
        cx.update(|_, cx| {
            list.update(cx, |view, cx| view.apply_index_failure(first, "ha", cx));
        });
        assert!(matches!(
            index_values(&list, cx),
            IndexArtistValues::Loading { .. }
        ));
        cx.update(|_, cx| {
            list.update(cx, |view, cx| {
                view.apply_index_names(current, "hal", &rows, cx);
            });
        });
        assert!(matches!(
            index_values(&list, cx),
            IndexArtistValues::Loading { .. }
        ));
        cx.update(|_, cx| {
            list.update(cx, |view, cx| {
                view.apply_index_names(current, "ha", &rows, cx);
            });
        });
        assert_eq!(
            index_values(&list, cx),
            IndexArtistValues::Loaded {
                key: "ha".to_owned(),
                names: vec!["Haleen".to_owned()],
            }
        );

        set_text(&input, "title:x", cx);
        assert_eq!(index_values(&list, cx), IndexArtistValues::NotRequested);
    }
}
