# ADR 0087 Task 003: The Search Results Update While The Curator Types

Status: Ready - 2026-10-23.

Orchestrator note - 2026-10-23: packets 001 and 002 are implemented. The run record lists their deviations. `LibrarySearchFacts::track` and `SearchResultsInspectorPageVm::index_skip` are `#[cfg(test)]`. Remove the attribute of each name that this packet calls. `search_local_library_tracks` is `pub(crate)`. `UnsupportedIndexTerms` is in `search_results::search_run`.

Packet 004 is implemented before this packet. Enter on a selected row of the field list does not emit `PressEnter`, and the test R87-4-17 proves it. After Enter, the field list view ignores one `Change` with the submitted text. The field `submitted_text` in `src/app/search_field_list.rs` holds that text. Keep this rule when you add the pause.

## Goal

The toolbar search runs while the curator types. After the curator stops typing for a pause, the
search runs against the Library and the Index. Enter runs it at once. The named token
`Motion::SearchPause` holds the pause, 300 ms at first.

Each run has a generation. The app drops a Library result or an Index result of an older
generation. A text with no runnable content runs nothing. A newer run cancels the Index request of
an older run when that request has not started.

The mounted search page updates in place. The first live run opens the search page as one entry of
the navigation stack. Each later live run replaces the text of that same entry. Back, Forward and
the restored toolbar text start no live run.

## Authority

- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 4: the pause, the named token,
  Enter at once and the generation. A text with no runnable content runs nothing. The page updates
  in place, and each run sends at most three Index requests.
- ADR 0087 invariant "A result of an older run never replaces the result of a newer run".
- ADR 0087 Decision 3: Enter with a selected row of the field list puts the row into the text. Enter
  with no selected row runs the search.
- ADR 0087 Decision 6: a saved search keeps its text.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 9: one navigation stack. A search is a
  page change. A new open of a page shows it at the top.
- [ADR 0040](../adr/0040-async-vm-runtime.md): screens do not call `cx.spawn`. Only
  `src/presentation/`, `src/runtime/` and `src/app/bootstrap.rs` call it.
- [ADR 0083](../adr/0083-design-language.md) Decision 6 and
  [ADR 0083 task 008](adr-0083-task-008-drag-track-to-playlist.md): the `Motion` token of the
  named interface times, and the timer `present_after`.
- [Packet 001](archive/adr-0087-task-001-query-parser.md): `SearchQuery::has_runnable_content`.
- [Packet 002](adr-0087-task-002-library-fields-and-index-words.md): `SearchRun`,
  `search_results_detail_for_run` and `start_index_search_for_run`. Packet 002 says "003 adds
  `generation`".
- [Packet 004](adr-0087-task-004-field-list.md): `FieldListState`, `FieldListEvent`,
  `FieldListKeyOutcome` and `TopApp::search_field_list`.
- The [ADR 0087 phase plan](../plans/adr-0087-search-fields-phase-plan.md), packet 003.
- [AGENTS.md](../../AGENTS.md): token discipline, "Current-view state must update in place", the
  async convention, "A guard names its class and its ADR" and "Collect responsiveness concerns".

## Recorded Facts - 2026-10-23

The orchestrator read each fact in the working tree on 2026-10-23. Packets 001, 002 and 004 are not
implemented. The last subsection lists the facts that come from their designs. `$KIT` is
`~/.cargo/git/checkouts/gpui-kit-7335f17435243f46/5463fe4/crates`.

### The toolbar field

- `src/app.rs:147` declares `global_search_input`. `src/app.rs:344-348` makes it and subscribes
  `on_global_search_event`.
- `src/app.rs:604-614` `on_global_search_event` reacts only to `InputEvent::PressEnter`. It calls
  `submit_global_search`.
- `src/app.rs:1937` draws the toolbar on each tab. Thus the field shows on Music, Show and
  Settings.
- `src/app/tab_bar.rs:119-123` draws the field with `cleanable(true)`.
  `src/app/tab_bar.rs:161-163`: the search button also calls `submit_global_search`.
- `src/app.rs:616-618` `select_tab` calls `focus_active_tab`. `src/app/tab_bar.rs:237-244`
  `focus_active_tab` moves the focus to the tab. Thus `select_tab` takes the focus from the field.

### The input events

- `$KIT/base/src/input/base/state.rs:122-127`: `InputEvent` has `Change`,
  `PressEnter { secondary, shift }`, `Focus` and `Blur`.
- `$KIT/base/src/input/base/state.rs:894-917` `set_value` sets `emit_events` to `false` before it
  replaces the text. Thus `set_value` emits no `Change`.
- `$KIT/base/src/input/base/state.rs:3603-3605` and `:3922-3924` emit `Change` after a typed edit
  and after a replace.
- `$KIT/base/src/input/base/state.rs:2001-2005` `clean` calls `replace_text("")`. Thus the clear
  button of the field emits `Change`.
- `$KIT/base/src/input/base/state.rs:1988-1998`: in a single-line input, Enter propagates and
  emits `PressEnter`.
- `src/ui/primitives/primary_selection.rs:213-222` makes an `InputState` in a
  `gpui::TestAppContext` test after `gpui_component::init`.

### The search page in the navigation stack

- `src/app/search_dispatch.rs:223-226` `submit_global_search` reads the field text.
  `src/app/search_dispatch.rs:228-267` `open_search_results_in_content_list` trims the text. When
  the tab is not Music, it calls `select_tab`. It then calls
  `WorkspaceLayout::open_search_results_in_content_list`, builds the page, scrolls it to the top and
  calls `restore_content_list_nav_entry`.
- `src/view_models/workspace/mod.rs:316-334` `open_search_results_in_content_list` calls
  `replace_active_search_or_push` and focuses the content list frame in the model.
- `src/view_models/workspace/nav.rs:826-838` `replace_active_search_or_push` replaces the current
  page in place when it is a `Search` page. Else it starts a new path with the `Search` page.
- `src/view_models/workspace/nav.rs:803-821` `replace_current` keeps the back history. It clears
  the forward history and gives the page a fresh scroll memory.
- `src/view_models/workspace/tests.rs:1134` proves the replace on a search page.
  `src/view_models/workspace/tests.rs:1162` and `:1194` prove the new path from a release page and
  from an artist page under a search.
- `src/view_models/workspace/nav.rs:678-683` `active_search_query` gives the query of the `Search`
  entry of the current path.
- `src/app.rs:742-754` (Back) and `src/app.rs:762-774` (Forward) call
  `pending_toolbar_search.record(&entry)` and `restore_content_list_nav_entry`.
  `src/view_models/workspace/nav.rs:374-399` records the text of a `Search` entry only.
- `src/app.rs:832-850` `restore_content_list_nav_entry` restores each page. For a `Search` entry it
  starts the Index search. It takes no `Window`.
- `src/app.rs:862-871` `apply_pending_toolbar_search` writes the recorded text with `set_value`,
  only when the trimmed field text is different. `src/app.rs:1894` calls it at each render.
- `src/app.rs:782-822` `open_music_page` is the open path of a sidebar row, a list row and a link.
  It calls `restore_content_list_nav_entry` too.
- `src/app/search_selection.rs:30-35` `SearchPageSlot` has `ToolbarSearch` (the list column) and
  `SavedSearch` (the detail column). `src/app/search_selection.rs:144`: a click on a search row
  moves the focus to the search page.

### The Index request

- `src/app/search_dispatch.rs:288-329` `start_index_search_for_query` marks the page as loading and
  dispatches `FetchIndexSearchResults`. `src/app/search_dispatch.rs:364-447`
  `dispatch_index_search` uses `CommandContext::next()` (line 383). Its success and its failure
  check `content_list_nav_matches_search` (`:465-470`) and the query of the page.
- `src/app/search_dispatch.rs:218-220` `index_search_error_retains_action` gives `true` for each
  error other than `QueryRejected`. Thus it gives `true` for `Cancelled` today.
- `src/app/saved_search_dispatch.rs:86-150` `run_saved_search` dispatches its own
  `FetchIndexSearchResults` with `CommandContext::next()` (line 126). It checks
  `saved_search_result_is_current`.
- `src/application/queries/search.rs:57-64`: `FetchIndexSearchResults::execute` gives
  `CommandError::Cancelled` when the token of its context is canceled before the start.
- `src/application/queries/search.rs:226-276` `fetch_index_search_result_rows` sends the three
  `GET /v1/search` requests one after the other. `:336-363` `index_artist_rows` then sends one
  `GET /v1/artist-names` check for each candidate name with no single feed match.
- `src/application/async_command_runner.rs:157-159`: a dropped receiver does not cancel a command.
- `src/application/command_context.rs:44-67`: `CancellationToken` is `Clone` and shares one
  `Arc<AtomicBool>`. `:79-89` `CommandContext::next()` makes a new token. `:113` `cancellation()`
  gives it. `src/application/errors/command.rs:171` holds `CommandError::Cancelled`.
- `src/presentation/async_command_presenter.rs:18-57` `present_command` takes a `CommandContext`.
  It drops a result only for an old app session (lines 32 and 40).
- `src/view_models/search_results/mod.rs:183-187` `mark_index_loading`, `:213-224`
  `replace_index_results_for_query` and `:226` `set_index_error` hold no generation.

### The timers and the time tokens

- `src/ui/tokens.rs:1072-1097` holds `Motion`, "the named times of the interface (ADR 0083
  Decision 6)". It has one value, `SpringLoadDelay`, 500 ms. `Motion::duration` gives a
  `std::time::Duration`.
- `docs/tasks/adr-0083-task-008-drag-track-to-playlist.md:39` and `:112-115` added `Motion`. No
  other time token exists.
- `src/presentation/delayed_presenter.rs:14-28` `present_after(delay, cx, apply)` waits on
  `cx.background_executor().timer(delay)` inside `cx.spawn`, then calls `apply` and
  `cx.notify()`. It calls `cx.notify()` also when `apply` does nothing.
- `src/library/app_impl.rs:3594-3626` starts the spring-load timer with
  `present_after(Motion::SpringLoadDelay.duration(), ..)`. `src/view_models/library.rs:694-758`
  `SpringLoadVm` gives a `SpringLoadGeneration` for each timer, and a timer of an older generation
  opens nothing.
- `tests/architecture_tests.rs:11104-11129` `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap`
  rejects `cx.spawn(` in each other file. `tests/architecture_tests.rs:22524-22569`
  `adr_0083_track_drag_r83_8_13_spring_load_delay_is_a_token` is the pattern of a time token guard.
- No file under `src/view_models/` imports `crate::ui::tokens`. `src/view_models/mod.rs:1-3` says
  that a view model is free of GPUI.
- `docs/adr/0040-async-vm-runtime.md:111-117`: `src/runtime/` must not import `gpui`.

### Facts from the designs of packets 001, 002 and 004

These facts are not in the code today.

- Packet 001: `SearchQuery::has_runnable_content` is `true` only for a word or a phrase with an
  alphanumeric character, or a `Valid` field other than `in:`, each without `-`. A field with no
  value is `Incomplete`.
- Packet 002: `SearchRun::new(text)` trims and parses the text one time. `is_runnable` reads
  `has_runnable_content`. `search_results_detail_for_run` and `start_index_search_for_run` are the
  one entry point of a run. `start_index_search_for_run` serves the two slots and keeps the stale
  check of each slot.
- Packet 002: the Library part runs on the main thread in the same call, with the database lock
  held (its responsiveness concern).
- Packet 004: `SearchFieldListView` subscribes to the same input. Enter with a selected row gives
  `FieldListKeyOutcome::Apply`, and the capture listener stops the key. Thus no `PressEnter` comes.
  The edit uses `replace`, so it emits `Change`.
- Packet 004: `FieldListEvent::Submit` closes the list on `PressEnter`.

## Design

### The pause and the generation

`LiveRunGate` (`src/view_models/live_run.rs`) is a pure state machine. It holds no GPUI type and
reads no clock. It has two counters:

- The pause counter. Each change of the text gives a new `PauseTicket`. Only the ticket of the last
  change can end the pause.
- The run counter. Each run gets a new `RunGeneration`. Only the newest generation is current.

| Method | Result |
|---|---|
| `text_changed(&mut self) -> PauseTicket` | The pause counter increases. The pause is armed. The function gives the new ticket |
| `pause_ended(&mut self, ticket: PauseTicket) -> bool` | `true` one time, when the pause is armed and the ticket is the last ticket. The pause is then not armed |
| `cancel_pause(&mut self)` | The pause is not armed. A later `pause_ended` gives `false` |
| `next_run(&mut self) -> RunGeneration` | The run counter increases. The pause is not armed. The function gives the new generation |
| `is_current(&self, generation: RunGeneration) -> bool` | `true` only for the last generation that `next_run` gave |
| `current_run(&self) -> RunGeneration` | The last generation |

The host starts one timer for each ticket. The timer calls `present_after` with
`Motion::SearchPause.duration()`. Each timer ends after the full pause from its own change. Thus
the timer of the last ticket ends one pause after the last change. A timer of an older ticket finds
`pause_ended` `false` and does nothing.

### The slots

`SearchSlotRuns` (`src/app/search_live.rs`) holds one `LiveRunGate` and the
`Option<CancellationToken>` of the Index request in flight. `TopApp` holds one value for each
`SearchPageSlot`: `toolbar_search_runs` and `saved_search_runs`.

- `SearchSlotRuns::next_run(&mut self) -> RunGeneration` calls `gate.next_run()`.
- `SearchSlotRuns::replace_index_cancel(&mut self, token: CancellationToken)` cancels the token in
  flight and keeps the new token.

`TopApp::next_search_run(&mut self, slot, text: &str) -> SearchRun` gives
`SearchRun::new(text).with_generation(runs.next_run())`. Each production `SearchRun` of a search page
comes from this function. Only the toolbar slot arms a pause.

### The live run

`on_global_search_event` handles two events:

- `InputEvent::Change` calls `start_search_pause(cx)`.
- `InputEvent::PressEnter { .. }` calls `submit_global_search`, as today.

`TopApp::start_search_pause(&mut self, cx)`:

1. When `self.tab` is not `AppTab::Music`, it returns.
2. It calls `toolbar_search_runs.gate.text_changed()` and keeps the ticket.
3. It calls `present_after(Motion::SearchPause.duration(), cx, move |this, cx| this.end_search_pause(ticket, cx))`.

`TopApp::end_search_pause(&mut self, ticket, cx)`:

1. When `pause_ended(ticket)` is `false`, it returns.
2. It reads the field text, and the `active_search_query` of the content list frame.
3. It calls `live_run_step(text, mounted_query, self.tab == AppTab::Music)`.
4. For `LiveRunStep::Run(run)`, it calls
   `show_toolbar_search_run(run.with_generation(toolbar_search_runs.next_run()), cx)`. For each other
   step, it does nothing.

`live_run_step` (`src/view_models/search_results/search_run.rs`) gives the first true row:

| Condition | Step |
|---|---|
| The tab is not Music | `NotMusic` |
| `SearchRun::new(text).is_runnable()` is `false` | `NotRunnable` |
| The trimmed text is the same as `mounted_query` | `SameQuery` |
| Each other text | `Run(run)` |

`TopApp::show_toolbar_search_run(&mut self, run: SearchRun, cx)` is the body of today's
`open_search_results_in_content_list` after the tab check. It takes no `Window`:

1. It calls `WorkspaceLayout::open_search_results_in_content_list(run.text())`.
2. It mounts `search_results_detail_for_run(&run)` with `with_run_generation(run.generation())`,
   only when `is_newer_than(self.search_results_detail.as_ref())` is `true`.
3. It scrolls the toolbar page to the top.
4. It calls `restore_content_list_nav_entry` with the `Search` entry of the text.

`open_search_results_in_content_list(text, window, cx)` keeps its tab check and its call of
`select_tab`. It then builds the run with `next_search_run(ToolbarSearch, text)` and calls
`show_toolbar_search_run`. A run that is not runnable returns before the tab check, as in packet
002.

`submit_global_search` calls `toolbar_search_runs.gate.cancel_pause()` first. Thus Enter runs one
time, and the pause of the last change does not run the same text again.

### The navigation rule

A live run uses the same model function as Enter: `WorkspaceLayout::open_search_results_in_content_list`.

- When the current page is not a `Search` page, the first live run starts a new path with the
  `Search` page. This is the one entry that the typed text adds to the stack.
- When the current page is the `Search` page, each later live run replaces its text in place. The
  back history does not change. The forward history clears, as for Enter today.
- The current page can be under a `Search` page, for example a release page that a result
  opened. Then a live run starts a new path, as Enter does today. Each later live run replaces it in place.
- Enter on the mounted search page replaces its text in place. A click on a result pushes the
  result page over the search page. Thus the search page stays in the stack.

`restore_content_list_nav_entry` calls `toolbar_search_runs.gate.cancel_pause()` first. Each page
change thus stops a pending pause: Back, Forward, a sidebar row, a link and a click on a result.
For a `Search` entry, it builds one run with `next_search_run(ToolbarSearch, query)`. It passes that
run to `start_index_search_for_run`. When `sync_search_results_detail_with_nav` builds a page, it
also builds its run with `next_search_run`.

### Back, Forward and the restored text

- Back and Forward call no `start_search_pause` and no `show_toolbar_search_run`.
- `apply_pending_toolbar_search` keeps `set_value`. `set_value` emits no `Change`. Thus the
  restored text arms no pause, and no live run starts.
- The restore of a `Search` entry starts the Index search of the restored page, as today. It takes
  a new generation. Thus a result of a run from before the Back is not current, and the app drops
  it.

### The results

`SearchResultsInspectorPageVm` gains the field `run_generation: RunGeneration`:

| Method | Rule |
|---|---|
| `with_run_generation(self, generation) -> Self` | Sets the generation of a new page |
| `run_generation(&self) -> RunGeneration` | Gives it |
| `is_newer_than(&self, current: Option<&Self>) -> bool` | `true` when `current` is `None`, or when this generation is equal to or greater than the generation of `current` |
| `begin_index_run(&mut self, generation) -> bool` | When the generation is older than the page generation, it gives `false` and changes nothing. Else it sets the generation and calls `mark_index_loading` |
| `replace_index_results_for_run(&mut self, generation, query, rows) -> bool` | `false` and no change when the generation is not the page generation. Else the rule of `replace_index_results_for_query` |
| `set_index_error_for_run(&mut self, generation, query, error, endpoint, at) -> bool` | The same generation rule for `set_index_error`, with the query check of today |

`start_index_search_for_run(slot, run, cx)` changes in packet 003:

1. It calls `begin_index_run(run.generation())` on the page of the slot, in place of
   `mark_index_loading`.
2. It makes `let context = CommandContext::next()`. It calls
   `runs.replace_index_cancel(context.cancellation().clone())` for the slot.
3. It dispatches with that context.
4. The success and the failure first check `runs.gate.is_current(generation)` for the slot. They then
   keep the stale check of packet 002 for the slot. They call `replace_index_results_for_run` and
   `set_index_error_for_run`.
5. A `CommandError::Cancelled` failure returns at once. It retains no action and changes no page.

`retry_index_search` and `run_saved_search` build their run with `next_search_run`. They use the
same steps.

### The cancel

`fetch_index_search_result_rows` gets one more argument, `cancel: &CancellationToken`.
`FetchIndexSearchResults::execute` passes `context.cancellation()`. The function checks the token
before each of the three `GET /v1/search` requests. `index_artist_rows` checks it before each
`GET /v1/artist-names` check. A canceled token gives `CommandError::Cancelled`.

A request that started finishes, because the client is blocking. The generation check then drops
its result.

### The field list

- Enter with a selected row puts the row into the text (packet 004). The edit emits `Change`, so the
  pause starts. Enter runs nothing at once in this case.
- Enter with no selected row emits `PressEnter`. The search runs at once, and the list closes.
- `artist:` with no value is `Incomplete`. A text with only `artist:` is not runnable, so the pause
  runs nothing. `haleen artist:` runs. The empty field applies nothing (packet 002 D14).
- A live run sends no `FieldListEvent` and does not change `FieldListState`. The list stays open
  while the results change. `TopApp::search_field_list` does not change.

### Names For Later Packets

Packet 005 reuses the pause and the generation for its `GET /v1/artist-names?from=` value request.
It uses these names exactly. A later packet adds a name only through its own packet text.

| Name | Module | Packets |
|---|---|---|
| `Motion::SearchPause`, 300 ms, read with `Motion::SearchPause.duration()` | `src/ui/tokens.rs` | 005 |
| `pub(crate) struct LiveRunGate`, with `text_changed`, `pause_ended`, `cancel_pause`, `next_run`, `is_current` and `current_run` | `src/view_models/live_run.rs` | 005 |
| `pub(crate) struct PauseTicket(u64)`, `Clone`, `Copy`, `Debug`, `Eq`, `PartialEq` | same | 005 |
| `pub(crate) struct RunGeneration(u64)`, `Clone`, `Copy`, `Debug`, `Default`, `Eq`, `Ord`, `PartialEq`, `PartialOrd` | same | 005 |
| `SearchRun::with_generation(self, generation: RunGeneration) -> SearchRun` and `fn generation(&self) -> RunGeneration`. `SearchRun::new` gives `RunGeneration::default()` | `src/view_models/search_results/search_run.rs` | none |
| `pub(crate) enum LiveRunStep { Run(SearchRun), NotRunnable, SameQuery, NotMusic }` and `pub(crate) fn live_run_step(text: &str, mounted_query: Option<&str>, music_tab: bool) -> LiveRunStep` | same | none |
| `SearchResultsInspectorPageVm::with_run_generation`, `run_generation`, `is_newer_than`, `begin_index_run`, `replace_index_results_for_run` and `set_index_error_for_run` | `src/view_models/search_results/mod.rs` | none |
| `pub(super) struct SearchSlotRuns { gate: LiveRunGate, index_cancel: Option<CancellationToken> }`, with `next_run` and `replace_index_cancel` | `src/app/search_live.rs` | none |
| `TopApp::start_search_pause`, `TopApp::end_search_pause(ticket, cx)`, `TopApp::next_search_run(slot, text)`, `TopApp::show_toolbar_search_run(run, cx)` and `TopApp::search_runs_mut(slot)` | same | none |
| `fn index_search_error_shows(error: &CommandError) -> bool` | `src/app/search_dispatch.rs` | none |
| `fetch_index_search_result_rows(endpoint, request, matches, cancel: &CancellationToken)` | `src/application/queries/search.rs` | 006 |

Packet 005 owns its own `LiveRunGate` in `SearchFieldListView`. It arms the gate on each `Change`
with the cursor in an `artist:` value, and starts `present_after` with
`Motion::SearchPause.duration()`. After `pause_ended`, it takes `next_run` and sends one request.
It drops a reply when `is_current` is `false`.

## Orchestrator Decisions

- **D1. The pause timer is `present_after`.** It is the timer owner that exists for a short
  interface delay (ADR 0083 task 008). Its `cx.spawn` stays in `src/presentation/`, so ADR 0040
  and its guard accept it. A `gpui::Timer` in `src/app/` needs `cx.spawn` in a screen, which the
  guard rejects. An actor in `src/runtime/` cannot hold a GPUI timer (ADR 0040), and it adds a
  channel and a bridge for one wait.
- **D2. `Motion::SearchPause` in `src/ui/tokens.rs` holds the pause.** `Motion` is the one owner
  of the named interface times. The token list of AGENTS.md does not name times, but ADR 0087
  Decision 4 asks for a named token. A second owner splits the times into two files. The host in
  `src/app/` reads the token, because no view model imports `crate::ui::tokens`.
- **D3. The pause is a pure state machine with tickets, and has no clock.** Each timer of
  `present_after` ends after its full delay. The ticket of the last change is the only ticket that
  acts, so the run comes one pause after the last change. A test then needs no clock and no wait.
- **D4. Each search page slot has one gate.** The toolbar slot uses the pause and the generation.
  The saved search slot uses only the generation. Each run of a slot takes a new generation, also
  a restore and a retry. Thus the newest action of the curator owns the result.
- **D5. The first live run opens the search page, and each later live run replaces it.** The live
  run calls the same model function as Enter. That function already replaces a `Search` page in
  place and starts a new path from any other page. Thus the typed text adds one entry at most, and
  Back returns to the page before the typing.
- **D6. A live run with the text of the mounted search runs nothing.** A typed space or a deleted
  space gives the same trimmed text. A run then sends three Index requests for the same rows.
  Enter still runs the same text again, as today.
- **D7. A text that is not runnable runs nothing and takes no generation.** The mounted page keeps
  its rows. Its Index run stays current and can finish. The clear button gives an empty text, so
  the page keeps its rows until the next runnable text.
- **D8. Outside the Music tab, typing arms no pause.** `select_tab` moves the focus to the tab, so a
  live run from Show or Settings takes the focus from the field. Show is the broadcast surface, and
  typing must not move the curator out of it. Enter keeps the behavior of today and opens Music.
- **D9. A live run does not move the focus.** The timer gives no `Window`. `show_toolbar_search_run`
  calls no `select_tab` and focuses nothing. The caret stays in the field.
- **D10. Each page change stops the pending pause.** Without the stop, a pause that ends after a
  click on a result opens a new search path over the result page.
- **D11. The restored text starts no run.** `set_value` emits no `Change`. The restore keeps
  `set_value` and does not use `replace`.
- **D12. A newer run cancels the token of the older Index request.** The command already checks the
  token at its start. Packet 003 adds a check before each request. A request that started cannot
  stop, because the HTTP client is blocking. The generation drops its result.
- **D13. "Three Index requests" counts the `GET /v1/search` requests.** ADR 0087 Decision 4 says
  "the same as one search today". That search also sends the name checks of `index_artist_rows`.
  This packet keeps them, and the cancel stops them for an older run. The orchestrator records
  this reading for the operator.
- **D14. A canceled request is not an error.** It retains no failed action and shows no error.
  `index_search_error_retains_action` gives `false` for `Cancelled`.
- **D15. The Library result has the generation check too.** The Library part runs in the same call
  as the run start, so an older Library result cannot come later today. The check keeps the
  invariant true when a later change moves the Library part off the main thread.
- **D16. Typing does not change a saved search page.** A saved search page has its own stored text.
  The first live run starts a toolbar search path, as Enter does today. The saved search page stays
  in the back history with its text and its rows.
- **D17. Enter with a selected field list row is an edit, not a run.** Packet 004 owns the key. The
  edit emits `Change`, so the pause starts as for typed text.
- **D18. A live run builds a new page.** The selection and the scroll position of the earlier text
  do not apply to a new text. ADR 0084 Decision 9 shows a new search at the top.

## Required Changes

1. `src/ui/tokens.rs`: add `Motion::SearchPause` with the text "300 ms. The pause after the last
   typed character before the search runs (ADR 0087 Decision 4)". `Motion::duration` gives
   `Duration::from_millis(300)` for it.
2. `src/view_models/live_run.rs` (new): add `LiveRunGate`, `PauseTicket` and `RunGeneration`. Start
   it with a `//!` comment that names ADR 0087 Decision 4 and its invariant, and with
   `#![warn(clippy::pedantic)]`. Declare `pub(crate) mod live_run;` in `src/view_models/mod.rs`, in
   alphabetical order.
3. `src/view_models/search_results/search_run.rs`: add the field `generation`, `with_generation`,
   `generation`, `LiveRunStep` and `live_run_step`.
4. `src/view_models/search_results/mod.rs`: add the field `run_generation` and the six methods of
   "The results".
5. `src/application/queries/search.rs`: add the argument `cancel` to
   `fetch_index_search_result_rows` and to `index_artist_rows`, and the checks of "The cancel".
6. `src/app/search_live.rs` (new): add `SearchSlotRuns` and the `TopApp` functions of "Names For
   Later Packets". Start it with a `//!` comment that names ADR 0087 Decision 4. Declare
   `mod search_live;` in `src/app.rs`, in alphabetical order.
7. `src/app.rs`: add the fields `toolbar_search_runs` and `saved_search_runs`. Handle
   `InputEvent::Change` in `on_global_search_event`. Call `cancel_pause` and `next_search_run` in
   `restore_content_list_nav_entry`.
8. `src/app/search_dispatch.rs`: change `submit_global_search`,
   `open_search_results_in_content_list`, `start_index_search_for_run`, `dispatch_index_search`,
   `retry_index_search` and `sync_search_results_detail_with_nav` as "Design" says. Change
   `index_search_error_retains_action`, and add `index_search_error_shows`.
9. `src/app/saved_search_dispatch.rs`: `run_saved_search` builds its run with
   `next_search_run(SearchPageSlot::SavedSearch, ..)`.
10. `src/view_models/workspace/tests.rs`: add the tests R87-3-10 and R87-3-11.
11. `tests/architecture_tests.rs`: add the guard of case R87-3-17.
12. `docs/architecture/source-map.md`: add a row "Music search live runs: the pause, the run
    generation and the cancel (ADR 0087 Decision 4, packet 003)". Its owners are
    `src/view_models/live_run.rs`, `src/app/search_live.rs` and `Motion::SearchPause` in
    `src/ui/tokens.rs`.

## Mechanical Acceptance Criteria

Use the prefix `adr_0087_live_results_` for each unit test. Each test is beside its owner code,
apart from R87-3-17. No test sends a request to a live service, and no test waits for a timer.

| Case | Owner | Required proof |
|---|---|---|
| R87-3-01 | `src/view_models/live_run.rs` | Two `text_changed` calls give tickets A and B. `pause_ended(A)` gives `false`. `pause_ended(B)` gives `true` one time, and a second `pause_ended(B)` gives `false` |
| R87-3-02 | same | Enter at once: after `text_changed` gives ticket A, `next_run` gives a generation. `pause_ended(A)` then gives `false`. Thus the pause does not run the text a second time |
| R87-3-03 | same | After `cancel_pause`, `pause_ended` of the last ticket gives `false`. A later `text_changed` arms the pause again, and its ticket ends it |
| R87-3-04 | same | Three `next_run` calls give three generations in increasing order. `is_current` is `true` only for the third. `current_run` gives the third |
| R87-3-05 | `src/view_models/search_results/search_run.rs` | `live_run_step` gives `NotRunnable` for an empty text, `"   "`, `!!!`, `-remix`, `artist:` and `in:library`. It gives `Run` for `haleen artist:` |
| R87-3-06 | same | `live_run_step("haleen ", Some("haleen"), true)` gives `SameQuery`. `live_run_step("haleen", Some("hale"), true)` gives `Run`. `music_tab` `false` gives `NotMusic` for each text |
| R87-3-07 | same | `SearchRun::new` gives `RunGeneration::default()`. `with_generation` gives the given generation, and the text, the query and the plan do not change |
| R87-3-08 | `src/view_models/search_results/mod.rs` | A page of generation 4 is loading. `replace_index_results_for_run` of generation 3 gives `false`, the page stays loading and has no Index rows. Generation 4 gives `true` and the rows |
| R87-3-09 | same | `set_index_error_for_run` of an older generation gives `false` and sets no error. `begin_index_run` of an older generation gives `false` and changes nothing. A newer generation sets the generation and marks the page loading |
| R87-3-10 | same | The Library drop: a page of generation 3 `is_newer_than` a page of generation 4 gives `false`. Generation 5 gives `true`. `None` gives `true` |
| R87-3-11 | same | Three runs of the texts `hal`, `hale` and `haleen` begin in that order. Their Index results come in the order 3, 1, 2. The page shows the rows of the third run only |
| R87-3-12 | `src/view_models/workspace/tests.rs` | The page is created once. From the Discover page, `open_search_results_in_content_list` with `h`, `ha` and `hal` gives the current page `Search("hal")`. One `go_back` then gives the Discover page |
| R87-3-13 | same | A release page is under `Search("hal")`. The texts `halx` and `halxy` give one new path with `Search("halxy")` only. One `go_back` gives the release page. After `push_nav` of a release page over `Search("halxy")`, one `go_back` gives `Search("halxy")` |
| R87-3-14 | `src/app/search_live.rs` | A `gpui::TestAppContext` test subscribes to an `InputState`. `set_value` emits no `InputEvent::Change`. `replace` emits one `Change`, and `clean` emits one `Change` |
| R87-3-15 | same | `SearchSlotRuns::replace_index_cancel` with token B cancels token A, and B is not canceled. `next_run` gives increasing generations |
| R87-3-16 | `src/application/queries/search.rs` | With the `Fixture` and a token that is not canceled, one run sends three requests with the path `/v1/search`, for `type=feed`, `type=track` and `type=publisher`. With a canceled token, the command gives `CommandError::Cancelled` and the fixture records no request |
| R87-3-17 | `tests/architecture_tests.rs` | The situational guard `adr_0087_live_results_r87_3_17_pause_token_and_no_screen_timer` passes. It fails against a copy of `src/app/search_live.rs` with `Duration::from_millis(300)` in place of the token |
| R87-3-18 | `src/app/search_dispatch.rs` | `index_search_error_retains_action(&CommandError::Cancelled)` gives `false`. `index_search_error_shows` gives `false` for `Cancelled`, and `true` for `QueryRejected` and `Other` |
| R87-3-19 | all | Each test of packets 001, 002 and 004 still passes. Each test of `src/view_models/workspace/tests.rs` and the guard R83-8-13 still pass |

The guard of R87-3-17 enforces these rules. It reads `production_source(&code_only(&source))`.

- `src/ui/tokens.rs` holds `Self::SearchPause => Duration::from_millis(300),`.
- `src/app/search_live.rs` holds `present_after(Motion::SearchPause.duration()`. It holds no
  `Duration::from_millis(`, no `Duration::from_secs(`, no `cx.spawn(` and no `timer(`.
- `src/app.rs`, `src/app/search_dispatch.rs`, `src/app/tab_bar.rs` and each file under `src/ui/`
  hold no `timer(` and no `background_executor(`.
- The source of `on_global_search_event` holds `InputEvent::Change` and `start_search_pause(`.
- The sources of `handle_content_list_back_select` and `handle_content_list_forward_select` hold no
  `start_search_pause(` and no `show_toolbar_search_run(`.
- The source of `apply_pending_toolbar_search` holds `set_value(` and no `.replace(`.
- The source of `restore_content_list_nav_entry` holds `cancel_pause(`.
- `src/app/search_dispatch.rs` holds exactly one `FetchIndexSearchResults::new(` outside
  `retry_index_search`, and it is in `start_index_search_for_run`.

The failure message of the guard is this text:

```text
ADR 0087 Decision 4 (results show while the curator types): <file> <violation>. Start the
pause only in TopApp::start_search_pause, through present_after with
Motion::SearchPause.duration(). Do not call cx.spawn or a timer in src/app or src/ui. A restore
writes the field with set_value, which starts no run. One run dispatches one
FetchIndexSearchResults through start_index_search_for_run.
```

## Visual Acceptance Criteria

These are for the operator. No test proves them. Examine V1 to V6 in Light and in Dark, at normal
width and at narrow width.

- V1: on the Discover page, type `hal` and stop. After a short pause, the search page opens. The
  caret stays in the field and continues to blink.
- V2: continue to type `een`. The rows change in place. No row of `hal` shows after a row of
  `haleen` shows. The page shows no flash of old rows and no empty frame between two results.
- V3: the Index loading rows show after the Library rows while the Index request runs. Then the
  Index rows replace them. With MusicIndex not reachable, one error shows, for the last text only.
- V4: type `haleen dri` and press Enter at once. The results show at once. No second update comes
  after the pause.
- V5: press Back one time. The Discover page shows, not a page of a shorter text. Press Forward.
  The search page shows with `haleen dri` in the field. The page does not update again.
- V6: type `artist:` in an empty field. Nothing runs. With the field list open, press Down and
  Enter. The row goes into the text, and the list stays in the window while the results change.
- V7: open the Show tab, and type a text. Nothing changes until Enter. Enter opens Music with the
  search, as before this packet.
- V8: open a saved search, then type in the toolbar field. The toolbar search page opens. Back
  shows the saved search page with its own text and rows.
- V9: at narrow width, the loading rows and the result rows wrap inside the column. No text
  overlaps a row.

## Exclusions

- No value rows and no `GET /v1/artist-names?from=` request (packet 005).
- No Index filter parameter (packet 006).
- No change to the field list rows, keys or composite (packet 004).
- No change to the Library SQL or to the Index plan (packet 002).
- No move of the Library search off the main thread. The responsiveness concern of packet 002 owns
  it.
- No change to `present_after` and no change to the Enter behavior outside Music.
- No setting for the pause. The named token holds it.
- No schema change, no migration and no configuration key.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- [ADR 0087](../adr/0087-search-fields-and-live-results.md), packets 001, 002 and 004 and their
  "Implementation Result" sections.
- `src/view_models/search_query.rs` (packet 001) and `src/view_models/search_results/search_run.rs`
  (packet 002).
- `src/app.rs`: `on_global_search_event`, `select_tab`, the Back and Forward handlers,
  `open_music_page`, `restore_content_list_nav_entry` and `apply_pending_toolbar_search`.
- `src/app/search_dispatch.rs`, `src/app/saved_search_dispatch.rs`, `src/app/search_selection.rs`,
  `src/app/tab_bar.rs` and `src/app/search_field_list.rs` (packet 004).
- `src/view_models/workspace/mod.rs`, `src/view_models/workspace/nav.rs` and
  `src/view_models/workspace/tests.rs`.
- `src/view_models/search_results/mod.rs`.
- `src/application/queries/search.rs` (`FetchIndexSearchResults`, `fetch_index_search_result_rows`,
  `index_artist_rows` and the `Fixture`), `src/application/command_context.rs` and
  `src/application/async_command_runner.rs`.
- `src/presentation/delayed_presenter.rs` and `src/presentation/async_command_presenter.rs`.
- `src/ui/tokens.rs` (`Motion`), `src/library/app_impl.rs` (`spring_load_pointer`) and
  `src/view_models/library.rs` (`SpringLoadVm`).
- `src/ui/primitives/primary_selection.rs`, for the `TestAppContext` setup of an `InputState`.
- `tests/architecture_tests.rs`: `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap`
  and `adr_0083_track_drag_r83_8_13_spring_load_delay_is_a_token`.

## Checks

```bash
cargo test --lib adr_0087_live_results_
cargo test --lib adr_0087_library_fields_
cargo test --lib adr_0087_query_parser_
cargo test --lib adr_0087_field_list_
cargo check --all-targets
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --test architecture_tests
touch src/main.rs && cargo build --bin v4vmm
```

Run `cargo build --bin v4vmm` last. `cargo test` can leave a binary with GPUI test support at
`target/debug/v4vmm`.

## Rollback

Revert the commit. This packet adds no migration, no stored data and no configuration key. After a
revert, the search runs only on Enter, as after packet 002.

## Responsiveness Concerns

Item 24 of "Throughout: Responsiveness And Background Feedback" in the overhaul plan already
records the request cost of a run after each pause. The orchestrator adds these concerns to that
list. This packet does not fix them.

- 2026-10-23, `src/app/search_dispatch.rs`: each live run runs the Library search on the main
  thread with the database lock held (packet 002 concern). Now this occurs after each pause, not
  only on Enter.
- 2026-10-23, `src/presentation/delayed_presenter.rs`: `present_after` notifies the entity at the
  end of each timer, also when the ticket is old. Each typed character thus gives one more render
  of `TopApp`, one pause later.
- 2026-10-23, `src/application/queries/search.rs`: a request that started cannot stop, because the
  HTTP client is blocking. A run that is not current still sends its started request to MusicIndex.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0087-task-003-live-results.md`
- ADR 0087 Decisions 3, 4 and 6 and its invariants, and ADR 0084 Decision 9
- Packets 001, 002 and 004, with their "Implementation Result" sections
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use
  the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- Use the names of packets 001, 002 and 004 exactly. Use each name of this packet's "Names For
  Later Packets" exactly.
- Start the pause only through `present_after` with `Motion::SearchPause.duration()`. Do not call
  `cx.spawn` or a timer in `src/app/` or `src/ui/`.
- Do not change `apply_pending_toolbar_search` to use `replace`.
- Tests use fixed values and the local `Fixture`. No test sends a request to a live service, and no
  test waits for a timer.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or
  `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- `src/view_models/search_query.rs`, `src/view_models/search_field_list.rs` and
  `src/ui/composites/search_field_list.rs`, apart from a read.
- `src/presentation/delayed_presenter.rs` and `src/presentation/async_command_presenter.rs`.
- The Library SQL in `src/db/library_search.rs`.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet and `docs/architecture/source-map.md`.

Acceptance criteria:
- Each case R87-3-01 to R87-3-19 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - <date>" section and an "Operator Visual Check"
  section for V1 to V9.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns,
and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- Packet 002 is not complete. This is true when `src/view_models/search_results/search_run.rs` is
  absent, or when `SearchRun`, `search_results_detail_for_run` or `start_index_search_for_run` is
  absent or has a different signature.
- Packet 001 is not complete, or `SearchQuery::has_runnable_content` is absent.
- Packet 004 is complete, and Enter with a selected row emits `InputEvent::PressEnter`.
- `present_after` or `Motion` is absent, or has a different signature.
- R87-3-14 fails: `set_value` emits `Change`, or `replace` emits none.
- `show_toolbar_search_run` cannot open the page without a `Window`.
- A recorded fact of this packet is not true in the working tree. Packets 001, 002 and 004 and
  ADR 0086 packets 002 to 005 did not cause the difference.
- A guard other than R87-3-17 fails because of this change.
- A change needs a new architecture decision, a migration or a file in "Do not touch".
