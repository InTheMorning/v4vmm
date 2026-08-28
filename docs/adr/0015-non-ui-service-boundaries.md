# ADR 0015: Non-UI Service Boundaries

## Status

Accepted - 2026-04-26.

## Context

The desktop UI currently owns too much workflow behavior. Discover and Library
event handlers call database helpers, network helpers, download paths,
metadata staging, and playlist operations directly.

This makes a UI redesign risky because behavior can diverge between old UI,
new UI, CLI tools, tests, and future adapters. It also makes the planned
download manager harder to build because local track identity, local file
state, and subscription behavior are not exposed through a stable non-UI API.

## Decision

New workflow behavior must be implemented in non-UI service modules before it
is wired into GPUI components.

The initial service boundaries are:

- track identity / local track inspection
- playlist operations
- library membership and local file state
- subscription workflows
- playback session state

UI modules may compose and render service results, but they should not own the
business rules for identity, persistence, provenance, or state transitions.

## Consequences

- CLI commands and UI actions can share behavior.
- Tests can target service functions without constructing GPUI state.
- Future UI redesign work can change presentation without rewriting core
  workflows.
- Refactors should move behavior one workflow at a time instead of introducing
  a broad application service layer in one pass.
