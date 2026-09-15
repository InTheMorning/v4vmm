# ADR 0071 Task 001: Shared Text Selection

Status: Complete - 2026-09-15; migration and pinned ADR 0072 correction verified; mechanical checks Green; available X11 operator checks, preservation and fixture/scratch cleanup accepted. IME composition and Wayland remain untested coverage limits. The completed log packet stays closed.

## Goal And Owners

Implement [ADR 0071](../adr/0071-shared-text-selection-and-linux-primary.md)
for HIG product polish backlog item 11 in one bounded packet.

- Published gpui-component and gpui-kit-assets 0.6.1; gpui-pre and
  gpui-pre-platform 0.3.1; Rust 1.97.1. All are explicitly pinned. gpui-base
  0.6.1 uses the commit-pinned correction specified by ADR 0072.
- gpui-base owns pointer boundaries, window selection and input editing.
- `src/ui/composites/selectable_text.rs` adapts the upstream selection handle
  to exact log Copy, Select All, append preservation and the typed context menu.
- `src/view_models/text_selection.rs` retains renderer-free log ranges and Copy
  availability. It no longer owns word or line boundary algorithms.
- `src/ui/primitives/primary_selection.rs` publishes input selection and handles
  middle-click insertion through public APIs, without adding a layout box.
- `src/ui/theme_bridge.rs` supplies the app's palette to component and base tokens.
- `src/ui/composites/split_pane.rs` keeps divider resizing separate from Root's
  window-wide text selection by consuming the initiating press.
- `tests/architecture_tests.rs` guards shared ownership, separate buffers,
  exact dependency sources and the existing configuration-editor Escape chain.
- `Cargo.toml` optimizes the published GPUI/Taffy layout dependencies in debug
  builds; application code retains ordinary debug compilation and assertions.

The app has no vendor tree or local selection crate. Its only Cargo patch is
the gpui-base correction from [ADR 0072](../adr/0072-pinned-gpui-base-selection-corrections.md),
pinned to `5463fe4e72fd740b0db08da92003488b32661867` in
[InTheMorning/gpui-kit](https://github.com/InTheMorning/gpui-kit). The dependency
guard checks this exact source and the remaining published pins.
Config format, recovery phases, playback, persistence and completed packets
remain outside this task. ADR 0063 task 005 remains closed.

## Migration Evidence

An isolated clone at `/tmp/v4vmm-gpui061-trial`, branch `trial/gpui-0.6.1`,
measured the migration before applying it to the working tree. The initial
app check produced 20 errors. Mechanical API changes covered focus context,
`Anchor`, `ScrollbarMode`, scroll offsets, menus, assets, theme fields and the
configuration editor's `TextareaState`. Tests needed two new event fields.
The isolated app check then passed. Required Clippy also needed equivalent
formatting and duration expressions under the newer compiler.

Rust 1.93.1 failed first in gpui-pre's `std::hint::cold_path` calls. Installed
Rust 1.97.1 compiled both the core and the separately packaged Linux platform.
The project toolchain now records that verified version.

The standalone upstream log element does not replace the complete accepted
log contract: Root Copy trims whitespace, and it supplies neither Select All
nor this app's context-menu/append policy. Retaining a small adapter avoids
those regressions while deleting the local boundary implementation. Upstream
word boundaries use character classes; ADR 0072 expands their endpoints to
complete graphemes and corrects double-click glyph hit testing. V1l exposed
these failures, recorded for a future upstream PR below; native retry is open.
Triple-click excludes the LF terminator. Exact drag/Select All copying
still preserves selected whitespace and line endings.

Mock pointer tests found that the public IME hit test misses empty fields and
space after glyphs, and can return an offset on the wrong logical row. The shared
adapter checks the returned row and falls back to public caret bounds within
the visible logical rows. Click containment uses the input viewport rather than
text bounds translated by scrolling, so visible trailing blank rows remain
reachable. It neither estimates positions from font widths nor
changes the caret before reading PRIMARY. Input insertion uses upstream
normalization, validation, Change events and atomic undo.

The lockfile changes from 965 to 906 package records relative to the vendor
attempt: 272 old name/version records disappear and 213 arrive, a net reduction
of 59. The new platform/rendering stack and mock-test support account for new
records; a net reduction of 155 was not borne out by resolution.

## Mechanical Acceptance

1. Mock GPUI tests, mounted through the real component Root, prove upstream
   word/line projection reaches exact log Copy
   and PRIMARY, preserves selected ranges on append, clears on replacement,
   and handles whitespace-only Select All. Renderer-free tests retain exact
   multiline/Unicode Copy and typed availability coverage.
   Pointer-menu tests cover mouse-down, redraw and mouse-up separately: Copy
   retains the exact range through activation, closes the menu and returns
   focus, while cancellation and source replacement do not copy stale text.
   An app-keyboard test installs the real bindings and active-pane context:
   Escape dismisses only the menu, preserves both buffers and returns focus;
   subsequent Ctrl+C preserves whitespace, and a later Escape reaches the pane.
   A second app-keyboard test mounts the shared button and textarea through
   Root with real app bindings. Input Enter inserts a newline, Escape focuses
   the button, and Enter/Space activate once through redraw, held repeats and
   key release. A subsequent press activates again; pane Enter still works
   when the pane itself has focus.
2. Input tests prove nonempty selection publication, masked-field exclusion,
   no PRIMARY claim during text edits, clipboard independence, Unicode offsets,
   normalized single-line paste, preserved multiline paste, Change events,
   disabled/read-only/empty no-ops, and separate atomic undo/redo.
3. Mock pointer events verify middle-click reaches empty fields, line ends, later
   rows and wrapped Unicode text. A regression covers three trailing blank rows
   in short and scrolled Unicode documents, exact insertion at each row, atomic
   Undo/Redo and separate PRIMARY/clipboard contents. Input pointer tests prove
   accented words, path segments and whole-value triple-click publish PRIMARY. A validation test
   proves rejection and independent clipboard replacement/undo. A theme test
   checks the component/base palette synchronization.
   These tests use GPUI's in-process test platform, without launching the app.
4. Architecture tests preserve the upstream selection seam, PRIMARY wiring,
   token synchronization and the accepted input-first Escape/Close behavior.
5. A mock Root with the shared split pane and selectable text in both panes
   proves horizontal and vertical resizing do not start text selection,
   release stops resizing, and subsequent ordinary text selection still works.

```bash
cargo fmt -- --check
cargo check --locked --offline
cargo clippy --locked --offline -- -D warnings
cargo test --locked --offline
cargo build --locked --offline
```

Required checks Green after the Copy/divider corrections, debug layout
optimization, Escape/Copy action routing, focused-button Enter and scrolled
PRIMARY hit-testing corrections (2026-09-14): format, check, strict Clippy,
debug build, 1,390 app unit tests (including sixteen focused migration/selection
tests) and 247 architecture
tests. The complete suite used unrestricted local socket fixtures and retained
ten existing ignored documentation examples. Production-code guards exclude
the keyboard tests' pointer coordinates and PRIMARY assertions. Changed-document
links and diff whitespace checks are Green. These checks do not establish the
remaining desktop acceptance.

The optional exploratory
`cargo clippy --all-targets -- -D warnings` also found existing test-only lint
debt outside this packet; the required Clippy command above is the repository's
gate. This packet does not undertake a test-suite lint cleanup.

## Documentation Inventory And Rollback

Created this task, ADR 0071 and the operator runbook in existing documentation
folders. Removed the now-obsolete vendor patch note. Updated the source map,
ADR/docs indexes, backlog, delivery order, AGENTS and pending-human index.
No documentation folders or root Markdown files were added.

Rollback restores the previously published dependencies, toolchain and original
log selection adapter. No configuration or database migration needs reversal.
For the later ADR 0072 correction alone, remove its gpui-base Cargo override
and restore that package's published lockfile source; the two known emoji
failures return. Other dependency pins and the adapter stay unchanged.
Do not restore the vendor attempt. Stop after this packet; ADR 0066 task 007
starts in a later session.

## Operator Record — 2026-09-13

- Desktop session: X11, reported by the operator. Cross-application peer:
  xfce4-terminal, using its explicit Copy/Paste shortcuts and middle-click.
- Fixture launch: accepted. The operator created a fresh fixture in the desktop
  terminal and confirmed that the app opened normally. The fixture is
  `/tmp/v4vmm-startup-bxg5v6ic`, retained in that terminal's `selection_fixture`
  variable. Startup output included Mesa/EGL PCI-driver warnings and
  `egl: failed to create dri2 screen`; launch and V1a passed as reported below.
- The first agent-created fixture was inaccessible from the desktop session;
  it was never launched and has been removed. Create subsequent fixtures in the
  desktop terminal and use the directory returned there.
- V1a: accepted. From xfce4-terminal, selecting
  `/tmp/café/café/music_dir/👩‍💻.flac` without Copy and middle-clicking between
  `|` and `R` in the Settings MusicIndex endpoint produced
  `LEFT|/tmp/café/café/music_dir/👩‍💻.flacRIGHT`. The subsequent explicit terminal
  clipboard paste still produced `CLIPBOARD-KEEP`. The operator confirmed both
  expected results, including insertion position and surrounding text.
- V1b: accepted. Undo removed only the PRIMARY path insertion; redo restored
  it. After another undo, inserting the newly selected `CLIPBOARD-KEEP` text
  discarded the old redo branch. Redo made no change, and one undo restored
  `LEFT|RIGHT`. The operator confirmed all six steps.
- V1c: accepted. PRIMARY inserted the exact Unicode path into an empty
  MusicIndex endpoint; one undo restored the empty field. Middle-clicking
  blank space after `PREFIX-` appended the path at the end; one undo retained
  exactly `PREFIX-`. The operator confirmed both cases.
- V1d: accepted. Double-click selected each complete `café`, combining-accent
  `café` and `music_dir` word without surrounding slashes. Each selection
  reached xfce4-terminal through PRIMARY without Copy. Triple-click retained
  the whole endpoint value after release and pasted the complete Unicode path.
  The operator confirmed all four cases.
- V1e: accepted. Forward and reverse drag of `music_dir`, Shift+Right selection
  of `/tmp`, and Ctrl+A of the whole path all published the expected PRIMARY
  text. Collapsing the whole-value selection and switching to the terminal
  retained PRIMARY. Explicit clipboard paste still returned `CLIPBOARD-KEEP`.
  The operator confirmed all six checks.
- V1f: accepted. With `TARGET` selected in `LEFT|TARGET|RIGHT`, middle-click
  inserted external PRIMARY text at the clicked position and produced
  `LEFT|music_dirTARGET|RIGHT`. Ctrl+V instead replaced `TARGET` with the
  clipboard text, producing `LEFT|CLIPBOARD-KEEP|RIGHT`. Each insertion was
  independently undone to `LEFT|TARGET|RIGHT`. The operator confirmed all cases.
- V1g: accepted. Explicit Ctrl+C on `TARGET` in the app replaced the clipboard.
  Selecting `LEFT` without Copy then changed PRIMARY. In xfce4-terminal,
  explicit clipboard paste returned `TARGET` and middle-click returned `LEFT`.
  The operator confirmed both results.
- Checklist correction: Settings renders Music directory as a current-session
  readout, not an editable input. Its editable `music_dir` value is checked in
  the configuration editor under V2. The runbook now follows that ownership.
- V1h: accepted. The operator questioned the emoji appearance after PRIMARY
  paste into `flac binary (optional)`. Selecting the whole field with Ctrl+A and
  returning it through PRIMARY to xfce4-terminal restored the expected emoji;
  the operator recognized the app's combined woman-technologist glyph. This
  resolves the reported appearance concern. The operator then confirmed that
  Undo emptied the field, Redo restored the path, each accented word and
  `music_dir` selected and published correctly, and triple-click retained and
  published the whole value. After resetting the clipboard marker, explicit
  terminal paste still returned `TARGET`.
- V1i: accepted. Toolbar search received the Unicode path through PRIMARY;
  Undo emptied the field and Redo restored it. Both accented words and
  `music_dir` selected and published exactly. Triple-click retained and
  published the whole value, including its hidden portion. Explicit terminal
  clipboard paste still returned `TARGET`. The operator confirmed the checks
  and cleared the query without submitting a search.
- V1j: accepted. In toolbar search, double-click selected and published the
  individual dot and slash near `END.flac`. Triple-click from either end of a
  long URL selected and published the complete value, including two leading
  and two trailing spaces, both accented segments, Chinese text and the joined
  emoji. The operator confirmed both bracketed terminal comparisons and
  cleared the query without submitting it. Copying the sample replaced the
  prior clipboard marker; reset it before further clipboard-preservation checks.
- V1k: accepted. In both MusicIndex endpoint and flac inputs, punctuation
  selected individually. Triple-click and forward/reverse dragging across
  hidden text each published the complete long Unicode URL with both leading
  and trailing pairs of spaces. The operator confirmed scrolling in both drag
  directions and all bracketed terminal comparisons. Settings remain unsaved.
- V1l baseline failure, resolved by the pinned ADR 0072 correction and native
  acceptance on 2026-09-15. The operator originally reported that double-clicking the final
  joined emoji in `中文 👩‍💻` did not select it. Drag selection pasted a person
  and laptop as separate visible symbols in xfce4-terminal. The subsequent
  whole-value PRIMARY check produced exactly
  `U+4E2D U+6587 U+0020 U+1F469 U+200D U+1F4BB`; the joiner and all text survived.
  The operator also confirmed steps 1–5: transfer from the Settings endpoint
  into search, scrolling during forward/reverse dragging, Ctrl+A, selection of
  the two spaces and `ht` with Shift+Right, and individual `中`/`文` selection.
  At that point, only the final-emoji double-click result remained unresolved
  within V1l. The correction and its acceptance are recorded below.
- A standalone diagnostic against the compiled gpui-pre 0.3.1 library
  reproduced the final-glyph hit-test problem without a window. For a
  `LineLayout` containing one emoji glyph at x=0, width 20 and UTF-8 length 11,
  `closest_index_for_x` returned byte 11 for x=1, 5, 10 and 19. The published
  gpui-base 0.6.1 word-boundary function returned no selection at that offset.
  At x=0 it selected only U+1F469. The owners are gpui-pre's
  `text_system/line_layout.rs` and gpui-base's `input/base/selection.rs` and
  `text_boundary.rs`. This synthetic-layout reproduction supports the reported
  failure; native whole-value character preservation is confirmed above. V2a
  below confirms that the log pointer path selects the full emoji, while the
  input path's final-emoji double-click failure remains open.
  No dependency or application implementation was changed for this diagnostic.
- The fixture helper's `selection-sample`
  command appends Unicode, final-emoji and long-value cases to both owned
  journals. Its mechanical check is Green: both prior journal prefixes and
  every non-journal fixture file remained unchanged; appended combining marks,
  the emoji joiner and trailing spaces matched exactly. The agent's temporary
  verification fixture was removed. The operator's fixture remains running.
- V2a: accepted. In the producer log's appended sample, double-click selected
  and published each `café`, combining-accent `café`, `music_dir`, `中` and `文`.
  On the `Final emoji:` line it selected and pasted the complete `👩‍💻` sequence.
  The operator confirmed every case. This establishes a difference between
  the successful log path and the failing editable-input path in V1l.
- V2b: accepted. Triple-click near either end of the producer log's long-value
  line retained and published the complete logical line, including all twelve
  repeated path segments, spacing before the URL and two trailing spaces.
  Neither selection added a newline. Horizontal scrolling did not shorten
  the selection, and explicit terminal clipboard paste retained
  `CLIPBOARD-KEEP`. The operator confirmed both comparisons and restored the
  horizontal scrollbar to the left.
- V2c step 1: failed. Right-click opened the producer log menu, but pressing
  Copy cleared the line selection. The menu remained open with Copy disabled,
  and explicit terminal paste still returned `CLIPBOARD-KEEP`. Keyboard Copy
  and read-only subchecks were not yet walked.
- The mock pointer regression reproduced V2c: Root cleared the upstream
  selection during mouse-down capture, before Copy activated on mouse-up.
  The shared log owner now registers its retained exact range as a local
  selection while the context menu remains open. This uses the published
  handle API and changes no dependency or input-selection behavior. Tests
  cover exact whitespace/Unicode Copy, independent buffers, menu dismissal,
  focus return, outside clicks and source replacement before mouse release.
  The pointer Copy regression failed before the correction and is now Green.
  The inherited Copy guard counts production paths only, excluding the new
  tests' clipboard-marker setup; all 246 architecture guards are Green.
  Native V2c recheck remains open; restart with the rebuilt binary and retain
  the existing fixture and samples.
- Remaining V1 checks, V2–V3, preservation and desktop-fixture cleanup remain
  open. Retain `/tmp/v4vmm-startup-bxg5v6ic` for further diagnosis.
- Resize/usability regression: before completing the Copy retry, the operator
  tried dragging the log divider. Resizing failed, the pointer rapidly
  alternated between normal and resize cursors elsewhere in the app, and the
  app became unusable. The supplied htop screenshot shows v4vmm at 99.5% CPU.
  The operator reported the same Mesa/EGL warnings as earlier and fixture exit
  code 0. Those warnings do not establish the cause. The fixture is stopped;
  its directory and samples remain retained, with preservation inspection open.
- A mock divider drag reproduced simultaneous resize and Root text selection.
  The shared split-pane handle now consumes the initiating mouse press, keeping
  resizing separate from selection. The regression failed before this change
  and is Green afterward for both axes, release and subsequent text selection.
  This proves the interaction conflict and its correction; it does not reproduce
  the native CPU/cursor symptom. Native resizing and idle responsiveness must
  pass before resuming V2c. No dependency change or vendor patch was needed,
  and the completed ADR 0063 log packet remains closed.
- Resize retest: failed. The operator reported that resizing still froze the
  app and the pointer continued alternating between normal and resize cursors.
  The divider's gesture-isolation correction did not resolve the native freeze.
  Retain that independently reproduced correction, but do not claim a cause or
  resolution for the freeze from that correction. That attempt left the
  CPU/cursor acceptance open.
- A temporary mock test mounted the complete Show shell with a 200-line log,
  resized the pane, and drew subsequent frames. The layout callback reported
  stable geometry in the mock window. It did not reproduce the native failure
  and was removed after diagnosis. No additional production change was made
  from this result. The agent cannot inspect the desktop process from its
  sandbox; native evidence uses the operator-collected
  [freeze diagnostic](../runbooks/text-selection-check.md#resize-freeze-diagnostic).
- The operator supplied `resize-stacks.txt`. Its main-thread stack is in
  Taffy 0.13.0 flexbox measurement through GPUI's layout tree. All 30 captured
  main-thread frames remain inside layout; the capture does not reach its
  caller. The GPUI worker and rendering threads are waiting at this instant.
  This establishes active UI-thread layout at the sampled time, but cannot
  distinguish one expensive layout from repeated redraws or establish the
  cursor-flicker cause. It does not implicate the Mesa warning.
- A second temporary mock probe populated the service cards and Live Metadata
  sidebar, supplied an 80-line Unicode/path journal, and added outer chrome.
  Across 1400×900, 1000×700 and 800×600 windows at Medium, XS and XL, repeated
  geometry measurements settled; forced mock redraws took roughly 54–67 ms.
  A stronger release assertion exposed the existing boundary of the pointer
  callbacks: release outside the split does not reach its resize-end callback.
  This did not reproduce sustained CPU load or cursor flicker. No production
  correction was justified by that probe alone; it was removed. A short native
  CPU profile was requested before choosing a layout or dependency-build
  correction. The supplied trace and fixture were retained, with native
  acceptance still open at that point. ADR 0063 task 005 remains closed.
- The operator supplied `resize-perf.data` and `resize-profile.txt`. The
  five-second CPU-clock capture contains 498 samples with none lost: 481 on
  v4vmm's main thread and 17 in fixture Python processes. Its binary build ID
  matches the pre-optimization executable. The main thread consumes nearly a
  full CPU core throughout the recording. Layout helpers dominate the visible
  hot symbols; samples also reach prepaint and log-text painting, so the
  observed work progresses through frames. This evidence does not establish
  what repeatedly invalidates the native window or causes cursor flicker.
- The ordinary perf report could not unwind most caller chains. Offline
  reconstruction of captured registers and stack bytes let GDB recover deeper
  chains from the existing file: four separated samples remain in nested
  Taffy layout, and a paint sample reaches the shared log-text element. No
  desktop process was inspected by the agent. Captured 32 KiB stacks still end
  before the outer window caller; missing memory was not inferred or fabricated.
- A controlled populated Show mock measured 20 forced redraws, after three
  warm-up frames, at 1400×900 and Medium scale. Both builds used the same
  service cards, Live Metadata sidebar, 80-line Unicode/path log and chrome:

  | Development build | Minimum | Median | Maximum |
  | --- | --- | --- | --- |
  | Existing font optimization only | 55.105 ms | 57.599 ms | 62.892 ms |
  | Also optimize gpui-pre and taffy | 6.143 ms | 6.551 ms | 9.220 ms |

  Every measured frame retained split geometry `(632.5, 5.0)`. The roughly
  ninefold mock improvement establishes a reduction in debug redraw cost,
  not native responsiveness or the cause of continuous redraws. The temporary
  benchmark was removed. `Cargo.toml` now applies opt-level 3 to `gpui-pre`
  and `taffy`, including the generic layout algorithms instantiated by GPUI.
  Pins, lockfile, application optimization, assertions and symbols are unchanged.
  A situational ADR 0071 architecture guard retains this build contract. The
  [native resize regression](../runbooks/text-selection-check.md#resize-responsiveness-regression)
  remains the acceptance check for dragging, release, cursor and idle CPU.
- Native resize and idle responsiveness: accepted. After the debug layout
  optimization, the operator confirmed that the app no longer hangs and the
  resize bar is responsive again. CPU usage remains high while the operator
  rapidly moves the divider up and down, then returns to idle when movement
  stops. This closes the reported resize/hang and sustained-idle-CPU failure;
  it does not claim low CPU cost during continuous resizing. V2c's context-menu
  Copy retry resumed afterward. Keep the fixture and diagnostic captures until the
  remaining selection checks, preservation inspection and cleanup are complete.
- V2c step 1, context-menu Copy: accepted on retry. The operator reset the
  explicit clipboard to `CLIPBOARD-KEEP`, selected a producer-log line, opened
  its menu, and pressed and briefly held Copy before releasing. Selection
  survived the press, release closed the menu, and explicit terminal paste
  returned the selected log line. This closes the reported pointer Copy failure.
  Keyboard Copy, read-only behavior and menu dismissal remain separate checks.
- V2c step 2, keyboard Copy: accepted. The operator double-clicked `café` on
  the producer log's `Unicode path:` line, pressed Ctrl+C, and explicitly pasted
  into the terminal scratch reader. The result was exactly `café`, replacing
  the previously copied full line. Read-only behavior and menu dismissal were
  still open at that point.
- V2c step 3, read-only log: accepted. With the producer log's `Unicode path:`
  line selected, the operator typed `X` and tried Backspace, Delete and Ctrl+V
  individually. The original log text remained unchanged and the app remained
  responsive. Menu dismissal was still open at that point.
- V2c step 4, Escape dismissal: failed at the first action. With the selected
  `Unicode path:` line's context menu open, Escape left the menu open. The
  selection remained highlighted and the log pane remained open. Clipboard
  preservation and keyboard Copy after dismissal were not reached. The earlier
  mock dismissal test did not install app shortcuts; the app's active-pane
  Escape action consumes the key before raw key handlers run.
- The app-keyboard regression reproduced that failure with the real bindings
  and active-pane context. A scoped action in the shared pointer menu now
  handles Escape before the app's pane action. Bootstrap installs the binding;
  a situational ADR 0071 guard checks that registration. The same test's
  post-dismissal Ctrl+C assertion exposed Root's bound Copy trimming whitespace
  before the raw log handler. The shared log now handles that bound action
  through its existing exact-copy method. These are app adapter corrections
  within the pinned dependencies. Required mechanical checks and the rebuilt
  debug binary were Green; native Escape acceptance remained open at that point.
- V2c step 4, first action, Escape dismissal: accepted on retry. After
  relaunching the rebuilt fixture app, the operator selected the producer log's
  `Unicode path:` line, opened its context menu and pressed Escape once. The
  menu closed, selection stayed highlighted and the log pane stayed open.
  Clipboard preservation and keyboard Copy after dismissal were still separate,
  open checks at that point.
- V2c step 4, clipboard preservation after Escape: accepted. The operator
  explicitly copied `café`, selected the whole `Unicode path:` line, opened
  its context menu and dismissed it with Escape. In xfce4-terminal, explicit
  clipboard paste still returned exactly `café`; middle-click returned the
  whole `Unicode path:` line. Both buffers retained their separate contents.
  Keyboard Copy without clicking after dismissal was still open at that point.
- V2c step 4, keyboard focus after Escape: accepted. The operator selected the
  `Unicode path:` line, opened its menu, pressed Escape and then Ctrl+C without
  clicking. Explicit paste into the terminal scratch reader returned the whole
  line, replacing the previously copied `café`. Escape returned keyboard focus
  to the log and retained the range for Copy. This completes the producer-log
  Escape check; outside-click dismissal and the remaining V2 checks were still
  open at that point.
- V2c step 5, outside-click dismissal: accepted. With `café` explicitly copied
  and the whole `Unicode path:` line selected, the operator opened the menu
  and clicked blank app space outside it. The menu closed and the log pane
  stayed open. Explicit terminal paste retained `café`; middle-click retained
  the whole line. The check permits the highlight to clear and does not record
  which highlight state the operator observed. V2c's producer-log Copy,
  read-only and menu-dismissal checks are complete. Reverse dragging, Select
  All, append preservation and the remaining V2 checks were still open at that
  point.
- V2d, reverse dragging: accepted. On the producer log's `Unicode path:` line,
  the operator dragged left from just after `music_dir` to just before the first
  `café` and released without Copy. Terminal middle-click returned exactly
  `café/café/music_dir`, without surrounding slashes. Explicit clipboard paste
  still returned `café`. Select All, append preservation and remaining V2
  checks were still open at that point.
- V2e, Select All: accepted. With producer-log text focused, Ctrl+A without
  Copy published the entire log body to PRIMARY. Terminal middle-click
  included entries outside the viewport through the final `Long value:` line,
  retaining line breaks and complete paths. Explicit clipboard paste still
  returned exactly `café`. Append preservation and remaining V2 checks were
  still open at that point.
- V2f, append preservation: accepted. The operator prepared the waiting
  append command, explicitly copied `café`, then selected the producer log's
  whole `Unicode path:` line without Copy. After releasing the command and
  receiving new entries, the original line remained selected without
  reselection. Terminal middle-click retained that whole line; explicit paste
  retained exactly `café`. The appended fixture remains in use for the
  Publisher, Diagnostics/editor and remaining migration checks.
- V2g, Publisher word and line selection: accepted. In Publisher Logs,
  double-clicking `music_dir` on the existing `Unicode path:` line published
  exactly that word to PRIMARY. Triple-clicking the line published its whole
  value. Terminal middle-click confirmed both results, and explicit clipboard
  paste still returned exactly `café`. Publisher Copy/menu checks and the
  remaining V2 checks were still open at that point.
- V2h, Publisher context-menu Copy: accepted. The operator selected the
  Publisher log's whole `Unicode path:` line, opened its menu and pressed
  and briefly held Copy. The selection remained highlighted and Copy stayed
  enabled. Release closed the menu; explicit terminal paste returned the
  whole line, replacing `café`. Publisher Escape/keyboard checks and remaining
  V2 checks were still open at that point.
- V2i, Publisher Escape and keyboard Copy: accepted. Ctrl+C on the selected
  `music_dir` word pasted exactly that word into the terminal. The operator
  then selected the whole `Unicode path:` line, opened its menu and pressed
  Escape. The menu closed, the selection remained highlighted and the log
  pane stayed open. Ctrl+C immediately afterward, without clicking, copied
  the whole line and replaced `music_dir` in the explicit clipboard. Publisher
  read-only behavior and remaining V2 checks were still open at that point.
- V2j, Publisher read-only behavior: accepted. With `music_dir` explicitly
  copied and the Publisher log's whole `Unicode path:` line selected, the
  operator tried typing `X`, Backspace, Delete and Ctrl+V individually. The
  original text remained unchanged after every action and the app remained
  responsive. Diagnostics/report selection, editor interactions and the
  remaining migration checks were still open at that point.
- V2k, Diagnostics report word and line selection: accepted. The operator
  opened the configuration editor, selected `musicindex_endpoint`, appended
  `/selection-check` and used Test draft and paths. Double-clicking `validated`
  in the repair report published exactly that word to PRIMARY. Triple-clicking
  its line published the complete logical line, including text outside the
  visible width. Terminal middle-click confirmed both, and explicit clipboard
  paste retained `music_dir`. The editor remains open with the unsaved suffix;
  report Copy/menu, editor interactions and remaining V2 checks were still
  open at that point.
- V2l, Diagnostics report context-menu Copy: accepted. The operator selected
  `validated` in the repair report, opened its menu and pressed and briefly
  held Copy. The word stayed highlighted and Copy remained enabled. Release
  closed the menu while leaving the configuration editor open. Explicit
  terminal paste returned exactly `validated`, replacing `music_dir`. The
  draft remains unsaved; report Escape/keyboard, editor interactions and
  remaining V2 checks were still open at that point.
- V2m, Diagnostics report Escape and keyboard Copy: accepted. The operator
  selected the complete report line containing `validated`, opened its menu
  and pressed Escape once. The menu closed, the line remained highlighted
  and the configuration editor stayed open. Ctrl+C immediately afterward,
  without clicking, copied the complete report line; explicit terminal paste
  replaced the previously copied word `validated`. The draft remains unsaved.
  Report read-only behavior, editor interactions and remaining V2 checks were
  still open at that point.
- V2n, Diagnostics report read-only behavior and draft isolation: accepted.
  The operator copied `validated`, selected its whole report line and tried
  typing `X`, Backspace, Delete and Ctrl+V individually. After each action,
  the report and endpoint draft remained unchanged, including the draft's
  `/selection-check` suffix, and the app remained responsive. The editor and
  fixture stay open with the draft unsaved. Editor interactions and remaining
  migration checks were still open at that point.
- V2o, editor word and whole-value selection: accepted. Double-clicking
  `selection` within the endpoint draft's `/selection-check` suffix published
  exactly that word to PRIMARY. Triple-clicking the editable value published
  the entire endpoint, including the suffix. Terminal middle-click confirmed
  both, while explicit clipboard paste retained `validated`. The draft stays
  unsaved. Incoming PRIMARY, Undo/Redo, multiline editing and remaining V2
  checks were still open at that point.
- V2p, editor incoming PRIMARY and Undo/Redo: accepted. The operator typed
  and selected `PRIMARY-EDITOR` in the terminal without Copy. Middle-clicking
  after the endpoint's last character appended the token without removing
  existing text. One Undo removed only that insertion; Redo restored it once;
  a final Undo restored the draft ending in `/selection-check`. Explicit
  clipboard paste still returned `validated`. The draft remains unsaved;
  multiline editing and remaining V2 checks were still open at that point.
- V2q, editor multiline logical-line selection: accepted. The operator replaced
  the endpoint draft with the runbook's four-line Unicode sample: `FIRST`
  with its accented path, `SECOND 中文 👩‍💻`, a blank third line and `LAST omega`
  without a final newline. After resetting the explicit clipboard to
  `validated`, triple-clicking the second line published exactly
  `SECOND 中文 👩‍💻` to PRIMARY, without a newline or neighboring rows.
  Terminal middle-click confirmed that result; explicit paste retained
  `validated`. This temporary multiline draft remains open, untested and
  unsaved. Final/blank-line behavior and remaining V2 checks were still open
  at that point.
- V2r, editor final and blank lines: accepted. Triple-clicking the final
  `LAST omega` line published exactly that text to PRIMARY without a newline.
  After triple-clicking the blank third line, typing `B` changed only that
  row; the second and final lines stayed separate. One Undo restored the
  blank row. Explicit clipboard paste retained `validated`. The four-line
  draft remains unsaved; remaining editor and migration checks stay open.
- V2s, editor Unicode word selection: accepted. Double-clicking `café`,
  `café` and `music_dir` on the first line, then `中`, `文` and `👩‍💻`
  individually on the second line published each complete token to PRIMARY.
  Terminal middle-click reproduced all six selections; explicit clipboard
  paste retained `validated`. This joined emoji is followed by a newline;
  its acceptance does not resolve V1l's final-value emoji double-click
  failure. The four-line draft remains unsaved, with remaining editor and
  migration checks open.
- V2t, editor multiline incoming PRIMARY and atomic Undo/Redo: accepted.
  Terminal-selected `PASTE-ONE` and `PASTE-TWO`, with one internal newline
  and no final newline, inserted at the blank third row on middle-click.
  Both Unicode rows remained unchanged, and `LAST omega` followed the two
  inserted rows. One Undo restored the original four-line draft; Redo
  restored both pasted rows once; a final Undo restored the blank third
  row. Explicit clipboard paste retained `validated`. The draft remains
  unsaved; remaining editor and migration checks stay open.
- V2u, editor soft-wrap logical-line selection: accepted. The operator
  replaced only the first draft line with the runbook's long `WRAP-BEGIN`
  through `WRAP-END` sample and narrowed the window until it wrapped.
  Triple-clicking a continuation row selected the entire logical line;
  terminal middle-click reproduced both ends, without added newlines or
  neighboring text. Explicit clipboard paste retained `validated`. One
  Undo restored the original `FIRST` line, and the previous window width
  was restored. The four-line draft remains unsaved; remaining editor and
  migration checks stay open.
- V2v, editor Enter and Tab: accepted. Enter at the end of `LAST omega`
  inserted a blank fifth line while retaining editor focus and the repair
  report. Undo removed the newline, Redo restored it once, and a final Undo
  restored the original four-line draft. Tab at the same position inserted
  indentation without moving focus to a button, closing the editor or
  triggering validation. Undo/Redo and a final Undo restored the original
  final line. Explicit clipboard paste retained `validated`. The draft
  remains unsaved; remaining editor and migration checks stay open.
- V2w, editor Escape and retained draft: accepted after correction. The input menu
  dismissed on the first Escape, the second Escape focused Close editor
  without closing, and reopening retained `musicindex_endpoint`, the exact
  four-line draft and the report. Explicit clipboard paste retained
  `validated`. Enter on the focused Close editor button did nothing; Space
  closed it successfully. The mock regression reproduced Enter doing nothing
  with the app bindings installed: the pane's ConfirmSelection action consumed
  Enter before the shared button's raw key-down handler. Keyboard-enabled
  buttons now declare the `ActionButton` context and the pane's Enter binding
  excludes it. The mock passes Enter/Space presses, repeats, release and
  subsequent activation, while input and ordinary-pane Enter remain usable.
  Required checks and the rebuilt binary are Green. The operator restarted
  the same fixture without saving, recreated an endpoint draft ending in
  `/selection-check` and obtained a repair report. The native retry passed:
  the first Escape dismissed the menu, the second focused Close editor,
  and holding Enter then releasing it closed the editor once without
  reopening it. Reopen retained the endpoint draft and report; the subsequent
  Escape/Space Close and Reopen cycle retained them again. The editor remains
  open and unsaved. The earlier four-line scratch draft was discarded during
  the restart; remaining editor and migration checks stay open.
- V2x, structured `music_dir` selection and PRIMARY paste: accepted.
  Double-clicking the final directory name published the selected word;
  triple-clicking published the complete path without quotes or a newline.
  Terminal-selected `-PRIMARY` appended at the path's end on middle-click.
  One Undo restored the original path, Redo restored the suffix once, and
  a final Undo restored the original again. Explicit clipboard paste retained
  `validated`. Returning to `musicindex_endpoint` retained its unsaved
  `/selection-check` suffix. No validation, save or session-end action was
  used during the path check; remaining editor and migration checks stay open.
- V2y, explicit reload and discarded-draft retention: accepted. Reload file
  added a configuration-loaded report entry and restored the saved endpoint
  without `/selection-check`. The original `music_dir` path remained without
  `-PRIMARY`. Closing and reopening the editor did not restore the discarded
  endpoint suffix. The mounted editor updated without an unexpected save.
  The editor and fixture remain open, with the saved field values loaded;
  appearance/scale checks, other migration checks and preservation remain open.
- V2z, Light appearance at M scale: accepted at normal and approximately
  560-pixel window widths. Selection and right-click menus remained readable
  and reachable in the Settings Library endpoint input, Diagnostics editor
  and report, and Show Producer Logs. Log word/line selection, context-menu
  dismissal and scrolling passed without overlaps, clipped controls, invisible
  selection or stale Dark-theme colors. Normal width was restored; Light/M
  was retained as an unsaved preview with the fixture. Dark appearance,
  the other scales and remaining migration checks were still open at that point.
- V2aa, Dark appearance at M scale: accepted at normal and approximately
  560-pixel window widths. Selection, focus indicators, right-click menus and
  scrollbars remained readable and reachable in the Settings Library endpoint
  input, Diagnostics editor/report and Show Producer Logs. Word/line selection,
  menu dismissal and scrolling passed without clipping, overlaps, invisible
  selection or stale Light-theme colors. Normal width and Dark/M were
  restored; the fixture remained open. XS, S, L and XL checks were still open
  at that point.
- V2ab, Dark appearance at XS scale: accepted at normal and approximately
  560-pixel window widths. Word/line selection, right-click menus, Escape
  dismissal and scrolling of overflowing content passed in the Library
  endpoint input, Diagnostics editor/report and Producer Logs. No cropped
  text, displaced highlights, missing focus indicators or unreachable
  controls/scrollbars were reported. Normal width was restored; Dark/XS
  was retained as an unsaved preview with the fixture. S, L, XL and
  remaining migration checks were still open at that point.
- V2ac, Dark appearance at S scale: accepted at normal and approximately
  560-pixel window widths. Word/line selection, right-click menus, Escape
  dismissal and scrolling of overflowing content passed in the Library
  endpoint input, Diagnostics editor/report and Producer Logs. No cropped
  text, displaced highlights, missing focus indicators or unreachable
  controls were reported. Normal width was restored; Dark/S was retained as
  an unsaved preview with the fixture. L, XL and remaining migration checks
  were still open at that point.
- V2ad, Dark appearance at L scale: accepted at normal and approximately
  560-pixel window widths. Word/line selection, right-click menus, Escape
  dismissal and scrolling of overflowing content passed in the Library
  endpoint input, Diagnostics editor/report and Producer Logs. No cropped
  text, displaced highlights, missing focus indicators or unreachable
  controls were reported. Normal width was restored; Dark/L was retained as
  an unsaved preview with the fixture. XL and remaining migration checks
  were still open at that point.
- V2ae, Dark appearance at XL scale: accepted at normal and approximately
  560-pixel window widths. Word/line selection, right-click menus, Escape
  dismissal and scrolling of overflowing content passed in the Library
  endpoint input, Diagnostics editor/report and Producer Logs. No cropped
  text, displaced highlights, missing focus indicators or unreachable
  controls were reported. Normal width and Dark/M were restored, and the
  fixture remains open. This completes the five scale checks in Dark and
  the Light/Dark M appearance checks; exact whitespace through explicit log
  Copy, other migration checks and preservation were still open at that point.

## Operator Record — 2026-09-14

- V2af, exact whitespace through explicit log Copy: accepted. Ctrl+C on the
  Producer Log's complete `Long value:` line retained its three spaces after
  the label, both trailing spaces and the entire path, without adding a
  newline. The terminal's bracketed paste ended in `END.flac  ]`. After
  replacing the clipboard with only `END`, pressing, holding and releasing
  the log menu's Copy closed the menu and produced the same complete line
  with its spaces. The fixture was retained; preservation inspection,
  the V1l failure, recovery checks and cleanup were still open at that point.
- Log-fixture preservation: accepted after normal Quit. Inspection of
  `/tmp/v4vmm-startup-bxg5v6ic` reported `session-held-command`, with changed
  configuration bytes accounted for solely by permitted workspace preferences.
  Configuration values, music, library, bindings, migration records and tool
  blockers all passed preservation. Counts remain three tracks, one playlist
  and three playlist memberships; bindings remain `a.wav`, `b.wav`, `c.wav`
  and migration versions remain 1–11. The music-probe list is empty and the
  database-probe count is zero. Retain this closed fixture for the V1l failure;
  a separate recovery fixture, its checks and preservation, and cleanup remain
  open.
- V3a, recovery fixture launch: accepted. The new desktop fixture is
  `/tmp/v4vmm-startup-1oj2crwd`. The operator reported the expected TOML
  parse error in its `config/v4vmm/config.toml` at line 7, column 12, with
  instructions to correct the document before saving. Recovery selection,
  raw-document editing, line endings, IME/Escape, preservation and cleanup
  remain open. The earlier log fixture remains separate and retained.
- V3b, recovery report outgoing PRIMARY: accepted. Double-clicking `TOML`
  in the report body published exactly that word. Triple-clicking the line
  containing `config.toml` published the complete logical line, including
  the full recovery-fixture configuration path, without an added newline
  or neighboring line. After each selection, explicit terminal clipboard
  paste retained `RECOVERY-KEEP`. Recovery remains open with its broken
  configuration unsaved; raw-document editing and remaining V3 checks are open.
- V3c, recovery raw-document PRIMARY paste: accepted after correction. Initially,
  the operator reported that
  pasting on trailing newlines did nothing. An in-process pointer regression
  reproduced the no-op after Ctrl+End scrolled a Unicode document to its three
  trailing blank rows; the short-document cases already passed. The shared
  handler rejected the visible click using text bounds translated by scrolling.
  It now checks the public input viewport, retaining the existing caret geometry
  and insertion path. All sixteen focused ADR 0071 tests are Green, including
  insertion into each trailing blank row, Undo/Redo and independent buffers.
  Required format, check, strict Clippy, full tests and rebuilt debug binary
  are Green (1,390 unit tests, 247 architecture guards; ten ignored doctests).
  The native X11 retry passed after reopening `/tmp/v4vmm-startup-1oj2crwd`
  without saving. Middle-click inserted `PRIMARY-RECOVERY` at the blank final
  row without replacing preceding text. Undo removed the insertion, Redo
  restored it once, and a final Undo restored the original broken document.
  Explicit terminal paste retained `RECOVERY-KEEP`. The recovery editor remains
  open and unsaved; remaining V3 checks, recovery preservation and cleanup are
  open. Both fixtures remain retained.
- V3d, recovery document Unicode word/line selection: accepted. Double-clicking
  the first `café` in the appended `RECOVERY /tmp/café/café/music_dir/中文/👩‍💻.flac`
  sample published exactly `café`. Triple-clicking published the complete line,
  including its label and Unicode path, without neighboring text or an added
  newline. Explicit terminal clipboard paste retained `RECOVERY-KEEP`. One
  Undo removed the appended sample. The recovery editor remains open with
  the original broken document unsaved; remaining V3 checks, preservation and
  cleanup remain open, and both fixtures are retained.
- V3e, recovery document reverse-drag selection: accepted. Dragging backward
  from the appended path's end to immediately before `/tmp/` published exactly
  `/tmp/café/café/music_dir/中文/👩‍💻.flac`, without its `RECOVERY ` label,
  missing characters or an added newline. Explicit terminal clipboard paste
  retained `RECOVERY-KEEP`. One Undo removed the appended sample. The editor
  remains open and unsaved; remaining recovery checks, preservation and cleanup
  remain open, and both fixtures are retained.
- V3f, recovery document multiline PRIMARY and atomic Undo/Redo: accepted.
  Terminal-selected `RECOVERY-ONE café` and `RECOVERY-TWO 👩‍💻`, including
  their internal newline but excluding a final newline, inserted in order
  on middle-click at the blank final row. Preceding TOML remained unchanged,
  and Ctrl+End landed after the emoji without an extra blank row. One Undo
  removed both inserted lines, Redo restored them once, and a final Undo
  restored the original broken document. Explicit terminal clipboard paste
  retained `RECOVERY-KEEP`. The editor remains open and unsaved; remaining
  recovery checks, preservation and cleanup remain open, with both fixtures
  retained.
- V3g, recovery Escape and retained draft: accepted. The first Escape dismissed
  the input menu; the second focused Close editor without closing it. Holding
  Enter briefly and releasing closed the editor once. Reopen retained the
  appended `RECOVERY-DRAFT` marker, preceding TOML and recovery report. The
  subsequent Escape/Space Close and Reopen cycle retained the same draft and
  report. Explicit terminal clipboard paste retained `RECOVERY-KEEP`. The
  operator removed only the marker with Ctrl+End, Shift+Home and Backspace.
  The recovery editor remains open with the original broken document unsaved.
  CRLF, IME composition if available, recovery preservation and cleanup remain
  open; both fixtures are retained. This accepts normal Escape routing, not
  the separate composition-first IME check.
- V3h, Mousepad source inspection: complete. The operator reported
  `'CRLF-ONE café\nCRLF-TWO 👩\u200d💻'` from the PRIMARY byte inspection.
  Mousepad supplied LF, with the emoji joiner preserved and no final newline;
  this result does not accept raw CRLF insertion in the app. V3i will publish
  the scratch file's original bytes directly through xclip and verify the
  source before app paste. The scratch path has not been reported; its
  `selection_crlf` variable remains the reference in the retained desktop
  terminal. IME availability remains unconfirmed. Recovery stays unsaved,
  with both fixtures and the Mousepad scratch file retained.
- V3i, raw CRLF incoming PRIMARY: accepted. The operator confirmed the expected
  source payload with one `\r\n` separator after xclip published the original
  scratch-file bytes. Middle-click inserted both Unicode lines once, without
  replacing preceding TOML, adding a blank row between them or showing a stray
  character. Ctrl+End landed after the emoji. Undo removed both lines together,
  Redo restored them once, and a final Undo restored the original document.
  Explicit terminal clipboard paste retained `RECOVERY-KEEP`. Recovery remains
  open and unsaved. Single-line CRLF handling, IME availability/composition,
  recovery preservation and cleanup remain open. The scratch-file path was
  not included in the response; retain its existing `selection_crlf` reference
  in the desktop shell, the Mousepad document and both fixtures.
- V3j, IME composition: untested in this operator workflow. The operator
  reported that they do not use an IME and insert emoji through rofi. No
  composition/candidate-list cancellation was exercised; this is not a pass
  or evidence that IME handling works. Normal Escape acceptance remains V3g.
  Proceed to recovery preservation without saving the broken document; keep
  the single-line CRLF check, V1l final-emoji failure and cleanup open.
- V3k, recovery preservation: accepted. Both inspection objects for
  `/tmp/v4vmm-startup-1oj2crwd` report `repair-toml`. Configuration bytes are
  unchanged, with the original and unedited values preserved, no changed
  fields, no backups and no residual candidates. Selected music, music files,
  migration records, bindings, library and tool blockers are preserved. Counts
  remain three tracks, one playlist and three memberships; bindings remain
  `a.wav`, `b.wav`, `c.wav`, and migration versions remain 1–11. The music-probe
  list is empty and the database-probe count is zero. False backup, external
  revision and workspace-preference flags are expected for this unsaved case.
  Retain the closed recovery fixture and the Mousepad scratch file. The
  single-line CRLF check returns to the separate normal fixture; inspect that
  fixture again after its remaining checks. V1l and cleanup remain open, and
  IME composition remains untested.
- V3l, single-line CRLF paste and Undo/Redo: accepted. The operator reopened
  `/tmp/v4vmm-startup-bxg5v6ic`, cleared toolbar search and middle-clicked the
  raw CRLF source into the empty input. It displayed
  `CRLF-ONE caféCRLF-TWO 👩‍💻` on one line without submitting a search. Undo
  emptied the query; Redo restored the whole value once. Explicit terminal
  clipboard paste retained `RECOVERY-KEEP`. The query remains in place for
  V3m's exact PRIMARY inspection; absence of hidden CR/LF is not yet accepted.
  Keep the normal fixture open, recovery closed, and both fixtures and the
  Mousepad scratch file retained. Final normal-fixture inspection, V1l and
  cleanup remain open; IME composition remains untested.
- V3m, exact single-line PRIMARY value: accepted. The operator reported
  `'CRLF-ONE caféCRLF-TWO 👩\u200d💻'`. The selected query contains neither
  CR nor LF, and retains the emoji joiner. The final Undo/query-clear step
  has not yet been confirmed. Both fixtures and the Mousepad scratch file
  remain retained; final normal-fixture inspection and cleanup remain open.
- V1l investigation resumed: the existing standalone diagnostic again
  reproduced gpui-pre's end-of-line hit result and gpui-base's empty word
  result at that byte offset. The upstream input mouse handler uses caret
  hit testing before its private word-selection method; the log owner uses
  glyph hit testing and projected selection geometry. A native comparison
  with and without a character after the emoji will check the failure's
  scope before choosing a correction. No dependency, registry source or app
  implementation was changed during this investigation.
- V1l comparison follow-up: the operator returned `中文 .\u200d💻`
  (U+4E2D U+6587 U+0020 U+002E U+200D U+1F4BB). The woman scalar is absent;
  a period precedes the retained joiner and laptop. The returned text does
  not establish the exact edit sequence or either comparison's selection
  result. Ctrl+End has an upstream input binding, but this report does not
  establish whether that action ran. The procedure now replaces the whole
  query with each complete sample through Ctrl+A/Ctrl+V, avoiding edits to
  an uncertain partial selection. Both comparison results remain pending;
  the V1l failure remains open. No implementation changed.
- V1l complete-value comparison: confirmed on X11. Double-clicking the emoji
  in `中文 👩‍💻` selected nothing; in `中文 👩‍💻.` it highlighted the emoji
  excluding the period. This narrows the observed failure to the final-value
  case. It accepts the second sample's reported highlight only; the exact
  selected bytes still need inspection. The standalone upstream diagnostic
  again reproduced the final-glyph caret hit returning the text length, where
  the word boundary returns no range. The input calls this caret hit test
  before its private word selector. That selector uses scalar character
  classes, so a highlight over a shaped joined emoji does not by itself prove
  that selection contains the whole sequence. Inspect the second sample's
  PRIMARY payload before choosing the upstream correction. The implementation
  and dependency pins are unchanged; V1l remains open.
- V1l exact emoji selection with a following period: failed. The operator
  reported `'👩'` from PRIMARY, containing only U+1F469. U+200D and U+1F4BB
  were absent even though the highlight covered the joined emoji. Two failures
  are now confirmed: the final-value glyph can produce no word selection, and
  selecting the emoji before a period selects only its first scalar. The app's
  PRIMARY observer publishes the selected text unchanged; both corrections
  belong in the upstream selection owner. An isolated 0.6.1 source trial under
  `/tmp/v4vmm-0071-upstream-trial` tests that correction without modifying the
  app dependencies or registry cache. Neither failure is accepted or fixed in
  the operator's running binary.

## Upstream PR Preparation

On 2026-09-14 the operator requested durable documentation for a future
upstream PR. The [upstream preparation record](../reviews/adr-0071-gpui-base-emoji-selection.md)
is the owner of the two reproductions, exact Unicode payloads, source causes,
verified upstream revisions, separate test/fix patches and validation limits.
It preserves the evidence without depending on temporary build logs.

The test-only patch reproduces all three regression failures against pristine
0.6.1 sources. Applying the implementation patch reconstructs the previously
verified candidate byte-for-byte. Its full isolated verification remains
Green: 900 upstream unit tests, 1,390 app unit tests, 247 architecture guards,
formatting, check and strict Clippy; ten existing doctests remain ignored.

The operator subsequently approved proceeding with the narrow gpui-base fork.
ADR 0072 records the exception to the earlier dependency restriction. A clean
upstream checkout at publication commit `96103905ea0c9c199db206ada7a9b2e3114e6339`
accepted both patches, changing only the three documented Rust files. The
original publication workspace passed four focused tests, 900 base unit tests,
strict Clippy for the base library/tests and workspace formatting. Its lockfile
uses gpui-pre/platform 0.3.2; this evidence is separate from the isolated app's
0.3.1 verification. Two local commits, attributed to `Codex <codex@localhost>`,
preserve the test/fix split; their exact revisions and a verified bundle handoff
are recorded in the preparation note. The operator published them with plain
Git to [InTheMorning/gpui-kit](https://github.com/InTheMorning/gpui-kit).
Read-only verification confirmed branch `fix/input-emoji-selection` at
`5463fe4e72fd740b0db08da92003488b32661867` on 2026-09-14. No upstream issue or
PR was submitted.

## Fork Integration — 2026-09-14

The app now overrides only gpui-base with that exact remote commit.
`Cargo.lock` changes only gpui-base's source and removes its registry checksum;
its version and dependencies, every other lockfile package and the app's
gpui-pre/platform 0.3.1 pins are unchanged. The fetched source matches the
previously tested candidate byte-for-byte. The new ADR 0072 architecture guard
checks the singleton Cargo override, exact revision, lockfile source and the
remaining published GPUI pins. The ADR 0071 selection ownership guard remains.

App check, formatting, strict Clippy, the desktop build, 1,390 unit tests and
248 architecture checks are Green; ten existing doctests remain ignored. Local
socket access was enabled for the app's service test fixtures. The app was not
launched by the agent.
Native retry follows the [fork correction procedure](../runbooks/text-selection-check.md#adr-0072-correction-retry).
Both native retries returned `'👩\u200d💻'` after double-clicking the emoji in
toolbar search values `中文 👩‍💻` and `中文 👩‍💻.`. Both exact PRIMARY payloads
are accepted on X11: woman, joiner and laptop are present, and the following
period is excluded. The operator also confirmed that both highlights cover the
complete emoji and exclude the period. These two toolbar-search cases are
accepted. Both Settings → Library MusicIndex endpoint cases are also accepted:
`中文 👩‍💻` and `中文 👩‍💻.` produced complete emoji highlights and exact PRIMARY
`'👩\u200d💻'`, excluding the period. After each test, one Undo restored the
original endpoint without saving. Both configuration-editor cases also passed
in the `musicindex_endpoint` value draft: complete emoji highlighting, exact
`'👩\u200d💻'` PRIMARY excluding the period, and one Undo restoring the previous
value after each sample, without Test or Save. The editor remains open for the
Unicode path checks. Final query removal,
normal-fixture reinspection and fixture/scratch cleanup
remain open, together with the focused correction regressions; IME composition
remains untested. This result does not reopen the completed log packet.

## Native Correction Follow-ups — 2026-09-15

The configuration editor's Unicode path word checks are accepted. In the
`musicindex_endpoint` draft containing `/tmp/café/café/music_dir/中文/👩‍💻.flac`,
double-clicking each `café`, `café`, `music_dir`, `中`, `文` and `👩‍💻`
produced exactly that complete token in both highlighting and outgoing PRIMARY.
Full-path triple-click also passed: the whole path was highlighted and the
exact Unicode comparison returned `PASS`, preserving both accent forms and the
joined emoji. One Undo restored the preceding value. No Test or Save was
requested.

Clipboard separation is accepted in the editor with `中文 👩‍💻`: after setting
the explicit clipboard to `CLIPBOARD-KEEP`, double-clicking the emoji without
Copy made terminal middle-click paste the complete emoji. Ctrl+U cleared that
terminal input, and Ctrl+Shift+V still pasted `CLIPBOARD-KEEP`. Explicit Copy
also passed: double-clicking the emoji, pressing Ctrl+C in the editor, then
Ctrl+U and Ctrl+Shift+V in the terminal pasted the complete emoji.

Emoji replacement and Undo/Redo are accepted: typing `.` over the selected
emoji changed `中文 👩‍💻` to `中文 .`; one Undo restored the complete emoji,
one Redo restored the period, and another Undo restored the emoji. Each action
left `中文 ` intact and changed the whole emoji in one step.

Escape/Close/Reopen is accepted: one Escape from the editor value kept the
editor open and moved focus to Close editor. Enter closed it once, and key
release did not reopen it. Reopen editor retained the selected
`musicindex_endpoint` field and its exact unsaved draft `中文 👩‍💻`.

Incoming PRIMARY insertion and Undo/Redo are accepted: selecting the complete
emoji in the terminal without Copy, then middle-clicking after the editor's
existing emoji produced `中文 👩‍💻👩‍💻`. One Undo restored the single emoji,
Redo restored both, and another Undo restored the single emoji. Each step
preserved the Chinese text and space.

Editor cleanup is accepted: Reload file (discard draft) restored the original
`musicindex_endpoint` value, and the editor was closed without saving. The
terminal scratch reader was stopped. The Settings → Library MusicIndex endpoint
was tested with the unsaved path `/tmp/café/café/music_dir/中文/👩‍💻.flac`.
Double-clicking the first, precomposed `café` highlighted only that word,
excluding both slashes, and exact PRIMARY inspection returned `'café'`.
The second, decomposed `café` also passed: the complete word and accent were
highlighted without either slash, and exact PRIMARY comparison with
`cafe\u0301` returned `PASS`. Double-clicking `music_dir` also passed: the
whole word, including the underscore and excluding both slashes, was
highlighted and exact PRIMARY comparison returned `PASS`.
Both Chinese character boundaries passed: double-clicking `中` and `文`
highlighted only the selected character and returned exactly `'中'` and `'文'`
through PRIMARY, respectively. The joined emoji in the path also passed:
double-click highlighted the complete `👩‍💻`, excluding the preceding slash
and `.flac`, and exact PRIMARY inspection returned `'👩\u200d💻'`.
Settings full-value triple-click also passed: the entire path was selected,
and exact PRIMARY comparison returned `PASS`, preserving both accent forms,
the Chinese characters and the joined emoji. One Undo restored the original
MusicIndex endpoint without saving. All focused Settings correction checks
are accepted.

The Producer Logs final-emoji PRIMARY check passed on the pinned correction:
double-clicking `👩‍💻` on `Final emoji: 中文 👩‍💻` highlighted the complete
emoji without the preceding space, and exact PRIMARY inspection returned
`'👩\u200d💻'`. Explicit log Copy also passed: Ctrl+C on the selected final
emoji and inspection of the separate clipboard returned exactly `'👩\u200d💻'`.
Both accented words on the log's `Unicode path:` line also passed: double-click
highlighted each complete word and accent without either slash, and exact
PRIMARY inspection returned `'caf\xe9'` for the first word and `'cafe\u0301'`
for the second, preserving their distinct accent forms. The remaining word
boundaries on that line also passed: `music_dir`, `中` and `文` each highlighted
only the selected item, and PRIMARY inspection returned exactly `'music_dir'`,
`'\u4e2d'` and `'\u6587'`, respectively. Whole-line triple-click also passed:
the `Unicode path:` label and complete path were selected without the next
line, and exact PRIMARY comparison returned `PASS`, with no LF terminator
and both accent forms, the Chinese characters and joined emoji preserved.
The log read-only check also passed: with the `Unicode path:` line focused and
selected, typing `X`, Backspace, Delete and Ctrl+V did not insert, remove or
replace text, and the app remained responsive.

All focused ADR 0072 correction checks are accepted on X11, resolving V1l's
two editable-input failures. Both fixture removals, scratch-query cleanup and
remaining scratch cleanup are accepted, as recorded below. IME composition remains untested
because this operator uses rofi emoji input rather than an IME; no Wayland
acceptance is claimed. The completed ADR 0063 task 005 log packet remains closed.

## Final Preservation — 2026-09-15

The operator's final inspection of `/tmp/v4vmm-startup-bxg5v6ic` passed after
the correction checks. It reported `session-held-command`, with
`config_bytes_unchanged: false` and `normal_workspace_preferences_only: true`:
only normal workspace preferences changed. Configuration, music, migration
records, bindings, library and tool blockers are preserved. Migrations remain
1–11; the library retains three tracks, one playlist and three playlist
memberships, with bindings `a.wav`, `b.wav` and `c.wav`. Residual music probes
are empty and the database-probe count is zero.

The separate recovery fixture `/tmp/v4vmm-startup-1oj2crwd` retains its earlier
accepted preservation result and was not reopened for this correction retry.
The operator confirmed removal of both `/tmp/v4vmm-startup-bxg5v6ic` and
`/tmp/v4vmm-startup-1oj2crwd`. Recovery cleanup succeeded when retried with
`python3` after the direct-execution permission error. Both fixture cleanups
are accepted. Clearing the toolbar query before quitting was not separately
confirmed; closing the fixture app and removing its isolated workspace satisfy
scratch-query cleanup without claiming an additional UI check.
The desktop terminal printed a blank value for `selection_crlf`. A read-only
listing identified `/tmp/v4vmm-selection-crlf-p1TRT7.txt`, and the operator
confirmed its removal. The Mousepad scratch document was closed without saving;
the terminal scratch reader had already been stopped during editor cleanup.

Prepared-source cleanup is complete. Both fork checkouts were clean at the
published correction commit. Replaying the durable test and correction patches
reconstructed their respective published commits byte-for-byte. Removed and
verified absent:

- `target/adr-0071-gpui-kit`
- `target/adr-0071-gpui-base-selection.bundle`
- `/tmp/v4vmm-0071-gpui-kit-fork`
- `/tmp/v4vmm-0071-upstream-trial`
- `/tmp/v4vmm-0071-app-selection-trial`
- `/tmp/v4vmm-0071-pr-repro`
- `/tmp/v4vmm-0071-patch-check`
- `/tmp/v4vmm-gpui061-trial`

## Completion — 2026-09-15

This packet is complete: migration, shared selection/PRIMARY integration,
the pinned ADR 0072 correction, mechanical verification, available X11 operator
checks, final preservation and cleanup are accepted. V1l's two failures are
resolved. IME composition was unavailable in this operator workflow, and
Wayland was not tested; neither is counted as passed. The upstream preparation
record retains its separate review work for a future PR. No upstream submission
was made. The completed log packet remains closed and ADR 0066 task 007 remains
unstarted for a fresh session.

Closure verification on 2026-09-15: all 248 architecture checks passed after
the status and regression-procedure updates. Local documentation links,
temporary-source removal and `git diff --check` are Green. The full application
and upstream Rust results remain the recorded 2026-09-14 verification; subsequent
acceptance recording and cleanup changed no Rust implementation.

## Operator Visual Check

No operator action remains for this packet. The
[shared text selection procedure](../runbooks/text-selection-check.md) remains
a regression check for future changes; create fresh fixtures for a new run.
Record platform and IME coverage separately. This completion does not close
inherited recovery, playback or other UI gates.
