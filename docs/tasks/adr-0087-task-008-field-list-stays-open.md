# ADR 0087 Task 008: The Field List Stays Open For The Next Term

Status: Ready - 2026-10-24. The operator answered Q1 on 2026-10-24, and the orchestrator decided Q2 by its recommendation (see "Operator Answers").

## Goal

Keep the field list open after a completed term and a space. Then the list shows the fields again,
and the curator can build a query of more than one term.

Two checks of the operator walk of 2026-10-24 failed because of one rule:

- Packet 003 V6: after Down and Enter on a value row, the text is `artist:"Able and The Wolf" `.
  The search runs, and the field list closes. The criterion requires that the list stays open.
- Packet 004 V6: the operator typed `in:index ` with a space at the end. The list closes, so the
  note "Not available with in:index" cannot show.

After this packet, the list shows the 11 field rows when the cursor is after a space. The curator
can type the space, or a value row can add it. Escape, blur and Enter with no selected row
still close the list.

## Authority

- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 3: "While the curator types a
  field name, the list shows the fields that start with that text." An empty field name is a
  prefix of each field. The list is the help for the next term.
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 1: the terms of one query all
  apply together. A query of more than one term is the normal case.
- [Packet 004](adr-0087-task-004-field-list.md): this packet replaces three items of packet 004.
  They are its decision D8, its case R87-4-14 and one part of its decision D9. That part closes
  the list after a row goes into the text. D8 says "In `Typing`, an empty name prefix in a text that is
  not empty gives no row".
- [Packet 003](adr-0087-task-003-live-results.md) V6 and packet 004 V6: the failed checks.
- [AGENTS.md](../../AGENTS.md): "Every fix gets a guard", "A guard names its class and its ADR",
  "Delete dead code" and "An acceptance criterion states how it is checked".

## Recorded Facts - 2026-10-24

An agent read each fact below on 2026-10-24 in the working tree at commit `e476192`.
`$REG` is `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f`.

### The cause: an empty field name after other text shows no row

- `src/view_models/search_field_list.rs:350-355`: for `CursorPartial::FieldName { prefix }`,
  `hide_all` is `true` when `prefix` is empty, `state.opening` is `FieldListOpening::Typing`, and
  the text holds a character that is not whitespace. Then the list has no row.
- `src/view_models/search_field_list.rs:384-387`: with no row and no notice, `FieldListVm::build`
  gives `None`. The list does not show.
- `src/view_models/search_query.rs:797-812`: `cursor_term` gives
  `CursorPartial::FieldName { prefix: "" }` when no term holds the cursor or ends at it. A cursor
  after a space is in this condition.
- `src/view_models/search_field_list.rs:106-131` `FieldListState::apply`: `Change` sets `Typing`
  when the field has the focus. Only `Focus` and `Click` set `Requested`.
- Thus each typed space after a term gives `Typing` and an empty prefix, and the list closes.
  This is the cause of packet 004 V6. `in:index ` at cursor 9 has no notice, because its scope is
  `IndexOnly` and not `Conflict`.

### The same cause after a value row goes into the text

- `src/view_models/search_field_list.rs:365-369` and `:533-545`: the edit of a value row adds one
  space when the value is the last item of its term. R87-5-10
  (`src/view_models/search_field_list.rs:1366-1402`) proves that Tab on `Haleen Moss` gives
  `artist:"Haleen Moss" ` with the cursor at the end.
- `src/app/search_field_list.rs:269-278` `apply_edit` calls `replace` in `input.update`, then
  `state.apply(FieldListEvent::Inserted)`. `src/view_models/search_field_list.rs:124-126` sets
  `Closed` for `Inserted`.
- `$REG/gpui-pre-0.3.1/src/app/context.rs:765-777`: `emit` puts the event in `pending_effects`.
  GPUI sends it after the current update. Thus the `Change` of `replace` comes after `Inserted`.
  `src/app/search_field_list.rs:97-104` then sends `FieldListEvent::Change`, and the state is
  `Typing`.
- `src/app/search_field_list.rs:529-538` (R87-4-17) proves this order. After Down and Enter put
  `kind:` into `artist: k`, the list shows the values of `kind:`. With the other order, `Inserted`
  closes the list and the test fails.
- Thus `Inserted` closes the list for a moment only. The next `Change` sets `Typing`, and the rule
  of `:350-355` hides each row, because the cursor is after the added space. This is the cause of
  packet 003 V6.

### The tests that hold the defect

- `src/view_models/search_field_list.rs:1155-1170` R87-4-14 asserts that `haleen ` at cursor 7
  in `Typing` gives `None`.
- `src/view_models/search_field_list.rs:1129-1137` R87-4-13 asserts that `Inserted` gives
  `Closed`.
- `src/view_models/search_field_list.rs:352` is the only production reader that tells `Typing`
  from `Requested`. A search of `src` and `tests` for `FieldListOpening` finds no other reader.

### Candidates that the facts exclude

- A blur from the live run. Packet 003 V1 passed on 2026-10-23: after the live run opens the
  search page, the caret stays in the field and blinks. Case R87-8-05 and the visual check V1 of
  this packet examine it again.
- The `submitted_text` rule (`src/app/search_field_list.rs:93-108`). Enter on a selected row gives
  `FieldListKeyOutcome::Apply`, and the capture listener stops the key. Thus no `PressEnter`
  comes, and `submitted_text` stays `None`.

## Design

### The rule

An empty field name prefix gives one `Field` row for each row of `QUERY_FIELDS`, in each open
state. This is the same rule as for an empty text. Remove `hide_all` and its condition from
`FieldListVm::build`.

### The open state

After the change of "The rule", no code tells `Typing` from `Requested`. Replace the two variants
with one variant `FieldListOpening::Open`. "Delete dead code" in AGENTS.md requires this change.

| Event | Result after this packet |
|---|---|
| `Focus` | `focused` is `true`. `opening` is `Open`. `selected` is `None` |
| `Click` | `opening` is `Open`. `selected` is `None` |
| `Change` | When `focused`, `opening` is `Open`. `selected` is `None` |
| `CursorMoved` | `selected` is `None`. `opening` does not change |
| `Inserted` | `opening` is `Open`. `selected` is `None` |
| `Escape` | `opening` is `Closed` |
| `Submit` | `opening` is `Closed` |
| `Blur` | `focused` is `false`. `opening` is `Closed` |

`Inserted` keeps the list open by itself. Thus the result does not depend on the order of
`Inserted` and the `Change` of `replace`.

### What does not change

- The rows, the edits, the notices, the keys and the composite of packets 004 and 005.
- Tab with no selected row puts the first available row into the text (packet 004 D7). See Q2.
- Escape closes the list. The next change of the text, a click in the field or a focus opens it
  again.
- Enter with no selected row emits `PressEnter`, runs the search and closes the list.
- `submitted_text` keeps the list closed after Enter runs the search.

## Orchestrator Decisions

- **D1. One rule for an empty field name.** An empty prefix shows each field, after a space, in
  an empty text, and after a value row. ADR 0087 Decision 3 says that the list shows the fields
  that start with the typed text, and each field starts with an empty text.
- **D2. `Inserted` opens the list, and does not close it.** The state then does not depend on the
  order of two events. The order is a fact of GPUI, not a rule of this app.
- **D3. `Requested` and `Typing` become `Open`.** With D1, the two variants give the same rows.
  A variant that changes nothing is dead code.
- **D4. The regression test is situational.** It cites ADR 0087 Decision 3. When an ADR replaces
  Decision 3, delete the test with that ADR.

## Questions

These questions need an operator answer. ADR 0087 does not decide them. The design of this
packet follows each recommendation.

- **Q1. Does the full list show after a plain word and a space, for example `haleen `?**
  - A: yes. Each space after a term shows the 11 fields.
  - B: only after a field term, for example `kind:single ` or `in:index `. After `haleen `, the
    list shows no row until the curator types a letter. A click in the field still shows the 11
    fields.
  - Recommendation: A. ADR 0087 Decision 3 makes no difference between a word and a field. The
    list has no focus handle, so it does not stop typing. Escape closes it. With B, the parser must
    give the kind of the term before the cursor, and `Typing` and `Requested` stay.
  - Consequence of A: while the curator types words, the list covers the top of the search results
    under the field. Packet 003 V6 already expects the list to stay in the window while the
    results change.
- **Q2. What does Tab do with no selected row after a space?**
  - A: as today, Tab puts the first available row into the text. After `haleen `, Tab gives
    `haleen artist:`.
  - B: with an empty field name prefix and no selected row, Tab gives `Pass`. Tab then puts a row
    into the text only after Down, Up or a typed letter. This also changes Tab in an empty field.
  - Recommendation: A. It keeps packet 004 D7 and the scope of this fix. Before this packet, Tab
    after a space did nothing, because the list was closed and the single-line input does nothing
    with Tab (packet 004 D3). With A, Tab after a space inserts `artist:`. If the operator finds
    that surprising in the walk, a later packet can apply B.

## Required Changes

1. `src/view_models/search_field_list.rs` `FieldListVm::build`: remove `hide_all`. An empty
   prefix gives the rows of `fields_with_prefix("")`.
2. `src/view_models/search_field_list.rs`: replace `FieldListOpening::Requested` and
   `FieldListOpening::Typing` with `FieldListOpening::Open`. Give it the doc comment "The list
   shows." Change `FieldListState::apply` as the table of "The open state" says. `Inserted` sets
   `Open` and `selected` `None`.
3. Change each test that names `Requested` or `Typing` to `Open`. Item 4 removes some
   assertions. Keep each other assertion of R87-4-01 to R87-4-18 and R87-5-01 to R87-5-20.
4. Delete the test `adr_0087_field_list_r87_4_14_typing_after_a_space_shows_no_full_list`. Keep
   its assertions for the selection `0..2` in the new test of R87-8-01. In
   `adr_0087_field_list_r87_4_13_events_change_the_state`, remove `Inserted` from the loop that
   expects `Closed`.
5. Add each test of "Mechanical Acceptance Criteria".
6. `src/app/search_field_list.rs`: change the doc comment of `submitted_text` and the comment in
   the `Change` arm only if they name `Typing` or `Requested`. Do not change `apply_edit`.
7. Change no other source file.

The orchestrator then makes these document changes in the same commit:

- [Packet 004](adr-0087-task-004-field-list.md): mark D8, the "after a row goes into the text"
  part of D9, the sentence "After `single `, it shows no row and closes", R87-4-14 and the words
  "the list closes" of V4 as replaced by this packet.
- [Pending human checks](../pending-human-checks.md): retire packet 003 V6 and packet 004 V6 into
  V1 and V2 of this packet. Add V1 to V5 of this packet.
- The [run record](../reviews/orchestrated-run-2026-10-23.md) and the
  [ADR 0087 phase plan](../plans/adr-0087-search-fields-phase-plan.md): name this packet.

## Mechanical Acceptance Criteria

Each criterion is mechanical. A unit test proves it. Use the prefix `adr_0087_field_list_` for
each test. Cases R87-8-01 to R87-8-04 are in `src/view_models/search_field_list.rs`. Case R87-8-05
is in `src/app/search_field_list.rs`. "Open" means a `FieldListState` with `focused` `true` and
`opening` `Open`.

| Case | Required proof |
|---|---|
| R87-8-01 | `FieldListVm::build` for `haleen ` at cursor 7, Open, gives 11 `Field` rows in the order of `QUERY_FIELDS`. Each `edit.range` is `7..7`. Each text of R87-4-14 with the selection `0..2` gives `None` |
| R87-8-02 | `FieldListVm::build` for `in:index ` at cursor 9, Open, gives 11 rows. `is:`, `playlist:` and `has:` are `Unavailable(ScopeConflict)` with the note "Not available with in:index". `Down` from no selection gives `Select(0)` and never selects these three rows |
| R87-8-03 | `FieldListVm::build` for `kind:single ` at cursor 12 and for `artist:"Haleen Moss" ` at its end, Open, gives 11 `Field` rows. The `Tab` edit of the selected row `release:` in `kind:single ` gives `kind:single release:` with the cursor 20 |
| R87-8-04 | `FieldListState::apply`: from Open, `Inserted` gives Open and `selected` `None`. `Inserted` then `Change` gives Open. `Change` then `Inserted` gives Open. `Escape`, `Submit` and `Blur` still give `Closed` |
| R87-8-05 | The regression test of ADR 0087 Decision 3, below |

R87-8-05 is a `gpui::TestAppContext` test with the setup of R87-4-17. Its name is
`adr_0087_field_list_r87_8_05_list_stays_open_for_the_next_term`. Its doc comment names "ADR 0087
Decision 3: the field list is the help for the next term" and names packet 003 V6 and packet 004
V6. It proves these facts:

1. Type `kind:`, then press `down` and `enter`. The text is `kind:album ` with the cursor 11. The
   list shows 11 `Field` rows. The input keeps the focus. No `PressEnter` comes.
2. Type `i n`. The list shows one row `in:`. Press `tab`, then type `i n d e x space`. The text is
   `kind:album in:index `. The list shows 11 rows, and the row `is:` is not available.
3. With the text `kind:`, `click_row(2)` gives `kind:single ` and a list of 11 `Field` rows.
4. Press `escape`. The list closes, and the text stays. Type `a`. The list shows again.
5. Clear the text, type `haleen`, and press `enter` with no selected row. One `PressEnter` comes,
   and the list closes.

Each assertion that the list shows has this failure message:

```text
ADR 0087 Decision 3 (the field list is the help for the next term): the list closed after "<text>".
An empty field name after a space shows each field. Do not hide the rows in FieldListVm::build, and do not close the list on FieldListEvent::Inserted.
```

## Visual Acceptance Criteria

Each criterion is visual. Only a person can examine it in the running app. No test proves it.

- V1 (packet 003 V6 again): type `artist:` in an empty field. With the field list open, press
  Down and Enter. The row goes into the text with a space after it. The list stays open under the
  field and shows the 11 fields while the results change.
- V2 (packet 004 V6 again): type `in:index ` by hand. The list stays open. The rows `is:`,
  `playlist:` and `has:` show the note "Not available with in:index", and Down skips them.
- V3: type `kind:`, then click `single`. The text is `kind:single `. The list shows the 11 fields,
  and the caret blinks in the field.
- V4: with the list open after a space, press Escape. The list closes. Type one letter. The list
  opens again.
- V5: with the list open after a space, press Enter with no selected row. The search runs, and the
  list closes.

## Exclusions

- Packet 004 V1: the list shows 9 of 11 rows and has no scroll bar. The visual overhaul owns it.
- Packet 005 V7: a long name wraps, but its row keeps one line of height. The visual overhaul owns
  it.
- Packet 003 V3: a live run shows no Index rows. It is a different defect.
- No change to the rows, the edits, the notices, the keys or the composite.
- No change to `src/view_models/search_query.rs`.
- No change to the live run, the pause or the run generation of packet 003.
- No change to gpui-kit or gpui-component.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decisions 1 and 3.
- [Packet 004](adr-0087-task-004-field-list.md) and
  [packet 005](adr-0087-task-005-field-list-values.md).
- `src/view_models/search_field_list.rs`: `FieldListOpening`, `FieldListState::apply`,
  `FieldListVm::build` and the tests R87-4-01 to R87-4-16 and R87-5-03 to R87-5-15.
- `src/app/search_field_list.rs`: `SearchFieldListView::new`, `apply_edit` and the tests R87-4-17,
  R87-4-18 and R87-5-18.
- `src/view_models/search_query.rs` `cursor_term`, to read only.

## Checks

```bash
cargo test --lib adr_0087_field_list_
cargo test --lib adr_0087_field_values_
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
- This packet: `docs/tasks/adr-0087-task-008-field-list-stays-open.md`
- ADR 0087 Decisions 1 and 3
- Packets 004 and 005 of ADR 0087
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes" items 1 to 7.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment in ASD-STE100 Simplified Technical English. Use the shared skill at
  `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions" and the answers to Q1 and Q2.
- Tests use fixed text. No test sends a request or opens a database.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or
  `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- `src/view_models/search_query.rs`, `src/ui/composites/search_field_list.rs` and
  `src/app/tab_bar.rs`.
- gpui-kit, gpui-component and `Cargo.toml`.
- Any ADR and any document other than this packet.

Acceptance criteria:
- Each case R87-8-01 to R87-8-05 has a passing test.
- Each command in "Checks" is Green.
- The packet has an "Implementation Result - <date>" section.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns,
and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- The operator did not answer Q1 or Q2.
- A recorded fact of this packet is not true in the working tree.
- R87-8-05 step 1 shows that the input loses the focus after the edit.
- A change needs a file in "Do not touch".

## Operator Visual Check

The check needs a desktop session, the built app and a Library with one or more artists. Step 4
needs MusicIndex or a Library artist for the value rows of `artist:`.

1. Build the app: `cd ~/build/v4vmm && cargo build --release`.
2. Start the app: `./target/release/v4vmm`.
3. Open the Music tab. Click in the empty toolbar search field. The field list shows.
4. V1: type `artist:`. Wait for the value rows. Press Down, then Enter.
   - Look at: the text is `artist:"<name>" ` with a space at the end. The list stays open under
     the field and shows the 11 fields. The results change under the list.
   - Wrong: the list closes, or the list shows no row.
5. V2: select the full text and delete it. Type `in:index ` by hand, with the space.
   - Look at: the list stays open. The rows `is:`, `playlist:` and `has:` show the note "Not
     available with in:index". Press Down more than 11 times. The selection never stops on these
     three rows.
   - Wrong: the list closes after the space, or Down selects one of the three rows.
6. V3: delete the text. Type `kind:`, then click `single`.
   - Look at: the text is `kind:single `. The list shows the 11 fields. The caret blinks in the
     field.
   - Wrong: the list closes, or the caret stops.
7. V4: press Escape. The list closes, and the text stays. Type `a`. The list opens again and shows
   `artist:` and `after:`.
   - Wrong: the list stays open after Escape, or it does not open again.
8. V5: delete the `a`. With the list open after the space, press Enter.
   - Look at: the search for `kind:single` runs, and the list closes.
   - Wrong: Enter puts a row into the text, or the list stays open.

Cleanup:

1. Delete the text in the search field.
2. Close the app.

The check changes no file and no stored data.

## Operator Answers - 2026-10-24

- Q1: A. The full list shows after each term and a space, also after a plain word such as `haleen `.
- Q2: the orchestrator decided A by its recommendation, inside the answer of Q1. Tab with no selected row puts the first available row into the text (packet 004 D7). If the operator finds this wrong in the walk, a later packet applies B.
