# Documentation

This repo documents the current desktop app and keeps planning material in
purpose-built folders.

Start here:

- [App overview](architecture/app-overview.md): tabs, dependencies, config, and the main operator model
- [Architecture diagrams](architecture/architecture-diagrams.md): current and ideal module/data-flow shape
- [Library and discovery workflows](runbooks/workflows.md): what subscribing, caching, compare, feed refresh, and now-playing CLI flows actually do
- [Storage and metadata model](schema/storage-and-metadata.md): SQLite tables, file layout, metadata sources, and current limits
- [ADR 0023 migration plan](plans/adr-0023-design-system-migration.md): remaining design-system and view-model work
- [Discovery and Library UI fixes plan](plans/discovery-library-ui-fixes.md): reviewed follow-up plan for search, recents, shared headers, contributor display, compare actions, and scrolling
- [ADR 0031: Release detail presentation contract](adr/0031-release-detail-presentation-contract.md): proposed contract for composing Library and Discovery release detail pages
- [Pre-UI and download manager preparation plan](plans/pre-ui-download-prep.md): service, schema, and CLI work to land before a UI/download revamp

Historical architecture decisions still live in [`docs/adr/`](adr/).
Older roadmap notes live in [`docs/archive/roadmap-notes/`](archive/roadmap-notes/).
