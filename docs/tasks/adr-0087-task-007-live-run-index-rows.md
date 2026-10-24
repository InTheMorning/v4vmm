# ADR 0087 Task 007: A Live Run Shows The Index Rows

Status: Implemented - 2026-10-24. V1 to V4 are open, in the final walk of the run of 2026-10-24.

The cause of the missing Index rows is not proved. "Candidates" lists the facts. Part A corrects a proved defect. Part B removes the candidate sites and adds tests for them. The walk of V1 and V3 decides the result.

## Goal

A live run of the toolbar search shows the Index rows, as a run with Enter does. While the Index
request runs, the loading rows show after the Library rows.

The operator walk of 2026-10-24 failed packet 003 check V3. The operator typed `survival` on the
Discover page and stopped. After the pause, the search page showed the 2 Library rows. It showed no
loading row, no Index row and no error, also 5 s later. MusicIndex 0.14.0 gives 17 feeds and 20
tracks for the same word. A search with Enter shows the Index rows.

This packet has two parts:

- Part A: the loading rows show after the rows of the active tab. Today they show only when the
  tab has no row. This part is proved by the code.
- Part B: each toolbar run takes one generation, and one tested function decides if an Index
  result changes the page. The static reading found no cause in these sites. The part removes the
  sites where a second generation or a silent drop can occur.

## Authority

- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 4: after the pause, the
  search runs against the Library and the Index. Each run has a generation. The app drops a result
  of an older generation.
- ADR 0087 invariant "A result of an older run never replaces the result of a newer run".
- [Packet 003](adr-0087-task-003-live-results.md): its design, its decisions D4, D12, D14 and D15,
  and its check V3: "the Index loading rows show after the Library rows while the Index request
  runs. Then the Index rows replace them."
- [ADR 0040](../adr/0040-async-vm-runtime.md): screens do not call `cx.spawn`.
- [AGENTS.md](../../AGENTS.md): token discipline, "Every fix gets a guard", "A guard names its
  class and its ADR" and "Current-view state must update in place".

## Recorded Facts - 2026-10-24

An agent read each fact below on 2026-10-24 in the working tree at commit `e476192`.

### The two entry points

- `src/app/search_live.rs:89-106` `end_search_pause` is the live entry. It reads the field text,
  calls `live_run_step`, takes a generation with `toolbar_search_runs.next_run()` (line 101) and
  calls `show_toolbar_search_run`.
- `src/app/search_dispatch.rs:237-264` is the Enter entry. `submit_global_search` calls
  `cancel_pause` (line 240). `open_search_results_in_content_list` takes a generation with
  `next_search_run` (line 262) and calls `show_toolbar_search_run` (line 263). On the Music tab it
  calls no `select_tab`.
- Thus the two entries call the same function with a run of one new generation. After that call,
  the code of the two paths is the same.
- The live entry runs in the timer task of `present_after`
  (`src/presentation/delayed_presenter.rs:14-28`), with no `Window`. The Enter entry runs in the
  input event handler, with a `Window`.

### One run takes two generations

- `src/app/search_live.rs:113-145` `show_toolbar_search_run` mounts the page with the generation
  of the run (lines 119-125). Then it calls `restore_content_list_nav_entry` (line 133).
- `src/app.rs:864-887` `restore_content_list_nav_entry` calls
  `sync_search_results_detail_with_nav` (line 873). For a `Search` entry, it takes a second
  generation with `next_search_run` (line 883) and starts the Index part with it (line 884).
- `src/app/search_dispatch.rs:607-633` `sync_search_results_detail_with_nav` takes a third
  generation when the page query is not the query of the navigation entry (line 624).
- `src/app/search_live.rs:42-47` `SearchSlotRuns::next_run` cancels the token of the Index request
  in flight at each new generation.

### The Index result can stop at four silent points

- `src/app/search_dispatch.rs:499`: the run is not the current run of the slot.
- `src/app/search_dispatch.rs:502`: the query of the current `Search` entry is not the query of
  the result.
- `src/view_models/search_results/mod.rs:217`: `replace_index_results_for_run` gives `false` when
  the generation is not the page generation.
- `src/app/search_dispatch.rs:524-534`: a `Cancelled` error changes no page and shows no error.
- `src/app/search_dispatch.rs:356-363`: when the page query is not the query of the run,
  `begin_index_run` does not run. The request still goes. Its result then stops at the third point.

### The loading rows

- `src/ui/shells/search_results_inspector.rs:435` shows the loading rows only when
  `index_loading` is `true` and the active tab has no row.
- `src/view_models/search_results/mod.rs:835-843` `refresh_empty_state` sets no empty state while
  the tab has rows or the Index part loads.
- `src/ui/shells/search_results_inspector.rs:436` holds the count `0..3` in the renderer.
- Thus a query with Library rows never shows the loading rows. The walk of `survival` showed 2
  Library rows, so "no loading row" comes from this rule.

### The requests of `survival`

An agent sent the requests of one run to `https://api.musicindex.org` with `curl` on 2026-10-24.

- `GET /v1/search` with `type=feed`, `type=track` and `type=publisher`: HTTP 200 in 0.10 s,
  0.09 s and 0.06 s.
- The feed and track replies name 20 artists. `GET /v1/artist-names?limit=1&from=<key>` took
  0.05 s for each of 4 samples.
- Thus one run needs about 1.5 s. The walk waited more than 6 s after the last character.

## Candidates

The static reading proves that the two entries run the same code after `show_toolbar_search_run`.
It found no call that takes a third generation after the Index request starts. Thus it cannot
prove why the Index rows did not show. These are the candidates:

| Id | Candidate | Evidence | A test that separates it |
|---|---|---|---|
| K1 | The loading rows do not show after Library rows | Proved: `search_results_inspector.rs:435` | R87-7-01 and R87-7-02 |
| K2 | A later toolbar generation makes the result old, and the result stops at line 499 or as `Cancelled` | Not found in the code. Each `next_search_run` caller is a page change or a run | R87-7-05 and R87-7-06 replay the live and Enter sequences |
| K3 | The page generation or query is not the generation or query of the request, and the result stops at `mod.rs:217` | Possible only when `sync_search_results_detail_with_nav` builds a page between the mount and the Index start | R87-7-04 and R87-7-07 |
| K4 | The request does not complete before the walk ends | `curl` gives about 1.5 s for one run | V2 with the field list closed, and a run of a word with no Library row |
| K5 | The first run of a new query fails, and a second run of the mounted query works | The walk pressed Enter on the mounted `survival` page. In the script of the walk, each `field_set` waits 0.6 s, so a live run comes before each Enter | V1 and V3 of this packet |

The screenshot `~/.claude/v4vmm-shots/20261024-120204/26-live2-v4-enter-at-once.png` shows the
empty state of `haleen dri` 0.15 s after an Enter with no live run before it. MusicIndex gives no
row for `haleen dri`. Thus this image does not separate K4 from K5.

## Design

### Part A: the loading rows follow the rows

`SearchResultsInspectorPageVm` gains one method:

| Method | Rule |
|---|---|
| `index_pending_row_count(&self) -> usize` | `INDEX_PENDING_ROW_COUNT` while `index_loading` is `true`, else 0. The rows of the active tab do not change the count |

`INDEX_PENDING_ROW_COUNT` is a `pub(crate) const usize` in `src/view_models/search_results/mod.rs`
with the value 3, the count of today.

`render_active_result_list` shows the rows of the active tab, then
`index_pending_row_count()` loading rows, then the note. A tab with no row shows only the loading
rows, as today. The loading rows are after the result rows, so `scroll_to_item` still finds each
row by its index (packet 028 D2). The renderer reads the count from the view model and holds no
literal count.

### Part B: one generation for each run

`restore_content_list_nav_entry` splits into two functions:

- `TopApp::restore_content_list_page(&mut self, content_list_id, entry, cx)` holds each step of
  today apart from the Index start. Its first step is `toolbar_search_runs.gate.cancel_pause()`.
- `TopApp::restore_content_list_nav_entry(&mut self, content_list_id, entry, cx)` calls
  `restore_content_list_page`. For a `Search` entry it then builds a run with
  `next_search_run(SearchPageSlot::ToolbarSearch, query)` and calls `start_index_search_for_run`
  with it. Back, Forward, a breadcrumb and `open_music_page` keep this function.

`show_toolbar_search_run(run, cx)` changes:

1. It opens the `Search` entry of `run.text()`, as today.
2. It mounts `search_results_detail_for_run(run)` with the generation of `run`, when
   `is_newer_than` is `true`, as today.
3. It scrolls the toolbar page to the top, as today.
4. It calls `restore_content_list_page` with the `Search` entry of the text.
5. It calls `start_index_search_for_run(SearchPageSlot::ToolbarSearch, run, cx)` with the same
   `run`. It takes no second generation.

Thus a live run and an Enter run each take one generation. The page, the Index request and the
cancel token of the run have that generation.

### Part B: one function decides the result

`index_result_step` (`src/app/search_live.rs`) is a pure function. It holds no GPUI type.

```rust
pub(super) enum IndexResultStep {
    Apply,
    MountPage,
    DropOlderRun,
    DropOtherQuery,
    DropPageGeneration,
}

pub(super) fn index_result_step(
    current_run: bool,
    entry_matches: bool,
    page: Option<&SearchResultsInspectorPageVm>,
    generation: RunGeneration,
    query: &str,
) -> IndexResultStep
```

The first true row gives the step:

| Condition | Step |
|---|---|
| `current_run` is `false` | `DropOlderRun` |
| `entry_matches` is `false` | `DropOtherQuery` |
| `page` is `None` | `MountPage` |
| The page query is not `query` | `DropOtherQuery` |
| The page generation is not `generation` | `DropPageGeneration` |
| Each other case | `Apply` |

The success and the failure of `dispatch_index_search` call `index_result_step` one time each.
They replace the checks of lines 499, 502 and 505-510 with it. `MountPage` builds the page as
today, then applies the result. Only `Apply` and `MountPage` change the page. A `Cancelled` error
keeps the rule of packet 003 D14.

`SearchResultsInspectorPageVm::run_generation` loses its `#[cfg(test)]` attribute, because
`index_result_step` reads it.

## Orchestrator Decisions

- **D1. Part A is in this packet.** Check V3 of packet 003 requires the loading rows after the
  Library rows. The renderer rule at line 435 makes that impossible. The fix is at the view model
  owner and the shared shell, not in a screen.
- **D2. The count of loading rows is a view model fact.** The renderer keeps no literal count. The
  value stays 3.
- **D3. A run keeps one generation from its start to its result.** Packet 003 gave the restore a
  second generation, because it served Back and Forward too. A run that brings its own generation
  does not need a second one. Back and Forward still take a new generation (packet 003 D4).
- **D4. One pure function decides each Index result.** The four silent points become one table,
  and a test reaches each row. The table keeps each rule of packet 003. It adds no new rule.
- **D5. The packet does not claim the cause.** The static reading did not prove K2, K3, K4 or K5.
  The walk of V1 and V3 decides.
- **D6. The page states why it shows no Index rows.** The operator decided this on 2026-10-24.
  The packet adds no log line, because AGENTS.md "Logging" gives no log path for the interface.
  The function of decision D4 gives each step a typed reason.

  A result of the same query can stop at a step while the page of the current run has no Index
  result. Then the search page view model gives one note. The note names the query, the run
  generation, the page generation and the reason. A newer run of a different query gives no note,
  because its page replaces the old page. A test reaches each reason. If V1 fails, the note names
  the step.
- **D7. The guard is situational.** It cites ADR 0087 Decision 4 and is deleted when that decision
  is replaced.

## Required Changes

1. `src/view_models/search_results/mod.rs`: add `INDEX_PENDING_ROW_COUNT` and
   `index_pending_row_count`. Remove `#[cfg(test)]` from `run_generation`. Add the note of
   decision D6 as display state of the page: `record_index_result_stop` records it, and
   `index_notice` and `empty_state` give it. `src/view_models/search_results/index_result_note.rs`
   holds `IndexResultDiagnostic` and `IndexResultStopReason`.
2. `src/ui/shells/search_results_inspector.rs`: `render_active_result_list` takes the count in
   place of `index_loading: bool`. It shows the result rows, then the loading rows, then the note.
3. `src/app.rs`: split `restore_content_list_nav_entry` as "Part B: one generation for each run"
   says.
4. `src/app/search_live.rs`: add `IndexResultStep` and `index_result_step`. Change
   `show_toolbar_search_run` as the design says.
5. `src/app/search_dispatch.rs`: the success and the failure of `dispatch_index_search` call
   `index_result_step`.
6. `tests/architecture_tests.rs`: add the guard of case R87-7-08. Change the rule of guard R87-3-17
   that reads `restore_content_list_nav_entry` for `cancel_pause(`. It reads
   `restore_content_list_page` now. The rule keeps its meaning. Change the three guards that
   "Implementation Notes - 2026-10-24" names in the same way.

## Mechanical Acceptance Criteria

Each criterion is mechanical. Use the prefix `adr_0087_live_index_rows_` for each unit test. Each
test is beside its owner code, apart from R87-7-08. No test sends a request to a live service, and
no test waits for a timer.

| Case | Owner | Required proof |
|---|---|---|
| R87-7-01 | `src/view_models/search_results/mod.rs` | A page with 2 Library release rows calls `begin_index_run`. `index_pending_row_count()` gives 3, and `visible_rows` still gives the 2 rows. After `replace_index_results_for_run` of the same generation, it gives 0 |
| R87-7-02 | same | A page with no row calls `begin_index_run`. `index_pending_row_count()` gives 3 and `empty_state()` gives `None`. After `set_index_error_for_run` of the same generation, it gives 0. A page with `set_index_skipped` gives 0 |
| R87-7-03 | `src/app/search_live.rs` | For fixed values, `index_result_step` gives each step of the table: `DropOlderRun`, `DropOtherQuery` for the entry, `MountPage`, `DropOtherQuery` for the page, `DropPageGeneration` and `Apply` |
| R87-7-04 | same | A page of query `survival` and generation 2. A result of generation 2 and query `survival` gives `Apply`. Generation 1 gives `DropPageGeneration`. Query `surviva` gives `DropOtherQuery` |
| R87-7-05 | same | The live sequence: `text_changed` gives a ticket, and `pause_ended` gives `true`. `SearchSlotRuns::next_run` gives generation G. A page of `survival` with generation G calls `begin_index_run(G)`. `replace_index_cancel` keeps token T. Then `gate.is_current(G)` is `true`, T is not canceled, and `index_result_step` gives `Apply`. The page then holds the Index rows, and `index_pending_row_count()` gives 0 |
| R87-7-06 | same | The Enter sequence after R87-7-05 with the same text: a second `next_run` gives H. T is canceled. A result of G gives `DropOlderRun`. A result of H gives `Apply` |
| R87-7-07 | same | A page of `hal` with generation 1 is mounted. A run of `haleen` with generation 2 mounts its page. `index_result_step` for generation 2 and `haleen` gives `Apply`. The result of generation 1 for `hal` gives `DropOlderRun` |
| R87-7-D6 | `src/view_models/search_results/tests.rs` and `src/app/search_live.rs` | `DropOlderRun` gives the reason `OlderRun`, and `DropPageGeneration` gives `PageGeneration`. Each other step gives no reason. A page of the same query with no Index result records one note for each reason, with the query, the two generations and the reason. An applied result and an Index error clear the note. A result of another query, a result after the Index rows show, and a new page give no note |
| R87-7-08 | `tests/architecture_tests.rs` | The situational guard `adr_0087_live_index_rows_r87_7_08_one_generation_for_each_run` passes. It fails against a copy of `src/app/search_live.rs` with `next_search_run(` in `show_toolbar_search_run` |
| R87-7-09 | all | Each test of packets 001 to 005 still passes, with R87-3-01 to R87-3-19. Each test of `src/view_models/search_results/tests.rs` and `src/view_models/workspace/tests.rs` still passes |

The guard of R87-7-08 reads `production_source(&code_only(&source))` and enforces these rules:

- The source of `show_toolbar_search_run` holds `start_index_search_for_run(` and
  `restore_content_list_page(`. It holds no `next_search_run(`, no `next_run(` and no
  `restore_content_list_nav_entry(`.
- The source of `restore_content_list_page` holds `cancel_pause(`. It holds no
  `next_search_run(` and no `start_index_search_for_run(`.
- The source of `dispatch_index_search` holds `index_result_step(` two times. It holds no
  `gate.is_current(` and no `index_search_result_is_current(`.
- `src/ui/shells/search_results_inspector.rs` holds `index_pending_row_count()` and no `0..3`.

The failure message of the guard is this text:

```text
ADR 0087 Decision 4 (results show while the curator types): <file> <violation>. A toolbar run
keeps one generation from its start to its Index result. show_toolbar_search_run starts the
Index part with its own run. Only restore_content_list_nav_entry takes a new run, for Back,
Forward and a page open. Each Index result goes through index_result_step. The renderer reads
the loading row count from the view model.
```

## Visual Acceptance Criteria

These are for the operator. No test proves them. Examine V1 to V4 in Dark, at normal width.
Examine V1 also at narrow width.

- V1 (re-walk of packet 003 V3): on the Discover page, type `survival` and stop. After the pause,
  the 2 Library rows show. Three loading rows show after them while the Index request runs. Then
  the Index rows replace the loading rows, for example "SURVIVAL OF THE FITTEST".
- V2: type `fittest` and stop. No Library row matches. The loading rows show, then the Index rows.
- V3: press Enter on the mounted `survival` page. The same Index rows show again. No row shows
  two times.
- V4 (re-walk of the second part of packet 003 V3): with MusicIndex not reachable, type `haleen`,
  then `haleen d`. One error shows after the Library rows, for `haleen d` only.

## Exclusions

- No change to the pause, the token `Motion::SearchPause` or `LiveRunGate`.
- No change to the field list (packets 004, 005 and 008).
- No change to the Index plan, to the Library SQL or to the request count of a run.
- No diagnostic log (decision D6). The note of decision D6 is display state of the search page.
- No schema change, no migration and no configuration key.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 4, and
  [packet 003](adr-0087-task-003-live-results.md) with its design and its decisions.
- `src/app/search_live.rs`, `src/app/search_dispatch.rs` and `src/app.rs`
  (`restore_content_list_nav_entry` and each of its callers).
- `src/view_models/search_results/mod.rs`, `src/view_models/search_results/tests.rs` and
  `src/view_models/live_run.rs`.
- `src/ui/shells/search_results_inspector.rs` and `src/ui/shells/search_result_rows.rs`.
- `tests/architecture_tests.rs`: the guard
  `adr_0087_live_results_r87_3_17_pause_token_and_no_screen_timer`.

## Checks

```bash
cargo test --lib adr_0087_live_index_rows_
cargo test --lib adr_0087_live_results_
cargo test --lib search_results
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
revert, the loading rows show only on a page with no row, and the restore takes a second
generation, as after packet 003.

## Responsiveness Concerns

Part A adds feedback: a page with Library rows now shows that the Index request runs. The
orchestrator adds no new concern to the list of the overhaul plan.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0087-task-007-live-run-index-rows.md`
- ADR 0087 Decision 4 and its invariants, and packet 003 with its "Design" section
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use
  the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- Use each name of this packet exactly.
- Tests use fixed values. No test sends a request to a live service, and no test waits for a
  timer.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or
  `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- `src/presentation/delayed_presenter.rs` and `src/presentation/async_command_presenter.rs`.
- `src/view_models/live_run.rs`, apart from a read.
- The field list files of packets 004, 005 and 008.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet and `docs/architecture/source-map.md`.

Acceptance criteria:
- Each case R87-7-01 to R87-7-09 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - <date>" section.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns,
and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- You find a cause of the missing Index rows that "Candidates" does not list. Give its file and
  line before you change code.
- A test of R87-7-05, R87-7-06 or R87-7-07 fails against the code before your change. Report the
  step that `index_result_step` gives and the line that stops the result.
- A recorded fact of this packet is not true in the working tree.
- A guard other than R87-3-17 and R87-7-08 fails because of this change.
- A change needs a new architecture decision, a migration or a file in "Do not touch".

## Operator Visual Check

The check needs a desktop session, the built app, network access to `https://api.musicindex.org`
and a Library with the album "Survival Guide". Steps 7 to 11 make MusicIndex not reachable.

1. Build the app: `cd ~/build/v4vmm && cargo build --bin v4vmm`.
2. Start the app: `./target/debug/v4vmm`.
3. Open the Music tab. Click Discover in the sidebar.
4. V1: click in the toolbar search field. Type `survival` and stop. Do not press Enter.
   - Look at: the 2 Library rows show after the pause. Three loading rows show under them. In
     3 s or less, the Index rows replace the loading rows. "SURVIVAL OF THE FITTEST" is one row.
   - Wrong: no loading row under the Library rows, or no Index row 10 s after the last character.
   - Repeat at narrow width. The loading rows wrap inside the column.
5. V2: select the text and type `fittest`. Stop.
   - Look at: the loading rows show, then the Index rows.
   - Wrong: the empty state "No releases" shows, and no Index row follows.
6. V3: select the text, type `survival` and press Enter at once.
   - Look at: the same Index rows show. No row shows two times.
   - Wrong: a row shows two times, or the Index rows do not show.
7. V4: close the app.
8. Copy the configuration:
   `cp ~/.config/v4vmm/config.toml ~/.config/v4vmm/config.toml.v4-check`.
9. In `~/.config/v4vmm/config.toml`, set `musicindex_endpoint = "http://127.0.0.1:9"`.
10. Start the app: `./target/debug/v4vmm`. Open Discover.
11. Type `haleen`, wait 1 s, then type ` d`. Stop.
   - Look at: one error shows after the Library rows, for `haleen d` only.
   - Wrong: two errors show, or the error names `haleen`.

Cleanup:

1. Close the app.
2. Restore the configuration:
   `mv ~/.config/v4vmm/config.toml.v4-check ~/.config/v4vmm/config.toml`.
3. Start the app and delete the text in the search field.

The check changes no stored data. Step 9 changes the configuration file. Step 2 of the cleanup
restores it.

## Operator Answers - 2026-10-24

- The packet adds the diagnostic now, not in a later packet (decision D6).

## Implementation Notes - 2026-10-24

Orchestrator decisions of 2026-10-24. The implementing agent found that three guards outside
R87-3-17 and R87-7-08 fail because of this packet. The orchestrator then decided these items:

1. The guard `adr_0084_search_list_submit_restores_through_the_back_path` reads
   `self.restore_content_list_page(content_list_id,&FrameNavigationEntry::Search(query),cx,);` in
   `show_toolbar_search_run`. It also requires that `restore_content_list_nav_entry` calls
   `restore_content_list_page`. Before this change, the guard required
   `restore_content_list_nav_entry` in `show_toolbar_search_run`, and R87-7-08 forbids it.
2. Guard R18-08 reads the body of `restore_content_list_page` for
   `library.hydrate_detail_from_nav(entry, cx);`.
3. The guard `adr_0048_index_search_is_async_and_vm_owned` requires
   `fn index_pending_row_count(` in the view model and `vm.index_pending_row_count()` in the
   renderer, in place of `is_index_loading`.
4. `SearchSlotRuns::is_current_run` gives `current_run`. `index_search_result_is_current` is
   now `index_result_entry_matches`, with the same body. The end marker of R87-3-17 reads
   `fnindex_result_entry_matches(`.
5. The note of decision D6 is `Option<(IndexResultDiagnostic, EmptyStateDisplay)>` on the page.
   The reasons are `OlderRun` and `PageGeneration`. `DropOtherQuery` gives no reason, because
   the page of another query does not record a note. A `Cancelled` error gives no note (packet
   003 decision D14). The page records the note only when the result query is the page query and
   the page shows no Index rows and no Index error.

   An applied result, an Index error and a new page clear the note. The note shows after the rows
   through `index_notice`. A page with no row and no loading shows it as its empty state. The text
   is
   `MusicIndex rows for "survival" did not show`, then one of these sentences:
   - `OlderRun`: `The result came from search run 3, but a newer search run started. The page shows run 4.`
   - `PageGeneration`: `The result came from search run 3, and the page shows run 4.`

Deviations from "Do not touch":

- `src/view_models/live_run.rs` gets `impl Display for RunGeneration`. The note must give the
  run number, and the number is a private field of that module. The change adds no state and no
  rule.

Other changes:

- `is_index_loading` and `index_result_diagnostic` are `#[cfg(test)]`, because no production
  code calls them. The renderer reads `index_pending_row_count`.
- A `MountPage` step on the saved-search slot changes nothing, as before this packet.
- `docs/architecture/source-map.md` has a row for the Index result step and the note.

The cause of the missing Index rows is not proved. The agent examined
`present_command` (`session.accepts`), the dispatch of `AsyncCommandRunner`, `request_reuse`,
the field list and each write of `search_results_detail`, and none of them is a cause. K4 and K5
stay open for V1 and V3. If V1 fails, the note names the step that stopped the result.
