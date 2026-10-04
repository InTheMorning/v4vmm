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
| [0012](0012-root-desktop-crate.md) | One root desktop crate | Accepted |
| [0028](0028-local-identity-source-fact-persistence.md) | Source links, ids, and contributors persist as source facts. ADR 0075 replaces its replacement key | Implemented |
| [0053](0053-local-detail-source-fact-parity.md) | Source-fact route for parity gaps not locally durable | Accepted |
| [0054](0054-local-metadata-source-fact-persistence.md) | Feed and track metadata facts persist by source; hydration visual checks remain open. ADR 0075 replaces its replacement key | Accepted, partial |
| [0066](0066-configuration-and-startup-failure-recovery.md) | Core startup requirements; in-app repair and optional-tool retry; [001–003 and 005–013 complete; 004 implemented, operator gate open](../plans/adr-0066-startup-recovery-phase-plan.md) | Accepted |

## Metadata

| ADR | Scope | Status |
|---|---|---|
| [0075](0075-metadata-ownership-and-completeness.md) | Shared metadata rules cover ownership, response completeness, storage by provider, each field with an accepted rule, and labeled identity sections. Decision I makes RSS the only provenance and MusicIndex a cache of RSS. The phase plan records packets and gates. ADR 0076 supersedes Decisions F to H and the source-selection rules | Accepted |
| [0076](0076-playlist-rss-check-for-stale-musicindex-records.md) | MusicIndex stays the normal checker. A playlist RSS check, also run when a show selects the playlist, applies and reports RSS differences. Tags and payment routes in files change only on operator confirmation. Supersedes ADR 0075 Decisions F to H and its source-selection rules. Amends ADRs 0059 and 0065. Amended 2026-10-03: a download writes the stored values | Accepted |
| [0077](0077-publisher-feed-artist-binding.md) | An artist page is keyed on the publisher feed GUID that an album names. Supersedes ADR 0045. Each proposal needs individual operator review. ADR 0078 supersedes its Decision 4 | Accepted |
| [0078](0078-publisher-page-type-from-stated-role.md) | A publisher page is a label page only when a feed states a label role. The artist count is derived information and never selects the page type. Supersedes ADR 0077 Decision 4 | Superseded by ADR 0082. Archived with ADR 0082 packet 002 |
| [0079](0079-remove-musicindex-artist-subject-storage.md) | MusicIndex artist subject storage is deleted. The publisher feed is the only artist identity. Person identity stays deferred. Supersedes ADR 0029 | Accepted |
| [0080](0080-tag-frames-follow-their-owner.md) | Tag frames follow their owner: one resolved Nostr key, the item page in `WOAF` and the channel website in `WOAR`, a separate album description frame, idempotent writes that keep MusicBrainz values, plain URLs in URL frames, and a compare that uses the resolution of the writer | Accepted |
| [0081](0081-remove-the-staged-frame-model.md) | Remove the staged frame model: no frame add, remove, detach or dock model with no caller, no reserved slots, and Forward completes ADR 0046 Invariant 2. Supersedes ADR 0046 Invariant 8 | Accepted |
| [0082](0082-publisher-roles-belong-to-each-album-link.md) | Publisher roles belong to each album link: no page type, albums grouped by `album_names_as` and by role agreement, and the feeds that share albums. Revised for Stophammer 0.7.0. Supersedes ADR 0078 | Accepted |
| [0083](0083-design-language.md) | Design language: the search.html palette, separate status, entity and value colors, Figtree, artwork first with a cover backdrop, an action hierarchy, one status bar, sources inspection one step away, and one tile for each release. Amends ADRs 0025, 0062 and 0066 | Accepted |
| [0004](0004-format-neutral-audio-tag-boundary.md) | Format-neutral audio tag boundary | Accepted |
| [0005](0005-musicbrainz-metadata-lookup.md) | Metadata-based MusicBrainz lookup. No fingerprinting | Accepted |
| [0006](0006-musicbrainz-release-detail-enrichment.md) | MusicBrainz release detail enrichment | Accepted |
| [0007](0007-metadata-compare-table-drag-copy.md) | Compare table drag copy | Accepted |
| [0008](0008-explicit-id3v24-write-boundary.md) | Only explicit ID3v2.4 frames are written | Accepted |

## Services And Boundaries

| ADR | Scope | Status |
|---|---|---|
| [0010](0010-musicindex-endpoint-setting.md) | Configurable MusicIndex endpoint | Accepted |
| [0015](0015-non-ui-service-boundaries.md) | Services stay free of UI concerns | Accepted |
| [0017](0017-cli-debug-contracts.md) | CLI debug contract and JSON output | Accepted |
| [0024](0024-command-query-event-application-layer.md) | Command, query, and event application layer | Implemented |
| [0040](0040-async-vm-runtime.md) | Async view-model runtime. Actors in `src/runtime/` | Implemented |
| [0041](0041-windowed-paged-view-models.md) | Windowed paged view models | Implemented |
| [0056](0056-remote-media-fetch-validation-boundary.md) | Remote media fetch validation and redirect policy. Amended 2026-10-03: no enclosure length check | Implemented |
| [0058](0058-outbound-http-client-policy.md) | All blocking HTTP clients built in `src/http_client.rs` | Implemented |

## Playback And Broadcast

| ADR | Scope | Status |
|---|---|---|
| [0014](0014-playback-session-authoritative-state.md) | `PlaybackSession` is the authoritative now-playing state | Accepted |
| [0020](0020-simulated-playlist-playback.md) | Simulated playlist transport for relay smoke tests | Accepted |
| [0021](0021-mpv-playback-driver.md) | mpv driver behind the `PlaybackDriver` trait | Implemented |
| [0059](0059-broadcast-control-surface.md) | External publisher control, saved event selection, configured-target readiness; tasks 001-017 complete. ADR 0076 amends its readiness rule | Accepted |
| [0068](0068-show-cue-and-audition-isolation.md) | Persisted Show cue, explicit Show transport, independent audition state/audio/publication boundaries; implementation not started | Proposed |

## UI Architecture

| ADR | Scope | Status |
|---|---|---|
| [0023](0023-design-system-and-view-models.md) | Design system and view-model architecture | Implemented |
| [0025](0025-theme-icon-style-boundary.md) | Theme, icon and style boundary. All eleven packets and recorded visual checks are complete. | Implemented |
| [0026](0026-shared-entity-projection-layer.md) | Shared entity projection layer | Implemented |
| [0027](0027-shared-entity-action-state.md) | Typed action state for shared entities | Implemented |
| [0032](0032-ui-backend-boundary-and-popover-contracts.md) | UI and backend boundary, popover contracts | Implemented |
| [0033](0033-hig-ui-architecture-governance.md) | HIG structural governance for UI work | Implemented |
| [0042](0042-layer-consolidation.md) | Primitive versus composite versus shell | Implemented |
| [0046](0046-workspace-frame-architecture.md) | Workspace frames, history, and chrome | Implemented |
| [0047](0047-library-search-unification.md) | One content surface for library and index rows | Implemented |
| [0060](0060-workflow-surface-structure.md) | Music, Show and Settings structure is delivered. Independent audition remains unfinished under proposed ADR 0068. | Accepted |
| [0062](0062-music-content-surface.md) | Five Music packets are complete. The default Index order returns releases. Broader mixed-row search/expansion still needs an evidence review. | Accepted |
| [0063](0063-show-dashboard-layout.md) | Show cards and diagnostics; [shared log frames and following](../tasks/archive/adr-0063-task-005-shared-log-frames-and-following.md) complete with operator acceptance, preservation and cleanup | Implemented |
| [0069](0069-grouped-settings-and-selective-presets.md) | Grouped Settings, live metadata resource selection, independent audio configuration and selective preset snapshots. Task 001 is complete. [Task 002 is Ready](../tasks/adr-0069-task-002-shared-guarded-editor.md). Implementation of task 002 has not started. | Accepted |
| [0074](0074-repair-and-diagnostics-pages.md) | Separate repair/diagnostics pages, action columns, report views and consistent text. Visual and preservation gates accepted; task 013 cleanup confirmed | Implemented |

## UI Presentation

| ADR | Scope | Status |
|---|---|---|
| [0030](0030-discovery-library-ui-fixes.md) | Current Music/Settings scroll check survives earlier surface fixes | Accepted, partial |
| [0031](0031-release-detail-presentation-contract.md) | Release detail composition | Implemented |
| [0035](0035-track-surface-consolidation.md) | One track detail surface | Implemented |
| [0037](0037-same-entity-surface-parity.md) | Same-entity surface parity. Feed task 001 is complete, including both themes and cleanup. Track task 002 retains its visual gate. | Accepted, partial |
| [0049](0049-inspector-source-ownership.md) | Inspector source tree and filter ownership | Implemented |

## Review Candidates

ADR 0060 replaced the Discover surface with Music. Legacy module records
0003 and 0013 are decided: Superseded by ADR 0060, archived 2026-10-03. This
record cleanup does not put back their earlier screens.

The [2026-09-10 reconciliation](../reviews/2026-09-10-governance-reconciliation.md)
retired replaced requirements before restoring current human checks:

- 0030, 0037, and 0054 have surviving checks in
  [pending human checks](../pending-human-checks.md).
- ADR 0043 is Implemented on 2026-09-18. The operator accepted normal/narrow
  toolbar checks in both themes and confirmed fixture cleanup.
- ADR 0044 is Implemented on 2026-09-19. The operator accepted playlist
  interaction, library removal and immediate row updates in both themes.
  Fixture cleanup is confirmed.
- ADR 0039 is Implemented on 2026-09-18. All three packets and numerical
  ratification are complete. The operator passed thirteen visual checks and
  confirmed fixture removal. The review records preservation as an inference
  from the conditional cleanup command. The dated reconciliation retains its
  original historical result.
- ADR 0025 is Implemented after the 2026-09-18 status correction. All eleven
  packets record implementation, and the review records the required visual passes.
  Further theme work uses bounded packets.
- ADR 0055 is Implemented after the 2026-09-18 module review and focused checks.
- ADRs 0060 and 0062 retain the unfinished scope named in their status sections.
  Their completed structural packets are not an implementation backlog.

The [2026-09-18 review](../reviews/2026-09-18-adr-status-and-remaining-work.md)
records these corrections and the remaining work. Historical reviews retain
their dated findings.

## Status Verification

The situational ADR 0057 guard `adr_0057_status_headers_are_canonical` checks
every current and archived numbered decision. The
[2026-09-11 reconciliation](../reviews/2026-09-11-pending-work-reconciliation.md)
closes the header-format drift. The guard verifies format, vocabulary and date;
implementation and human acceptance still require their named evidence.
