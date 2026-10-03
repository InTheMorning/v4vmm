# Screen inventory: Settings, Diagnostics, repair and recovery

Scope: Settings and its groups (ADR 0069). Scope: each `src/config.rs` setting and
where the operator can change it. Scope: Background tools and Diagnostics. Scope: the
repair and recovery pages (ADR 0066, ADR 0074). Scope: the startup report, the startup
notice, and the shared log frames.

| Screen | Reached by | Shell file | View model | Key composites |
|---|---|---|---|---|
| Settings — General | Music/Show/Settings tab bar → Settings (default group) | `src/app/settings.rs` | `src/view_models/settings.rs` (`SettingsVm`) | `src/ui/composites/settings.rs` |
| Settings — Library | Settings → group menu → Library | `src/app/settings.rs` | `src/view_models/settings.rs` | `src/ui/composites/settings.rs` |
| Settings ▸ Diagnostics ▸ Database tools | Settings → group menu → Diagnostics (default page). Startup recovery → recovery page menu → Database shows the same page. | `src/app/settings.rs`, `src/app/startup.rs` | `src/view_models/startup/database.rs` (`DatabaseVm`) | `src/ui/composites/maintenance_forms.rs` (`database_tools`), `maintenance_page.rs`, `log_frame.rs` |
| Settings ▸ Diagnostics ▸ Configuration repair | Settings → Diagnostics → Configuration. Library's "Configuration repair" button. Library's "Audio converter (optional)" → Converter setup opens here on the `flac_path` field. Startup recovery → Configuration shows the same page. | `src/app/settings.rs`, `src/app/capabilities.rs`, `src/app/startup.rs` | `src/view_models/startup/correction.rs` (`CorrectionVm`) | `src/ui/composites/maintenance_forms.rs` (`configuration_correction`), `maintenance_page.rs` |
| Settings ▸ Diagnostics ▸ Background tools | Settings → Diagnostics → Background tools. Or: "Open report" on the in-app capability notice. | `src/app/capabilities.rs`, `src/app/settings.rs` | `src/view_models/startup/capabilities.rs` (`CapabilityReportVm`) | `src/ui/composites/startup_report.rs` (`capability_report`), `maintenance_page.rs` |
| Settings ▸ Diagnostics ▸ App session | Settings → Diagnostics → App session | `src/app/settings.rs` | `src/view_models/startup/session.rs` (`SessionReportVm`) | `src/ui/composites/maintenance_forms.rs` (`session_entry`), `maintenance_page.rs` |
| Settings ▸ Diagnostics ▸ Cached files | Settings → Diagnostics → Cached files | `src/app/settings.rs`, `src/app.rs` | `src/view_models/cached_files.rs` (`CachedFilesVm`) | `src/ui/composites/settings.rs` (`settings_plain_page`, `settings_cached_row`) |
| Startup recovery | At launch, before the primary window mounts, when core checks block opening. Shown again after "End app session". | `src/app/startup.rs` (`StartupScreen`) | `src/view_models/startup.rs` (`StartupReportVm`). Embeds `DatabaseVm` and `CorrectionVm`. | `src/ui/composites/startup_report.rs` (`startup_report`), `maintenance_page.rs`, `log_frame.rs` |
| Session drain | "End app session" from App session, Database tools, or Configuration repair. Or: Quit while work is outstanding. | `src/app/startup.rs` (`StartupScreen::render`, drain branch) | `src/view_models/startup/session.rs` (`SessionReportVm`) | `src/ui/composites/maintenance_forms.rs` (`session_drain`), `log_frame.rs` |
| Shared log frame | Embedded "Report" view of Database tools, Configuration repair, Background tools, App session and Startup recovery | — (composite only) | `src/view_models/log_view.rs` (`LogReadingVm`, `LogSource`) | `src/ui/composites/log_frame.rs` |

Settings has one window-level shell (`settings_frame` in `src/ui/composites/settings.rs`).
Each group below shares it. Navigation is a dropdown menu, not a sidebar. A header row
holds the "Settings" title and a `PageMenu` popover button ("Choose settings group").
When the operator selects Diagnostics, a second `PageMenu` appears ("Choose diagnostics
page").

Below the header, General and Library render as a scrolling field list.
Diagnostics renders one `MaintenancePage`-shaped composite. That composite manages its
own scrolling.

---

## 1. Settings — General

1. **Name / reach.** "Settings", General group (the default). Tab bar → Settings.
2. **Owner files.** `src/app/settings.rs` (`render_settings`). `src/view_models/settings.rs`
   (`SettingsVm`, `SettingsGroup::General`). `src/ui/composites/settings.rs`.
3. **Layout.** Header (title and group menu). Below it, a scrollable column, padding LG:
   group heading "General", then the Scale row, the Theme row, the "Save scope" help
   text, and the Save and Use Defaults buttons.
4. **Elements.** Two fields, each as label, control, and caption help
   (`settings_field`). "UI scale" has 5 buttons, XS/S/M/L/XL
   (`SettingsVm::scale_choices`). "Theme" has 5 buttons, System/Dark/Light/High
   Contrast Dark/High Contrast Light (`ThemeProfile::USER_SELECTABLE`), and the
   selected button shows a leading check icon. The page has no artwork.
5. **Actions.** Each Scale or Theme button applies immediately (`SetScale`/`SetTheme`
   effects). "Save" (`ControlStyle::Primary`) stores General and Library together.
   "Use Defaults" resets Scale to Moderate, Theme to the default, the Endpoint field,
   and the hidden FLAC input, then saves. The two buttons write to disk. The two are
   not marked destructive.
6. **Data.** `config.rs` keys `ui_scale` and `theme_profile`. The app holds live
   copies in `TopApp` fields `ui_scale`/`theme_profile`, and writes them through
   `save_app_settings`. This is app configuration data, not RSS, MusicIndex, or
   ID3 data.
7. **Visual notes.** `SemanticColor::SystemBackground` page background,
   `FontSize::Title2` heading, `FontSize::Caption` help text, `Spacing::XS/SM/MD/LG`
   tokens, `ControlStyle::Primary`/`Ghost`. Density is low. The page has a value at
   all times, so it has no empty or loading condition.
8. **Redesign notes.** `ui_scale` and `theme_profile` are also raw-text-editable in
   Configuration repair (`CorrectionField("ui_scale")`/`"theme_profile"`). This gives
   two editing paths for the same two keys. Navigation is a hidden dropdown, not a
   shown group list. This works against "one clear place for each setting."

## 2. Settings — Library

1. **Name / reach.** Settings → group menu → Library.
2. **Owner files.** Same shell as General. `SettingsGroup::Library` content list.
3. **Layout.** Same header and shell. Content, in this sequence: "Library" heading,
   Endpoint row, Music directory row, Audio converter row, Save scope text, Save and
   Use Defaults buttons, "Configuration repair" button.
4. **Elements.** "MusicIndex endpoint" is a live text `Input` bound to
   `endpoint_input`. "Music directory" is **read-only** caption text
   (`settings_message`), showing `app.music_dir`, not an input. "Audio converter
   (optional)" is one button, "Converter setup" (`SettingsVm::converter_setup`), not a
   path field.
5. **Actions.** Save and Use Defaults work as in General, scoped to Endpoint and the
   hidden music-dir and flac inputs. "Converter setup" dispatches `OpenConverter`, a
   `ConfigureConverter` effect. "Configuration repair" (`SettingsVm::repair_entry`)
   opens the Configuration repair page.
6. **Data.** `config.rs` keys `musicindex_endpoint`, `music_dir`, `flac_path`.
7. **Visual notes.** Same tokens as General. Endpoint is the only shown text box on
   this screen group. Music directory is deliberately inert text.
8. **Redesign notes.** "Save" on this page reads `music_dir` and `flac_path` from
   `TopApp.music_dir_input` / `flac_path_input` (`src/app.rs` `save_settings`,
   lines ~890‑941). These two `Entity<InputState>` fields exist but are **not
   rendered** anywhere in General or Library. A click on Save thus writes back
   values the operator cannot see or edit here. The actual editing of `music_dir`
   and `flac_path` happens only in Configuration repair. This is a live example of
   the plan's "two sources of truth" defect shape.

## 3. Settings ▸ Diagnostics ▸ Database tools

1. **Name / reach.** "Database tools". Settings → Diagnostics (default page) →
   Database task menu. The same composite mounts at Startup recovery → recovery page
   menu → Database. That mount happens before the primary window opens.
2. **Owner files.** `src/view_models/startup/database.rs` (`DatabaseVm`,
   `DatabaseTask`, `DatabaseAction`). `src/ui/composites/maintenance_forms.rs`
   (`database_tools`). `src/ui/composites/maintenance_page.rs`
   (`MaintenancePage`, `PageMenu`). `src/db/maintenance.rs`,
   `src/db/maintenance/{preservation,restore}.rs`, and `src/db/upgrades.rs` hold the
   checks. The checks run on `src/presentation/maintenance_executor.rs`, a worker
   thread apart from the usual app runtime and database.
3. **Layout.** Toolbar row: a task selector dropdown ("Check", "Backup", "Preserve
   files", "Restore", "Repair upgrade") and an Instructions-or-Report toggle. On wide
   layouts (760 px and above) a 256 px actions column sits to the left of the
   content viewport. Below 760 px the actions make a wrapped band (at most a
   quarter of the height) above the content. The content shows scrollable
   Instructions help text or the Report.
4. **Elements.** Path fields change by task: `Source`, `Destination`,
   `RestoreSource`, with label text that also changes, for example "New backup
   file" against "New preservation directory". "Check" shows only Source,
   "Restore" shows RestoreSource and Destination, and the other tasks show Source
   and Destination. A one-line limits caption reads "60-second limit…", and
   "Restore" shows a "Ready for explicit Restore…" block after an inspection. The
   page has no artwork.
5. **Actions.** `ConfiguredSource` reads the config path. `Check`, `Backup`,
   `Preserve` (a file copy, not a verified backup), `EndSession`, `ReviewRestore`,
   `UpgradeBackup` (migrates one new candidate only), `Restore`, `RepairUpgrade`
   (`Restore` and `RepairUpgrade` are **destructive**, `ControlStyle::Destructive`),
   `Cancel`, `CopyReport`. Availability depends on the condition and the task, for
   example `RepairUpgrade` shows only after a Check finds
   `SchemaCompatibility::InterruptedUpgrade`.
6. **Data.** This is not RSS, MusicIndex, or ID3 data. All of it is live SQLite
   file inspection, for example `PRAGMA integrity_check`, foreign-key checks, and
   schema or migration-ledger compatibility. A worker thread, apart from
   the usual app runtime, runs these checks on a private copy. The path fields are
   the only stored app configuration shown here.
7. **Visual notes.** `SecondarySystemBackground` actions column.
   `ControlStyle::Destructive` red-toned buttons mark the two risky actions.
   `ControlStyle::Secondary` marks the other buttons. The Report pane is the
   shared monospace `LogFrame` (`LogSource::Database`). Working, suspended, and
   unavailable conditions show as plain caption sentences above the Report, not a
   spinner or badge.
9. **Redesign notes.** One `DatabaseVm` and one composite serve five different
   tasks in two different screen contexts, Settings and pre-launch recovery. A
   redesign should keep that sharing.
   It could give the task picker more visual weight than a hidden dropdown.
   "Preserve files" is the AGENTS.md and ADR 0066 "maintenance" task. "Repair
   upgrade" is "interrupted upgrade repair". The names do not match the UI labels.

## 4. Settings ▸ Diagnostics ▸ Configuration repair

1. **Name / reach.** "Configuration repair". Settings → Diagnostics →
   Configuration; or Library's "Configuration repair" button; or Library's
   "Audio converter (optional)" → "Converter setup" button, which opens this
   same page with the `flac_path` field pre-selected. Also mounts at Startup
   recovery → Configuration.
2. **Owner files.** `src/view_models/startup/correction.rs` (`CorrectionVm`,
   `CorrectionAction`); `src/config/correction.rs` (`CorrectionField`,
   `CorrectionDraft`); `src/ui/composites/maintenance_forms.rs`
   (`configuration_correction`); `src/view_models/startup/converter.rs` and
   `src/audio_format/probe.rs` (the "Test converters" check).
3. **Layout.** Same `MaintenancePage` toolbar/actions/viewport shape as Database
   tools, but with a plain page heading (no task dropdown). Instructions pane:
   explanation text → field-select button row (one per `CorrectionField`) →
   close help → a raw text editor frame (160 px tall) for the selected field's
   draft value.
4. **Elements.** 20 `CorrectionField` entries map to every `config.rs` key:
   `music_dir`, `db_path`, `musicindex_endpoint`, `flac_path`, `ui_scale`,
   `theme_profile`, `playback.driver`, `playback.mpv_path`, `broadcast.hosts`,
   `broadcast.selected_host`, `broadcast.drop_directory`,
   `broadcast.drop_file_target`, `broadcast.encoder`, `workspace_layout`, two
   `workspace.layout.*` fields, and 4 whole-table fallbacks (`playback`,
   `broadcast`, `workspace`, `workspace.layout`) shown only when a field could
   not be extracted. Selecting `flac_path` additionally shows the "Converter
   setup" help block (`converter::TITLE`/`HELP`/`INSTALLATION`) — this is where
   "Converter setup" actually lives; it is not a separate page.
5. **Actions.** Load / Reload (discard draft) / Close‑Reopen editor / Select
   field / Validate / Test converters (only when `flac_path` selected) / Save /
   End app session (shown when a core field — `music_dir` or `db_path` — is
   dirty) / Copy draft (redacted) / Copy repair report. None are marked
   destructive in code, but Save on a core field requires ending the session
   first.
6. **Data.** Reads/writes the actual `config.toml` on the independent worker
   (`CorrectionCommand`), not a cache. Report text is the raw config file +
   validation outcome, not RSS/MusicIndex/ID3.
7. **Visual notes.** Same tokens as Database tools. The raw editor is a plain
   `Textarea`, i.e. unstructured TOML/line text — no per-field typed control for
   paths, enums or tables.
8. **Redesign notes.** This is the one place every setting can truly be edited,
   but as raw text, with field names like `playback.mpv_path` shown verbatim —
   it reads as a developer console, not a Settings page. `music_dir` is shown
   read-only in Library and only editable here (after ending the session);
   `flac_path` is shown as a button in Library that opens this same editor.

## 5. Settings ▸ Diagnostics ▸ Background tools

1. **Name / reach.** "Background tools". Settings → Diagnostics → Background
   tools; or the compact in-app capability notice's "Open report" button,
   reachable from any tab whenever a background tool has an issue.
2. **Owner files.** `src/view_models/startup/capabilities.rs`
   (`CapabilityReportVm`); `src/application/capability.rs` (`Dependency`,
   `CapabilityFailure`); `src/application/capability_recovery.rs`
   (`RecoveryAction`, `RecoveryIntents` — the retained actions); `src/app/
   capabilities.rs`; `src/ui/composites/startup_report.rs`
   (`capability_report`, `capability_notice`).
3. **Layout — expanded (Settings).** `MaintenancePage` shape: actions column +
   viewport; Instructions pane lists one row per issue/retained action/tool,
   each with label, help caption and its own action buttons; Report pane is the
   `LogSource::Background` log frame.
4. **Elements.** Rows for: (a) current failures across the 9 `TOOLS`
   dependencies (background runtime, thumbnail maintenance, MusicIndex,
   playback, publisher, producer, encoder, converter, presentation); (b)
   **retained actions** — failed original operations (e.g. a stalled audio
   conversion, `RecoveryAction::Conversion`) kept so the operator can act later;
   (c, expanded only) a row per tool with no issue, for visibility.
5. **Actions.** `Configure` (jump to Configuration repair on that field),
   `CheckAgain`, `Repair`/`Verify`/`Retry`/`Review`/`Dismiss` on a retained
   entry (set varies by entry state), `Dismiss` can discard a retained
   conversion via `DiscardConversion` — the one destructive-flavoured action
   here. `CopyReport`.
6. **Layout/data — compact (in-app notice).** Same `CapabilityReportVm`, no
   `MaintenancePage`: a thin banner (`capability_notice`, max 160 px, own
   scrollbar) above the normal workspace, with a one-line issue/action count
   summary and an "Open report" button that jumps to this same page. Not a
   separate screen, a second rendering of the same rows with fewer actions per
   row.
7. **Visual notes.** `SecondarySystemBackground` banner/body, `Spacing::MD/SM`,
   caption-size help text. The compact notice caps height and scrolls
   internally rather than growing, to protect workspace height.
8. **Redesign notes.** "Background tools" (Settings) and the capability notice
   (always-on banner) are the same data shown twice at different detail levels
   — worth naming consistently in a redesign. This page is also the home of
   ADR 0066 task 004 "optional tool isolation."

## 6. Settings ▸ Diagnostics ▸ App session

1. **Name / reach.** "App session". Settings → Diagnostics → App session.
2. **Owner files.** `src/view_models/startup/session.rs` (`SessionReportVm`,
   `SessionAction`); `src/ui/composites/maintenance_forms.rs`
   (`session_entry`/`session_drain`); `src/app/settings.rs`
   (`render_session_tools`).
3. **Layout.** `MaintenancePage` shape: one action button in the actions slot,
   Instructions pane shows the explanation + "Current app session: N", Report
   pane is the `LogSource::Session` log frame.
4. **Elements.** Explanation text (what ending a session stops and keeps
   running), a generation counter, and the session report log. No fields.
5. **Actions.** "End app session" (`EndSession`) — stops app commands and
   built-in playback, discards unsaved Settings edits, then opens recovery and
   maintenance; external publisher/encoder services keep running. This is the
   most consequential action on this page though not styled destructive.
6. **Data.** Live session-lifecycle state (`src/application/
   session_lifecycle.rs`), not content data.
7. **Visual notes.** Same tokens/shape as the other diagnostic pages.
8. **Redesign notes.** "End app session" here, in Database tools, and in
   Configuration repair are three entry points to the same drain — consistent,
   but worth a single shared affordance in redesign rather than three buttons
   with the same label in different contexts.

## 7. Settings ▸ Diagnostics ▸ Cached files

1. **Name / reach.** "Cached files". Settings → Diagnostics → Cached files.
2. **Owner files.** `src/view_models/cached_files.rs` (`CachedFilesVm`);
   `src/app/settings.rs` (`render_cached_files`); `src/application/queries/
   library.rs` (the read); `src/ui/composites/settings.rs`
   (`settings_plain_page`, `settings_cached_row`).
3. **Layout.** Not a `MaintenancePage` — a single scrollable list
   (`settings_plain_page`): heading "Cached files (N)" → flat rows (artist name
   rows, then track rows under each artist) → optional "Delete All Cached"
   button → status line.
4. **Elements.** Artist-name rows are plain caption text; track rows show a
   compact title plus a per-row "Delete" button. No artwork, no provenance
   columns — this is a local-file presence list, not a metadata view.
5. **Actions.** Per-track "Delete" (destructive, `ControlStyle::Destructive` via
   `cached_action`) removes one cached file; "Delete All Cached" (destructive)
   removes all. Both require the background runtime to be available.
6. **Data.** Reads the Library tree filtered to tracks with a bound local file
   (`library_service`/`LibraryTree`), i.e. database + filesystem presence, not
   RSS/MusicIndex/ID3 content.
7. **Visual notes.** Caption-size text throughout, no state badges;
   loading/unavailable/failed/empty are plain one-line messages
   (`CachedFilesVm::status`), and a stale list is explicitly captioned "showing
   the last loaded list" rather than silently refreshing.
8. **Redesign notes.** This is the only Diagnostics page that is a live content
   list rather than a tool/report — its visual language (plain rows, no
   artwork) is far plainer than the Library it is describing.

## 8. Startup recovery

1. **Name / reach.** "Startup checks" (page title). Shown automatically at
   launch, before the normal window mounts, whenever core checks block opening;
   also reached again after "End app session" from inside the running app.
   Internal navigation: a recovery page menu (Startup / Configuration /
   Database).
2. **Owner files.** `src/app/startup.rs` (`StartupScreen`, the single
   root-level `Render` for the whole window — it swaps between recovery,
   normal `TopApp`, and session-drain content; it is one window, not a separate
   one); `src/view_models/startup.rs` (`StartupReportVm`); `src/ui/composites/
   startup_report.rs` (`startup_report`).
3. **Layout.** Heading row: title (Startup page only) + recovery page menu +
   inline check/open feedback. Below: the page named by the menu — Startup
   renders its own `MaintenancePage` (actions: Check again, Open app, Copy
   report, Quit; Instructions = summary sentence; Report = `LogSource::Startup`
   log); Configuration renders the **same** Configuration repair composite as
   §4; Database renders the **same** Database tools composite as §3.
4. **Elements.** Title switches between "Checking startup requirements" /
   "Ready to open the app" / "App needs attention before it can open". Report
   log accumulates every check, preparation receipt and prior session's
   retained report across repeated "Check again" presses.
5. **Actions.** `CheckAgain`, `OpenApp` (primary, only available once core
   checks pass), `CopyReport`, `Quit`. None destructive, but `OpenApp` gates all
   normal use.
6. **Data.** `CoreCheckOutcome` from `src/startup.rs` — live checks of
   configuration file, music directory, and SQLite (open/schema/migrate), plus
   any `PreparationReceipt`/`PreparationError` from database preparation. Not
   RSS/MusicIndex/ID3 data.
7. **Visual notes.** Same tokens/shape as the Settings diagnostic pages — this
   screen and Settings Diagnostics are visually the same design language
   because they share composites.
8. **Redesign notes.** The same three sub-pages (Startup/Configuration/
   Database) exist twice: once as this pre-launch screen, once inside Settings
   Diagnostics. A redesign should decide if that duplication of entry points
   (not of code — the composites are already shared) stays or is unified.

## 9. Session drain

1. **Name / reach.** No title shown; a full-window wait state entered right
   after "End app session" (from §6, §3 or §4) or Quit while work is
   outstanding. Exits automatically into Startup recovery (§8) once drained.
2. **Owner files.** `src/app/startup.rs` (`StartupScreen::render`, the
   `self.draining.is_some()` branch); `src/ui/composites/maintenance_forms.rs`
   (`session_drain`); `src/view_models/startup/session.rs`.
3. **Layout.** Single full-size column, padded LG: title "App session" →
   waiting sentence → `LogSource::Session` log frame (fills remaining height) →
   action row.
4. **Elements.** One explanatory sentence ("App is finishing its work and
   releasing its database connections…") and the live session report log.
5. **Actions.** "Retry drain" (only enabled once a drain attempt has finished
   and failed), "Copy report", "Quit".
6. **Data.** Session-lifecycle drain state; the report lists exactly which
   admitted work is still outstanding.
7. **Visual notes.** `SystemBackground`, `FontSize::Title2` heading,
   `ControlStyle::Secondary` buttons — visually plain, intentionally a holding
   screen.
8. **Redesign notes.** This is a real, user-visible screen with no entry in
   AGENTS.md's screen vocabulary; a redesign pass should name it explicitly so
   it is not rediscovered as a surprise state.

## 10. Shared log frame (composite)

1. **Name / reach.** Not independently navigable; it is the "Report" half of
   every Instructions⇄Report toggle in §3, §4, §5, §6 and §8.
2. **Owner files.** `src/ui/composites/log_frame.rs` (`LogFrame`, `LogFrames`,
   `LogFrameState`); `src/view_models/log_view.rs` (`LogReadingVm`,
   `LogSource`, `FollowAvailability`, `LogFooterLayout`).
3. **Layout.** Bordered, rounded panel; scrollable text viewport with an
   overlaid always-on scrollbar; a footer row with a status caption (left) and
   a "go to latest" button (right, icon-only when narrow).
4. **Elements.** Plain monospace, selectable text (`log_font_family`,
   `LOG_TEXT_SIZE`/`LOG_LINE_HEIGHT`), 200 px tall unless `.fill()`-ed to the
   viewport. No structure, headings or color-coding inside the text itself —
   it is one long accumulating string per `LogSource`.
5. **Actions.** Implicit scroll-to-follow/pause-on-manual-scroll, and an
   explicit "go to latest" action when not following.
6. **Data.** Exactly five sources are in this scope's reach:
   `LogSource::Startup`, `Session`, `Configuration`, `Database`, `Background`
   (two more, `Service`/`Event`, belong to Show). Each source keeps its own
   scroll position and follow state even while its page is hidden — switching
   tasks/pages does not reset reading position.
7. **Visual notes.** `SecondarySystemBackground` fill, `Separator`-colored
   border, `Radius::SM`. Identical appearance in every host screen.
8. **Redesign notes.** The report is always a plain scrolling log, never a
   structured/foldable view — appropriate for "verbose reports and logs stay,"
   but a candidate for lightweight formatting (timestamps, severity) without
   losing the raw text.

---

## Cross-screen observations

**Setting inventory — every `config.rs` key and where it can change.**

| Key | Friendly UI | Raw UI | Notes |
|---|---|---|---|
| `ui_scale` | General: 5 scale buttons | Configuration repair field | Two places |
| `theme_profile` | General: 5 theme buttons | Configuration repair field | Two places |
| `musicindex_endpoint` | Library: text input | Configuration repair field | Two places |
| `music_dir` | Library: **read-only** text | Configuration repair field (core, needs End session) | Shown in one place, editable in another |
| `flac_path` | Library: "Converter setup" button → opens repair | Configuration repair field + Test converters | Library button and raw editor are the same destination |
| `db_path` | — | Configuration repair field (core, needs End session); also just *read* by Database tools' "Use configured database" | No friendly UI anywhere |
| `playback.driver` | — | Configuration repair field only | No friendly UI |
| `playback.mpv_path` | — | Configuration repair field only | No friendly UI |
| `broadcast.hosts` | Show screens (out of this scope) | Configuration repair whole-table fallback | Two places, one outside Settings |
| `broadcast.selected_host` | Show screens | Configuration repair field | Two places |
| `broadcast.drop_directory` | Show screens | Configuration repair field | Two places |
| `broadcast.drop_file_target` | Show screens | Configuration repair field | Two places |
| `broadcast.encoder` | Show screens | Configuration repair whole-table fallback | Two places |
| `workspace_layout` | Set implicitly by resizing/arranging the Library workspace | Configuration repair field | No dedicated Settings UI; changed by direct manipulation elsewhere |
| `workspace.layout.content_pane_width` | Set implicitly by dragging the content pane splitter (`src/app/resize.rs`) | Configuration repair field | No dedicated Settings UI |
| `workspace.layout.content_list_view_mode` | Set implicitly by toggling the Library view mode (`src/ui/shells/workspace.rs`) | Configuration repair field | No dedicated Settings UI |

Also flagged: `TopApp.music_dir_input` and `flac_path_input` (`src/app.rs`) are live
`InputState` entities that `save_settings` reads on every Save, yet neither is ever
rendered in General or Library — a hidden pass-through, not a second genuine editing
surface, but a latent two-sources-of-truth risk if either diverges from the visible
value.

**Reports/logs that must stay reachable** (one `LogSource` each, independent scroll
position): Startup report (§8), Session report (§6, §9), Configuration repair report
(§4), Database tools report (§3), Background tools report (§5). All five share one
`LogFrame` composite and must keep a Report view in any redesign, per the plan's "the
verbose error reports and logs stay."

**Repeated patterns.**
- Every Diagnostics page (§3–§6, and §8's two sub-pages) is the *same*
  `MaintenancePage` shape: toolbar with Instructions⇄Report toggle, actions
  column/band, scrollable Instructions, `LogFrame` Report. Cached files (§7) is the
  one exception — a plain list, no actions column, no log.
- Three composites — Database tools, Configuration repair, and the whole Startup
  recovery screen — are mounted in two different screen contexts (Settings
  Diagnostics vs. pre-launch recovery) from the same view model and composite code.
  This is good code reuse but means a redesign of "where things live" must move both
  mount points together.
- Navigation throughout Settings is dropdown popovers (`PageMenu`), never a visible
  sidebar or tab strip — true for the group menu, the diagnostics-page menu, the
  database-task menu and the recovery-page menu alike.

**Inconsistencies / two-places findings.**
- `ui_scale`, `theme_profile`, `musicindex_endpoint` each have a friendly control
  *and* a raw Configuration-repair field for the same key (see table above) — the
  raw editor is reachable at all times, not disabled once a friendly control exists.
- `music_dir` is read-only where it is visible (Library) and only editable where it
  is not labeled as a setting at all (Configuration repair's field-button list).
- The in-app capability notice and the Background tools page render the same
  `CapabilityReportVm` rows at two detail levels; they are consistent with each
  other but are easy to mistake for two different features.
- "Converter setup," named as if it were its own screen in Library's help text and
  in AGENTS.md, is actually a disclosure state inside Configuration repair
  (`CorrectionVm::converter_selected`), not a distinct page.
- "Database maintenance," "interrupted upgrade repair," and "optional tool setup"
  (as named in AGENTS.md/ADR 0066) are internal task labels — "Preserve files,"
  "Repair upgrade," and the Background tools retained-action flow, respectively —
  inside the three composites above, not separate screens.
