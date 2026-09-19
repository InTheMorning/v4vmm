# Dynamic Type Ramp Check

Status: Accepted regression procedure - 2026-09-18.
The operator passed all thirteen visual checks and confirmed fixture removal.
The agent inferred preservation and preference restoration from the conditional cleanup command.
The [review checklist](../reviews/adr-0039-review-checklist.md#operator-visual-check--task-003)
records the evidence limits. ADR 0039 is Implemented.
Use this procedure for future checks in a Linux desktop terminal.

## Purpose And Prerequisites

Inspect the compact playlist row, track detail and Add to Playlist popover in Music.
Check these surfaces at x-small and x-large in Light and Dark.
Use Python 3.11+, the normal desktop build and a new disposable fixture.
The fixture supplies local tracks, Null playback and local service stubs.
The checks require no audio device, reachable Index, installed player, converter or live broadcast service.

Close the normal app before fixture setup.
Keep the window size, pane width and fixture data constant for the twelve type checks.
Record both dimensions. Use Medium as the reference.
V13 checks Show cards at any scale and theme.
Do not reuse the retained ADR 0066 task 004 fixture.

## Operator Visual Check

1. Run these build and setup commands in your desktop terminal.
   These commands do not open the app.
   Keep this terminal and the printed fixture path for all later steps.
   An agent session may use a different temporary filesystem.
   Do not create a replacement fixture during the checks.

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --locked --offline --bin v4vmm
   type_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
   python3 docs/runbooks/startup-recovery-fixture.py verify "$type_fixture"
   ```

2. Keep the fixture app closed while the next commands prepare its first track.
   Run the following commands once.
   They add long text and a description to the disposable database.
   Track records, paths, playlist membership and audio remain intact.
   The final command opens the fixture app.

   ```bash
   python3 - "$type_fixture" <<'PY'
   import json, pathlib, sqlite3, sys
   root = pathlib.Path(sys.argv[1]).resolve()
   manifest = json.loads((root / 'fixture.json').read_text())
   assert manifest['root'] == str(root)
   assert root.parent == pathlib.Path('/tmp')
   assert root.name.startswith('v4vmm-startup-')
   with sqlite3.connect(root / 'data/library.sqlite') as db:
       assert db.execute('SELECT count(*) FROM tracks').fetchone()[0] == 3
       db.execute('UPDATE tracks SET track_title=?, artist_name=? WHERE id=1', (
           'Élan gyp — A long track title for the dynamic type inspection',
           'Type fixture artist — ascenders, descenders and accents'))
       db.execute('INSERT INTO entity_metadata_facts '
                  '(owner_kind, track_id, fact_key, value_text, source) '
                  'VALUES (?, ?, ?, ?, ?)', (
           'track', 1, 'description',
           'Élan gyp. This fixture description must remain readable as text '
           'size changes. Inspect wrapped lines, accents and descenders. '
           'The end of the paragraph must be reachable without overlapping '
           'the next field or its actions.', 'rss'))
   PY
   python3 docs/runbooks/startup-recovery-fixture.py run "$type_fixture"
   ```

3. Open Settings → General with Ctrl+Comma.
   Record the starting theme and scale.
   Select Medium.
   Inspect the first row in Music → `Startup fixture playlist`.
   Open that track's detail.
   Open its Add to Playlist popover.

   Record the visible labels, glyphs, row lines and controls as the reference.
   If you cannot reach a named surface, record the failed route.
   Leave that surface's checks open.
   Do not substitute an empty screen.

4. Use Settings → General to select each combination below.
   Inspect the same row, detail and popover after each selection.
   Record each check separately.

   | Scale | Theme | Row | Detail | Popover |
   |---|---|---|---|---|
   | x-small | Light | V1 | V5 | V9 |
   | x-small | Dark | V2 | V6 | V10 |
   | x-large | Light | V3 | V7 | V11 |
   | x-large | Dark | V4 | V8 | V12 |

   The current view must reflect each setting change without an app restart.
   Settings selects the values. It is not another inspection surface.

   - Row: check small metadata, title and artist lines, glyphs and adjacent controls.
     The text must remain readable and retain its line count.
     Long titles may clip horizontally.
     Row height may follow the existing chrome step, but must not depend on text length.
     New wrapping or vertical clipping relative to Medium fails the check.
   - Detail: check the title, metadata, description and actions.
     Wrapped lines must remain readable and reachable.
     Overlap, hidden paragraphs or column text reduced to `...` fails the check.
   - Popover: inspect list mode.
     Open New Playlist input mode.
     Type the fixture title into the draft input.
     Inspect the text and controls.
     Select Back.
     Press Escape to dismiss the popover.

   Do not submit Create & Add.
   Do not select a playlist.
   Hidden buttons, clipped glyphs, overlapping text or an unreachable field fails the popover check.
   The single-line input may scroll horizontally.

   At x-large, check whether the title remains distinct from smaller text.
   Task 003 changed page titles from 30.00 px to 26.88 px and increased small metadata.
   Report the surface if the hierarchy is unclear.
   A numeric correction requires a decision in ADR 0039.
   Do not compensate with a screen-specific font adjustment.

5. Open Show at any scale and theme.
   Inspect each card's two summary lines for V13.
   Each summary must occupy one line.
   Narrow the window until a long summary overflows horizontally.
   The summary must clip without an ellipsis or a second line.
   A wrapped line hidden by the card's bottom edge fails the check.

   Record V13. One observation completes this check because the correction does not depend on scale.

6. Restore the starting theme and scale in Settings.
   Close the app normally.
   Wait for the launch command to return.
   Run the preservation inspection:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py inspect "$type_fixture"
   ```

   Check that these flags are true:

   - `config_preserved`
   - `music_preserved`
   - `migration_records_preserved`
   - `bindings_preserved`
   - `library_preserved`

   Check that one playlist remains and no residual probes exist.
   The fixture permits normal workspace preference changes.
   If inspection fails, retain the fixture for diagnosis.
   Do not restore files to conceal an unexpected change.

7. Record the revision, viewport dimensions and all thirteen results in task 003 and the review checklist.
   Include an observation or screenshot reference for each result.
   After a correction, repeat affected checks on the corrected revision.
   Mechanical checks alone do not establish visual acceptance.

   Continue only if the preservation inspection passed.
   Run the cleanup commands:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$type_fixture"
   test ! -e "$type_fixture"
   unset type_fixture
   ```

   Retain the inspection output and removal message before closing the terminal.
   If a later command reports a missing fixture, check the retained output first.
   The `locate` command can return an unrelated fixture.

   Cleanup removes the disposable configuration, database, source text and audio.
   It changes no system unit or retained fixture.
   Update the three packet statuses, ADR, phase plan, review, delivery rows and pending-human index.
   Leave any failed or unperformed check open.

## Rollback And Failure Handling

Correct a failed check at its shared owner.
Repeat that check after the correction.
Return numeric changes to ADR 0039 and its tests.
A change to chrome requires a separate packet and density check.

Restore the starting preferences before closing the fixture.
Use the phase plan for code rollback.
This procedure requires no configuration downgrade or live-service cleanup.
