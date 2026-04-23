# ADR 0012: Root Desktop Crate

## Status

Accepted

## Context

The project has moved from a CLI-first tool with an auxiliary GPUI search binary to a desktop application. Keeping the Rust crate under `rust/` and preserving CLI-only entry points now makes the repository layout and binary names misleading.

## Decision

Move the Rust crate manifest, lockfile, source tree, and tests to the repository root. Remove CLI-only commands and dump utilities. Keep the RSS subscription implementation because the desktop UI uses it for feed and track subscription workflows.

The remaining executable is named `v4vmm` and launches the GPUI desktop application.

## Consequences

- Cargo commands run from the repository root.
- `v4vmm` is the desktop application binary.
- Former commands such as `show-config`, `id3-dump`, `subscribe`, and `rss-dump` are no longer supported as CLI entry points.
