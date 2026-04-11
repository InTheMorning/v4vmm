# ADR 0002: Rust CLI With Local SQLite State

## Status
Accepted

## Context
`v4vmm` is currently an early-stage tool focused on ingesting feed metadata, inspecting local MP3 tags, and tracking which feed items exist in a local library. The codebase is small, Linux-first, and currently has no GUI runtime or background service boundary.

## Decision
We will keep the core as a Rust CLI backed by a local SQLite database. The CLI remains the primary integration surface while the local database stores feed metadata, track metadata, and library-specific state such as local file bindings and `is_in_library` flags.

## Consequences
- The project stays easy to run and inspect during early development.
- State remains queryable and portable without introducing a separate daemon or remote dependency.
- Future UI layers can build on top of the same local store instead of re-implementing ingestion and library bookkeeping.
