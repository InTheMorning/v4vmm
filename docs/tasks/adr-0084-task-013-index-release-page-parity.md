# ADR 0084 Task 013: Index Release Page Parity

Status: Implemented - 2026-10-16. Mechanical checks Green. The operator visual check (V1 to V6) is open.

## Goal

An Index release page works as a Library release page works. A click on a track row opens its track page. The Description section opens and closes. An Index track page shows its description.

A long description that is collapsed shows its first lines, with "Show more" at the end. This rule applies to each release page and each track page, for both origins.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 4: one page for each item, whatever its origin. The origin selects only the data source.
- [ADR 0047](../adr/0047-library-search-unification.md) item 8, with its amendment of 2026-10-16: a collapsed long description shows a preview and "Show more".
- The operator decided on 2026-10-16 to do this packet before packet 007.

## Recorded Facts - 2026-10-16

- The operator found three defects during the task 006 check. On an Index release page, a click on a track row does nothing. The "hide" control and the arrow of the Description section do nothing. A long description shows in full, with no collapse.
- `index_feed_track_rows` in `src/app/search_dispatch.rs` builds each row with `ReleaseTrackRowSlot` and sets no `on_click`. The Library release page sets one (`src/ui/shells/library/feed_detail.rs`).
- `TopApp::open_track_in_frame(content_frame_id, &TrackKey, label, cx)` in `src/app/search_dispatch.rs` opens a track page by its key, from any origin. `TrackKey::from_index_track_target(feed_guid, track_guid)` builds the key. `apply_track_page_source` then selects the Library source or starts the Index fetch. `track_guid_from_view` and `feed_guid_from_view` give the GUIDs of a `TrackView` and a `FeedView`.
- `index_feed_description_panel` in `src/app/search_dispatch.rs` builds a `DisclosureTextPanel` with `collapsed: false` and no `on_toggle`. It keeps no description state.
- `index_track_detail_slots` sets no `description_panel`, but `TrackView` has a `description` field. So an Index track page shows no description.
- The Library pages keep a `DescriptionState` for each release (`LibraryViewModel::album_description_states`, keyed by feed id) and for each track (`track_description_states`). `DescriptionState` and `DESCRIPTION_AUTO_COLLAPSE_LINES` (5) are in `src/view_models/library.rs`. A collapsed state hides the full body (`DisclosureTextPanel` in `src/ui/composites/disclosure_group.rs`).
- `description_line_count` counts source lines only. A long paragraph with no line break counts as one line, so it never collapses.
- `MultilineText` in `src/ui/primitives/multiline_text.rs` keeps at most `max_lines` source lines and wraps them with `wrap_lines()`.

## Design

### The description display

The view model gives one display for each description. Add it beside `DescriptionState` in `src/view_models/library.rs`:

```rust
/// What a description panel shows (ADR 0047 item 8, amended 2026-10-16).
pub(crate) struct DescriptionDisclosureDisplay {
    /// The header shows the closed chevron and "show".
    pub(crate) collapsed: bool,
    /// The text to show. `None` shows no body.
    pub(crate) body: Option<String>,
    /// "Show more" at the end of a preview. `None` when the full text shows.
    pub(crate) show_more: Option<DescriptionShowMoreDisplay>,
}

pub(crate) struct DescriptionShowMoreDisplay {
    pub(crate) label: &'static str,      // "Show more"
    pub(crate) a11y_label: &'static str, // "Show the full description"
}

impl DescriptionDisclosureDisplay {
    pub(crate) fn project(text: &str, state: DescriptionState) -> Self;
}
```

The projection rules:

| State | Long description | Short description |
|---|---|---|
| Visible (`AutoExpanded`, `UserExpanded`) | Full text, no "Show more", header open | Full text, no "Show more", header open |
| Collapsed (`AutoCollapsed`, `UserCollapsed`) | Preview, "Show more", header closed | No body, no "Show more", header closed |

- A description is long when `description_line_count` is more than `DESCRIPTION_AUTO_COLLAPSE_LINES`.
- Change `description_line_count`: each source line counts as its estimated wrapped lines, `ceil(chars / DESCRIPTION_CHARS_PER_LINE)`, with a minimum of 1. Add `DESCRIPTION_CHARS_PER_LINE: usize = 90`. A long paragraph thus collapses too.
- The preview is the first `DESCRIPTION_AUTO_COLLAPSE_LINES` estimated lines. The view model cuts the text at the last word boundary inside that budget and adds "…". It never cuts inside a word, and it never cuts a multi-byte character.

### The panel composite

`DisclosureTextPanelDisplay` takes the projection. It replaces `body: SharedString` and `collapsed: bool` with `collapsed: bool`, `body: Option<SharedString>` and `show_more: Option<(SharedString, SharedString)>` (label, accessibility label). The composite renders:

1. the header, as today, with `on_toggle`,
2. the body, when present, with `MultilineText` and `wrap_lines()`,
3. "Show more", when present, as a `UiButton` with `ControlStyle::Ghost`, its label and its accessibility label. Its click calls the same `on_toggle` handler.

The renderer decides nothing. It shows what the display gives.

### The Index description state

`TopApp` keeps a `BTreeMap<String, DescriptionState>` for Index releases, keyed by feed GUID, and one for Index tracks, keyed by the Index track target (`TrackKey::index_target`). The first projection uses `DescriptionState::project(description_line_count(..))`, as the Library does. A toggle stores `state.toggle()` and calls `cx.notify()`. The state lives for the session only. It is not stored.

### The Index track rows

`index_feed_track_rows` sets `ReleaseTrackRowSlot::on_click` on each row that has a track GUID. The click builds `TrackKey::from_index_track_target(feed GUID, track GUID)` and calls `open_track_in_frame` on the content-list frame, with the track title as its label. A row with no track GUID gets no click, as today. The track opens in the current path, so Back returns to the release page.

### The Index track page description

`index_track_detail_slots` sets `description_panel` from `TrackView::description`, with the Index track state above. Use the same panel, the same projection and the same id pattern as the Index release page.

## Required Changes

1. Add `DescriptionDisclosureDisplay`, `DescriptionShowMoreDisplay`, `DESCRIPTION_CHARS_PER_LINE` and the preview function in `src/view_models/library.rs`. Change `description_line_count` as "The description display" gives.
2. Change `DisclosureTextPanelDisplay` and `DisclosureTextPanel` as "The panel composite" gives.
3. Change the three present callers to the projection: the Library release page (`feed_detail.rs`), the Library track page (`track_detail.rs`) and the Index release page (`search_dispatch.rs`).
4. Add the Index description states in `TopApp`, and give the Index release panel its `on_toggle`.
5. Give each Index track row with a track GUID its click, as "The Index track rows" gives.
6. Give the Index track page its description panel.
7. Add the tests of the mechanical criteria. Update each test that asserts the old line count or the old panel fields.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_index_page_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R13-01 | `DescriptionDisclosureDisplay::project` gives the four cases of the table: long and visible, long and collapsed, short and visible, short and collapsed |
| R13-02 | The preview of a long description ends with "…" and cuts at a word boundary. It keeps a multi-byte character whole, and it is shorter than the full text |
| R13-03 | `description_line_count` counts a single paragraph of 600 characters as more than `DESCRIPTION_AUTO_COLLAPSE_LINES` lines, and a short paragraph as 1 |
| R13-04 | The Index description state of a feed GUID starts from `DescriptionState::project` and changes with each toggle. A test calls it through a pure function or a view-model method, with no GPUI |
| R13-05 | The Index track row click builds the key `TrackKey::from_index_track_target(feed GUID, track GUID)`, through a pure function that a test can call. A track with no GUID gives no key |
| R13-06 | A guard names ADR 0084 Decision 4: `index_feed_track_rows` sets `on_click`, and `index_feed_description_panel` takes an `on_toggle` handler |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: on an Index release page, a click on a track row opens its track page. Back returns to the release page.
- V2: on an Index release page with a long description, the Description section shows its first lines and "Show more". "Show more" opens the full text. The header "hide" closes it again to the preview.
- V3: on an Index release page with a short description, the full text shows. The header "hide" hides it, and "show" shows it again.
- V4: a Library release page with a long description shows the same preview and "Show more".
- V5: an Index track page with a description shows it, with the same rules.
- V6: Light and Dark, and a larger scale, show no clipped preview, "Show more" or header.

## Exclusions

- No other change to the release page or the track page.
- No stored description state. The state lasts for one session.
- No change to the Library track row click.
- No release kind, no scope and no singles groups. Packet 007 owns them.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md), the [source map](../architecture/source-map.md), the [phase plan](../plans/adr-0084-music-column-browser-phase-plan.md) and the archived [task 006](archive/adr-0084-task-006-discover.md), with its Orchestrator Review.
- `src/view_models/library.rs` (`DescriptionState`, `description_line_count`, the album and track description states, and their tests).
- `src/ui/composites/disclosure_group.rs`, `src/ui/primitives/multiline_text.rs`, `src/ui/primitives/section_header.rs`, `src/ui/control_styles.rs`.
- `src/ui/shells/library/feed_detail.rs`, `src/ui/shells/library/track_detail.rs`, `src/ui/shells/entity.rs`, `src/ui/shells/track.rs`.
- `src/app/search_dispatch.rs`, `src/app.rs`.
- `src/view_models/workspace/nav.rs` (`TrackKey`).
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test --lib adr_0084_index_page_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

Run `cargo build --bin v4vmm` last, after every test run. A test run can leave a GPUI test-support binary at `target/debug/v4vmm` (AGENTS.md, "Build, Test, Lint").

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`, `docs/architecture/source-map.md` and `docs/plans/adr-0084-music-column-browser-phase-plan.md`
- This packet: `docs/tasks/adr-0084-task-013-index-release-page-parity.md`
- ADR 0084 Decision 4, and ADR 0047 item 8 with its amendment of 2026-10-16
- The archived task 006 packet, with its Orchestrator Review
- Each file in "Files To Inspect"

Goal:
- The Index release page track click, its description toggle, the Index track page description, and the collapsed description preview with "Show more", as "Design" gives.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- View models carry no GPUI type. The renderer decides no state and no label.
- Use named tokens and named control styles. No raw size, color or glyph string in a renderer. The "…" of the preview comes from the view model.
- Do not call `truncate()` on stacked text (ADR 0063).
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The content list, the Discover views and the sidebar.
- The search results view, its rows and its chips.
- The navigation model in `src/view_models/workspace/`, except a read of `TrackKey`.
- The Settings and Show screens.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R13-01 to R13-06 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result" section and an "Operator Visual Check" section for V1 to V6. The check gives numbered steps, the expected result, an incorrect result, and the cleanup. It names an Index release with a long description and one with a short description, or it tells the operator how to find them.

Test commands:
- Each command in "Checks".

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- `open_track_in_frame` cannot open an Index track page from the release page, or it needs a cached search result.
- A fourth caller of `DisclosureTextPanel` exists, or a caller needs a state that "Design" does not give.
- The preview needs a renderer measurement, not the view-model estimate.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-16

Each Required Change is done.

### Files Changed

- `src/view_models/library.rs`:
  - Adds `DESCRIPTION_CHARS_PER_LINE`, `DescriptionShowMoreDisplay` and
    `DescriptionDisclosureDisplay`, with its `project` method. Adds the private
    `description_preview` function.
  - Changes `description_line_count`. It now sums each line's estimated wrapped lines,
    `ceil(chars / DESCRIPTION_CHARS_PER_LINE)` with a minimum of one, not a plain count of
    source lines.
  - Adds `keyed_description_state` and `toggle_keyed_description_state`. These two pure
    functions read and toggle a disclosure state by its key, with no GPUI type.
  - Adds tests R13-01 to R13-04.
- `src/ui/composites/disclosure_group.rs`:
  - Changes `DisclosureTextPanelDisplay`. Field `body` is now `Option<SharedString>`. A new
    field, `show_more`, holds an optional label pair.
  - The `DisclosureTextPanel` render method shows the body only when it is present. It adds a
    "Show more" button, styled `ControlStyle::Ghost`, when `show_more` is present. The button's
    click calls the same `on_toggle` handler as the header.
  - Updates two existing tests for the new fields. Adds one test for `show_more`.
- `src/ui/shells/library/feed_detail.rs`: The Library release page's description panel now reads
  `DescriptionDisclosureDisplay::project`, not a raw `collapsed`/`body` pair.
- `src/ui/shells/library/track_detail.rs`: The Library track page's description panel now reads
  `DescriptionDisclosureDisplay::project`, by the same change.
- `src/app/search_dispatch.rs`:
  - `index_feed_track_rows` sets `ReleaseTrackRowSlot::on_click` on each row with a track GUID.
    The click opens the track's page in the content-list frame, with the track title as its
    label. A row with no track GUID keeps no click. The new pure function
    `index_track_row_click_key` builds its `TrackKey`, or gives `None` with no track GUID.
  - `index_feed_description_panel` is now a `TopApp` method. It reads the Index release
    description state, projects it with `DescriptionDisclosureDisplay::project`, and gives the
    panel an `on_toggle` handler.
  - `index_track_detail_slots` sets `description_panel` through the new
    `index_track_description_panel` method, by the same pattern. The new free function
    `index_track_description_target` builds its state key: the Index activation target of the
    track.
  - Adds `index_release_description_state`, `toggle_index_release_description`,
    `index_track_description_state` and `toggle_index_track_description`. These are thin `TopApp`
    wrappers around the pure functions in `src/view_models/library.rs`.
  - Adds test R13-05.
- `src/app.rs`: Adds two `TopApp` fields, `index_release_description_states` and
  `index_track_description_states`. Each field is a `BTreeMap<String, DescriptionState>` that
  lives for the session only. Adds their construction in `TopApp::new`.
- `tests/architecture_tests.rs`: Updates
  `adr_0047_phase_c_inspector_rewire_uses_vm_state_and_shared_disclosures`. Its required string for
  `track_detail.rs` changes from the retired `description_state.is_visible()` call to the new
  `DescriptionDisclosureDisplay::project(` call, since Required Change 3 replaces that call site.
  Adds `adr_0084_index_page_r13_06_track_row_click_and_description_toggle_are_wired` (R13-06).

### Behavior Changed

- A click on a track row of an Index release page now opens that track's page, the same as a
  click on a Library row. Back returns to the release page.
- The Description section's header, and its "show" or "hide" word, now open and close the
  section on an Index release page. Before this change, a click there did nothing.
- A long description that is collapsed now shows its first lines, with a "Show more" button at
  the end. This rule applies to a release page and a track page, for both origins, the Library
  and the Index. Before this change, a Library page showed its full text with no preview. An
  Index release page always showed its full text, with no collapse at all.
- An Index track page now shows its description, with the same collapse rule. Before this change,
  it showed no description.
- `description_line_count` now counts a long paragraph with no line break as more than one line,
  so it collapses too. Before this change, only a description with several source lines could
  collapse.

### Checks

| Command | Result |
|---|---|
| `cargo test --lib adr_0084_index_page_` | Green, 5 tests |
| `cargo test` | Green: 1,867 unit tests, 312 guards, 10 ignored doc tests |
| `cargo test --test architecture_tests` | Green, 312 tests |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

### Deviations From The Task

1. `tests/architecture_tests.rs` also updates
   `adr_0047_phase_c_inspector_rewire_uses_vm_state_and_shared_disclosures`. Required Change 7 does
   not name this guard by number. Required Change 3 replaces the `description_state.is_visible()`
   call in `track_detail.rs` with `DescriptionDisclosureDisplay::project(`. The guard's old
   required string no longer appears in that file. This fix is a direct, mechanical result of
   Required Change 3, inside this packet's own scope.
2. This packet does not record the open V1 to V6 gate in `docs/pending-human-checks.md` or in
   `docs/plans/broadcast-chain-delivery-order.md`. AGENTS.md asks for an open visual gate to go in
   both files. This packet's own rules name only this packet file as an editable document. Tasks
   004, 005 and 006 recorded the same scope limit, in their own deviations.

### Unresolved Concerns

- The two files named in Deviation 2 still need the open V1 to V6 gate recorded, outside this
  packet's own editable scope.
- No person walked V1 to V6 yet. A green mechanical suite gives no evidence about them.

## Operator Visual Check

This check reads the app only. It needs a Linux desktop session, this checkout, and a reachable
MusicIndex endpoint. It needs one Index release with a long description, and one Index release
with a short description. A long description has more than 450 characters, or more than five
source lines.

"New releases" in Discover, or a search result, finds such releases. A release with long-form
album notes commonly gives the long description. A release with one short line of liner notes
commonly gives the short one.

1. Close v4vmm. Make the binary.

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```
2. Open the app.

   ```bash
   target/debug/v4vmm
   ```

**V1 - a track row of an Index release page opens its track page**

3. Open Discover. Open an Index release from "New releases" that is not in your Library.
4. Click a track row.
   - Correct: the detail column shows that track's page. The breadcrumb adds the track title.
   - Incorrect: the click does nothing.
5. Click Back in the detail column.
   - Correct: the detail column returns to the release page.

**V2 - a long description collapses, with "Show more"**

6. Open the Index release with the long description.
7. Look at its Description section.
   - Correct: the section shows its first lines, cut at a word, ending in "…", with a "Show more"
     button under them.
   - Incorrect: the section shows the full text, or shows no text at all.
8. Click "Show more".
   - Correct: the section shows the full text. The header word changes from "show" to "hide".
9. Click the header's "hide" word.
   - Correct: the section returns to its preview and "Show more".

**V3 - a short description shows in full, and its header hides and shows it**

10. Open the Index release with the short description.
11. Look at its Description section.
    - Correct: the section shows the full text, with no "Show more" button.
12. Click the header's "hide" word.
    - Correct: the section shows no text. The header word changes to "show".
13. Click the header's "show" word.
    - Correct: the section shows the full text again.

**V4 - a Library release page shows the same preview and "Show more"**

14. Open Releases, then a Library release with a long description, for example "Chatti und
    Claude". Do not download a release for this step.
15. Look at that Library release page. Look at its Description section.
    - Correct: it shows the same preview and the same "Show more" rule as V2.

**V5 - an Index track page shows its description**

16. Open an Index track that names its own description. Not every track has one. A track row of
    an Index release page (V1), or a track search result, opens a track page.
17. Look at its Description section.
    - Correct: it shows the same collapse rule as V2 and V3, by the length of that track's text.
    - Incorrect: the track page shows no Description section at all.

**V6 - Light, Dark and a larger scale show no clipping**

18. Open Settings, then General (`Ctrl+Comma`). Select Light, then Dark, then a larger scale.
    Return to the Index release of V2 after each change.
    - Correct: the preview, "Show more", and the header stay fully visible, with no clipped text,
      icon, or control, in every combination.
    - Incorrect: the preview, "Show more", or the header is clipped, cut off, or overlaps another
      element.
19. Close the app.

**Clean-up**

None. This check reads the app only. It writes no file and changes no stored data. Set the theme
and the scale of V6 back to your usual values.

## Orchestrator Review - 2026-10-16

The orchestrator reviewed the diff against ADR 0084 Decision 4, ADR 0047 item 8 with its amendment, this packet and the [review checklist](../reviews/adr-0084-review-checklist.md). The orchestrator also ran each check.

- The projection, the preview cut, the composite, the Index states, the row click and the Index track description follow "Design". The renderer decides no state and no label. No stop condition fired.
- The row click gave the fixed text "Track" when a track has no title. The orchestrator changed it to the track GUID, the same fallback as the search path.
- The visual check had three errors. Step 14 downloaded a release, but the check reads the app only. The clean-up named a removal path that nobody confirmed. Step 16 said that a "New releases" row opens a track page, but it opens a release page. Step 14 now uses a Library release, and step 16 uses a track row of an Index release page.
- The agent changed the ADR 0047 Phase C guard because Required Change 3 removed its marker. The new marker pins the same contract.
- The orchestrator recorded the open gate in the pending checks and in the delivery order.

Merge recommendation: pass. Packet 007, Releases, comes after the visual check.
