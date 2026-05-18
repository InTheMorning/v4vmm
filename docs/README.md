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
- [ADR 0031: Release detail presentation contract](adr/0031-release-detail-presentation-contract.md): implemented contract for composing Library and Discovery release detail pages
- [ADR 0032: UI backend boundary and popover contracts](adr/0032-ui-backend-boundary-and-popover-contracts.md): boundary rules for projection, screen behavior, shared UI chrome, and popovers
- [ADR 0046: Workspace frame architecture](adr/0046-workspace-frame-architecture.md): frame ownership, history, chrome, and workspace layout model
- [ADR 0047: Library and search unification](adr/0047-library-search-unification.md): shared content surface and inspector across Library and Search origins
- [ADR 0048: ContentList frame breadcrumb search](adr/0048-content-list-frame-breadcrumb-search.md): toolbar search result surface in ContentList with breadcrumb navigation
- [ADR 0049: Inspector source ownership](adr/0049-inspector-source-ownership.md): source-tree, inspector filter, remote drill-down, and same-view mutation ownership
- [ADR 0052: Library / Index data parity triage](adr/0052-library-index-data-parity-triage.md): documentation-only triage for Library versus live Index detail fields
- [ADR 0053: Local detail source-fact parity](adr/0053-local-detail-source-fact-parity.md): proposed source-fact route for parity gaps that are not locally durable yet
- [UI backend boundary](architecture/ui-backend-boundary.md): practical guide for keeping services, projections, screens, and UI composites in their lanes
- [Library / Index data parity follow-up plan](plans/adr-0024-library-index-data-parity-follow-up-plan.md): routed loading-shape slices from the ADR 0052 triage
- [Inspector source ownership plan](plans/inspector-source-ownership-phase-plan.md): active follow-up plan for ContentList inspector ownership regressions
- [Active-frame search dispatch plan](plans/active-frame-search-dispatch-plan.md): superseded focused-frame toolbar search routing plan
- [Pre-UI and download manager preparation plan](plans/pre-ui-download-prep.md): service, schema, and CLI work to land before a UI/download revamp

Historical architecture decisions still live in [`docs/adr/`](adr/).
Older roadmap notes live in [`docs/archive/roadmap-notes/`](archive/roadmap-notes/).
