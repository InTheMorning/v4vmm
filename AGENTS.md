# v4vmm Agent Guidelines

`v4vmm` is a Linux-first GPUI desktop app for Value4Value music curation and
show production. A curator finds music, takes it into a local library, keeps
that library ready, and uses it to run or record a show.

This file describes the present. It carries no history and no superseded rule.
`.github/copilot-instructions.md` holds the source map. Read it to find a file.

Governance model: ADR 0061.

## Where The Work Stands

The broadcast control surface of ADR 0059 is partly built. Backend packets 001
through 004 shipped: the live surface reduction, the event registry, and the
relay observation actor.

ADR 0060 then replaced the surface design. The shipped `Broadcast` frame, its
view model, and its shell are superseded before the remaining packets ran.

Current work, in order:

1. Rewrite the pending ADR 0059 packets against ADR 0060.
2. Build the `Music` and `Show` surfaces.
3. Resume the broadcast packets.

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
separate. Never collapse them into one inferred truth. Placeholder-looking
source text is a source-boundary problem, never a renderer problem.

**The broadcast chain runs when this app is closed.** This app is a control
surface and a status display, never a required part of the chain. The built-in
player is the one exception.

**Real time is canonical.** This app, the encoder, and the publisher all
present real time. Delivery delay serves the live path only and never reaches a
recording or an interface.

**Current-view state updates in place.** A mutation refreshes the mounted view.
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

## Working Rules

**ADR first.** Do not implement an architectural change before an ADR records
it. Use the status vocabulary of ADR 0057.

**One phase for each session.** Complete a phase, verify it, then start a fresh
session. Do not chain phases.

**Zero warnings.** Every commit passes `cargo check`, `cargo fmt --check`, and
`cargo clippy -- -D warnings`.

**Silence success.** Report an error. When a check passes, say "Green" and
continue.

**Every fix gets a guard.** A user-confirmed fix is incomplete without a test,
an architecture guard, or a documented manual check that blocks the same class
of regression.

**A guard names its class and its ADR.** A durable guard enforces the set
above. A situational guard cites one ADR and is deleted when that ADR is
superseded. When the class is unclear, choose situational.

**Prose that a guard replaces is deleted** in the same change.

**Delete dead code.** Code no composition root reaches is removed, not parked.
Copy any pattern worth keeping into the live surface first.

**Visual proof.** A user-visible layout, hierarchy, or presentation fix is not
fixed until it is inspected in the running app or a screenshot, or the missing
check is reported as residual risk.

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

A binding rule lives in an ADR. This file restates rules and names their
owners. It does not create them.
