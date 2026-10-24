# ADR 0087 Task 004: The Field List Under The Toolbar Search Field

Status: Implemented - 2026-10-23. Cases R87-4-01 to R87-4-20 pass. Mechanical checks are Green. The operator passed V2, V3 and V5 on 2026-10-23 and V4, V7 and V8 on 2026-10-24. V1 and V6 failed. V1, V6 and V9 are open, in the final walk of the run of 2026-10-23.

## Goal

Show the field list under the toolbar search field. A click in the empty field opens the list.
Each row gives one field, its values text and one example. While the curator types a field name,
the list shows the fields that start with that text. After `kind:`, `in:`, `is:` or `has:`, the
list shows the fixed values of that field.

The keyboard focus stays in the search field. Up and Down move the selected row. Tab and Enter put
the selected row into the text. Escape closes the list. Enter with no selected row runs the search,
as today. A click on a row puts it into the text.

The list shows the error of the field under the cursor. It marks the Library-only fields "Library
only". This packet shows no artist names, release titles or playlist names. Packet 005 adds them.

## Authority

- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 3: the field list, its rows,
  its keys, the focus and the error text. It also names the view model rule and the shared
  composite that does not use the popover primitive.
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 2: the fields, the
  Library-only fields, the "Library only" mark and the conflict of `in:index` with a Library-only
  field.
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) invariant "One parser reads the query
  text. No other code reads field syntax."
- [Packet 001](archive/adr-0087-task-001-query-parser.md): `QUERY_FIELDS`, `QueryFieldInfo`,
  `fields_with_prefix`, `cursor_term`, `CursorTerm`, `CursorPartial`, `QueryFieldName`,
  `fixed_values`, `values_with_prefix`, `FixedValueInfo`, `SearchQuery`, `QueryScope` and
  `FieldValueError::message`. Packet 001 is not implemented. Each name of packet 001 in this
  packet comes from its design.
- [ADR 0063](../adr/0063-show-dashboard-layout.md): column text does not call `truncate()`.
- The [ADR 0087 phase plan](../plans/adr-0087-search-fields-phase-plan.md), packet 004.
- [AGENTS.md](../../AGENTS.md): renderer portability, token discipline, typed action state,
  accessibility, "A guard names its class and its ADR" and "An acceptance criterion states how it
  is checked".

## Recorded Facts - 2026-10-23

The orchestrator read each fact below on 2026-10-23. `$KIT` is the gpui-kit checkout
`~/.cargo/git/checkouts/gpui-kit-7335f17435243f46/5463fe4/crates`. `$REG` is
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f`.

### The toolbar field

- `src/app.rs:147` declares `global_search_input: Entity<InputState>` on `TopApp`.
- `src/app.rs:344-348` makes the input with the placeholder of `AppToolbarVm`, and subscribes
  `on_global_search_event`. `src/app.rs:604-614` reacts only to `InputEvent::PressEnter`, and calls
  `submit_global_search`.
- `src/app.rs:862-871` `apply_pending_toolbar_search` writes a restored query with `set_value`.
  `src/app.rs:873` `focus_global_search` calls `InputState::focus`.
- `src/app/tab_bar.rs:100` `render_global_search` draws the field. `src/app/tab_bar.rs:113-123`
  wraps `Input::new(&app.global_search_input)` in a `div` with the id `display.input_id` and
  `max_w(TokenSize::ColumnTall)`. `src/app/tab_bar.rs:134` gives the toolbar center
  `overflow_hidden()`.
- `src/view_models/app_toolbar.rs:22-31` `GlobalSearchDisplay` holds the ids, the placeholder and
  the labels of the field. It holds no field list data.

### The keys today

- `$KIT/base/src/input/base/state.rs:175-183` binds `escape`, `up`, `down` and `tab` to `Escape`,
  `MoveUp`, `MoveDown` and `IndentInline` in the context `Input`.
- `$KIT/base/src/input/base/state.rs:4162-4175` gives the input element the key context `Input`,
  and registers `enter` and `escape` on it.
- `$KIT/base/src/input/base/state.rs:4180-4199` registers `indent_inline`, `up` and `down` only
  for a multi-line input. The toolbar field is single-line, so Up, Down and Tab do nothing in it
  today. `$KIT/base/src/input/editor/indent.rs:253-256` propagates Tab in a single-line input.
- `$KIT/base/src/input/base/state.rs:1988-1998`: in a single-line input, Enter propagates and
  emits `InputEvent::PressEnter`.
- `$KIT/base/src/input/base/state.rs:2007-2034`: Escape clears the text only with
  `clean_on_escape`. Else, it propagates.
- `src/app/keyboard.rs:14` binds the pane keys in the context `ActivePane && !Input`.
  `src/app/keyboard.rs:163-178` gives `escape`, `up` and `down` to the active pane. Thus these keys
  do not reach the pane while the search field has the focus.
- `src/app/keyboard.rs:916-960` proves that `escape`, `up`, `down` and `enter` resolve to the
  input actions in the context `Input`.

### The input interface

- `$KIT/base/src/input/base/state.rs:122-127`: `InputEvent` has `Change`,
  `PressEnter { secondary, shift }`, `Focus` and `Blur`.
- `$KIT/base/src/input/base/state.rs:2736` `cursor()` gives the UTF-8 byte offset of the cursor.
  `:2801` `selected_range()` gives the active selection as a byte range. `:2816`
  `set_selected_range(range, cx)` sets it, on character boundaries.
- `$KIT/base/src/input/base/state.rs:1229` `cursor_position()` and `:1237`
  `set_cursor_position(...)` use a line and column `Position`. `set_cursor_position` also focuses
  the input.
- `$KIT/base/src/input/base/state.rs:894-917` `set_value` emits no `Change`. It clears the undo
  history and scrolls the text to its start.
- `$KIT/base/src/input/base/state.rs:970-984` `replace(text, window, cx)` replaces the selection
  as one undo step and puts the cursor after the new text. `:3603-3605` then emits `Change`.
- `$KIT/base/src/input/base/state.rs:1252` `focus` focuses the input and starts the caret blink.
- `$KIT/base/src/input/base/state.rs:660` notifies the input entity on each caret blink.
- `$KIT/base/src/input/base/state.rs:3159-3184` emits `Focus` when the input gets the focus and
  `Blur` when it loses the focus.

### The overlays that exist

- `src/ui/primitives/popover.rs:1-14` wraps `gpui_component::popover::Popover`. Its text names a
  focus trap. ADR 0087 Decision 3 rejects it for the field list.
- `src/ui/primitives/context_menu.rs:212` gives the pointer menu `track_focus`.
  `src/ui/primitives/context_menu.rs:241-253` draws it with `deferred` and `anchored`, over one
  full-window layer with `occlude()`.
- `Cargo.toml:33` uses `gpui-component` 0.6.1 from crates.io. `Cargo.toml:63` patches only
  `gpui-base` to the git checkout.
- `$REG/gpui-component-0.6.1/src/input/overlay.rs:134` gives `InputMode` an empty `OverlayMode`
  implementation. Only `EditorMode` builds the completion menu (`:137-196`). A
  `CompletionProvider` thus shows no menu for a single-line input.
- `$REG/gpui-component-0.6.1/src/input/popovers/completion_menu.rs:390-428` draws the completion
  menu with `deferred`, an absolute position and `on_mouse_down_out`. The menu has no focus handle.
  The focus stays in the editor. Its keys come through the `pub(super)` field
  `overlay_action_handler` (`$KIT/base/src/input/base/state.rs:433`).

### GPUI

- `$REG/gpui-pre-0.3.1/src/elements/div.rs:413-427` `capture_action` runs its listener in the
  capture phase only.
- `$REG/gpui-pre-0.3.1/src/window.rs:6094-6113` runs the capture phase from the root to the
  focused element. It stops when a listener calls `stop_propagation`
  (`$REG/gpui-pre-0.3.1/src/app.rs:2281`). The bubble phase then does not run.
- `$REG/gpui-pre-0.3.1/src/elements/deferred.rs:65` draws a deferred child with no content mask.
  An `overflow_hidden()` parent thus does not clip it.
- `$REG/gpui-pre-0.3.1/src/elements/anchored.rs:27-35` places an anchored element at its own
  layout position when it has no `position`. `:74-76` `snap_to_window_with_margin` keeps it in the
  window.
- `$REG/gpui-pre-0.3.1/src/elements/div.rs:146` `capture_any_mouse_down` and `:4192`
  `ScrollHandle::scroll_to_item` exist.

### Tokens and tests

- `src/ui/tokens.rs:837-900` gives `Size::RowMd` (36 px), `Size::ColumnRegular` (320 px) and
  `Size::ColumnTall` (480 px). `src/ui/tokens.rs:383-397` gives `Spacing`. `src/ui/tokens.rs:603-608`
  gives `Weight`.
- `src/ui/primitives/surface.rs:22-24` gives `SurfaceElevation::Raised` for popovers and floating
  menus.
- `src/ui/primitives/status_badge.rs:32` `render_row_badge` draws the neutral row badge, with no
  `truncate()`.
- `tests/architecture_tests.rs:14584` `adr_0063_column_text_does_not_truncate` reads three Show
  files. `tests/architecture_tests.rs:10574`, `:10593`, `:10612`, `:10665` and `:10891` hold
  `collect_rust_files`, `production_source`, `code_only`, `string_literals` and `read_source`.
- `src/ui/primitives/primary_selection.rs:214-222` builds an `InputState` in a
  `gpui::TestAppContext` test after `gpui_component::init`. `:458` sends keys with
  `simulate_keystrokes`.

## Design

### The layers

| Layer | File | Owns |
|---|---|---|
| View model | `src/view_models/search_field_list.rs` | The rows, their text, labels, actions and availability. The notices. The open state and its events. The key outcomes and the text edits |
| Composite | `src/ui/composites/search_field_list.rs` | The surface, the row layout, the selected row style and the overlay placement. It holds no text of its own |
| Screen glue | `src/app/search_field_list.rs` | `SearchFieldListView`: it reads the input, keeps the open state, applies an edit, captures the keys and composes the composite |
| Toolbar | `src/app/tab_bar.rs` and `src/app.rs` | The wrapper of the field, the key capture and the list view as a child |

### The open state

`FieldListState` holds `opening: FieldListOpening`, `focused: bool` and `selected: Option<usize>`.
`FieldListState::apply(&mut self, event: FieldListEvent)` changes it:

| Event | Source | Result |
|---|---|---|
| `Focus` | `InputEvent::Focus` | `focused` is `true`. `opening` is `Requested`. `selected` is `None` |
| `Click` | A left mouse down in the field wrapper | `opening` is `Requested`. `selected` is `None` |
| `Change` | `InputEvent::Change` | When `focused`, `opening` is `Typing`. `selected` is `None` |
| `CursorMoved` | A new cursor or selection with the same text | `selected` is `None`. `opening` does not change |
| `Escape` | The Escape key while the list shows | `opening` is `Closed` |
| `Submit` | `InputEvent::PressEnter` | `opening` is `Closed` |
| `Inserted` | The view applied the edit of a row | `opening` is `Closed` |
| `Blur` | `InputEvent::Blur` | `focused` is `false`. `opening` is `Closed` |

`FieldListState::select(&mut self, index: Option<usize>)` sets the selected row.

### The rows

`FieldListVm::build(text, selection, state)` gives `Option<FieldListVm>`. It gives `None` in each
of these conditions:

- `opening` is `Closed`, or `focused` is `false`.
- `selection` is not empty.
- The list has no row and no notice.

Else, the function calls `cursor_term(text, selection.start)` and reads `CursorTerm::partial`:

- `FieldName { prefix }` gives one `Field` row for each row of `fields_with_prefix(prefix)`, in
  the order of `QUERY_FIELDS`. An empty prefix in `Typing` gives no row when the text holds a
  character that is not whitespace. Thus a space after a word does not show the full list.
- `FieldValue { field, prefix, .. }` gives one `FixedValue` row for each row of
  `field.values_with_prefix(prefix)`. A field with no fixed values gives no row.
- `Other` gives no row.

Each `FieldListRowVm` has these fields:

| Field | `Field` row | `FixedValue` row |
|---|---|---|
| `element_id: String` | `search-field-list-field-` and the token without the colon | `search-field-list-value-`, the token without the colon, `-` and the word |
| `kind: FieldListRowKind` | `Field { field }` | `FixedValue { field, word }` |
| `label: String` | `info.token` | `FixedValueInfo::word` |
| `values_text: Option<String>` | `Some(info.values_text)` | `Some(FixedValueInfo::description)` |
| `example_text: Option<String>` | `Some(info.example)` | `None` |
| `mark: Option<FieldListMark>` | `Some(LibraryOnly)` when `info.library_only` | `None` |
| `note: Option<String>` | The text of the unavailable reason, or `None` | The same |
| `a11y_label: String` | "Field `token`, `values_text`, for example `example`", with ", Library only" when marked, and ", " and the note when unavailable | "Value `word` for `token`, `description`", with ", " and the note when unavailable |
| `action: FieldListRowAction` | `InsertField(field)` | `InsertValue { field, word }` |
| `availability: FieldListRowAvailability` | `Available`, or `Unavailable(ScopeConflict)` | The same |
| `edit: FieldListEdit` | See "The edit" | See "The edit" |
| `selected: bool` | `true` for the selected row | The same |

`FieldListMark::LibraryOnly.label()` gives "Library only".

The availability reads `SearchQuery::parse(text).scope()`:

- With `IndexOnly`, each Library-only `Field` row and each `FixedValue` row of `is:` and `has:`
  is `Unavailable(ScopeConflict)`. The note is "Not available with in:index".
- With `LibraryOnly`, the `FixedValue` row `index` of `in:` is `Unavailable(ScopeConflict)`. The
  note is "Not available with a Library-only field or in:library".
- Each other row is `Available`.

### The notices

`FieldListVm::notices: Vec<FieldListNoticeVm>`. Each notice has `kind: FieldListNoticeKind`,
`text: String` and `a11y_label: String`. A notice is not a row. Up and Down do not select it.

- `FieldError`: the term of `SearchQuery::terms()` whose span holds the cursor or ends at the
  cursor is a field with `FieldValueState::Invalid(error)`. The text is `error.message()`.
- `ScopeConflict`: `scope()` is `QueryScope::Conflict`. The text is "This query asks for the
  Library only and for MusicIndex only. It shows no rows."

The `FieldError` notice comes first.

### The edit

`FieldListEdit { range: Range<usize>, text: String, cursor: usize }` gives a byte range of the
old text, the new text for that range, and the cursor offset in the changed text.
`FieldListEdit::apply(&self, text: &str) -> String` gives the changed text.

- A `Field` row replaces `CursorTerm::range.start` to the cursor with `info.token`. A `-` before
  the term stays. Text after the cursor in the same term stays, and becomes the value. `cursor` is
  after the token.
- A `FixedValue` row replaces `value_range` with the word. When `value_range.end` is
  `CursorTerm::range.end`, and no whitespace character follows it, the edit adds one space.
  `cursor` is after the word, or after the space.

`SearchFieldListView` applies an edit in this sequence:

1. `input.set_selected_range(edit.range.clone(), cx)`.
2. `input.replace(edit.text.clone(), window, cx)`.
3. `input.set_selected_range(edit.cursor..edit.cursor, cx)`.
4. `input.focus(window, cx)`.
5. `state.apply(FieldListEvent::Inserted)`.

The `Change` event of step 2 then sets `Typing`. After `kind:`, the list thus shows the values of
`kind:`. After `single `, it shows no row and closes.

### The keys

`FieldListVm::key(&self, key: FieldListKey) -> FieldListKeyOutcome`:

| Key | Condition | Outcome |
|---|---|---|
| `Down` | One or more available rows | `Select(index)`: the next available row. After the last row, the first. With no selected row, the first |
| `Up` | One or more available rows | `Select(index)`: the previous available row. Before the first row, the last. With no selected row, the last |
| `Up`, `Down` | No available row | `Pass` |
| `Tab` | A selected available row | `Apply(edit)` of that row |
| `Tab` | No selected row, one or more available rows | `Apply(edit)` of the first available row |
| `Tab` | No available row | `Pass` |
| `Enter` | A selected available row | `Apply(edit)` of that row |
| `Enter` | No selected row | `Pass` |
| `Escape` | Always | `Close` |

`SearchFieldListView::handle_key(key, window, cx) -> bool` builds the view model. With no view
model, it gives `false`. Else it acts on the outcome:

- `Select(index)` calls `state.select(Some(index))` and `scroll.scroll_to_item(index)`.
- `Apply(edit)` applies the edit.
- `Close` calls `state.apply(FieldListEvent::Escape)`.
- `Pass` does nothing.

Each outcome other than `Pass` gives `true`.

`install_field_list_keys(wrapper, &list)` adds `capture_action` listeners to the field wrapper,
for `MoveUp`, `MoveDown`, `IndentInline`, `Enter` and `Escape` of `gpui_component::input`. A
listener calls `stop_propagation` only when `handle_key` gives `true`. Otherwise the input gets
the key, as today. `Enter` with `secondary` or `shift` is always `Pass`.

`install_field_list_keys` also adds `capture_any_mouse_down`. A left button sends
`FieldListEvent::Click`.

### The view

`SearchFieldListView` is an entity in `src/app/search_field_list.rs`. It holds the input entity,
the `FieldListState`, a `ScrollHandle` and the last text and selection that it read.

- `SearchFieldListView::new(input, window, cx)` subscribes to the input events. `Focus`, `Blur`,
  `Change` and `PressEnter` become `Focus`, `Blur`, `Change` and `Submit`.
- It observes the input entity. When the text or the selection is not the same as the last read,
  it sends `CursorMoved` and calls `cx.notify()`. Else, it does nothing. A caret blink thus does
  not draw the list again.
- `render` builds `FieldListVm` and gives the composite, or an empty element.
- `click_row(index, window, cx)` applies the edit of an available row. An unavailable row does
  nothing.

`TopApp` holds `search_field_list: Entity<SearchFieldListView>`. `TopApp` makes it right after
`global_search_input`. `on_global_search_event` does not change.

### The composite

`SearchFieldList` (`src/ui/composites/search_field_list.rs`) is a `RenderOnce` element:
`SearchFieldList::new(vm, scroll).on_row(handler)`. It draws this tree:

1. A zero-size `div` with `absolute()`, `left_0()` and `bottom_0()`. The field wrapper gets
   `relative()`, so this point is the bottom left corner of the field.
2. `deferred(anchored().snap_to_window_with_margin(Spacing::SM.scaled(cx)))`.
3. A `Surface` with `SurfaceElevation::Raised`, `occlude()`, the width `Size::ColumnTall`, the
   padding `Spacing::XS` and a top margin `Spacing::XS`. With the id `FieldListVm::LIST_ID`, it
   scrolls to a maximum height of `Size::ColumnRegular`, and tracks the `ScrollHandle`.
4. The notices first. Each notice text uses `FontSize::Caption`. A `FieldError` notice uses
   `SemanticColor::WarningLabel`, and a `ScopeConflict` notice uses `SemanticColor::SecondaryLabel`.
   The text names the problem, so the color is not the only cue.
5. One row for each `FieldListRowVm`, with `min_h(Size::RowMd)`, `px(Spacing::SM)`,
   `py(Spacing::XS)` and `rounded(Radius::MD)`.
   - Line 1: the label in `FontSize::Body` with `Weight::Semibold`, and the mark through
     `render_row_badge(mark.label(), cx)`.
   - Line 2, with `flex_wrap()` and the gap `Spacing::MD`: the values text in
     `SemanticColor::SecondaryLabel`, the example in `SemanticColor::TertiaryLabel` and the note
     in `SemanticColor::SecondaryLabel`, each in `FontSize::Caption`.
   - A selected row has the fill `SemanticColor::SelectedContent` and a border in
     `SemanticColor::Focus`. The border and the fill are two cues.
   - An unavailable row draws its label in `SemanticColor::TertiaryLabel`. Its note gives the
     reason in words.
   - A left mouse down on a row calls `stop_propagation` and the `on_row` handler with the row
     index.

The composite has no focus handle and no `track_focus`. Each text element has `min_w_0()` and
wraps. No text calls `truncate()` or `Label::truncated()` (ADR 0063). The composite holds no string
literal: each id, text and label comes from the view model.

### Names For Later Packets

Module `crate::view_models::search_field_list` (`src/view_models/search_field_list.rs`):

| Name | Kind | Packets |
|---|---|---|
| `FieldListVm { rows, notices, selected, a11y_label }` | struct | 005 |
| `FieldListVm::LIST_ID: &'static str` | constant | 005 |
| `FieldListVm::build(text: &str, selection: Range<usize>, state: &FieldListState) -> Option<FieldListVm>` | function | 005 |
| `FieldListVm::key(&self, key: FieldListKey) -> FieldListKeyOutcome` | function | 005 |
| `FieldListRowVm { element_id, kind, label, values_text, example_text, mark, note, a11y_label, action, availability, edit, selected }` | struct | 005 |
| `FieldListRowKind::{Field { field }, FixedValue { field, word }}` | enum | 005 |
| `FieldListRowAction::{InsertField(QueryFieldName), InsertValue { field, word }}` | enum | 005 |
| `FieldListRowAvailability::{Available, Unavailable(FieldListRowUnavailable)}` | enum | 005 |
| `FieldListRowUnavailable::{ScopeConflict}` | enum | 005 |
| `FieldListMark::{LibraryOnly}` with `label(self) -> &'static str` | enum | 005 |
| `FieldListNoticeVm { kind, text, a11y_label }` | struct | 005 |
| `FieldListNoticeKind::{FieldError, ScopeConflict}` | enum | 005 |
| `FieldListEdit { range: Range<usize>, text: String, cursor: usize }` with `apply(&self, text: &str) -> String` | struct | 005 |
| `FieldListState { opening, focused, selected }` with `apply` and `select` | struct | 003, 005 |
| `FieldListOpening::{Closed, Requested, Typing}` | enum | 005 |
| `FieldListEvent::{Focus, Blur, Click, Change, CursorMoved, Escape, Submit, Inserted}` | enum | 003, 005 |
| `FieldListKey::{Up, Down, Tab, Enter, Escape}` | enum | 005 |
| `FieldListKeyOutcome::{Select(usize), Apply(FieldListEdit), Close, Pass}` | enum | 005 |

Module `crate::app::search_field_list` (`src/app/search_field_list.rs`):

| Name | Kind | Packets |
|---|---|---|
| `SearchFieldListView` | struct, entity | 003, 005 |
| `SearchFieldListView::new(input: Entity<InputState>, window, cx) -> Self` | function | 005 |
| `SearchFieldListView::handle_key(&mut self, key: FieldListKey, window, cx) -> bool` | function | 005 |
| `SearchFieldListView::click_row(&mut self, index: usize, window, cx)` | function | 005 |
| `install_field_list_keys(wrapper: Stateful<Div>, list: &Entity<SearchFieldListView>) -> Stateful<Div>` | function | 005 |
| `TopApp::search_field_list: Entity<SearchFieldListView>` | field | 003, 005 |

Module `crate::ui::composites::search_field_list` (`src/ui/composites/search_field_list.rs`):
`SearchFieldList` with `new(vm: FieldListVm, scroll: ScrollHandle)` and `on_row(handler)`.

Packet 005 extends the row model. It does not add a second row model:

- It adds `FieldListRowKind::Value { field, text, source: FieldValueSource }` and
  `FieldListRowAction::InsertText { field, text }` for a Library name, a release title, a playlist
  name and a `GET /v1/artist-names?from=` name. `FieldValueSource::{Library, Index}` gives the
  source.
- It adds `FieldListMark::Source(FieldValueSource)`, with a word for each source. The mark is not
  color alone.
- It adds `FieldListRowKind::Loading { field }` with `FieldListRowUnavailable::Loading`. The
  `key` function already skips an unavailable row.
- `FieldListVm::build` gets one more argument for the value rows that the runtime gave. The
  edit rule of a `FixedValue` row applies to a `Value` row. A value with whitespace gets double
  quotes from a function of packet 001 or of packet 005, never from the renderer.

## Orchestrator Decisions

- **D1. A new composite draws the list with `deferred` and `anchored`, and has no focus handle.**
  ADR 0087 Decision 3 rejects the popover primitive. The pointer menu of `context_menu.rs` also
  takes the focus (`track_focus`, line 212), and covers the window with an occluding layer. The
  completion menu of gpui-component uses the same method as this packet: `deferred`, no focus
  handle, and the focus stays in the editor (`completion_menu.rs:390-428`).
- **D2. The packet does not use `CompletionProvider`.** `InputMode` builds no completion menu
  (`overlay.rs:134`). Only `EditorMode` does.

  A single-line field with a menu thus needs a change to gpui-component. The menu also takes
  `lsp_types::CompletionItem` rows, not view model rows. Its key path (`overlay_action_handler`)
  is `pub(super)`, and its width is a raw 320 px default.
- **D3. A capture listener on the field wrapper takes the keys.** The capture phase runs before
  the input element. A listener stops the key only when the list used it.

  With the list closed, each key reaches the input as today. The single-line input has no handler
  for Up, Down or Tab (`state.rs:4180-4199`). The capture thus changes nothing for these keys
  while the list is closed.
- **D4. The list is its own entity, `SearchFieldListView`.** The input notifies on each caret
  blink (`state.rs:660`). An observer in `TopApp` then draws the full app two times each second. The
  view compares the text and the selection, and notifies only on a change.
- **D5. This packet shows the fixed values.** `fixed_values` and `values_with_prefix` are typed
  data of packet 001. They need no request and no runtime.

  Packet 001 names packet 005 as their reader. This packet reads them first, and adds no name to
  packet 001. Packet 005 then adds only the values of `artist:`, `release:` and `playlist:`.
- **D6. Up and Down wrap.** A macOS menu goes from the last item to the first item with Down.
  Down with no selected row selects the first row. Up selects the last row. The keys skip an
  unavailable row.
- **D7. Tab with no selected row inserts the first available row.** A tab character has no
  meaning in a query, and the single-line input does nothing with Tab today. Enter with no selected
  row runs the search, as ADR 0087 Decision 3 says.
- **D8. The list opens on focus, on a click and on typing.** `FocusSearch` focuses the field
  (`keyboard.rs:358`). The keyboard curator then gets the same list as the pointer curator. In
  `Typing`, an empty name prefix in a text that is not empty gives no row. Otherwise each space
  between words shows the full list.
- **D9. The list closes on Escape, on blur, on submit and after a row goes into the text.** It
  opens again on the next change of the text, on a click in the field, or on focus. The list has
  no `on_mouse_down_out`.

  A click in the field occurs inside and outside the list at the same time. That handler then
  closes the list after the click opened it. A click on a focusable item moves the focus, and the
  blur closes the list.
- **D10. An edit uses `set_selected_range` and `replace`, not `set_value`.** `set_value` clears
  the undo history and scrolls the text to its start (`state.rs:894-917`). `replace` keeps one undo
  step and emits `Change`. Then the list and the live run of packet 003 read the new text through
  the same event as typed text.
- **D11. A field row replaces the prefix only.** The text after the cursor in the same term
  becomes the value. Thus `ar|jsmovie` with Tab gives `artist:jsmovie`. A value row replaces the
  full value item, because the item is one value.
- **D12. A value row adds one space only after the last item of its term.** Then the curator can
  type the next term. After a value item that is not the last item, for example `s` in `kind:s,ep`,
  no space goes in.
- **D13. The list does not remove a `kind:` value that the term already holds.** That needs the
  item list of the term, and only the parser reads field syntax (ADR 0087 invariant). The parser
  counts a repeated kind one time (packet 001, R87-1-07).
- **D14. A row that conflicts with the scope shows, but is not available.** The curator then
  sees the field and the reason. Its note gives the reason in words. ADR 0087 Decision 2 says
  that the field list shows the conflict.
- **D15. The list shows the error of the term under the cursor only.** ADR 0087 Decision 3 names
  "the error of a field". An error of another term shows when the cursor goes to that term.
- **D16. The list has the width `Size::ColumnTall`.** That is the largest width of the field
  (`tab_bar.rs:117`). The text bounds of the input (`input_bounds`) do not include the search
  icon, so they do not give the field edge. At a narrow window, `snap_to_window_with_margin` keeps
  the list in the window, and the text wraps.
- **D17. A row reacts on mouse down.** A mouse down on a row with no focus handle does not move
  the focus. The row stops the event, and `occlude()` keeps it from the rows under the list. The
  view also focuses the input again after each edit.
- **D18. A non-empty selection closes the list.** The edit replaces a range from the cursor term.
  A selected range has no clear term.

## Required Changes

1. Add `src/view_models/search_field_list.rs` with `//!` module text that names ADR 0087
   Decision 3, and `#![warn(clippy::pedantic)]`. Add each view model name of "Names For Later
   Packets" and the rules of "Design". Declare `pub(crate) mod search_field_list;` in
   `src/view_models/mod.rs`, in alphabetical order.
2. Add `src/ui/composites/search_field_list.rs` with `SearchFieldList`, as "The composite" says.
   Declare `pub(crate) mod search_field_list;` in `src/ui/composites/mod.rs`, in alphabetical
   order.
3. Add `src/app/search_field_list.rs` with `SearchFieldListView` and `install_field_list_keys`.
   Declare `mod search_field_list;` in `src/app.rs`, in alphabetical order.
4. `src/app.rs`: add the field `search_field_list: Entity<SearchFieldListView>` to `TopApp`, and
   make it after `global_search_input` (`src/app.rs:344-348`).
5. `src/app/tab_bar.rs` `render_global_search`: give the wrapper of the input `relative()`. Pass
   the wrapper through `install_field_list_keys`. Add `app.search_field_list.clone()` as the last
   child of the wrapper.
6. Add each unit test of "Mechanical Acceptance Criteria" beside its owner.
7. Add the situational guard `adr_0087_field_list_r87_4_19_list_keeps_focus_and_reads_view_model`
   in `tests/architecture_tests.rs`. It cites ADR 0087 Decision 3. It reads
   `production_source(&code_only(&source))` of `src/ui/composites/search_field_list.rs`, and
   asserts these facts:
   - The file calls `deferred(` and `anchored(`.
   - The file holds no `Popover`, no `track_focus(`, no `.focus(`, no `.truncate()` and no
     `.truncated()`.
   - `string_literals` of the file is empty.
   - `src/app/search_field_list.rs` calls `capture_action` and holds no `Popover`.

   The failure message is:

   ```text
   ADR 0087 Decision 3 (the field list keeps the focus in the search field): <file> <violation>.
   Draw the list with deferred and anchored and no focus handle, and take each text, label and id
   from FieldListVm. Do not use the popover primitive or truncate().
   ```

8. Add one row to `docs/architecture/source-map.md`: "Music search field list: the rows, the
   keys and the overlay under the toolbar search field (ADR 0087 Decision 3, packet 004)", owners
   `src/view_models/search_field_list.rs`, `src/ui/composites/search_field_list.rs` and
   `src/app/search_field_list.rs`.

## Mechanical Acceptance Criteria

Use the prefix `adr_0087_field_list_` for each unit test. Cases R87-4-01 to R87-4-16 are in
`src/view_models/search_field_list.rs`. Cases R87-4-17 and R87-4-18 are in
`src/app/search_field_list.rs`. "Open" means a `FieldListState` with `focused` `true`.

| Case | Required proof |
|---|---|
| R87-4-01 | An empty text, `Requested`, gives 11 `Field` rows in the order of `QUERY_FIELDS`. Each row has the `token` as label, the `values_text` and the `example`. The view model has no notice and no selected row |
| R87-4-02 | `ar` at cursor 2, `Typing`, gives one row `artist:`. `-ar` at cursor 3 gives one row `artist:`. `zz` at cursor 2 gives `None` |
| R87-4-03 | `kind:` at cursor 5 gives three `FixedValue` rows `album`, `ep` and `single`, each with the description of packet 001. `kind:s` gives `single` only. `kind:album,` at cursor 11 gives three rows |
| R87-4-04 | `in:` gives two rows, `is:` gives four rows and `has:` gives two rows. `artist:` at cursor 7, `Typing`, gives `None` |
| R87-4-05 | In the R87-4-01 list, the rows `is:`, `playlist:` and `has:` have the mark `LibraryOnly`. Each other row has no mark. `FieldListMark::LibraryOnly.label()` is "Library only" |
| R87-4-06 | `after:yesterday` at cursor 15, `Typing`, gives no row and one `FieldError` notice. Its text is "after: needs a date, for example 2026-10-01". `after:yesterday ar` at cursor 18 gives the row `artist:` and no notice |
| R87-4-07 | `in:index ` at cursor 9, `Requested`, makes `is:`, `playlist:` and `has:` `Unavailable(ScopeConflict)`, with the note "Not available with in:index". `is:ready in:` at cursor 12 makes the row `index` unavailable. `in:index is:ready ` gives one `ScopeConflict` notice |
| R87-4-08 | With three available rows, `Down` from no selection gives `Select(0)`. `Down` from 2 gives `Select(0)`. `Up` from no selection gives `Select(2)`. `Up` from 0 gives `Select(2)`. `Down` skips an unavailable row. With no available row, `Up` and `Down` give `Pass` |
| R87-4-09 | `Tab` with the selected row `artist:` in `ar` gives the edit `0..2`, `artist:`, cursor 7, and the text `artist:`. In `haleen -ar` it gives `haleen -artist:` with cursor 15. In `arjsmovie` at cursor 2 it gives `artist:jsmovie` with cursor 7 |
| R87-4-10 | `Tab` on `single` in `kind:s` gives `kind:single ` with cursor 12. In `kind:s,ep` at cursor 6, it gives `kind:single,ep` with cursor 11. `Tab` with no selected row gives the edit of the first available row. `Tab` with no available row gives `Pass` |
| R87-4-11 | `Enter` with a selected available row gives the same `Apply` edit as `Tab`. `Enter` with no selected row gives `Pass` |
| R87-4-12 | `Escape` gives `Close`. After `apply(Escape)`, `build` gives `None`. After `Change`, the list shows again. After `Click`, it shows again in `Requested` |
| R87-4-13 | `Blur` gives `Closed` and `focused` `false`. `Change` with `focused` `false` keeps `Closed`. `Submit` and `Inserted` give `Closed`. `Focus` gives `Requested`. `Change` and `CursorMoved` set `selected` to `None` |
| R87-4-14 | `haleen ` at cursor 7 gives `None` in `Typing` and 11 rows in `Requested`. An empty text in `Typing` gives 11 rows. Each text with a selection `0..2` gives `None` |
| R87-4-15 | The row `artist:` has the label "Field artist:, an artist name, for example artist:jsmovie". The row `is:` ends with ", Library only". The row `single` has the label "Value single for kind:, A release with one track". Each `Field` row has `InsertField(name)` and each `FixedValue` row has `InsertValue { field, word }`. Each row of R87-4-01 is `Available` |
| R87-4-16 | `café ar` at cursor 8 gives the edit `6..8` and the text `café artist:` with cursor 13. Each `edit.range` and each `edit.cursor` is on a character boundary |
| R87-4-17 | A `gpui::TestAppContext` test makes an `InputState`, a `SearchFieldListView` and a wrapper with `install_field_list_keys`. It focuses the input and types `ar`. `down` then `tab` give the text `artist:` and the cursor 7, and the input keeps the focus. `escape` then closes the list and keeps the text. `enter` with no selected row emits one `InputEvent::PressEnter` |
| R87-4-18 | In the same test setup, `click_row(0)` with the text `ar` gives the text `artist:`. The input keeps the focus. `click_row` of an unavailable row does not change the text |
| R87-4-19 | The situational guard of "Required Changes" item 7 passes. It fails against a copy of the composite that calls `Popover::new` or `.truncate()`, or that holds a string literal |
| R87-4-20 | Each existing test in `src/view_models/app_toolbar.rs`, `src/app/keyboard.rs` and `src/ui/primitives/primary_selection.rs` still passes |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: click in the empty toolbar search field. The field list opens directly under the field. It
  shows 11 rows. Each row shows a field, its values and one example.
- V2: while the list is open, the caret in the field continues to blink. Type `ar`. The text goes
  into the field, and the list shows only `artist:`.
- V3: press Down and Up. The selected row moves and wraps at the two ends. The selected row has a
  fill and a border. Press Tab. The text changes to `artist:`, and the caret is after the colon.
- V4: type `kind:`. The list shows `album`, `ep` and `single`. Click `single`. The text changes to
  `kind:single `, the list closes, and the caret blinks in the field.
- V5: type `after:yesterday`. The list shows the error text "after: needs a date, for example
  2026-10-01".
- V6: the rows `is:`, `playlist:` and `has:` show the mark "Library only". Type `in:index `. These
  rows show the note "Not available with in:index", and Down skips them.
- V7: press Escape. The list closes, and the text stays. Type one more character.

  The list opens again. Press Enter with no selected row. The search runs, as before this packet.
- V8: click a row in the Music column. The list closes. Press Up and Down in the field with the
  list closed. Nothing changes in the field or in the Music column, as before this packet.
- V9: Light and Dark themes, a normal width and a narrow width. The list stays in the window. It
  shows over the Music columns, not under them. No text is clipped. Long text wraps.

## Exclusions

- No artist names, release titles or playlist names, and no `GET /v1/artist-names` request
  (packet 005).
- No loading row and no source mark (packet 005).
- No live search run and no pause token (packet 003).
- No change to `submit_global_search`, the Library search, the Index search or the saved
  searches (packet 002).
- No change to gpui-kit, gpui-component or the popover primitive.
- No colored chips in the text (ADR 0087 Non-Goals).
- No change to the keys of the active pane in `src/app/keyboard.rs`.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- [ADR 0087](../adr/0087-search-fields-and-live-results.md), the
  [ADR 0087 phase plan](../plans/adr-0087-search-fields-phase-plan.md) and
  [packet 001](archive/adr-0087-task-001-query-parser.md).
- `src/view_models/search_query.rs` (packet 001) and `src/view_models/app_toolbar.rs`.
- `src/app.rs` (`global_search_input`, `on_global_search_event`, `focus_global_search`) and
  `src/app/tab_bar.rs` (`render_global_search`).
- `src/app/keyboard.rs`, to read only.
- `src/ui/primitives/context_menu.rs` (`deferred`, `anchored`), `src/ui/primitives/surface.rs`,
  `src/ui/primitives/status_badge.rs` (`render_row_badge`) and `src/ui/tokens.rs`.
- `src/ui/primitives/primary_selection.rs`, for the `TestAppContext` setup of an `InputState`.
- `$REG/gpui-component-0.6.1/src/input/popovers/completion_menu.rs`, for the overlay method.
- `$KIT/base/src/input/base/state.rs`: `cursor`, `selected_range`, `set_selected_range`,
  `replace`, `focus` and the key bindings.
- `tests/architecture_tests.rs`: `adr_0063_column_text_does_not_truncate` and the helpers of the
  recorded facts.

## Checks

```bash
cargo test --lib adr_0087_field_list_
cargo test --lib adr_0087_query_parser_
cargo check --all-targets
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --test architecture_tests
touch src/main.rs && cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration, no stored data and no request.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0087-task-004-field-list.md`
- ADR 0087 Decisions 2 and 3, and its invariants
- Packet 001: `docs/tasks/archive/adr-0087-task-001-query-parser.md`
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use
  the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- Use each name of "Names For Later Packets" exactly. Use the names of packet 001 exactly.
- The view model gives each text, label, id and availability. The composite only composes.
- Use named tokens for size, spacing, color and typography. Column text does not call
  `truncate()`.
- Do not read field syntax outside `src/view_models/search_query.rs`. Read `CursorTerm`,
  `SearchQuery` and `QUERY_FIELDS` only.
- Do not use the popover primitive. Do not give the list a focus handle.
- Tests use fixed text. No test sends a request or opens a database.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or
  `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- `src/view_models/search_query.rs`, except to read it.
- `src/app/search_dispatch.rs`, `src/application/queries/search.rs`, `src/db.rs` and
  `src/db/saved_searches.rs`.
- The key bindings of `src/app/keyboard.rs`.
- `src/ui/primitives/popover.rs` and `src/ui/primitives/context_menu.rs`.
- gpui-kit, gpui-component and `Cargo.toml`.
- Any ADR, and each document other than this packet and `docs/architecture/source-map.md`.

Acceptance criteria:
- Each case R87-4-01 to R87-4-20 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-23" section and an "Operator Visual Check"
  section for V1 to V9.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns,
and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- Packet 001 is not complete: `src/view_models/search_query.rs` does not exist, or one of its
  names in this packet is missing.
- The list cannot keep the focus in the field without a change to gpui-kit or gpui-component.
- A mouse down on a row moves the focus out of the input in the R87-4-18 test.
- `replace` does not replace the selected range, or `capture_action` on the wrapper does not get
  a key before the input.
- A change needs a new architecture decision.
- A change needs a file in "Do not touch".
- A recorded fact of this packet is not true in the working tree.
