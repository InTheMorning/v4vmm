# v4vmm Agent Guidelines

`v4vmm` is a Linux-first GPUI desktop app for Value4Value music curation and
show production. A curator finds music, takes it into a local library, keeps
that library ready, and uses it to run or record a show.

This file describes the present. It carries no history and no superseded rule.
`.github/copilot-instructions.md` holds the source map. Read it to find a file.

Governance model: ADR 0061.

## Where The Work Stands

Music and Show are built. ADR 0059 tasks 001-017 are complete. ADRs 0059 and
0063 are Implemented, including operator acceptance and fixture cleanup.

[Show action feedback task 001](docs/tasks/show-action-feedback-task-001-command-state-and-result.md)
is complete, including operator visual acceptance.

Inherited operator checks remain open for scrolling, identity/detail parity,
toolbar search, playlist reordering, and metadata hydration. Their current
requirements are indexed in [pending human checks](docs/pending-human-checks.md).
Task 017's acceptance remains closed.

ADR 0066 tasks 001–003 are complete, including operator acceptance,
preservation inspection and fixture cleanup. ADR 0067 is Implemented;
its platform-shortcut packet is complete with all operator checks accepted.
Settings responsiveness and cached-file recovery are accepted, and the
temporary timing/profiling tools are removed.

[ADR 0069 task 001: Grouped Settings foundation](docs/tasks/adr-0069-task-001-grouped-settings-foundation.md)
is complete with mechanical checks Green, operator V1–V3 and preservation
acceptance, and confirmed fixture cleanup. Its
[operator procedure](docs/runbooks/settings-foundation-check.md) remains a regression check.
The packet groups existing controls and reports without changing the configuration format.
[ADR 0069](docs/adr/0069-grouped-settings-and-selective-presets.md) remains Accepted;
later implementation has not started. The
[Settings phase plan](docs/plans/adr-0069-settings-presets-phase-plan.md)
keeps guarded editing, live metadata selection and presets behind their named
recovery prerequisites; audio integration remains deferred.

[ADR 0066 task 004: Optional tool isolation](docs/tasks/adr-0066-task-004-optional-tool-isolation.md)
still has an open acceptance gate.
Implementation and mechanical checks are complete; presentation acceptance and
producer preservation passed. Remaining operator checks and fixture cleanup
are open.
[Task 005: Session drain and resumption](docs/tasks/adr-0066-task-005-session-drain-and-resumption.md)
is complete with mechanical checks Green, operator V1–V3 and preservation
accepted, and fixture cleanup confirmed.
[Task 006: Configuration repair and resumption](docs/tasks/adr-0066-task-006-configuration-repair-and-resumption.md)
is complete with mechanical checks Green, operator V1–V6 and preservation
accepted, and fixture cleanup confirmed on 2026-09-13. Its
[procedure](docs/runbooks/startup-recovery-check.md#task-006-configuration-repair-and-resumption)
remains a regression check.
[Task 007: Optional tool correction and retry](docs/tasks/adr-0066-task-007-optional-tool-correction-and-retry.md)
is complete on 2026-09-16 with mechanical checks Green, operator V1–V3 and
preservation accepted. The narrow Library and
[ADR 0073 Show card overflow](docs/adr/0073-show-card-overflow-scrolling.md)
follow-ups are accepted, including preservation and cleanup. V2/V3 cleanup is
confirmed; no earlier startup fixtures remain in the checked temporary
directories. ADR 0073 is Implemented. The
[operator procedure](docs/runbooks/startup-recovery-check.md#task-007-optional-tool-correction-and-retry)
remains a regression check with isolated Index/service stubs and Null playback.
Task 004 retains its remaining checks under the recorded scheduling exception.
[Task 008: Converter verification and setup](docs/tasks/adr-0066-task-008-converter-verification-and-setup.md)
is complete on 2026-09-17 with mechanical checks Green; operator V1–V3,
Settings/core-recovery presentation and preservation in both fixture cases are
accepted. Normal-mode restoration and fixture cleanup are confirmed. The
[operator procedure](docs/runbooks/startup-recovery-check.md#task-008-converter-verification-and-setup)
remains a regression check.
[Task 009: Conversion retry and retained input](docs/tasks/adr-0066-task-009-conversion-retry-and-retained-input.md)
is complete on 2026-09-17 with mechanical checks Green. Operator V1–V3,
normal/narrow presentation, configuration restoration and preservation are
accepted; fixture cleanup is confirmed. Its
[procedure](docs/runbooks/startup-recovery-check.md#task-009-conversion-retry-and-retained-input)
remains a regression check.
[Task 010: Database check and backup](docs/tasks/adr-0066-task-010-database-check-and-backup.md)
is complete on 2026-09-17 with mechanical checks Green. Operator V1–V3,
Settings/recovery presentation, report copy, responsiveness and preservation in
both fixture cases are accepted; all 28 final database preservation flags are
Green. Normal-mode restoration and fixture cleanup are confirmed. The fixture
lock correction has two real-process regression tests. Its
[procedure](docs/runbooks/startup-recovery-check.md#task-010-database-check-and-backup)
remains a regression check.
[Task 011: Database maintenance and preservation](docs/tasks/adr-0066-task-011-database-maintenance-and-preservation.md)
is complete on 2026-09-17 with mechanical checks Green. Operator V1–V3,
Settings/recovery normal/narrow presentation, report retention, preservation in
both cases, normal-mode restoration and fixture cleanup are accepted. Its
[operator procedure](docs/runbooks/startup-recovery-check.md#task-011-database-maintenance-and-preservation)
remains a regression check. Task 012 is complete in the
[phase plan](docs/plans/adr-0066-startup-recovery-phase-plan.md).

[Task 012: Database restore](docs/tasks/adr-0066-task-012-database-restore.md)
is complete on 2026-09-17 with mechanical checks Green. Operator V1–V3,
Settings/recovery normal/narrow presentation, report copy, input-change
protection, same-window resumption and both preservation inspections are
accepted; cleanup of all three operator fixtures is confirmed. Its
[procedure](docs/runbooks/startup-recovery-check.md#task-012-database-restore)
remains a regression check. Task 013 is complete, including operator acceptance, preservation and cleanup.

[Task 013: Interrupted upgrade repair](docs/tasks/adr-0066-task-013-interrupted-upgrade-repair.md)
is complete on 2026-09-18 with mechanical checks Green.
Operator V1–V3, Settings/recovery presentation and both final preservation
inspections are accepted. The
[ADR 0074 page correction](docs/adr/0074-repair-and-diagnostics-pages.md) is
Implemented. Its visual gate is accepted, including normal/narrow and short
heights, Settings XL scale in both themes, report copy, scrolling, retained
inputs and preference restoration. Cleanup of both fixtures is confirmed. The
[focused check](docs/runbooks/startup-recovery-check.md#repair-and-diagnostics-pages--adr-0074)
remains a regression procedure.
Use the asd-ste100 skill for repair and diagnostics text. ADR 0066 stays Accepted.
Task 004 retains its independent gate. Configuration-format work stays gated.

[ADR 0063 task 005](docs/tasks/adr-0063-task-005-shared-log-frames-and-following.md)
is complete on 2026-09-13 with mechanical checks Green, V1–V3 operator checks,
configuration-editor disclosure and Escape follow-ups, final preservation and
fixture cleanup accepted. ADRs 0063 and 0070 are Implemented. Show, Diagnostics
and recovery share the frame and reading-state owner. The
[operator procedure](docs/runbooks/log-frame-check.md) remains a regression
check. ADR 0066 task 007 completion is recorded above.

[ADR 0071 task 001: Shared text selection](docs/tasks/adr-0071-task-001-shared-text-selection.md)
is complete on 2026-09-15, with mechanical checks Green, available X11 operator
checks, preservation and cleanup accepted. ADRs 0071 and 0072 are Implemented;
the narrow gpui-base correction is commit-pinned. IME composition and Wayland
remain untested coverage limits. The packet owns double/triple-click selection
and Linux primary paste across shared logs and inputs. The completed log packet
stays closed. Its separate [operator procedure](docs/runbooks/text-selection-check.md)
covers Unicode, paths, cross-application paste, Undo/Redo and accepted Escape.
ADR 0066 tasks 007–012 are complete, including operator acceptance, preservation
and cleanup. Task 013 is complete, including operator acceptance, preservation and cleanup.

[ADR 0068: Show cue and audition isolation](docs/adr/0068-show-cue-and-audition-isolation.md)
is Proposed; its implementation has not started. Task 004's playback checks
remain paused pending that separation and resolution of the observed mpv IPC error.
Playback work is deferred; independent report/path-repair and Music checks
remain available in the pending-human index.
Relay durability through reserved-event adoption follows.
The delivery order also schedules remaining narrow Show layout,
external UTC timestamp corrections, and steady-state work.
When a real show is scheduled, publisher show-log task 001 takes priority.

[ADR 0039: Dynamic type ramp](docs/adr/0039-dynamic-type-ramp.md) is Accepted
on 2026-09-18. Its [two-packet plan](docs/plans/adr-0039-dynamic-type-ramp-phase-plan.md)
is scheduled after completed recovery task 013, before Settings follow-through
and relay adoption. Implementation has not started. The type coefficient table
remains a proposal; chrome retains today's exact five-step values. The twelve
row/detail/popover inspections remain open under task 002. Existing `UiScale`
variants and configuration format stay unchanged; ADR 0066's gate is independent.

`docs/plans/broadcast-chain-delivery-order.md` is the only cross-repository
order. Read it before starting a session on broadcast work.

## Current Design Philosophy

**Three app sections.** `Music`, `Show`, `Settings`. `Music` is curation and
holds index results and the library together. `Show` is broadcasting. Search is
a toolbar command, never a section. ADR 0060.

**Curation shows nothing operational.** With no show active, `Music` and
`Settings` show no queue, no transport, and no broadcast status. A surface that
cannot act is absent, not disabled.

**Provenance first.** RSS, MusicIndex, embedded tags, and MusicBrainz stay
separate. Never collapse them into one inferred truth. Placeholder-looking source text is a source-boundary problem, never a renderer problem.

**The broadcast chain runs when this app is closed.** This app is a control
surface and a status display, never a required part of the chain. The built-in
player is the one exception.

**Real time is canonical.** This app, the encoder, and the publisher all
present real time. Delivery delay serves the live path only and never reaches a
recording or an interface.

**Current-view state must update in place.** A mutation refreshes the mounted view.
Never require navigating away and back to reveal a result.

## The Durable Set

These are permanent. They describe how this project is built, not what any
screen looks like. Adding to this set requires an ADR. ADR 0061.

**Renderer portability.** View models carry no renderer type. Presentation
facts, default labels, availability, and command intent live in
`src/view_models` or `src/views.rs`. Reusable chrome, row layout, and
interaction geometry live in `src/ui/primitives` or `src/ui/composites`.
Screens stay thin: they compose views, resolve images, wire callbacks, manage
focus, and dispatch commands.

The reason is portability. These rules are what let the interface move to or
from GPUI without rewriting product logic. A violation welds product logic to
one renderer.

**Element hierarchy.** Every element has a defined place. Title, subtitle,
metadata, state, and actions have predictable placement, weight, and
visibility. Nothing is positioned ad hoc.

**Apple HIG structure.** Predictable hierarchy and disclosure, clear state,
comfortable density, accessibility, and cautious destructive actions. HIG is
structural guidance, not visual imitation.

**Token discipline.** Size, spacing, color, typography, icons, and roles come
from named tokens. No raw literal and no glyph string in a renderer. Never rely
on color alone.

**Typed action state.** Every action carries typed availability and an
accessibility label before it renders. No renderer decides whether a control is
enabled.

**UI change acceptance gate.** A user-visible UI change is accepted only when
the shared owner, view model, token path, and verification path are all clear.
ADR 0033 and ADR 0061.

**No isolated visual tweaks.** Fix repeated visual affordances at their shared
primitive, composite, shell, or view-model owner. A renderer patch is only for
screen-local composition.

**Button and action discipline.** Buttons render from typed action state,
named icons, accessibility labels, and named control styles. A renderer does
not invent action availability.

**Workflow-first requirement.** UI structure follows the curator workflow
surface first, then the component hierarchy. Component availability does not
earn screen space by itself.

**HIG product polish is separate from restructuring.** HIG product-completeness
gaps live in the product polish backlog. Do not mix them into structural
rework unless the active ADR says to do so.

## Working Rules

**ADR first.** Do not implement an architectural change before an ADR records
it. Use the status vocabulary of ADR 0057.

**One phase for each session.** Complete a phase, verify it, then start a fresh
session. Do not chain phases.

**Zero warnings.** Every commit passes `cargo check`, `cargo fmt --check`, and
`cargo clippy -- -D warnings`.

**Silence success.** Report an error. When a check passes, say "Green" and
continue.

**Write reports for the operator.** When writing reports, log messages, or
status text, name the subject, action, and object. Explain what happened and
what the app did with the result. Name the event, endpoint, service, or file
being checked instead of an internal phase such as "liveness". Timestamp
reported actions and observations using their actual recorded times. Do not
invent times during rendering or turn an unanswered question into a fact.
Omit idle actions and repeated filler. Put technical details after the useful
explanation. Short badges may stay concise, but their surrounding report must
make the subject and consequence clear. ADRs 0059/0063 own Event report data.

**Every fix gets a guard.** A user-confirmed fix is incomplete without a test,
an architecture guard, or a documented manual check that blocks the same class
of regression.

**Reconcile human gates by meaning.** Read ADR status sections and review gate
statements, including prose. Checkbox syntax is not a gate inventory. Before
listing an inherited check, retire replaced requirements with the superseding
ADR named, and preserve each surviving requirement in the pending-check index.
Scope a claim of no open checks to the packet or ADR actually reviewed.

**A guard names its class and its ADR.** A durable guard enforces the set
above. A situational guard cites one ADR and is deleted when that ADR is
superseded. When the class is unclear, choose situational.

**Prose that a guard replaces is deleted** in the same change.

**Delete dead code.** Code no composition root reaches is removed, not parked.
Copy any pattern worth keeping into the live surface first.

**Visual proof.** A user-visible layout, hierarchy, or presentation fix is not
fixed until a person inspects it in the running app.

**Never run the app.** No `cargo run`, no `xvfb-run`,
and no other headless display attempt. GPUI cannot initialize an X11 client in
an agent session. That failure is known, it is not a defect, and it carries no
information. Do not attempt it, do not report it as an unresolved concern, and
do not describe it as blocking.
It blocks nothing that any agent could do.

**Write operator instructions instead.** End the report with an
`Operator visual check` section that a person can follow at a terminal in a
desktop session:

- numbered steps, including the commands that reach the state
- what to look at, and what would count as wrong
- any hardware or system state the check needs, named plainly, such as a failed
  unit, an unreachable host, or an installed binary
- the cleanup that undoes any state the steps created

Leave the gate open in the packet `Status:` line, in
`docs/plans/broadcast-chain-delivery-order.md`, and in
`docs/pending-human-checks.md`, which holds every check that is open today. A
gate that no person has walked is never met, and a green mechanical suite is not
evidence about it.

**An acceptance criterion states how it is checked.** It is mechanical, proved
by a test and phrased at the layer that owns it, or it is visual and listed
separately. `The section shows six states` is untestable. `The view model
exposes six states` is the same requirement where a test can reach it. A
criterion that is neither passes by omission.

**Column text does not call `truncate()`.** ADR 0063 decides this, and
`docs/troubleshooting/column-text-truncation.md` explains why. Read them before
you style stacked text.

## Build, Test, Lint

After running tests, run `cargo build --bin v4vmm` before handing a desktop
binary to the operator. `cargo test` can replace `target/debug/v4vmm` with a
binary linked to GPUI test-support and its synchronous drawing loop. The startup
fixture's `run` command rebuilds the normal binary before opening it.

```bash
cargo build                          # Debug build
cargo build --release                # Production build
cargo test                           # All tests
cargo test --test architecture_tests # The guards
cargo fmt -- --check                 # Check formatting
cargo clippy -- -D warnings          # Strict clippy
```

CI runs build, test, clippy, then format check.

## Conventions

**Naming.** PascalCase types and enums, snake_case functions, variables and
modules, SCREAMING_SNAKE_CASE constants.

**Imports.** Group std, external, then `crate::`. No wildcard outside tests.

**Errors.** `anyhow::Result` with `.context(...)` in services, the CLI, and I/O.
Typed enums in the application layer and the view models, because callers match
on variants. A view model returns a display state, never a transport error.

**Async.** Background work uses the ADR 0040 runtime. Actors live in
`src/runtime/`, and `src/runtime/playback_polling.rs` is the reference. Never
call `cx.spawn` from a screen. Blocking database access uses
`Arc<Mutex<Connection>>`.

**Logging.** This crate has no `tracing` dependency. The CLI prints JSON on
stdout and errors on stderr. UI code reports failure through view-model display
state.

**Docs.** Module-level `//!` comments naming the owning ADR. Most modules carry
`#![warn(clippy::pedantic)]`. An opt-out uses `#![expect(...)]` with a reason.

**Serde.** `#[serde(default)]` for optional config so an older `config.toml`
keeps loading. `rename_all = "snake_case"` for enums.

**Tests.** Unit tests in `#[cfg(test)] mod tests` beside the code.
`tests/architecture_tests.rs` is the only integration file and holds the
guards. There is no shared test helper module.

**Secrets.** Broadcaster tokens are files with mode `0600`. Never in the
database, a log line, a command argument, or `Debug` output. Construct blocking
HTTP clients only in `src/http_client.rs`. Treat RSS and MusicIndex responses as
untrusted input.

## What Binds You

`docs/adr/README.md` lists every current ADR with its status and scope. It is
the answer to what binds a change today.

`docs/adr/archive/` holds superseded and fully guarded decisions. Read it for
research, never to find a live rule.

This file owns how an agent works. An ADR owns how the code is shaped. The two
do not overlap, and neither one holds the other's rules.

- **Agent behaviour lives here.** What to run, what never to run, what a report
  must contain, how to leave a gate open. These need no ADR, and this file is
  their owner.
- **Code shape lives in an ADR.** Layering, ownership, element hierarchy, a
  contract between two modules, how a surface is built. This file may point at
  such a rule and name the ADR that decides it. It never states one as its own.

Corrected 2026-09-09, after two code-shape rules were written here as though
this file decided them.
