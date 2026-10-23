# ADR 0086 Task 006: The Window Length Setting

Status: Ready - 2026-10-23. It waits for ADR 0069 task 002 and ADR 0086 task 002.

Orchestrator note - 2026-10-23: packet 002 is implemented. These names are `#[cfg(test)]` until a packet calls them: `WindowLength::{MIN_DAYS, MAX_DAYS, from_days, days}`, `LibraryViewModel::{window_length, set_window_length}` and `NewReleasesLayout::any_window_open`. Remove the `#[cfg(test)]` of each name that this packet calls. The group renderers are in `src/ui/shells/library/window_groups.rs`.

## Goal

The Library group of Settings has one row for the window length in days. The value is 1 to 60. The default is 14. No value turns the grouping off.

The value is in `config.toml` as one new top-level key, `singles_window_days`. A `config.toml` with no key loads with 14 days. A change of the value makes the window groups again in the mounted New releases view, with no navigation. The saved searches of packet 005 use the same value.

This packet adds exactly one configuration key. The configuration exception of ADR 0086 Decision 7 releases no other key, no preset and no other format change. The ADR 0066 task 004 gate stays open for each other change.

The packet starts after ADR 0069 task 002 and ADR 0086 task 002 are complete. The phase plan puts packets 003 to 005 before it.

## Authority

- [ADR 0086](../adr/0086-new-releases-singles-window.md) Decision 7: the range 1 to 60, the default 14 and no "off" value. It also gives `#[serde(default)]`, the change in the mounted view, the shared guarded editor and the configuration exception.
- ADR 0086 Decision 1: the window length controls the chain of each window group.
- ADR 0086 Decision 6: the saved searches use the same window as New releases.
- ADR 0086 "Operator Answers - 2026-10-23", answers 3 and 5.
- [ADR 0069](../adr/0069-grouped-settings-and-selective-presets.md): the Settings groups, the draft, the saved value and the running value, Save, Cancel and Use Defaults. "Appearance and other safe local preferences may apply without a service transition."
- [ADR 0069 task 002](adr-0069-task-002-shared-guarded-editor.md): the shared guarded editor, typed field errors and "Do not serialize displayed fallback values for unedited fields".
- [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md): one snapshot, scoped field errors, focused correction, preservation of unedited values, and the pause of automatic saves while an optional value is invalid. Its table row "Theme, scale, or layout value" says: "Use the documented presentation default and report the invalid setting. Do not rewrite it automatically."
- [ADR 0086 task 002](adr-0086-task-002-new-releases-window-groups.md): "Names For Later Packets".
- [AGENTS.md](../../AGENTS.md): "Current-view state must update in place", typed action state, renderer portability, "Every fix gets a guard", and the Serde convention.

## Recorded Facts - 2026-10-23

The orchestrator read each fact in the code on 2026-10-23. Packet 002 and ADR 0069 task 002 are not implemented. Each fact below is about the code of today, unless its heading says "design".

### How the configuration loads

- `src/config.rs` has no `Config` structure with `#[derive(Deserialize)]`. `ConfigSnapshot` (`src/config.rs:370-391`) holds one `ConfigField<T>` for each setting. `ConfigField<T>` is `Result<T, ConfigFieldIssue>` (`src/config.rs:277`).
- `ConfigFieldIssue` (`src/config.rs:252-256`) holds `field`, `kind` and `explanation`, all `Copy`. `ConfigIssueKind` (`:259-263`) is `Missing`, `InvalidValue` or `NotATable`. The rejected value is never stored.
- `ConfigSnapshot::from_bytes` (`src/config.rs:580-699`) parses the TOML table one time. Then it decodes each field with `decode_field` (`:425-440`) or with a special function, for example `snapshot_width` (`:516-529`).
- A missing optional key takes its default in the decoder. For example, `ui_scale` and `theme_profile` use `.map(Option::unwrap_or_default)` (`src/config.rs:663-676`). This is the effect of `#[serde(default)]` in this file.
- `snapshot_width` matches `toml::Value::Float` and `toml::Value::Integer` directly, and gives `InvalidValue` for each other type (`src/config.rs:516-529`).
- The existing tables are `[playback]`, `[broadcast]`, `[workspace]` and `[workspace.layout]` (`src/config.rs:603-606`). The top-level keys are `music_dir`, `db_path`, `musicindex_endpoint`, `flac_path`, `ui_scale`, `theme_profile` and `workspace_layout`.
- Each value that Settings edits today is a top-level key: `musicindex_endpoint`, `flac_path`, `ui_scale` and `theme_profile` (`save_app_settings`, `src/config.rs:953-1000`). The `[workspace]` table holds only values that the app writes automatically (`save_workspace_layout_prefs`, `:1015-1064`).
- `ConfigSnapshot::issues` (`src/config.rs:712-740`) lists the error of each field. `require_saveable` (`:742-750`) refuses each ordinary save while one issue exists. `read_config_for_save` (`:929-933`) calls it.
- `src/config.rs:21` imports `crate::view_models::workspace`. The configuration layer thus already reads one view model type.
- Unknown keys load with no issue and survive a save. The test `adr_0066_ordinary_settings_preserve_core_paths_and_unedited_values` (`src/config.rs:1872-1928`) keeps `extra = "keep"`.
- The test `load_config_parses_workspace_layout_prefs` (`src/config.rs:1570-1601`) loads a file with `[workspace]` and `[workspace.layout]` tables and unknown keys in them.
- The helper `config_snapshot(extra)` (`src/config.rs:2233-2239`) builds a snapshot from `SNAPSHOT_CORE` and extra text.
- `default_config_toml` (`src/config.rs:1138-1205`) writes the first-run document. It sets `theme_profile` and gives each other optional value as a comment.

### How the configuration saves

- `save_app_settings` (`src/config.rs:953-1000`) inserts each value into the existing table and calls `write_existing_config`. That function serializes the whole table with `toml::to_string_pretty` (`:935-951`). Comments of the original file are thus lost. ADR 0066 calls comment-preserving serialization separate work.
- `CorrectionField` (`src/config/correction.rs:17-41`) lists 20 editable units in `ALL`. `text()` (`:47-62`) names the fields whose draft is plain text. Each other field takes a TOML assignment named `value`.
- `CorrectionSource::value` (`src/config/correction.rs:162-176`) gives a text field as its string and each other value as `value = ...`.
- `CorrectionSource::proposed_bytes` (`src/config/correction.rs:199-233`) applies each draft field to a copy of the document. An empty draft of a field that is not core removes the key.
- `CorrectionSource::validate_bytes` (`src/config/correction.rs:235-260`) refuses the draft when an edited field has an issue, or when the draft adds a new issue.
- `CorrectionSource::save_with` (`src/config/correction.rs:286-340`) checks the revision, writes a backup of the exact original bytes, replaces the file and reads it again. `CorrectionReceipt` (`:354-357`) holds `backup` and `fresh: ConfigSnapshot`.
- `CorrectionVm::fields` (`src/view_models/startup/correction.rs:259-276`) shows each entry of `CorrectionField::ALL` in the repair editor. `fields_for_issue` (`:464-470`) selects the field of the first issue.

### How a saved value reaches the running app

- `mount_normal` (`src/app/bootstrap.rs:221-279`) reads `ui_scale` and `theme_profile` from the startup snapshot with `unwrap_or_default()`, then calls `TopApp::new`. It moves `cfg.music_dir`, `cfg.flac_path` and `cfg.workspace_layout` into the call.
- `TopApp::new` (`src/app.rs:292-312`) creates `LibraryApp` with `LibraryApp::new_with_content_view_mode` (`src/library/app_impl.rs:1035-1046`). `TopApp::library` is an `Entity<LibraryApp>` (`src/app.rs:134`). `LibraryApp::vm` is the `LibraryViewModel` (`src/library.rs:221`).
- `LibraryApp::set_content_view_mode` (`src/library/app_impl.rs:452-459`) and `set_musicindex_endpoint` (`:1400-1407`) change one value and call `cx.notify()`.
- `TopApp::save_settings` (`src/app.rs:1172-1247`) applies the saved values to the running owners after `save_app_settings`. ADR 0069 task 002 replaces this route.
- `TopApp::refresh_corrected_settings` (`src/app/settings.rs:102-129`) applies the values of a fresh snapshot to the form and to appearance. `src/app/startup.rs:94` calls it after `CorrectionEvent::Saved`. `src/app/capabilities.rs:382` calls it after a check of `Dependency::Presentation`.
- `configuration_dependency` (`src/application/capability.rs:252-263`) gives `Dependency::Presentation` for each field that it does not name.

### The Settings groups

- `SettingsGroup` (`src/view_models/settings.rs:12-17`) is `General`, `Library` or `Diagnostics`. `General` holds `Scale` and `Theme`. `Library` holds `Endpoint`, `MusicDirectory` and `FlacPath` (`:30-46`).
- `SettingsContent::label` and `SettingsContent::help` (`src/view_models/settings.rs:104-127`) give the text of each row.
- `render_settings` (`src/app/settings.rs:132-229`) gives each row to `settings_field` (`src/ui/composites/settings.rs:205`). A text row uses `settings_text_input` (`src/ui/composites/settings.rs:223`). `TopApp::endpoint_input` is an `Entity<InputState>` (`src/app.rs:151`).
- The guard `adr_0069_settings_group_ownership` (`tests/architecture_tests.rs:16774`) forbids `serde`, `std::fs` and `gpui` in `src/view_models/settings.rs`.
- The ADR 0069 table gives the Library group "Music storage, MusicIndex endpoint and download/conversion tools". It has no Music group.

### From the packet 002 design (not implemented)

- `src/view_models/window_groups.rs` holds `pub(crate) struct WindowLength(u32)` with `DEFAULT` (14), `MIN_DAYS` (1), `MAX_DAYS` (60), `from_days(days: u32) -> Option<WindowLength>`, `days(self) -> u32` and `seconds(self) -> i64`. `days(self)` takes `self` by value, so the type is `Copy`.
- `LibraryViewModel` holds `window_length: WindowLength`, with `WindowLength::DEFAULT` at the start.
- `LibraryViewModel::set_window_length(&mut self, length: WindowLength)` calls the private `refresh_new_releases_layout`. That function builds the `NewReleasesLayout` again from the loaded rows and the tree.
- `LibraryViewModel::window_length(&self) -> WindowLength` and `new_releases_layout(&self) -> &NewReleasesLayout`.
- Packet 002 case R86-2-30 proves that `set_window_length` with 7 days divides a group, and that `from_days` refuses 0 and 61.

### From the ADR 0069 task 002 design (not implemented)

- Settings and recovery share one transaction: a retained source, a saved baseline, one draft and one validation owner.
- The validation owner gives typed field errors. Settings shows each error beside its field.
- Save validates again and keeps the draft after each failure. Cancel restores the baseline. Use Defaults changes the declared fields in the draft only.
- Save does not serialize a fallback value for a field that the curator did not edit.
- While the editor holds a source revision, the automatic workspace writes wait.
- An ordinary Save does not apply a resource. Appearance and other safe local preferences may apply locally.

## Design

### The key

The key is `singles_window_days`, at the top level of `config.toml`. Its value is a TOML integer. Example:

```toml
music_dir = "/home/curator/V4Vmusic"
db_path = "/home/curator/.local/share/v4vmm/v4vmm.sqlite"
singles_window_days = 7
```

`toml::to_string_pretty` writes the top-level values before the tables. A hand edit must also put the key before the first table header. A key below `[workspace]` is `workspace.singles_window_days`, and the app ignores it.

### The loader

`ConfigSnapshot` gets the field `pub(crate) singles_window_days: ConfigField<WindowLength>`. `snapshot_window_length(root)` decodes it:

1. No key gives `Ok(WindowLength::DEFAULT)`.
2. A `toml::Value::Integer(n)` gives `Ok(length)` when `u32::try_from(n)` and `WindowLength::from_days` both give a value.
3. Each other value gives `Err(ConfigFieldIssue)` with the field `singles_window_days`, the kind `InvalidValue` and the explanation `SINGLES_WINDOW_DAYS_EXPLANATION`.

Thus 0, 61, -3, 14.5, `"14"` and `true` each give the field error. The loader never clamps a value.

`issues()` lists the new field. The running app then uses `WindowLength::DEFAULT`, and Settings and Diagnostics show the issue. Automatic saves stop until the curator corrects the value. This is the rule of ADR 0066 for an invalid theme or scale.

### The text of the editor

`window_length_from_text(text)` in `src/config.rs` is the validation owner of the row. It removes the outer spaces. It accepts only ASCII digits, then calls `WindowLength::from_days`. Each other text gives the same `ConfigFieldIssue` as the loader. One explanation thus serves the file and the row.

`CorrectionField("singles_window_days")` joins `CorrectionField::ALL`. The new `CorrectionField::integer` is true for this field only. For an integer field:

- `CorrectionSource::value` gives the digits of a `toml::Value::Integer`. It gives each other value as `value = ...`, as today.
- `CorrectionSource::proposed_bytes` parses the trimmed draft as `i64` into a `toml::Value::Integer`. Text that is not an integer gives the error "App could not read the proposed field. Enter a whole number of days from 1 to 60."
- An empty draft removes the key, as for each other optional field. The app then uses 14 days.

The repair editor and the Settings row thus edit the same digits.

### The Settings row

`SettingsContent::SinglesWindow` is the second row of the Library group, after `Endpoint`. Its label is "Singles window (days)". Its help text is this text:

> New releases and saved searches show one tile for the singles of one artist in this number of days. Enter 1 to 60. The default is 14.

The row is a text input, `TopApp::singles_window_input`. It uses `settings_text_input` and `settings_field`. It adds no new primitive and no new composite.

- **First value.** When the source has the key, the row shows `CorrectionSource::value(CorrectionField("singles_window_days"))`. When the source has no key, the row shows "14", and the draft is not edited.
- **Field error.** When `window_length_from_text` gives an error, the row shows the explanation beside the input. Save is not available while the draft has a field error.
- **Save.** Save writes the key only when the draft differs from the baseline. Save goes through the ADR 0069 task 002 transaction. That transaction validates the whole document again and keeps a backup of the original.
- **Use Defaults.** Use Defaults puts "14" into the draft of this row. It does not save. The Defaults scope text names this row.
- **Cancel.** Cancel restores the baseline text of the row.

### The path of the value to the layout

The value reaches the layout only from a `ConfigSnapshot`. `LibraryApp::apply_configured_window_length(configured, cx)` takes a `ConfigField<WindowLength>`. It calls `configured.unwrap_or_default()`. When the length differs from `self.vm.window_length()`, it calls `self.vm.set_window_length(length)` and `cx.notify()`. Else it does nothing.

Three callers pass the value:

1. `mount_normal` in `src/app/bootstrap.rs`, after `TopApp::new`, with `cfg.singles_window_days`. The field is `Copy`, so the partial moves of `cfg` do not stop this read.
2. `TopApp::refresh_corrected_settings` in `src/app/settings.rs`, with `snapshot.singles_window_days`. This covers a repair correction (`CorrectionEvent::Saved`) and a check of `Dependency::Presentation`.
3. The Settings save route of ADR 0069 task 002 passes `receipt.fresh` to `refresh_corrected_settings`. If that route does not call `refresh_corrected_settings`, it calls `apply_configured_window_length` with `receipt.fresh.singles_window_days`.

The value applies after a successful Save, not while the curator types. A failed Save thus changes no group, and Cancel has no group to restore.

`set_window_length` builds the layout again from the loaded rows. The pager, its cursor, the loaded rows, the scroll position and the selection do not change. The mounted New releases view thus shows the new groups at once. When packet 005 is complete, its saved search groups read `LibraryViewModel::window_length()` and update in the same call.

### Names added by this packet

- `src/config.rs`: `pub(crate) const SINGLES_WINDOW_DAYS_KEY: &str = "singles_window_days"`, `const SINGLES_WINDOW_DAYS_EXPLANATION: &str = "expected a whole number of days from 1 to 60"`, the field `ConfigSnapshot::singles_window_days: ConfigField<WindowLength>`, `fn snapshot_window_length(table: ConfigTable<'_>) -> ConfigField<WindowLength>` and `pub(crate) fn window_length_from_text(text: &str) -> ConfigField<WindowLength>`.
- `src/config/correction.rs`: `CorrectionField("singles_window_days")` in `ALL` (21 entries) and `pub(crate) fn integer(self) -> bool`.
- `src/view_models/window_groups.rs`: `impl Default for WindowLength`, which gives `WindowLength::DEFAULT`.
- `src/view_models/settings.rs`: `SettingsContent::SinglesWindow`.
- `src/app.rs`: the field `TopApp::singles_window_input: Entity<InputState>`.
- `src/library/app_impl.rs`: `pub(crate) fn apply_configured_window_length(&mut self, configured: ConfigField<WindowLength>, cx: &mut Context<Self>)`.
- `tests/architecture_tests.rs`: `adr_0086_window_length_reaches_layout_only_through_config`.

This packet uses these names of packet 002 without a change: `WindowLength`, `WindowLength::DEFAULT`, `WindowLength::MIN_DAYS`, `WindowLength::MAX_DAYS`, `WindowLength::from_days`, `WindowLength::days`, `LibraryViewModel::window_length`, `LibraryViewModel::set_window_length`, `LibraryViewModel::new_releases_layout` and `refresh_new_releases_layout`.

## Orchestrator Decisions

- **D1. The key is `singles_window_days`, at the top level.** Each value that Settings edits today is a top-level key. The `[workspace]` table holds automatic layout writes, and ADR 0069 task 002 makes those writes wait during an edit. A new table adds a table and a whole-table `CorrectionField`, which is more than one key. The name gives the subject and the unit, as `content_pane_width` does.
- **D2. The loader decodes the key directly, with the default for a missing key.** `ConfigSnapshot` is not a serde structure. `snapshot_window_length` gives `WindowLength::DEFAULT` for a missing key, as `ui_scale` does with `unwrap_or_default`. This is the effect of `#[serde(default)]` that ADR 0086 Decision 7 requires.
- **D3. An out-of-range or non-integer file value is a field error, not a clamp.** ADR 0066 keeps each invalid optional value as a scoped issue. It uses the documented default and does not rewrite the file. The app uses 14 days, shows the issue and stops automatic saves until a correction. A clamp hides the error and can write a value that the curator did not choose.
- **D4. The loader matches `toml::Value::Integer` itself.** It does not use `decode_field::<u32>`. The rule thus does not depend on how the `toml` crate converts a float or a string.
- **D5. One validation owner serves the file and the row.** `window_length_from_text` and `snapshot_window_length` give the same `ConfigFieldIssue`. `src/config.rs` imports `WindowLength`, as it imports `crate::view_models::workspace` today.
- **D6. The row is in the Library group, after the MusicIndex endpoint.** General holds the mode and the appearance. A new group needs an ADR 0069 change and is not necessary for one value. The endpoint is the source of New releases, so the two Music values are adjacent.
- **D7. The row is a text input with digits.** The existing `settings_text_input` serves it. A stepper control is a new composite and is out of scope.
- **D8. An integer field kind in the correction editor.** The repair editor and the Settings row both edit plain digits. The repair editor then needs no `value = 7` text for this field.
- **D9. The value applies after a successful Save.** ADR 0069 permits a safe local preference to apply locally. A preview on each key press rebuilds the groups many times and needs a Cancel revert. The fresh snapshot of the save receipt is the only source of the new value.
- **D10. One function changes the length of the running app.** `LibraryApp::apply_configured_window_length` takes the `ConfigField` of a snapshot. The guard of R86-6-19 enforces it.
- **D11. Use Defaults puts 14 in the draft.** When the file has no key, 14 equals the baseline, so Save writes no key. When the file has another value, Save writes 14.
- **D12. Save writes the key only after an edit.** ADR 0069 task 002 forbids a fallback value for an unedited field. A Save of the endpoint thus adds no `singles_window_days` key.
- **D13. The first-run document gives the key as a comment.** `default_config_toml` adds "# singles_window_days = 14" with a short comment. The first-run document thus documents the default and sets no value.
- **D14. A repair correction refreshes the layout through `Dependency::Presentation`.** `configuration_dependency` already gives `Presentation` for an unnamed field. This packet adds no arm, only a test.
- **D15. No other key.** The packet adds exactly one key. It adds no table, no preset, no migration and no change to an existing key.

## Required Changes

1. `src/view_models/window_groups.rs`: add `impl Default for WindowLength` that gives `WindowLength::DEFAULT`. If `WindowLength` does not derive `Copy`, add `Copy`.
2. `src/config.rs`:
   - Import `crate::view_models::window_groups::WindowLength`.
   - Add `SINGLES_WINDOW_DAYS_KEY` and `SINGLES_WINDOW_DAYS_EXPLANATION`.
   - Add the field `singles_window_days` to `ConfigSnapshot`, and set it in `from_bytes` with `snapshot_window_length(root)`.
   - Add `snapshot_window_length` and `window_length_from_text`.
   - Add `self.singles_window_days.as_ref().err()` to `issues()`.
   - In `default_config_toml`, add the comment of D13 before the `[playback]` comment.
   - Add the unit tests of the `config.rs` cases below.
3. `src/config/correction.rs`: add `CorrectionField("singles_window_days")` to `ALL`, change the length to 21, and add `integer()`. Add the integer arm to `value` and to `proposed_bytes`. Add the unit tests of the `correction.rs` cases below.
4. The ADR 0069 task 002 owners of the Settings draft, as that packet implements them: add `singles_window_days` to the Settings field set, to the Use Defaults scope and to the field errors. Use `window_length_from_text` for the field error.
5. `src/view_models/settings.rs`: add `SettingsContent::SinglesWindow`, its label and its help text. Put it after `Endpoint` in `SettingsGroup::Library.contents()`.
6. `src/app.rs`: add `singles_window_input: Entity<InputState>`. Create it as `endpoint_input` is created.
7. `src/app/settings.rs`:
   - In `render_settings`, render `SettingsContent::SinglesWindow` with `settings_text_input(&app.singles_window_input, cx)`.
   - In `refresh_corrected_settings`, set the input text from the snapshot, then call `apply_configured_window_length` with `snapshot.singles_window_days`.
   - In the Use Defaults route, put "14" into the input and the draft.
8. `src/library/app_impl.rs`: add `apply_configured_window_length`.
9. `src/app/bootstrap.rs`: in `mount_normal`, after `TopApp::new`, call `app.library.update(cx, |library, cx| library.apply_configured_window_length(cfg.singles_window_days, cx))`.
10. The Settings save route of ADR 0069 task 002: pass `receipt.fresh` to `refresh_corrected_settings`, or call `apply_configured_window_length` with `receipt.fresh.singles_window_days`.
11. `tests/architecture_tests.rs`: add the guard of R86-6-19. Add `singles_window_input` to the input list of `adr_0069_settings_group_ownership` if that guard lists the Settings inputs.
12. `docs/architecture/screen-inventory/settings.md`: add the row to the Library group.

## Mechanical Acceptance Criteria

Use the prefix `adr_0086_window_length_` for each test. Each test is a unit test beside its owner code, apart from R86-6-19. The test data are recorded values. No test reads the real `config.toml`.

| Case | Owner | Required proof |
|---|---|---|
| R86-6-01 | `src/config.rs` | `config_snapshot("")` gives `singles_window_days` as `Ok(WindowLength::DEFAULT)`, and `days()` is 14. `issues()` is empty. |
| R86-6-02 | `src/config.rs` | A file with `[workspace]`, `[workspace.layout]` and unknown keys, and no top-level key, gives 14. A `singles_window_days = 7` line below `[workspace]` also gives 14 and no issue. |
| R86-6-03 | `src/config.rs` | The file values 1 and 60 give `Ok` with `days()` 1 and 60. |
| R86-6-04 | `src/config.rs` | The file values 0, 61, -3, 14.5, `"14"` and `true` each give `Err` with the field `singles_window_days`, the kind `InvalidValue` and `SINGLES_WINDOW_DAYS_EXPLANATION`. |
| R86-6-05 | `src/config.rs` | In R86-6-04, `issues()` holds the issue one time. `musicindex_endpoint`, `ui_scale` and `theme_profile` stay `Ok`. `singles_window_days.unwrap_or_default()` is `WindowLength::DEFAULT`. |
| R86-6-06 | `src/config.rs` | With `singles_window_days = 0` in the file, `save_workspace_layout_prefs` gives an error, and the file bytes do not change. |
| R86-6-07 | `src/config.rs` | `window_length_from_text` gives `Ok` for "1", "60" and " 7 ". It gives the field error for "0", "61", "", "abc", "14.0", "-1" and "+5". |
| R86-6-08 | `src/config.rs` | The `default_config_toml` text holds "# singles_window_days = 14". Its snapshot gives `WindowLength::DEFAULT` and no issue. |
| R86-6-09 | `src/config/correction.rs` | `CorrectionField::ALL` holds `singles_window_days`. Its `integer()` is true and its `text()` is false. No other field has `integer()` true. |
| R86-6-10 | `src/config/correction.rs` | `CorrectionSource::value` gives "7" for `singles_window_days = 7`, and an empty text for a file with no key. |
| R86-6-11 | `src/config/correction.rs` | A draft "abc" gives the error of the integer arm. A draft "0" gives the field error from `validate_bytes`. Neither draft writes the file. |
| R86-6-12 | `src/config/correction.rs` | A draft that edits only this field to "7" saves. The saved table equals the original table with one more key, `singles_window_days = 7`, at the top level. The backup holds the exact original bytes. |
| R86-6-13 | `src/config/correction.rs` | In R86-6-12, the unknown keys, `[playback]`, `[workspace]` and a nested unknown value keep their values. A draft "" removes the key and keeps each other value. |
| R86-6-14 | ADR 0069 task 002 Settings draft owner | A Settings draft of "0" or "61" gives the typed field error of the row. Save is not available, the draft stays, and the file bytes do not change. A draft of "1" or "60" saves. |
| R86-6-15 | ADR 0069 task 002 Settings draft owner | From a file with no key, a Save that edits only the endpoint writes no `singles_window_days` key. |
| R86-6-16 | ADR 0069 task 002 Settings draft owner | Use Defaults puts "14" into the draft and writes nothing. From a file with no key, a later Save writes no key. From a file with 7, a later Save writes 14. |
| R86-6-17 | `src/view_models/settings.rs` | `SettingsGroup::Library.contents()` is `[Endpoint, SinglesWindow, MusicDirectory, FlacPath]`. The label is "Singles window (days)". The help text is the text of the Design. General and Diagnostics do not hold the row. |
| R86-6-18 | `src/view_models/library.rs` | With the loaded rows of R86-2-01, `set_window_length` with 7 days changes `new_releases_layout()` at once. The loaded rows, the pager cursor, `has_more` and the selection do not change. |
| R86-6-19 | `tests/architecture_tests.rs` | The situational guard `adr_0086_window_length_reaches_layout_only_through_config` passes. It fails against a copy of `src/library/app_impl.rs` with a second `.set_window_length(` call. |
| R86-6-20 | `src/application/capability.rs` | `configuration_dependency("singles_window_days")` is `Dependency::Presentation`. |
| R86-6-21 | `src/view_models/library.rs` or the packet 005 owner | This case applies when packet 005 is complete. A mounted saved search has two singles of one key, 10 days apart. It shows one group with 14 days, and two singles after `set_window_length` with 7 days. |
| R86-6-22 | All | Each test of packet 002, each `adr_0066_` test, each `adr_0069_` test and each existing `config.rs` test still passes. |

The guard of R86-6-19 enforces these rules:

- Outside `src/view_models/`, production code calls `.set_window_length(` one time, inside `fn apply_configured_window_length` in `src/library/app_impl.rs`.
- `fn apply_configured_window_length` takes a `ConfigField<WindowLength>` and calls `unwrap_or_default()`.
- Production code calls `apply_configured_window_length(` only in `src/app/bootstrap.rs` and `src/app/settings.rs`. Each call holds the text `singles_window_days`.
- No file under `src/ui/`, `src/app/`, `src/library/` or `src/presentation/`, and not `src/app.rs`, holds `WindowLength::from_days(` or `WindowLength::DEFAULT`.

The failure message of the guard is this text:

```text
ADR 0086 §7 The Window Length Is A Setting: the window length reaches the New releases layout only from the `singles_window_days` value of a `ConfigSnapshot`.
Call `LibraryApp::apply_configured_window_length` with `snapshot.singles_window_days` after a load or a save. Do not set or build a `WindowLength` in a screen, a shell or a handler.
```

## Visual Acceptance Criteria

These are for the operator. No test proves them. Examine each one in Light and in Dark, at normal width and at narrow width.

- V1: open Settings, Library. The row "Singles window (days)" shows after the MusicIndex endpoint, with "14" and the help text. The label, the input and the help text align with the other Library rows.
- V2: enter 0, then 61, then "abc". Each time, the field error shows beside the row and Save is not available. No text overlaps, and no text goes outside the page.
- V3: enter 7 and click Save. Open Music, Discover, New releases. The groups of 7 days show at once, with no reload and at the same scroll position. Then enter 14 and Save again. The groups of 14 days show again.
- V4: open a saved search that shows a group, when packet 005 is complete. After a change of the value, its groups change in the same way.
- V5: click Use Defaults. The row shows "14". Click Cancel. The row shows the saved value again.
- V6: with the app closed, put `singles_window_days = 0` in `config.toml`. Start the app. New releases uses 14 days. Settings shows the issue beside the row, and Diagnostics, Configuration shows the same issue.
- V7: in the condition of V6, correct the value in the row and click Save. The issue goes.

## Exclusions

- No other configuration key, no new table, no preset, no migration and no change to an existing key.
- No "off" value and no value outside 1 to 60.
- No preview of the groups while the curator types.
- No stepper control, no new primitive and no new composite.
- No change to the chain of Decision 1, to the pager of packet 003 or to the window group page of packet 004.
- No change to the Releases list or to the Decision 6 groups.
- No release of the ADR 0066 task 004 gate for any other change.
- No comment-preserving serialization.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md), the rust-dev skill and [the source map](../architecture/source-map.md).
- [ADR 0086](../adr/0086-new-releases-singles-window.md) Decisions 1, 6 and 7, and its operator answers.
- [ADR 0069](../adr/0069-grouped-settings-and-selective-presets.md), [ADR 0069 task 002](adr-0069-task-002-shared-guarded-editor.md) and the [Settings phase plan](../plans/adr-0069-settings-presets-phase-plan.md).
- [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md): "Configuration Reads Have One Snapshot And One Creation Policy" and "Optional Failures Stay Visible And Limit Dependent Actions".
- [ADR 0086 task 002](adr-0086-task-002-new-releases-window-groups.md): "Names For Later Packets".
- `src/config.rs`: `ConfigSnapshot`, `from_bytes`, `snapshot_width`, `issues`, `require_saveable`, `save_app_settings`, `default_config_toml` and the tests near `config_snapshot`.
- `src/config/correction.rs`: `CorrectionField`, `value`, `proposed_bytes`, `validate_bytes` and `save_with`.
- `src/view_models/startup/correction.rs`: `CorrectionVm::fields` and `fields_for_issue`.
- `src/view_models/settings.rs`, `src/app/settings.rs`, `src/ui/composites/settings.rs` and `src/app.rs`.
- `src/app/bootstrap.rs` (`mount_normal`), `src/app/startup.rs` and `src/app/capabilities.rs`.
- `src/application/capability.rs` (`configuration_dependency`).
- `src/library/app_impl.rs` and `src/view_models/library.rs`.
- `src/view_models/window_groups.rs`, as packet 002 implements it.
- `src/view_models/search_results/saved_search.rs`, as packet 005 implements it.
- `tests/architecture_tests.rs`: `adr_0069_settings_group_ownership`, `adr_0066_config_creation_and_save_ownership` and the guard of R86-2-27.

## Checks

```bash
cargo check
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib adr_0086_window_length_
cargo test
cargo test --test architecture_tests
cargo build --bin v4vmm
```

Run `cargo build --bin v4vmm` last. `cargo test` can leave a binary with GPUI test support at `target/debug/v4vmm`.

## Rollback

Before the first start of the new build, make a backup of `config.toml`:

```bash
cp -p ~/.config/v4vmm/config.toml ~/.config/v4vmm/config.toml.before-adr-0086-task-006
```

The packet is one commit with no migration. To roll back, revert the commit.

- An older build loads a `config.toml` with `singles_window_days`. It reads only the keys that it knows, and it keeps unknown keys when it saves (`src/config.rs:953-1000`).
- An older build ignores the value. New releases then uses the default of packet 002, 14 days.
- An older build reports no issue for an invalid value of the key, because it does not read the key.
- To remove the key, delete its line from `config.toml` with the app closed. Or restore the backup with the app closed. A restore also removes each later change to the file.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0086-task-006-window-length-setting.md`
- ADR 0086 Decisions 1, 6 and 7, ADR 0069, ADR 0069 task 002 and ADR 0066
- ADR 0086 task 002, and its "Names For Later Packets"
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- Use each name in "Names added by this packet" and each name of packet 002 exactly.
- Add exactly one configuration key, `singles_window_days`. The exception of ADR 0086 Decision 7 releases no other key.
- The view model gives each text and availability. The renderer only composes. Use named tokens. Column text does not call `.truncate()`.
- Tests use recorded values. No test reads the real `config.toml` or sends a request to a live service.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The database schema and the migration registry.
- Each configuration key other than `singles_window_days`, and each table.
- The chain in `src/view_models/window_groups.rs`, apart from `impl Default for WindowLength`.
- `src/view_models/singles_group.rs`, the Releases list and the artist pages.
- Any ADR, and each document other than this packet and `docs/architecture/screen-inventory/settings.md`.

Acceptance criteria:
- Each case R86-6-01 to R86-6-22 has a passing test or guard. R86-6-21 applies only when packet 005 is complete.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - <date>" section and an "Operator Visual Check" section for V1 to V7. The Operator Visual Check includes the backup of `config.toml` and its cleanup.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns, and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- ADR 0069 task 002 or ADR 0086 task 002 is not complete.
- A name of packet 002 that this packet uses is missing or has a different signature.
- The ADR 0069 task 002 transaction cannot hold `singles_window_days` without a second transaction, a second writer or a change to its persistence rules.
- The ADR 0069 task 002 Settings save route gives no fresh `ConfigSnapshot` after a save.
- A guard forbids `src/config.rs` to import `crate::view_models::window_groups`.
- Packet 005 is complete, and its saved search groups do not read `LibraryViewModel::window_length()`.
- A change needs a second configuration key, a new table, a change to an existing key or a migration.
- A change needs a release of the ADR 0066 task 004 gate.
- A change needs a new architecture decision.
