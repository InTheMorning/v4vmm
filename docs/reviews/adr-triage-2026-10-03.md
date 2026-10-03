# ADR Triage - 2026-10-03

## Status

Open - 2026-10-03. This record is advisory. It states no rule. The operator decides each group.
The [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md) owns this work.

## Method

Eight read-only agents each read about ten current ADRs. For each ADR, an agent listed its rules and searched the guards and the tests for enforcement. It compared the rules with the code and the later ADRs. Then it recommended one action.

The orchestrator merged the eight results and checked a sample of the claims against the repository.
The archive rule comes from the shared governance. An ADR archives when a later ADR supersedes it, or when a test enforces each rule. An ADR with an unenforced rule stays current.

The batch files stay in the orchestrator scratchpad. This record keeps the decisions and the evidence summary.

## Orchestrator Checks

- The modules of ADRs 0003, 0013 and 0055 do not exist: `src/musicindex.rs`, `src/search.rs`, `src/ui_track.rs` and `src/view_models/search/`. No code names `SearchApp` or `TrackRowMode`.
- ADR 0052 is a record of a finished documentation triage. ADR 0053 holds its rules.
- ADR 0076 packet 003 implemented the ADR 0076 amendment of ADRs 0059 and 0065 on 2026-09-24. Both status lines still say "The amendment is not implemented".
- The rubric gave the staged frame model as an example of removed code. That is not true: ADR 0081 task 001 is Ready, and the model stays in the code. The batch agents checked the code and did not use the example as evidence.

## Groups

| Group | Meaning | ADRs |
|---|---|---|
| A | Archive now as superseded or obsolete. Its header becomes "Superseded by ADR NNNN" | 0002 (by 0012), 0003 (by 0060), 0009 (by 0076), 0013 (by 0060), 0052 (by 0053), 0055 (by 0060) |
| B1 | Archive now as fully guarded. Its guards stay | 0034, 0036, 0038, 0039, 0043, 0044, 0048, 0050, 0051, 0067, 0070, 0071, 0072, 0073 |
| B2 | Archive as fully guarded after a citation packet. The enforcing tests cite another ADR, or the status text is wrong | 0011, 0016, 0022, 0064, 0065 |
| C | Archive after code work | 0078, after ADR 0082 packet 002 |
| K | Keep current | The other 52 |

## Text Corrections For Current ADRs

- ADRs 0059 and 0065: the status line says that the ADR 0076 amendment is not implemented. ADR 0076 packet 003 implemented it on 2026-09-24.
- ADR 0049: one decision names `IndexArtistDetail`. ADR 0077 packet 006 replaced it with `IndexNameMatches`.
- ADRs 0023, 0024 and 0026 name files that later ADRs renamed or split, for example `src/ui_entity.rs` and `src/search.rs`. The decisions stay true.
- `docs/adr/README.md` says "guard recorded" for ADR 0055. No such guard exists.
- `docs/adr/README.md` "Review Candidates" asks for a decision on ADRs 0003 and 0013. Group A answers it.

## Dependents To Change In The Same Commit

- Group A: the README index rows, the "Review Candidates" section, and each document that restates their rules. No guard cites a Group A ADR.
- Group B: each guard stays and keeps its citation. An archived, fully guarded ADR stays the owner of its guards.
- ADR 0046: the guard `workspace_frame_phase_6_detach_dock_model_only_contract` requires the staged model. ADR 0081 task 001 deletes that guard with the model.
- ADRs 0077 and 0078: two tests named `adr_0077_publisher_page_*` test the ADR 0078 page type. ADR 0082 packet 002 deletes them.

## Per-ADR Recommendations

| ADR | Title | Group | Agent recommendation | Reason |
|---|---|---|---|---|
| 0001 | Record Architecture Decisions | K | KEEP-UNENFORCED | Numbering and Nygard structure continue. Only the immutability clause is superseded. |
| 0002 | Rust CLI With Local SQLite State | A | ARCHIVE-SUPERSEDED | ADR 0012 removed CLI-only commands and made the GPUI app the primary surface. |
| 0003 | MusicIndex Search UI Module | A | ARCHIVE-OBSOLETE | The named module and the separate search binary do not exist in today's tree. |
| 0004 | Format-Neutral Audio Tag Boundary | K | KEEP-UNENFORCED | `AudioTags` / `read_audio_tags` own tag reading today. ADR 0080 confirms the boundary. |
| 0005 | MusicBrainz Metadata Lookup | K | KEEP-UNENFORCED | `src/musicbrainz.rs` lookup flow still matches the decision. |
| 0006 | MusicBrainz Release Detail Enrichment | K | KEEP-UNENFORCED | Release-detail fetch and per-id cache still implemented as decided. |
| 0007 | Metadata Compare Table Drag Copy | K | KEEP-UNENFORCED | Staged-edit model (`PendingId3Edit`) and the compare grid still exist. |
| 0008 | Explicit ID3v2.4 Write Boundary | K | KEEP-UNENFORCED | `write_id3v24_edits` is the one write path today. ADR 0080 extends it. |
| 0009 | Search Thumbnail Cache and Feed Batch Tagging | A | ARCHIVE-SUPERSEDED | The cache moved to ADR 0066. Automatic tag writes at subscribe time are gone. |
| 0010 | Configurable MusicIndex Endpoint | K | KEEP-UNENFORCED | `musicindex_endpoint` config field and normalization match the decision. |
| 0011 | MusicIndex GUID in ID3 TXXX frames | B2 | ARCHIVE-FULLY-GUARDED | The one rule is live in code and proved by tests filed under ADR 0075/0080 names. |
| 0012 | Root desktop crate | K | KEEP-UNENFORCED | True today, but no test stops a regression back to a nested crate or a retired CLI command. |
| 0013 | Shared Discover track row module | A | ARCHIVE-OBSOLETE | `search.rs`, `ui_track.rs`, `SearchApp`, `TrackRowMode` do not exist in `src/`. |
| 0014 | PlaybackSession authoritative state | K | KEEP-ENFORCED-PARTIAL | Field-for-field match in code and unit-tested, but no guard blocks a driver from later owning identity. |
| 0015 | Non-UI service boundaries | K | KEEP-ENFORCED-PARTIAL | Core service files stay UI-free under a guard, but it is filed under a different ADR. Not every named boundary, for example playlist operations or track identity, has its own check. |
| 0016 | Schema migration discipline | B2 | ARCHIVE-FULLY-GUARDED | Fresh-schema and legacy-schema migration paths are both unit-tested, and the registry is in daily use at schema version 17+. |
| 0017 | CLI debug contracts | K | KEEP-ENFORCED-PARTIAL | All four original commands, and more, are live in `src/cli.rs`, and ADR 0060 explicitly preserves the contract, but only one command name is guarded. |
| 0020 | Simulated playlist playback | K | KEEP-ENFORCED-PARTIAL | `NullDriver` still matches the decision text and has unit tests, but no guard stops `--dry-run` from later touching a real driver. |
| 0021 | mpv playback driver | K | KEEP-ENFORCED-PARTIAL | `src/playback_driver/mpv.rs` matches the trait and IPC contract with its own unit tests, but no architecture guard protects the contract, and two follow-ups stay open in the deferred-work index. |
| 0022 | UI-agnostic core extraction | B2 | ARCHIVE-FULLY-GUARDED | Every named service module exists, owns its domain, and stays UI-free under a guard; `search.rs` is gone entirely, which only strengthens the separation. |
| 0023 | Design System and View-Model Architecture | K | KEEP-ENFORCED-PARTIAL | Core layering still runs the app. Many named rules (layer-import table, VM naming) have no cited guard. |
| 0024 | Command/Query/Event Application Layer | K | KEEP-ENFORCED-PARTIAL | Guards cover the migrated workflows well. Typed-error and event-shape rules stay a convention only. |
| 0025 | Theme, Icon, and Style Boundary | K | KEEP-ENFORCED-PARTIAL | Ten named guards plus separate high-contrast WCAG tests cover this ADR. Role-admission criteria stay a review judgment, not a test. |
| 0026 | Shared Entity Projection Layer | K | KEEP-ENFORCED-PARTIAL | Guards cover the purity and boundary rules. No guard checks the "no inference" or "raw facts survive" rules. |
| 0027 | Shared Entity Action State | K | KEEP-ENFORCED-PARTIAL | Only one shared guard exists. `ReleaseActionState`/`TrackActionState` behavior has unit tests, but none cite ADR 0027. |
| 0028 | Local Identity Source Fact Persistence | K | KEEP-ENFORCED-PARTIAL | The schema is live in `src/db.rs`. Only the contributor-panel rule has a cited guard, and ADR 0075 already replaced the replacement-key rule. |
| 0030 | Discovery and Library UI Correctness Fixes | K | KEEP-UNENFORCED | No test cites "ADR 0030" to enforce its own rules. One open human check (Task 006 scroll) remains listed. |
| 0031 | Release Detail Presentation Contract | K | KEEP-UNENFORCED | `ReleaseDetailPageVm`/hero/panel rules are implemented with ordinary unit tests, but no test names "ADR 0031". |
| 0032 | UI Backend Boundary and Popover Contracts | K | KEEP-ENFORCED-PARTIAL | Guards cover playlist-popover ownership well. The general "screen-local chrome only if no shared owner exists" rule is a review rule, not a test. |
| 0033 | HIG UI Architecture Governance | K | KEEP-ENFORCED-PARTIAL | Mechanical coverage is near-exhaustive. AGENTS.md itself cites ADR 0033 as the owner of the visual-proof/UI-acceptance-gate rule, and that rule stays a human check by design. |
| 0034 | Scale-Aware UI Tokens and Controls | B1 | ARCHIVE-FULLY-GUARDED | Core scale-token rules are guarded. ADR 0032 guards the popover-ownership clause too. |
| 0035 | Track Surface Consolidation | K | KEEP-ENFORCED-PARTIAL | All 8 named "Enforcing tests" exist, but the artwork-handoff and loading/empty-state ownership rules have no guard. |
| 0036 | Feed, Visual, and Provenance Surface Consistency | B1 | ARCHIVE-FULLY-GUARDED | All 5 named tests exist and enforce the three passes (feed shell, visual tokens, provenance panels). |
| 0037 | Same-Entity Surface Parity | K | KEEP-ENFORCED-PARTIAL | Ownership guards pass, but the ADR itself says they "do not substitute" for the open Task 002 track-detail visual parity check. |
| 0038 | Presentation Contract Enforcement | B1 | ARCHIVE-FULLY-GUARDED | All 8 Task 001-008 guards exist. The ADR text once called six of them "planned." Task 008 readiness gate reads `Proceed`. |
| 0039 | Dynamic Type Ramp | B1 | ARCHIVE-FULLY-GUARDED | Mechanical suite is extensive and the operator's own status line says "No ADR 0039 operator gate remains open." |
| 0040 | Async View-Model Runtime | K | KEEP-ENFORCED-PARTIAL | Runtime-boundary and `cx.spawn` rules are guarded. No guard checks that `src/presentation/` is the only module that imports `tokio` and `gpui` together. |
| 0041 | Windowed Paged View-Models | K | KEEP-ENFORCED-PARTIAL | The `PagedListVm` contract has full unit-test coverage. The adoption rule, "any list VM that may exceed ~10k rows MUST use it," has no architecture guard. |
| 0042 | Layer Consolidation | K | KEEP-ENFORCED-PARTIAL | The guard checks three named call sites against a historical audit doc. The ≥2-call-site rule, the primitive no-domain-types rule, and the <300 LOC screen rule rely on manual audit cadence. No guard checks them. |
| 0043 | Top Toolbar Now Playing Frame and Global Search | B1 | ARCHIVE-FULLY-GUARDED | Three guards cover toolbar structure, the VM and query boundary, and duplicate-search removal. The status line says "No gate under this ADR remains open." |
| 0044 | Playlist drag handle reordering | B1 | ARCHIVE-FULLY-GUARDED | Every invariant is tested or has a named, completed manual check. No open gate. |
| 0046 | Workspace frame architecture | K | KEEP-ENFORCED-PARTIAL | Most invariants are guarded. Invariant 8 is superseded by ADR 0081, but its guard test is correct only for the earlier model. Task 015 Forward keeps an open visual gate. |
| 0047 | Library and search unification | K | KEEP-ENFORCED-PARTIAL | Phases B-F are guarded, and the code confirms search.rs, Discover, and GlobalSearchScope are gone. Phase G visual proof is historical, and some invariants (for example no-raw-transport-error) have no named ADR-0047 ... |
| 0048 | ContentList frame breadcrumb search | B1 | ARCHIVE-FULLY-GUARDED | Status line already claims guard-only evidence. Guards were updated in place for ADR 0060's tab renames and still pass. |
| 0049 | Inspector source ownership | K | KEEP-ENFORCED-PARTIAL | Ownership/filter/mutation rules stay guarded. The "IndexArtistDetail" artist-activation bullet is stale — code now uses `IndexNameMatches`/`ArtistDetail`/`PublisherDetail` per ADR 0077. |
| 0050 | Post-ADR-0048 module decomposition | B1 | ARCHIVE-FULLY-GUARDED | One-shot refactor the ADR itself says needs no recurring test. Its own "Verified files" list matches today's tree and the split files anchor many other live guards. |
| 0051 | Workspace pane width persistence | B1 | ARCHIVE-FULLY-GUARDED | One test covers persistence, clamp, default fallback, and write-once-on-end semantics for every invariant in the ADR. |
| 0052 | Library/Index data parity triage | A | ARCHIVE-SUPERSEDED | A documentation-only triage whose routing table is now restated and carried forward as binding decision in ADR 0053. Its own 2026-09-24 amendment already hands identity routing to ADR 0077/0079. |
| 0053 | Local detail source-fact parity | K | KEEP-UNENFORCED | Parent contract reserving a large unimplemented scope (artist biography, track language/lyrics, playlist semantics). No guard cites it and most of its field list has no ADR or schema yet. |
| 0054 | Local metadata source-fact persistence | K | KEEP-ENFORCED-PARTIAL | Storage/layering/fact-key rules are well guarded. The per-source replacement-key rule is superseded by ADR 0075, and feed/track hydration visual acceptance is still an open gate (`pending-human-checks.md` item 3). |
| 0055 | Search view-model module decomposition | A | ARCHIVE-OBSOLETE | The decomposed `src/view_models/search/` tree is deleted. `search_results/` is a different, unrelated module. |
| 0056 | Remote media fetch validation boundary | K | KEEP-ENFORCED-PARTIAL | Heavily guarded and the 2026-10-03 amendment already has matching tests, but not every invariant has a dedicated named guard. |
| 0057 | ADR status vocabulary and amendment policy | K | KEEP-ENFORCED-PARTIAL | Header format and vocabulary are guarded. The amendment and verification-requirement rules are prose only. |
| 0058 | Outbound HTTP client policy | K | KEEP-ENFORCED-PARTIAL | Single-owner rule is guarded. "named constant, not a literal" is not separately tested. |
| 0059 | Broadcast control surface | K | KEEP-ENFORCED-PARTIAL | Extremely well guarded, but the ADR's own header says the ADR 0076 Decisions 7/9 amendment "is not implemented," which the code contradicts (see below). |
| 0060 | Workflow surface structure and vocabulary | K | KEEP-ENFORCED-PARTIAL | Structural rules guarded. The audition mechanism it names has zero code and zero guard (deferred to Proposed ADR 0068). |
| 0061 | Current-state governance | K | KEEP-ENFORCED-PARTIAL | One durable rule is guarded. Most invariants (reading-path size, guard classification correctness, archive bookkeeping, README-matches-directory) are process rules with no test. |
| 0062 | Music content surface | K | KEEP-ENFORCED-PARTIAL | Row contract, filter, and recency rules are guarded. The broader search/expansion follow-up is explicitly unverified. |
| 0063 | Show dashboard layout | K | KEEP-ENFORCED-PARTIAL | Very thoroughly guarded. A few rules (equal card height, log readability) are visual-only and covered by passed operator checks, not tests. |
| 0064 | Local file addressing | B2 | ARCHIVE-FULLY-GUARDED (borderline) | Every stated invariant has a passing test (resolver ownership, relative-path format, write-outside-folder failure, idempotent repair, no-resolve/no-folder no-op), but most of those tests do not cite ADR 0064 by name. |
| 0065 | Payment Route Tag Repair | B2 | ARCHIVE-FULLY-GUARDED (after a status-text fix) | All 6 invariants are tested. The ADR's own status line incorrectly says the change is not done. |
| 0066 | Configuration And Startup Failure Recovery | K | KEEP-ENFORCED-PARTIAL | Large decision. Task 004's operator gate is still open, and ADR 0068 playback work still depends on it. |
| 0067 | Platform Shortcut Modifiers | B1 | ARCHIVE-FULLY-GUARDED | Every invariant has a direct test; no open human gate remains. |
| 0068 | Show Cue And Audition Isolation | K | PROPOSED-STALE — keep | Detailed, still-referenced proposal; explicitly deferred, not abandoned. |
| 0069 | Grouped Settings And Selective Presets | K | KEEP-ENFORCED-PARTIAL | Only task 001 (foundation) shipped; presets, Live Metadata and Audio groups are unimplemented and unguarded. |
| 0070 | Show Log Space Priority | B1 | ARCHIVE-FULLY-GUARDED | Remaining (non-superseded) rules are all tested; operator visual acceptance is closed. |
| 0071 | Shared Text Selection And Linux Primary | B1 | ARCHIVE-FULLY-GUARDED | Dense mechanical coverage; available X11 checks accepted; IME/Wayland are a named, accepted coverage limit, not an open rule. |
| 0072 | Pinned gpui-base Selection Corrections | B1 | ARCHIVE-FULLY-GUARDED | Manifest/lockfile pin is mechanically checked; native X11 acceptance is closed. |
| 0073 | Show Card Overflow Scrolling | B1 | ARCHIVE-FULLY-GUARDED | Short, focused decision; its one behavior is directly tested and visually accepted. |
| 0074 | Repair and diagnostics pages | K | KEEP-ENFORCED-PARTIAL | Operator-accepted and well tested, but layout/typography detail stays operator-checked, not test-checked. |
| 0075 | Metadata ownership and completeness | K | KEEP-ENFORCED-PARTIAL | This ADR still owns dozens of live field rules. ADR 0076 supersedes only its source-selection parts. Field policy packets 031/034 stay open. |
| 0076 | Playlist RSS check for stale MusicIndex records | K | KEEP-ENFORCED-PARTIAL | Nine packets are implemented and Green. Visual gates stay open under the operator's paused-visual-check rule. |
| 0077 | Publisher feed artist binding | K | KEEP-ENFORCED-PARTIAL | Decisions 1, 3, 5, 6 and 7 are live and guarded. Decision 4 is dead code twice over, and its tests still carry the ADR 0077 name. |
| 0078 | Publisher page type from stated role | C | ARCHIVE-SUPERSEDED | The header is correct now. The code it governs (`page_type()`) still ships today, and the ADR 0082 packet 002 commit must delete it when it archives the file. |
| 0079 | Remove MusicIndex artist subject storage | K | KEEP-ENFORCED-PARTIAL | The storage-deletion rule is fully guarded. The person-identity invariant has no guard, and the visual gate is open. |
| 0080 | Tag frames follow their owner | K | KEEP-ENFORCED-PARTIAL | Heavily unit-tested, but the ADR's own phase plan records an unfixed MP4 date-tag defect with no owning packet. |
| 0081 | Remove the staged frame model | K | KEEP-ENFORCED-PARTIAL | Forward (Decision 3) shipped, and a guard covers it. The actual model and slot deletion (Decisions 1-2) has not started. The staged frame model still exists in `src/`. |
| 0082 | Publisher roles belong to each album link | K | KEEP-ENFORCED-PARTIAL | The 0.7.0 contract decode is Green. The page itself (Decisions 1-6) is unbuilt, so the UI today still shows the superseded ADR 0078 behaviour. |
