# Grouped Settings Foundation Check

ADR 0069 task 001 complete - 2026-09-11. V1–V3 and all preservation inspections
accepted; fixture cleanup confirmed. The packet records operator evidence.
This procedure remains a manual regression check.
Run these steps in a Linux desktop terminal with Python 3.11 or newer.
No audio hardware, installed player/converter, reachable Index or running
broadcast service is needed. The fixture supplies local tracks and service
stubs. Close the normal app first.

## Operator Visual Check

1. Build and create a **new** disposable fixture from this checkout. Keep this
   terminal and its `settings_fixture` variable for every command below. Do not
   use `locate` or the operator's retained ADR 0066 task 004 directory.

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --locked --offline --quiet
   settings_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
   python3 docs/runbooks/startup-recovery-fixture.py verify "$settings_fixture"
   python3 docs/runbooks/startup-recovery-fixture.py run "$settings_fixture"
   ```

2. **V1 — grouping, keyboard and scroll.** Press Ctrl+Comma. Initial entry must
   show General. General has UI scale and Theme; Library has endpoint, music
   directory and flac; Diagnostics has Background tools and cached files.
   Walk the group buttons with mouse and Tab/Shift+Tab plus Enter/Space.
   Tab must show an outline on the focused button; moving focus alone must not
   select a group. Enter/Space activates that button and keeps focus there;
   the next Tab continues from it. Also inspect focus on
   appearance choices and Save/default buttons without activating the latter.
   Keyboard focus and the selected-group check must remain distinguishable.
   The selected group has a visible check and its heading below the navigation.
   Repeat at about 1400 and 560 pixels wide, in Light and Dark, with M and XL
   scale. Appearance choices preview immediately. Check all choices, labels and
   Save/default actions remain reachable by scrolling. Navigation must stay
   above the scrolling content. Clipped choices, overlap, lost keyboard delivery
   or reports pushing navigation out of reach fail.

3. **V2 — unsaved text and session selection.** In Library, append
   `/unsaved-settings-check` to the endpoint without saving. Switch to General
   and back; the text remains. Visit Music with Ctrl+1, then return with
   Ctrl+Comma; Library and its text remain. Repeat through Show with Ctrl+2
   and return with Ctrl+3. Do not use transport or service controls. From the
   endpoint field, Ctrl+F must focus the toolbar search; Ctrl+1/2/3 and
   Ctrl+Comma must still reach their sections. Return to General: the appearance
   preview remains. Settings must contain no queue, transport or broadcast
   status panel. Quit with Ctrl+Q, then inspect:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py inspect "$settings_fixture"
   ```

   Config must be unchanged except permitted workspace preferences. Music,
   playlist membership, local bindings and migration records must be preserved;
   no residual probe files are permitted. A failure means keep the fixture.

4. **V2 — Save from General includes Library.** Relaunch:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py run "$settings_fixture"
   ```

   In Library, set endpoint to `http://127.0.0.1:9/settings-foundation`, leave
   music directory unchanged, and set flac to `/usr/bin/flac`. This is only a
   configuration value; the check does not invoke it or require it installed.
   In General select Light and L and read the shared Save/default scope.
   Before saving, return to Library and confirm the endpoint and flac fields
   still contain the exact values above. If either changed, record which
   value changed and stop before Save. If both remain, return to General and
   click Save. Return to Library again and verify both values. Record any
   error or reversion, then quit. These five field values are intentional;
   every other config key except workspace preferences must remain the same.
   Verify below. A failed assertion leaves this check open and does not create
   the snapshot required by step 5; keep the fixture for diagnosis.

   ```bash
   python3 - "$settings_fixture" <<'PY'
   import pathlib, sys, tomllib
   root = pathlib.Path(sys.argv[1])
   before = tomllib.loads((root / 'case.config').read_text())
   after = tomllib.loads((root / 'config/v4vmm/config.toml').read_text())
   expected = dict(musicindex_endpoint='http://127.0.0.1:9/settings-foundation',
                   music_dir=str(root / 'music'), flac_path='/usr/bin/flac',
                   theme_profile='light', ui_scale='large')
   assert all(after.get(key) == value for key, value in expected.items()), after
   for key in (*expected, 'workspace', 'workspace_layout'):
       before.pop(key, None)
       after.pop(key, None)
   assert after == before, 'An unrelated setting changed'
   (root / 'settings-save-general.toml').write_bytes((root / 'config/v4vmm/config.toml').read_bytes())
   print('Green')
   PY
   ```

5. **V2 — Save from Library includes General.** Relaunch using the same `run`
   command. In General select Dark and XL, switch to Library and click Save.
   The endpoint, music directory and flac values must remain. Quit and verify:

   ```bash
   python3 - "$settings_fixture" <<'PY'
   import pathlib, sys, tomllib
   root = pathlib.Path(sys.argv[1])
   before = tomllib.loads((root / 'settings-save-general.toml').read_text())
   after = tomllib.loads((root / 'config/v4vmm/config.toml').read_text())
   before.update(theme_profile='dark', ui_scale='x-large')
   for key in ('workspace', 'workspace_layout'):
       before.pop(key, None)
       after.pop(key, None)
   assert after == before, 'Save lost a hidden-group value or changed another setting'
   (root / 'settings-save-library.toml').write_bytes((root / 'config/v4vmm/config.toml').read_bytes())
   print('Green')
   PY
   ```

6. **V2 — Use Defaults has global scope.** Prepare the default music root in
   this verified fixture before relaunching. The startup fixture creates its
   seeded `music` directory and isolated `home`, but not `home/V4Vmusic`.
   Save prepares only the `artists` child of an existing music root.

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py verify "$settings_fixture" &&
     mkdir -p -- "$settings_fixture/home/V4Vmusic" &&
     python3 docs/runbooks/startup-recovery-fixture.py run "$settings_fixture"
   ```

   Enter Library and click Use Defaults. General must preview Dark and M;
   Library must show the default endpoint, empty flac and a music directory
   under this fixture's `home`. Defaults immediately saves both groups, as
   before this packet. Quit and verify the intended reset before restoring
   the fixture baseline:

   If the directory setup was omitted, the app saves defaults but reports that
   it could not prepare `home/V4Vmusic/artists` because the parent is absent.
   That preparation failure does not mean the configuration save failed.
   Complete the value assertion below before any fixture reset. After it passes,
   restore the baseline before relaunching so the configured music root exists.
   Step 9 removes any default directory created under this fixture.

   ```bash
   python3 - "$settings_fixture" <<'PY'
   import pathlib, sys, tomllib
   root = pathlib.Path(sys.argv[1])
   before = tomllib.loads((root / 'case.config').read_text())
   after = tomllib.loads((root / 'config/v4vmm/config.toml').read_text())
   expected = dict(musicindex_endpoint='https://api.musicindex.org',
                   music_dir=str(root / 'home/V4Vmusic'), theme_profile='dark', ui_scale='medium')
   assert all(after.get(key) == value for key, value in expected.items()), after
   assert 'flac_path' not in after, 'Defaults did not clear flac'
   for key in (*expected, 'flac_path', 'workspace', 'workspace_layout'):
       before.pop(key, None)
       after.pop(key, None)
   assert after == before, 'Defaults changed an unrelated setting'
   (root / 'settings-defaults.toml').write_bytes((root / 'config/v4vmm/config.toml').read_bytes())
   print('Green')
   PY
   ```

   Only after that assertion prints Green, restore the baseline and inspect:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py mode "$settings_fixture" normal
   python3 docs/runbooks/startup-recovery-fixture.py inspect "$settings_fixture"
   ```

   The assertion verifies the deliberate config edits; the subsequent inspection
   verifies music/database preservation after the baseline config is restored.
   If the assertion fails, keep the fixture unchanged for diagnosis.

7. **V3 — invalid optional settings and direct report routing.** With the app
   closed, select the paired Index/player failure case and launch:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py mode "$settings_fixture" endpoint-and-player-unavailable
   python3 docs/runbooks/startup-recovery-fixture.py run "$settings_fixture"
   ```

   From Music's notice click Open report. One click must enter Settings with
   Diagnostics selected. Both issues, their actual recorded UTC times and
   locations must remain. Copy report and paste into a scratch editor; the full
   report must match even at narrow width. Scroll to the cached files and their
   actions without deleting anything. Walk General/Library and return to
   Diagnostics; report identity and times must remain. In Library, enter a valid
   endpoint and click Save: the existing invalid on-disk configuration must
   still reject the save, with the error visible in Library. General's Save and
   Use Defaults must also reject that same invalid document. Check that useful
   errors are visible in the editable group. Quit and inspect:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py inspect "$settings_fixture"
   ```

   This case requires `config_bytes_unchanged: true` as well as preservation of
   all music/database facts and the runtime blocker. It tests report navigation
   and guarded saves only; it provides no playback acceptance for ADR 0066.

8. **V3 — unavailable runtime.** With the app closed:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py mode "$settings_fixture" runtime-and-cache-unavailable
   python3 docs/runbooks/startup-recovery-fixture.py run "$settings_fixture"
   ```

   Open report must again reach Diagnostics. Confirm both reports remain,
   Copy works, editable groups stay reachable, and cached-file status explains
   the unavailable background tools rather than claiming an empty library.
   Group changes must not create repeated reads or replace recorded reports.
   The fixture's explicit retry procedure remains the
   [ADR 0066 check](startup-recovery-check.md); retry acceptance is separate.
   Quit and inspect:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py inspect "$settings_fixture"
   ```

9. Record V1, V2 and V3 results separately, including theme, width, scale,
   keyboard delivery, config assertions, preservation and any failing step.
   Keep failed fixtures for diagnosis. After recording passing results, remove
   only this new fixture:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$settings_fixture" &&
   unset settings_fixture
   ```

   Close the scratch editor buffer; if you saved a separate report file, remove
   that test report too. Cleanup removes the disposable config, database, music, default music folder
   and saved config evidence inside that directory. It changes no system units
   or desktop audio routes. This packet does not close inherited Music checks;
   record any overlapping ADR 0030 Settings scroll evidence by its exact scope.

## Keyboard Focus Regression Check

For a future focus change, create a new Settings fixture and launch it using
steps 1–2 above. Use these assertions during V1 with the rebuilt binary:

Press Ctrl+Comma, then Tab/Shift+Tab through the group buttons. Each focused
button must have an outline without shifting or resizing its neighbors.
Enter and Space must activate the focused group and leave the outline on it.
The next Tab must continue from that button. Also click another group while a
Library field is focused: focus must move to the clicked group before the field
is hidden, and Tab must continue from that group. Record the keyboard result
alongside V1's width, theme and scale checks. Use step 9 for cleanup after the
relevant regression checks and preservation inspection pass.
