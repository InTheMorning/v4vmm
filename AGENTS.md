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

Next work is review and acceptance of
[ADR 0066](docs/adr/0066-configuration-and-startup-failure-recovery.md), then its
core-startup and in-app repair packets, followed by relay durability through
reserved-event adoption. The delivery order also schedules narrow Show layout,
long-line inspection, per-log following, UTC timestamps, and steady-state work.
When a real show is scheduled, publisher show-log task 001 takes priority.

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
