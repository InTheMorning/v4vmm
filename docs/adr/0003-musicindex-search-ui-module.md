# ADR 0003: MusicIndex Search UI Module

## Status
Accepted - 2026-04-11.

## Context
The Stophammer search page in `../stophammer/dist/search.html` defines the current operator workflow for searching feeds, tracks, and publishers. The Rust project already had a `musicindex` client module and a local GPUI search binary, but the UI behavior and data model had drifted from the Stophammer page.

## Decision
We will make `rust/src/musicindex.rs` own the MusicIndex search client models, Stophammer-compatible endpoint helpers, and the GPUI search app. The `search` binary remains a thin launcher for that module.

## Consequences
- The Rust search UI has one implementation path for search, result inspection, nested feed/track/publisher navigation, and lazy contributor/value-route loading.
- The MusicIndex client now models the Stophammer feed, track, publisher, contributor, and value-route payloads directly.
- Future UI refinements can happen in `musicindex.rs` without duplicating API behavior in the binary entry point.
