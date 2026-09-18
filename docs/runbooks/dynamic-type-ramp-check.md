# Dynamic Type Ramp Check

Status: Planned - 2026-09-18. Amended the same day: this is
[task 003](../tasks/adr-0039-task-003-type-curve-ratification.md)'s
procedure. Run after ADR 0039 tasks 001, 002 and 003 are implemented — task
003 is the packet that lands the ratified per-role type curves; tasks 001 and
002 land at identity and have no visual gate of their own. All twelve cells
in the [review checklist](../reviews/adr-0039-review-checklist.md#operator-visual-check)
are open. This procedure is for a person in a Linux desktop terminal.

## Purpose And Prerequisites

Inspect a compact Music playlist row, its track detail and its Add to Playlist
popover at x-small/x-large in Light/Dark. Use Python 3.11+, the normal desktop
build and a new disposable fixture. The fixture supplies local tracks, Null
playback and local service stubs. No audio device, reachable Index, installed
player/converter or live broadcast service is required. Close the normal app.

Keep the same window size, pane width and fixture data for all twelve cells;
record those dimensions. Medium is the reference, not an extra acceptance
matrix. Do not reuse the retained ADR 0066 task 004 fixture.

## Operator Visual Check

1. From the checkout, build and create the disposable fixture. These commands
   do not launch the app yet. Keep this terminal for the remaining steps.

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --locked --offline --bin v4vmm
   type_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
   python3 docs/runbooks/startup-recovery-fixture.py verify "$type_fixture"
   ```

2. With the fixture app closed, give its first track repeatable long text and
   a description. This edits only the new disposable database. It leaves the
   fixture's tracks, paths, playlist membership and audio intact.

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

3. Record the fixture's starting theme and scale in Settings → General
   (Ctrl+Comma). At medium, inspect Music → `Startup fixture playlist` and
   its first row. Open that track's detail, then its Add to Playlist popover.
   Record a reference for visible labels, glyphs, row lines and controls.
   If a named surface cannot be reached, leave its cells open and record the
   failed route. Do not substitute an empty screen.

4. Select x-small/Light in Settings → General. Inspect all three surfaces,
   recording V1, V5 and V9. Repeat x-small/Dark for V2, V6 and V10, then
   x-large/Light for V3, V7 and V11, and x-large/Dark for V4, V8 and V12.
   After every setting change, the mounted view must update without restart.
   Settings is the selector, not a fourth inspection surface.

   - Row: small metadata is readable, title/artist lines keep their line count,
     and glyph tops/bottoms and adjacent controls are not cut off. Long titles
     may clip horizontally under the existing rule. Height may follow the
     existing chrome step; it must not depend on text length. A new wrapped
     row or text lost vertically compared with medium is wrong.
   - Detail: inspect the title, small metadata, description and actions. Wrapping
     must preserve readable lines and reachable content; overlap, a paragraph
     trapped behind controls, or column text reduced to `...` is wrong.
   - Popover: inspect list mode, then New Playlist input mode. Type the fixture
     title into the draft input, inspect text and controls, use Back, then
     dismiss with Escape. Do not submit Create & Add or select a playlist.
     Hidden buttons, cut glyphs, overlapping text or an unreachable field fail
     the cell. A single-line input may scroll horizontally.

5. Restore the recorded starting theme and scale in Settings. Close the app
   normally so the launch command returns, then inspect preservation:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py inspect "$type_fixture"
   ```

   Expect `config_preserved`, `music_preserved`, `migration_records_preserved`,
   `bindings_preserved` and `library_preserved` to be true, one playlist, and
   no residual probes. Normal workspace preference changes are permitted by
   the fixture. A failure stays open; keep that fixture for diagnosis rather
   than restoring files to conceal an unexpected change.

6. Record revision, viewport, each of the twelve outcomes and screenshot or
   observation evidence in task 003 and the review checklist. After a correction,
   repeat affected cells on the corrected revision. Mechanical checks alone
   do not close cells. Confirm preservation, then clean up:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$type_fixture"
   test ! -e "$type_fixture"
   unset type_fixture
   ```

   Cleanup removes the disposable configuration, database, source text and
   audio. It changes no system unit or retained fixture. Reconcile all three
   packet Status lines, ADR, phase plan, review, delivery rows and
   pending-human entry. Leave any unwalked or failed cell open.

## Rollback And Failure Handling

A failed cell requires a correction at its shared owner and a repeat of that
cell. Numerical corrections return to ADR 0039 and its tests; chrome retuning
requires a future packet with its density gate. Restore the starting preferences
before closing the fixture. Use the phase plan for code rollback; there is no
configuration-format downgrade or live-service cleanup in this procedure.
