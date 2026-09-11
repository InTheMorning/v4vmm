# ADR 0066: Configuration And Startup Failure Recovery

## Status

Accepted - 2026-09-10.

Implementation partial: tasks 001–003 complete, including operator acceptance,
preservation inspection and fixture cleanup. Tasks 004–013 have not started in
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md).
Amended 2026-09-11: task 003 completion is verified; task 004 is the next packet.
Other optional-tool isolation and in-app correction tools remain unimplemented.

Amended 2026-09-10: [ADR 0067](0067-platform-shortcut-modifiers.md) changes Linux
shortcuts to Ctrl at the operator's request. Task 003's keyboard checks
use those bindings; its runtime-rejection requirement and recorded passes remain.

Revised 2026-09-10 after operator review: normal startup requires valid core
configuration, usable storage for music files, and a working SQLite database.
Optional tools and external services do not become startup requirements because
the current constructor expects them. Their failures must be visible in the
app, with tools to correct the problem and retry the intended action. Settings
and core recovery share configuration and database maintenance tools. The
operator accepted this decision on 2026-09-10. It owns deferred item 6;
item 7 and other configuration format changes remain behind its implementation
and verification in the [delivery order](../plans/broadcast-chain-delivery-order.md).

## Context

A structurally invalid `config.toml` caused a startup panic on 2026-09-07.
Operators edit this file because Settings exposes only some of its fields.
A panic names an internal call but gives a desktop-launched user no reliable
recovery surface.

`run_app` in [bootstrap.rs](../../src/app/bootstrap.rs) also uses `expect` for
directory creation, database setup, path repair, playback construction,
broadcast producer construction, runtime creation, and window operations.
These failures need different explanations. Replacing the configuration
loader's `expect` alone leaves the same failure mode at the next stage.

Three current behaviors constrain recovery:

- At decision time, [config.rs](../../src/config.rs) treated `Path::exists`
  failure as permission to write defaults, including from ordinary saves.
  Task 001 replaces that behavior; its preservation and ownership guards are
  linked below. The GUI still reads configuration again for its endpoint
  until task 004 migrates scoped consumers.
- `open_db` initializes schema and applies migrations.
  `repair_local_file_paths` updates rows and records unresolved paths before
  removing their download bindings. Neither function wraps its entire work in
  one transaction. An error does not prove that no database changes occurred.
- `ConfiguredPlaybackDriver::from_config` prepares mpv's runtime directory;
  it does not start mpv. Binary availability is checked lazily under
  [ADR 0021](0021-mpv-playback-driver.md). A missing binary is not a failure
  of this constructor.

Malformed optional layout values already fall back with a warning under
ADRs 0046/0051. An unreadable file or invalid TOML document is a different case.

## Decision

### The Minimum Is Configuration, Music Storage, And SQLite

Expected configuration, filesystem, database, and resource failures produce a
typed result. They do not panic. The normal app requires:

1. A readable TOML document with valid core settings: `music_dir` and `db_path`.
2. A usable music directory at the configured location.
3. A working SQLite database at the configured path, with the expected schema.

MusicIndex, playback, the drop-file producer, publisher control, the relay,
the encoder, and media-conversion tools are optional capabilities. Invalid
settings or initialization failures for one of them prevent only the operations
that depend on it. An external service being down never blocks app startup.
No successful network response is a condition of opening the app.

If a core requirement fails, show a recovery surface when the desktop
window system is available. It names the failed operation, path, reason, and
recovery action and the applicable correction tools. Normal library and show
operations do not start until core checks pass. Recovery is a startup surface,
not a fourth app section. It shares maintenance tools with Settings without
requiring a healthy library database or the normal app runtime.

Today's `TopApp` constructor couples the shell to a playback owner and command
runner. Change that wiring to express unavailable capabilities. Do not turn it
into a product requirement, substitute an empty database, or silently replace
a failed configured player with `Null`. An explicitly selected/default Null
driver retains ADR 0021's existing meaning.

### Verify Storage Through Operations, Not Path Existence

For music storage, verify that the configured directory can be listed and that
the app can create, write, read back, and remove a uniquely named probe file.
Never open an existing music file for this test. A failure names the operation
and path; failed probe cleanup reports the remaining path. An existing config
whose music folder is absent requires correction or mounting that storage.
Do not create a replacement folder that conceals an absent mount. First-run
setup may create the newly configured default music folder.

For SQLite, opening a connection alone is insufficient. Verify the expected
schema, a database read, and a bounded transactional write to the configured
database whose probe changes are rolled back. A write to a separate in-memory
or TEMP database does not verify this database. A failure prevents normal
startup; it never authorizes reset or replacement. Database owners supply the
checks and a finite lock wait, with concrete mechanics named in the packet.

These checks establish usability at that time. They do not certify every
track, scan all database pages, or guarantee that a drive will remain mounted.
Later operation failures must still be reported. This decision does not change
the established first-run database creation or schema-migration policy.

### Configuration Reads Have One Snapshot And One Creation Policy

Resolve the configuration path separately from creating its parent directory,
so a directory error can still identify the intended file. Read one document
per startup attempt. Validate the core settings and decode optional settings
independently from those same bytes before initializing resources. One invalid
optional block must not make deserialization of the whole `Config` fail.
Share the parsing policy with CLI readers; each command requires only the
settings it uses and reports an error if one of those is invalid.
This adds no TOML keys and does not merge the workspace sections.

An existing configuration is never rewritten automatically during startup or
recovery. An explicit correction follows the guarded save policy below. A
read failure is not evidence that the file is absent. In particular, permission
errors, invalid UTF-8, invalid TOML, invalid field values, and a dangling
configuration symlink must not invoke default creation.

Retain first-run creation only for an absent configuration path. Publish a
complete, validated default document without replacing an existing directory
entry. If another writer creates the file first, read that file. If creation
fails, report it; do not proceed with an unsaved in-memory configuration or
leave a partial default document as if setup succeeded. Failure to remove a
temporary creation artifact is reported with its path.

Only first-run loading may create the file. `save_app_settings`,
`save_workspace_layout`, and `save_workspace_layout_prefs` must return an error
when the current file is missing, unreadable, structurally invalid, or has
invalid core settings. They must not recreate defaults. Dedicated recovery
commands may correct the document explicitly; they are not ordinary autosaves.

When optional settings are invalid, pause automatic configuration persistence
and unrelated configuration saves. The app must not serialize fallback values
over the settings the operator needs to repair. An explicit correction through
a Settings field that already owns that setting may save the corrected value:
validate the core document and the corrected setting, preserve all unedited
TOML values, and keep reporting any other optional errors. Settings and recovery
provide focused editors for affected settings; a raw document editor handles
syntax errors that prevent field extraction. Resume automatic
persistence only after a fresh read confirms no optional validation errors.
Normal database edits do not depend on configuration-file persistence.
Concurrent edit merging and comment-preserving serialization are separate work.

Missing optional settings retain their documented defaults. An explicitly
supplied `musicindex_endpoint` must be a valid string accepted by
`normalize_musicindex_endpoint`; a value of the wrong type disables MusicIndex
requests and identifies the setting to correct. It does not select the default
server or prevent startup. No network request is required to validate this URL.

### Optional Failures Stay Visible And Limit Dependent Actions

Maintain typed capability availability and a collection of scoped issues.
Invalid configuration, unattempted contact, and a failed service observation
are different facts. A valid URL does not prove that a server is reachable.
Do not claim a service is healthy just because its adapter was constructed.
Deliberately unconfigured tools do not generate failure notices merely by
being optional. Their own surfaces may explain what setup enables them.

| Optional setting or dependency | Scope of failure |
|---|---|
| `musicindex_endpoint` or MusicIndex request | MusicIndex-backed requests only; local library work and independent RSS operations remain available. |
| Playback settings or driver setup | Built-in playback operations; browsing and external broadcast control remain available. |
| Broadcast host list or selected host | Operations requiring that publisher host. Preserve independently valid encoder and local-producer settings. |
| Drop-file directory or target | Built-in metadata file publication; audio playback and unrelated publisher operations remain available. |
| Encoder settings or encoder command | Encoder operations only. |
| `flac_path` or conversion process | Conversion operations that require it. Preserve the existing download-format policy; do not silently substitute a different configured executable. |
| Theme, scale, or layout value | Use the documented presentation default and report the invalid setting. Do not rewrite it automatically. |
| Shared background runtime | Every operation that needs that runtime. Keep the shell, reports, and actions that do not require it available; do not perform the blocked work on the UI thread. |

A malformed whole optional table disables its dependent capabilities. A bad
independent field inside a readable table does not disable valid siblings.
The packet must inventory these dependencies at both command and query
boundaries. Toolbar and keyboard dispatch use the same typed availability as
buttons. No command can bypass it by assuming a constructed adapter exists.

The normal app shows a concise notice when tools are unavailable, with a route
to the full issue list in Settings. Show also exposes the issue in its affected
item's status. The list names the tool, what failed, the resulting limitation,
and the recovery action. Issues remain visible until corrected or superseded
by an actual successful observation; they are not confined to stderr or a
transient toast. Multiple failures must not overwrite one shared status string.

### Offer A Fix And Return To The Intended Action

An unavailable execution dependency must not leave a dead control as the only
route forward. Keep the feature discoverable and expose an enabled, typed
remediation action such as **Configure converter**, **Choose folder**,
**Check again**, or **Database tools**. A blocked primary action may open that
repair flow; it must not dispatch the unsafe operation. The view model owns
both the limitation and its remediation intent. Keyboard entry follows the
same flow.

Open the relevant Settings tool directly and retain the original operation's
identity and inputs. Let the operator correct the problem, verify it with a
fresh observation, and **Retry** the operation. An explicit **Save and retry**
may combine those steps. Saving a setting alone never silently repeats a
download, publisher mutation, or other user action. Revalidate availability
and the operation's subject at retry time so changed selections cannot act on
the wrong track or event.

For a WAV conversion failure, name the track and failed converter operation.
Offer converter setup: choose the FLAC executable, test it, or obtain platform
installation guidance when no converter is installed. Recheck after setup and
retry that track's failed work without requiring the user to find it again.
Installation is an explicit operator action; a status check never installs
packages. Preserve downloaded input while repair is pending where it is already
available; validate it before reuse, clean failed output, and do not create a
duplicate library entry. If input cannot be reused, the retry reports that it
will download the track again. Existing artifact-validation rules still apply.

Current [`audio_format::transcode_wav_to_flac`](../../src/audio_format.rs) also
supports ffmpeg fallback, and [`track_compare`](../../src/track_compare.rs) can
record a conversion warning rather than fail the whole
download. The report must reflect the actual outcome; it must not claim that
every WAV download needs FLAC or that an existing usable fallback failed.
`flac_cli_available` and `ffmpeg_cli_available` cache PATH results with
`OnceLock`. Explicit verification/retry must make a fresh probe and refresh
dependent availability instead of reusing a cached negative result.

Use the same correction-and-retry pattern for service configuration and
connection errors. Existing service check/start actions remain available where
their own prerequisites permit them. Do not silently restart external services
or block unrelated work while one tool is being repaired.

### Settings And Core Recovery Share Repair Tools

Recovery offers the applicable tools plus **Check again**, **Copy report**,
and **Quit**. Selecting a file or editing text does not itself save or rerun
startup. Configuration corrections validate a proposed document, preserve the
original as a separately named backup, and replace it only after an explicit
save. If the document changed since editing began, report the conflict instead
of overwriting it. An unreadable file has no invented editable contents.
Folder/path tools identify the current location and test a proposed one before
saving it. Choosing another location does not silently move music files or
create an empty replacement database.

**Check again** performs fresh core checks without reapplying migrations or
path repair as a hidden side effect. It discards stale prepared resources.
After the checks pass, an explicit **Open app** completes preparation and
mounts normal operation once. Schema repair is a separate named action below.
For a healthy running app, reinitialize only the corrected optional capability;
do not rerun the whole bootstrap. Where core maintenance requires restarting
the session, the app manages the transition and retains its report rather than
requiring the operator to close and relaunch it manually.

Database tools are available both in Settings and when a core database check
fails. Their commands do not depend on `TopApp` holding an open connection:

| Tool | Contract |
|---|---|
| **Check database** | Inspect the selected database without implicitly running `open_db` migrations or path repair. Report access, integrity, and schema compatibility separately, with a named next action. |
| **Back up database** | Produce a consistent SQLite snapshot at a chosen destination, including committed data still in WAL. Validate and report the saved snapshot. Do not copy only the main file of a live database. The UI explains that this backup covers database records, not music files or broadcaster token files. |
| **Preserve database files** | When a database cannot be opened for a valid snapshot, offer preservation of the original file and associated journals after writers are quiesced. Label it a preservation copy, not a verified restorable backup. If safe access cannot be obtained, report that limitation and retain the original. |
| **Restore database** | Validate a chosen backup and schema compatibility in a separate candidate before changing the active database. Explain which database will be replaced and obtain an explicit restore action. Quiesce dependent work and close connections, preserve the current database and journals, then install the validated candidate. On failure retain the original and report recovery paths. Reopen and verify before resuming work; never swap beneath live connections. |
| **Repair interrupted upgrade** | Recognize a supported incomplete schema upgrade and offer its named, tested migration recipe after making a preservation backup. Work on a candidate copy, validate it, and apply it through the same controlled replacement path as Restore. Report what changed. Arbitrary corruption is not evidence that replaying migrations will repair it; unsupported cases offer preservation and restore instead of guessing. |

Upgrade repair remains a migration under
[ADR 0016](0016-schema-migration-discipline.md). It uses the same registry and
execution/version-recording path as normal startup: currently `MIGRATIONS`,
`migrate_schema`, and `record_migration` in `src/db.rs`. Running against a
candidate changes the destination connection, not the schema authority.
The installed candidate carries its migration records with it. No repair-only
SQL registry or independent version markers are permitted. A new corrective
schema change gets a new migration in that registry. The database-maintenance
packet must prove fresh, migrated, and interrupted-upgrade paths, including
identical version recording and safe handling of partially applied work.

Replacement requires exclusive maintenance access. If another process or an
in-flight operation prevents safe access, report it and offer a fresh check;
do not assume that closing one connection stopped every writer.

No generic repair button may delete rows, drop tables, or replace the database
with an empty one to make checks pass. A repair recipe identifies the condition
it fixes, preserves a rollback copy, and records its result. Corruption salvage
beyond a supported recipe requires a separate decision; this ADR does not
promise automatic recovery of every damaged database.

Maintenance commands use an independent, bounded background execution path
owned by the presentation/application boundary, with file and SQL work in the
backend owners. They must remain callable when the normal runtime cannot start;
they never perform blocking repair work in a renderer. If that execution path
also cannot start, retain the report and available non-I/O actions.

### Failure Policy By Startup Stage

Core configuration/music/SQLite failures enter recovery. Optional preparation
failures remain scoped issues. A missing window uses stderr and unsuccessful
exit; failure to activate an existing window is nonfatal, and a closed window
ends startup. Expected failures are not programmer-invariant violations.

An error from `repair_local_file_paths` does not prove rollback. Recheck the core
resources; if they are still usable, keep independently validated bindings
available and contain unvalidated legacy bindings. If core usability is lost,
enter recovery. Preserve ADR 0064's skip contracts: `NothingResolved` alone is
not a core failure; `MusicFolderMissing` means required storage is unavailable.

[Task 002's stage inventory](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md#startup-stage-inventory)
assigns every current bootstrap boundary to its implementation packet, including
the runtime/cache follow-through in 003 and other optional paths in 004. It owns the procedures
and failure-injection checks; no constructor adds to the minimum requirements.

Keep programmer-owned invariants explicit, including the resolved configuration
path having a parent, fixed ApplicationServices wiring and unique workspace
frame identifiers. Do not catch arbitrary panics and label them as operator
configuration errors. This decision does not promise recovery from aborts inside
the window-system library.

### Recovery Reports Explain The Failure And The Next Action

The report contains an actual UTC observation time, a typed failed stage, the
known file/resource, a short cause, the consequence, and a recovery instruction.
The view model owns this wording and typed actions. Technical details follow
the explanation and are available for copying.

This ADR extends the recorded-UTC precedent already enforced for
[ADR 0063 Event reports](0063-show-dashboard-layout.md#event-diagnostics-reuses-the-bottom-pane)
to startup and maintenance reports. The later
[cross-repository timestamp contract](../plans/broadcast-chain-delivery-order.md#consistent-utc-log-timestamps)
must retain those canonical instants and explicit UTC reporting. It still owns
common precision, emitter/adapter corrections, and future locale display
preferences. This scoped decision does not claim that upstream logs already
conform or reopen A11/A12's navigation decisions.

For example, a parse report reads:

```text
App could not parse its configuration.
File: /home/operator/.config/v4vmm/config.toml
TOML error at line 12, column 4: expected an equals sign.
App did not change this file. Library and show operations have not started.
Open the configuration editor to fix line 12, then choose Validate and save.
```

The actual report includes its recorded UTC time; the example does not invent
an observation. A database error must name the database operation instead of
using the configuration wording. No report promises that all startup work was
rolled back. Existing configuration bytes are protected, but directory creation,
schema work, and completed path-repair statements can precede a later failure.

Do not copy raw TOML or token contents into a report, including stderr and the
clipboard. Parser diagnostics must retain the useful location and reason
without reproducing a source excerpt that could contain credentials. Redact
credentials in URL details and omit quoted rejected configuration values as
well. File paths and configuration key names remain useful diagnostic subjects.

Recovery/report actions have typed availability and accessibility labels,
including the applicable repair, check, and open-app actions. **Copy report**
produces the complete safe report,
including long paths and technical details, for the platform clipboard. GPUI's
current clipboard write returns no result; do not invent a confirmed write or
a detectable write failure. The operator check verifies the pasted text.
Closing core recovery before successful resumption leaves a failure exit
status. After core recovery succeeds and the normal app opens, a later normal
quit is not reported as a failed startup.
The same report is written to stderr. Nonfatal issues also go to stderr and
the in-app issue list; they do not set a failure exit status merely because
an optional tool was unavailable. CLI commands keep their JSON success
contracts: a configuration error relevant to that command goes to stderr with
path and recovery context and a nonzero exit. An invalid unrelated optional
setting does not fail the command. First-run notices belong on stderr too.

No recovery action offers **Continue anyway** or replaces a broken document
with defaults. Check, correction, and resumption are explicit operations with
separate results. A fresh check does not imply rollback of earlier startup work.

### Ownership And Presentation

Assigned owners; implementation packets pin the concrete file inventory:

| Owner | Responsibility |
|---|---|
| `src/config.rs` | One snapshot; separate core and optional validation; safe first-run creation; guarded persistence, document validation, preservation backup, and explicit correction. No GPUI types. |
| `src/db.rs`, `src/library_path.rs` | SQLite checks, database backup/restore/recognized-upgrade repair, existing path-repair ownership, and validation of usable file bindings. Do not duplicate their rules in bootstrap. The packet may extract database maintenance into a backend module. |
| New `src/startup.rs` | Renderer-free core outcome, per-capability preparation results, scoped issues, music-storage verification, and safe diagnostic context. Calls database and driver owners. |
| New `src/view_models/startup.rs` | Core recovery and Settings maintenance models, normal-app notice and issue list, typed repair/check/report/resumption actions, and timestamps. No transport errors or renderer types in the display contract. |
| `src/view_models`, `src/application`, `src/app/show.rs` | Project typed action availability from actual dependencies and revalidate at dispatch. Preserve independent queries/actions; expose tool failures in the existing feature status models. The packet names each affected owner. |
| `src/audio_format.rs`, `src/track_compare.rs`, download application/view-model owners | Fresh converter checks, truthful conversion outcomes, recoverable operation identity, and retry without duplicate materialization. The packet names the download/staging owners before changing retention. |
| `src/presentation` and application maintenance commands | Bounded maintenance execution independent of the failed normal runtime; explicit teardown/resumption and original-action retry. Domain work stays in backend owners. |
| `src/app/bootstrap.rs`, `src/main.rs`, `src/app.rs` and affected child composition roots | Mount `TopApp` when core checks pass, including when optional preparation fails. Carry unavailable resources explicitly; do not call a constructor needing a missing runtime or player. Wire the notice and Settings issue list. Own lifecycle, stderr fallback, and exit outcome. |
| New `src/ui/composites/startup_report.rs` and shared maintenance form composites | Recovery/Settings repair hierarchy, notice, editors, and responsive geometry; use shared button/text/scroll owners. App screens compose them and wire callbacks. |
| `src/ui/tokens.rs`, `src/ui/control_styles.rs`, `src/ui/icons.rs`, `src/ui/theme_bridge.rs` | Existing named typography, spacing, semantic colors, controls, icons, and theme path. The recovery surface uses the pre-config Dark/Medium theme already installed by bootstrap. No configuration write is needed to display it. |

Keep the failure subject and recovery action visible. The normal-app notice
must be available even when the tool that failed has no mounted page. Put extended technical
details behind disclosure in a bounded, scrollable area; long paths must wrap
and remain fully copyable. Actions must remain reachable at narrow widths and
with long diagnostics. This is a report, not a streaming log, and does not
decide A11/A12's log navigation or cross-repository timestamp contract.

## Invariants

1. Normal startup requires valid core configuration, verified music storage,
   and verified SQLite. Optional tools and network responses are not additional
   requirements. A working window system is necessary to present either surface.
2. Expected failures are typed results. An optional failure restricts dependent
   actions and remains visible in the app; it never silently selects another
   server, driver, or storage location.
3. Existing configuration bytes are unchanged by automatic startup and recovery.
   Invalid
   optional settings suspend automatic configuration persistence. Only explicit
   correction can write such a document, preserving unedited values and a
   separately saved original. Only first-run loading may create an absent
   configuration file.
4. One startup attempt reads one snapshot and validates core and optional
   settings independently. A bad optional field cannot become a core parse
   failure after the TOML document itself parsed successfully.
5. Failed core checks mount repair tools without normal library/show operations
   or automatic configuration persistence.
   Successful core checks permit startup with explicit optional availability.
   A missing runtime cannot fall through to an implicit runtime constructor.
6. Storage probes do not alter existing music files or retain database probe
   changes. Repair failure never authorizes reset, replacement, unsafe path use,
   or a claim of rollback. Explicit restore/repair preserves the original,
   validates a candidate, and never replaces a live database beneath connections.
   ADR 0064's binding-preservation rules stay in force.
7. Reports identify the subject, failure, consequence, recovery action, and
   actual observation time. They omit tokens and credential-bearing excerpts.
8. Window-system failure has a stderr and unsuccessful-exit path that needs
   neither a window nor the ADR 0040 runtime.
9. A repairable missing dependency has an enabled path to correction and retry.
   Retry uses a fresh observation and revalidates its original subject; saving
   a setting alone cannot replay a user operation.

## Verification Required For Implementation

The [review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md)
maps every invariant and transferred verification requirement to a named packet.
Each [packet](../plans/adr-0066-startup-recovery-phase-plan.md#sequence-and-stopping-points)
owns concrete tests, guard names, failure fixtures and any operator visual check.

Mechanical proof covers core probes, configuration preservation, scoped
dependencies, safe reports, resource lifecycle, explicit correction/retry,
consistent SQLite snapshots and migration-authority reuse. Human inspection
covers readable recovery, correction without relaunch, optional-tool remediation
and the database tools in Settings and core recovery.

When a packet's implementation is ready, record its runnable gate in that
packet, the delivery row and [pending human checks](../pending-human-checks.md).
Task 001's backend verification is recorded in its
[implementation evidence](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md#implementation-and-proof).
`adr_0066_config_creation_and_save_ownership` in
[architecture_tests.rs](../../tests/architecture_tests.rs) guards first-run and
save ownership under invariants 3–4; the packet links the filesystem and snapshot
tests in [config.rs](../../src/config.rs). Task 001 opens no visual gate.

Task 002's [implementation evidence](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md#implementation-and-proof)
links the storage, SQLite, worker, generation and report tests.
`adr_0066_core_recovery_ownership` and `adr_0066_recorded_report_context` in
[architecture_tests.rs](../../tests/architecture_tests.rs) guard the recovery
boundary and report ownership. Its [operator check](../runbooks/startup-recovery-check.md#task-002-core-checks-and-reports)
passed on 2026-09-10; the packet records the operator evidence. Later packets
open their own runnable checks when implemented.

Task 003 adds `adr_0066_missing_runtime_has_no_implicit_runner` and
`adr_0066_runtime_retry_keeps_one_host_and_independent_reports` in the same
architecture suite. Its [proof inventory](../tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md#mechanical-evidence)
links runner rejection, independent local queries, runtime retry, issue isolation
and cache-failure tests. Its [operator check](../runbooks/startup-recovery-check.md#task-003-background-tools)
remains open.

Task 003's operator-reported search-error clipping correction is guarded by
`adr_0066_search_failure_report_stays_readable_and_vm_owned`. The packet's
[correction evidence](../tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md#operator-correction-readable-search-failures)
links typed failure wording, shared wrapping/disclosure, recorded UTC and
safe report-copy checks. The focused operator recheck passed, including
Library-filter separation and playlist uniqueness. Task 003's remaining gate
stays open.

## Non-Goals

This decision does not rename configuration sections, move music, introduce an
empty replacement database, promise arbitrary corruption salvage, change the
download-format policy, or implement ADR 0064's full repair-history surface.
Log navigation, local-time preferences and the cross-repository timestamp
contract remain separate work.

## Alternatives Considered

- **Start with defaults after an error.** Rejected. A later settings or layout
  save can replace the file the operator is trying to repair, and a new default
  database can make the library appear lost.
- **Only replace `load_config.expect` with a stderr error.** Rejected. Other
  startup failures still panic, and a desktop launch can hide stderr.
- **Require every configured tool to initialize.** Rejected after operator
  review. It makes optional tooling a requirement for music curation. Express
  unavailable capabilities and preserve independent work instead.
- **Treat every semantic configuration error as a broken document.** Rejected.
  Invalid TOML prevents reliable core parsing; a bad field in a parsed optional
  block has a known, narrower consequence.
- **Open the normal app without usable music storage or SQLite.** Rejected.
  Both are operator-confirmed minimum requirements. An empty replacement
  database or another music location is not a recovery action.
- **Only disable affected controls or instruct the operator to relaunch.**
  Rejected after operator review. Missing optional tooling should lead to
  configuration, verification, and retry from the original workflow. Core
  recovery also needs access to repair tools.
- **Blindly rerun bootstrap after each correction.** Rejected. Check again,
  optional reinitialization, database maintenance, and Open app are separate
  operations with explicit resource ownership.
- **Automatically reset or restore a database after failure.** Rejected.
  Restoration changes operator data. It requires a validated candidate,
  preservation of the original, and an explicit operator action. The app never
  assumes that all earlier startup effects were rolled back.

## Consequences

An operator can diagnose a failed desktop launch and keep the file they need
to repair. The same failure categories are mechanically testable without GPUI.
First-run creation remains available, while save paths lose their unsafe
default-creation fallback.

Startup still refuses the normal app when a required resource is unavailable,
but optional failures leave independent work available and explain the affected
tools. This requires separating config decoding, resource construction, and
action dependencies; changing a few `expect` calls is insufficient. Constructors
that assume a runtime or player exists must change along with their callers.

Core recovery and optional-tool failures become repair workflows inside the
app. That adds maintenance commands, guarded configuration editing, operation
retention, and explicit resumption; these need bounded implementation packets.
A failed window system can only receive stderr reporting. Database work
completed before a later failure remains completed until an explicit restore
or repair changes it. Storage verification does not guarantee the next
operation will succeed, and unsupported corruption may still need external help.

## Follow-Up Work

Execute the thirteen [bounded packets](../plans/adr-0066-startup-recovery-phase-plan.md)
one per session. They separate configuration safety, core recovery, optional
capability/remediation wiring, session draining, converter setup/retry and
database maintenance. Deferred item 7 stays blocked until this decision is
implemented and verified. The delivery plan retains relay durability through
adoption as the next block.

### Mechanism Retirement Owner

[Task 001](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md#mechanism-handoff)
owns the documentation handoff. Stage procedures and detailed verification
have moved into their packets and the review checklist. Task 001's
[handoff review](../reviews/adr-0066-startup-recovery-review-checklist.md#task-001-review--2026-09-10)
verified coverage and each successor's explicit prose-retirement criterion.

Each packet removes duplicated mechanism prose when its guards land, recording
the actual guard symbol and verification artifact. Binding decisions and
invariants stay here. Task 001 closes its own work without waiting for later
packets; no unnamed final cleanup owns this obligation.

## References

- [Deferred work, items 6 and 7](../plans/deferred-architecture-work-index.md#priority-order).
- [ADR 0010: MusicIndex endpoint](0010-musicindex-endpoint-setting.md).
- [ADR 0016: Schema migrations](0016-schema-migration-discipline.md).
- [ADR 0021: Playback driver](0021-mpv-playback-driver.md).
- [ADR 0040: Runtime ownership](0040-async-vm-runtime.md).
- [ADR 0060: App sections](0060-workflow-surface-structure.md).
- [ADR 0061: Governance and shared UI ownership](0061-executable-governance.md).
- [ADR 0064: Local path repair](0064-local-file-addressing.md).
