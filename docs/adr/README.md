# Current Decisions

Every ADR here still constrains a judgment call. This is the answer to what
binds a change today.

Superseded and fully guarded decisions live in `archive/`. Read those for
research, never to find a live rule. ADR 0061 owns this split.

Status values are the ADR 0057 vocabulary: `Proposed`, `Accepted`,
`Implemented`, `Superseded by ADR NNNN`.

## Governance

| ADR | Scope | Status |
|---|---|---|
| [0001](0001-record-architecture-decisions.md) | ADRs record decisions. Sequential numbering, Nygard structure | Accepted |
| [0057](0057-adr-status-vocabulary-and-amendment-policy.md) | Status vocabulary and in-place amendment rules | Accepted |
| [0061](0061-executable-governance.md) | Reading path, durable and situational guards, archive rule | Accepted |

## Foundation And Storage

| ADR | Scope | Status |
|---|---|---|
| [0002](0002-rust-cli-with-local-sqlite-state.md) | Rust CLI over local SQLite | Accepted |
| [0012](0012-root-desktop-crate.md) | One root desktop crate | Accepted |
| [0016](0016-schema-migration-discipline.md) | New tables and columns go through the migration registry | Accepted |
| [0028](0028-local-identity-source-fact-persistence.md) | Source links, ids, and contributors persist as source facts | Implemented |
| [0029](0029-artist-person-identity-persistence.md) | Artist subject facts persist. Person identity deferred | Implemented |
| [0045](0045-track-artist-binding.md) | Track to artist binding for library artist views | Implemented |
| [0052](0052-library-index-data-parity-triage.md) | Triage of Library versus Index detail gaps | Implemented |
| [0053](0053-local-detail-source-fact-parity.md) | Source-fact route for parity gaps not locally durable | Accepted |
| [0054](0054-local-metadata-source-fact-persistence.md) | Feed and track metadata facts persist by source | Implemented |

## Metadata

| ADR | Scope | Status |
|---|---|---|
| [0004](0004-format-neutral-audio-tag-boundary.md) | Format-neutral audio tag boundary | Accepted |
| [0005](0005-musicbrainz-metadata-lookup.md) | Metadata-based MusicBrainz lookup. No fingerprinting | Accepted |
| [0006](0006-musicbrainz-release-detail-enrichment.md) | MusicBrainz release detail enrichment | Accepted |
| [0007](0007-metadata-compare-table-drag-copy.md) | Compare table drag copy | Accepted |
| [0008](0008-explicit-id3v24-write-boundary.md) | Only explicit ID3v2.4 frames are written | Accepted |
| [0011](0011-musicindex-guid-id3-tags.md) | MusicIndex feed and track GUIDs in TXXX frames | Accepted |

## Services And Boundaries

| ADR | Scope | Status |
|---|---|---|
| [0010](0010-musicindex-endpoint-setting.md) | Configurable MusicIndex endpoint | Accepted |
| [0015](0015-non-ui-service-boundaries.md) | Services stay free of UI concerns | Accepted |
| [0017](0017-cli-debug-contracts.md) | CLI debug contract and JSON output | Accepted |
| [0022](0022-ui-agnostic-core-extraction.md) | Core is free of renderer types | Implemented |
| [0024](0024-command-query-event-application-layer.md) | Command, query, and event application layer | Implemented |
| [0040](0040-async-vm-runtime.md) | Async view-model runtime. Actors in `src/runtime/` | Implemented |
| [0041](0041-windowed-paged-view-models.md) | Windowed paged view models | Implemented |
| [0056](0056-remote-media-fetch-validation-boundary.md) | Remote media fetch validation and redirect policy | Implemented |
| [0058](0058-outbound-http-client-policy.md) | All blocking HTTP clients built in `src/http_client.rs` | Implemented |

## Playback And Broadcast

| ADR | Scope | Status |
|---|---|---|
| [0014](0014-playback-session-authoritative-state.md) | `PlaybackSession` is the authoritative now-playing state | Accepted |
| [0020](0020-simulated-playlist-playback.md) | Simulated playlist transport for relay smoke tests | Accepted |
| [0021](0021-mpv-playback-driver.md) | mpv driver behind the `PlaybackDriver` trait | Implemented |
| [0059](0059-broadcast-control-surface.md) | This app controls an external publisher and sends no payloads | Accepted |

## UI Architecture

| ADR | Scope | Status |
|---|---|---|
| [0023](0023-design-system-and-view-models.md) | Design system and view-model architecture | Implemented |
| [0025](0025-theme-icon-style-boundary.md) | Theme, icon, and style boundary | Accepted, partial |
| [0026](0026-shared-entity-projection-layer.md) | Shared entity projection layer | Implemented |
| [0027](0027-shared-entity-action-state.md) | Typed action state for shared entities | Implemented |
| [0032](0032-ui-backend-boundary-and-popover-contracts.md) | UI and backend boundary, popover contracts | Implemented |
| [0033](0033-hig-ui-architecture-governance.md) | HIG structural governance for UI work | Implemented |
| [0034](0034-scale-aware-ui-tokens-and-controls.md) | Scale-aware tokens and controls | Implemented |
| [0038](0038-presentation-contract-enforcement.md) | Presentation contracts enforced by guards | Implemented |
| [0042](0042-layer-consolidation.md) | Primitive versus composite versus shell | Implemented |
| [0046](0046-workspace-frame-architecture.md) | Workspace frames, history, and chrome | Implemented |
| [0047](0047-library-search-unification.md) | One content surface for library and index rows | Implemented |
| [0048](0048-content-list-frame-breadcrumb-search.md) | Search is a toolbar command. Amended by ADR 0060 | Implemented |
| [0050](0050-post-adr-0048-module-decomposition.md) | Module decomposition after ADR 0048 | Implemented |
| [0055](0055-search-view-model-module-decomposition.md) | Search view-model module decomposition | Accepted |
| [0060](0060-workflow-surface-structure.md) | Music, Show, Settings. Show is a screen mount | Proposed |

## UI Presentation

| ADR | Scope | Status |
|---|---|---|
| [0003](0003-musicindex-search-ui-module.md) | MusicIndex search UI module | Accepted |
| [0009](0009-search-thumbnail-cache-and-batch-tagging.md) | Thumbnail cache and feed batch tagging | Accepted |
| [0013](0013-shared-discover-track-row.md) | Shared track row module | Accepted |
| [0030](0030-discovery-library-ui-fixes.md) | Discovery and library correctness fixes | Accepted |
| [0031](0031-release-detail-presentation-contract.md) | Release detail composition | Implemented |
| [0035](0035-track-surface-consolidation.md) | One track detail surface | Implemented |
| [0036](0036-feed-visual-and-provenance-surface-consistency.md) | Feed visual and provenance consistency | Implemented |
| [0037](0037-same-entity-surface-parity.md) | Same-entity surface parity | Accepted, partial |
| [0039](0039-dynamic-type-ramp.md) | Dynamic type ramp | Proposed |
| [0043](0043-top-toolbar-global-search.md) | Toolbar now-playing frame and global search | Accepted, partial |
| [0044](0044-playlist-drag-handle-reordering.md) | Playlist drag handle reordering | Accepted, partial |
| [0049](0049-inspector-source-ownership.md) | Inspector source tree and filter ownership | Implemented |
| [0051](0051-workspace-pane-width-persistence.md) | Content pane width persistence | Implemented |

## Review Candidates

ADR 0060 replaces the `Discover` surface with `Music`. These ADRs describe
surfaces that decision affects, and each needs a supersede-or-keep judgment
before the restructure lands:

- 0003, MusicIndex search UI module
- 0013, Shared track row module
- 0030, Discovery and library UI correctness fixes
- 0043, Toolbar now-playing frame and global search

Two more need a status decision rather than a scope one:

- 0039 sits at `Proposed` with no phase plan or task packets.
- 0025, 0037, 0043, and 0044 record partial implementation. Each needs its open
  gate closed or routed to the deferred work index.

## Known Drift

Seven ADRs place the status on the line directly below `## Status` while 53 use
a blank line first. ADR 0061 requires one canonical format. Normalizing them is
follow-up work, to land with the corpus guard.
