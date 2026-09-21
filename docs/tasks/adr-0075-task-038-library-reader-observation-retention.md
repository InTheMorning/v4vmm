# ADR 0075 Task 038: Library Reader Observation Retention

Status: Implementation, technical review, and mechanical checks complete - 2026-09-21.
Coding agent: `library_retention_038`. Root accepted the code and tests. The normal binary build is Green.

All visual gates remain open and paused. Do not request a visual batch.

## Goal

Retain provider observations from Library tag comparison and album hydration through the existing shared writer.
Preserve committed receipts through ordinary failures, storage failures, successful results, and selection changes.
Preserve existing comparison inputs, hydration writes, request profiles, and field policies.

This packet converts only `CompareLibraryTrack` and `HydrateAlbumIdentity`.
It inherits packet 013's reviewed RSS completeness behavior through the shared writer. It adds no completeness contract.
MusicIndex hydration retains evidence without provider snapshot replacement. MusicIndex completeness remains disabled.

## Authority And Dependencies

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), provider evidence, ownership, collection state, and atomic replacement.
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md#additional-packets-from-the-correction), the bounded Library reader assignment.
- [Packet 014](adr-0075-task-014-provider-observation-retention.md), capture, request generations, receipts, storage failures, and retry certainty.
- [Packet 013](adr-0075-task-013-verified-snapshot-replacement.md), registered RSS coverage and the shared writer's final result types.
- [Storage design](../schema/adr-0075-provider-snapshot-storage.md), provider/request identity and durable observation boundaries.
- [Request baseline](../notes/adr-0075-request-and-write-baseline.md), measured hydration requests and legacy row mutations.

Preparation used the working tree after packet 014. Root completed compatibility review after packet 013 passed technical and mechanical review.
The reviewed boundary includes `ProviderReadError`, exact local request binding, and receipt-preserving reads.
No packet 013 implementation work remains active.
The visual pause does not prevent this packet's mechanical work after dispatch.

Root approved the application-owned ordinary failure boundary below.
This packet accepts no new comparison, hydration, field, or presentation policy.
Packet 017 owns named request profiles. Packet 019 owns visible storage retry. Packet 020 owns shared metadata projection.

## Files To Inspect

Read the authority documents and these exact owners:

- [AGENTS.md](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- [Library queries](../../src/application/queries/library.rs): `CompareLibraryTrack`, `HydrateAlbumIdentity`, their result types, and `query_error`.
- The same file: `compare_library_track`, `hydrate_album_identity_facts`, and packet 014's result assembly.
- [Library callbacks](../../src/library/app_impl.rs): comparison opening/reload, `start_compare_library_track`, and `hydrate_album_identity_on_view`.
- The same file: `command_error_detail`, selected-frame checks, and packet 014's error callback.
- [Library state](../../src/view_models/library.rs): receipt/capsule maps, narrow status retention, and album updates.
- [Command errors](../../src/application/errors/command.rs): clone/equality, `Query` display, and `ObservationWriteFailure`.
- [Observation types](../../src/provider_observation.rs): recorder, receipts, failure capsules, safe displays, and token states.
- [Observation writer](../../src/db/provider_observations.rs) and packet 013's completed contract owner.
- [API client](../../src/api.rs): `with_observation_recorder`, detail routes, observed DTO decoding, and exact request profiles.
- [Feed service](../../src/feed_service.rs): recorder-aware detail fetch, merge, defaults, and local context construction.
- [Subscription service](../../src/subscribe_service.rs): RSS resource selection and `compare_downloaded_track_path`.
- [RSS enrichment](../../src/rss/enrich.rs): exact request parameters and the existing single DOM parse.
- [Track context](../../src/metadata.rs): receipt transport, cloning, and sanitization.
- [Legacy ingestion](../../src/identity_ingest.rs): feed persistence, source grouping, and DTO-derived `raw_json`.
- [Local identity reads](../../src/local_identity.rs), [local metadata reads](../../src/local_metadata.rs), and [audio tag reads](../../src/audio_tags.rs).
- [Architecture guards](../../tests/architecture_tests.rs): packet 014/013 writer ownership and excluded comparison assertions.
- [Fixture launcher](../runbooks/startup-recovery-fixture.py) and [fixture seed](../../src/startup/fixture.rs), for prospective operator instructions only.

## Files To Change

| File | Permitted change |
| --- | --- |
| `src/application/queries/library.rs` | Add comparison database access, observed caller wiring, hydration receipts, application result assembly, and behavioral tests. |
| `src/application/errors/command.rs` | Add `ObservedQueryFailure` and its narrow command variant. Preserve existing query display and storage variants. |
| `src/view_models/library.rs` | Retain success/error evidence separately from presentation. Preserve the narrow storage-status behavior. |
| `src/library/app_impl.rs` | Pass comparison's connection and consume evidence before selection checks. Preserve existing panel and album updates. |
| `tests/architecture_tests.rs` | Replace the old comparison exclusion with explicit packet 038 caller and consumer guards. Preserve shared writer boundaries. |
| This packet | Record implementation results, checks, limits, and prospective operator evidence. |

The existing recorder-aware helpers are sufficient. No new production module or dependency is expected.
No DTO, `TrackContext`, or Discover literal change is required for the proposed result boundary.
Report any additional required owner to root before editing it.
Keep unit tests beside their owners. Do not add another integration file or shared test helper module.

## Current Roots And Request Bounds

### Library Tag Comparison

The comparison panel dispatches `CompareLibraryTrack` through `start_compare_library_track`.
Initial opening can dispatch once. A loaded panel stays cached until an explicit reload.
The existing `redownload_tag_compare` and `reread_tag_compare` methods call the same reload path.
Do not introduce media downloading or change those actions in this packet.

`compare_library_track` checks for a local path, fetches source context, then reads and compares local audio tags.
It currently has no database parameter. Add the existing shared connection to this command and pass `Arc::clone(&self.conn)` from Library.
Use `fetch_library_track_context_with_recorder` with one recorder for the complete command.

Keep the current request order:

1. Scoped track detail when the local feed GUID exists.
2. Unscoped track detail when the scoped result is unavailable.
3. Feed detail when the returned track or local row supplies a feed GUID.
4. RSS enrichment when the merged context has a usable feed URL.

All Index requests use this exact include string:

```text
source_links,source_ids,source_release_claims,source_contributors,payment_routes
```

A retained ordinary remote failure still permits `track_row_to_track_context(track)` as the existing comparison fallback.
Do not substitute `fetch_library_track_context_with_local_fallback`. Its hydrated local defaults would change comparison inputs.
An observation allocation or write failure must bypass ordinary fallback and remain an error.

### Library Album Hydration

`hydrate_album_identity_on_view` dispatches `HydrateAlbumIdentity` when the existing hydration condition requires it.
The condition skips work only when identity actions, a description, and nonempty metadata already exist.
Missing local feed ID or feed GUID also prevents dispatch. Preserve these conditions.

Hydration already owns the shared connection. Attach a recorder to its existing API client before `fetch_feed`.
It sends one feed-detail request with this exact include string:

```text
source_links,source_ids,source_release_claims,source_contributors
```

After observation retention succeeds, keep the existing operations and their order:

1. Build the current feed description through `FeedView::from_api`.
2. Update the local description when that projection supplies one.
3. Run `identity_ingest::persist_musicindex_feed` without changing its grouping or transaction behavior.
4. Read local identity and metadata facts.
5. Return the existing hydration values with the command's committed receipts.

Original bytes must commit before any DTO projection or legacy write can discard evidence.
A later legacy failure does not undo that committed observation.
Do not combine legacy writes and observation retention into a new transaction.
Do not claim complete rollback for the existing sequence of legacy writes.

### Measured And Static Bounds

| Case | Index requests | RSS requests | Evidence basis |
| --- | ---: | ---: | --- |
| Successful comparison with a scoped track | 2 | At most 1 | Current shared caller inspection. |
| Scoped comparison fallback succeeds | At most 3 | At most 1 | Current shared caller inspection. |
| Comparison with known feed GUID and all Index responses HTTP 503 | 3 | 0 | Same request path as packet 014. Comparison itself was not measured by packet 016. |
| Comparison without a local feed GUID | At most 2 | At most 1 | One unscoped track request, then an optional feed request. |
| Comparison with no local path | 0 | 0 | Path validation precedes metadata work. |
| Album hydration, including repetition or HTTP 503 | 1 | 0 | Packet 016 measured each case. |
| Failed request allocation | 0 for that allocation | 0 for that allocation | Packet 014 contract. Earlier command requests can precede this allocation. |
| Storage-only retry | 0 additional | 0 additional | Packet 014 contract. |

These counts exclude redirects and transport-internal behavior. They do not authorize new retries or caching.
A present but unreadable local file differs from a missing local-path binding. Tag reads currently follow metadata requests.

Packet 016 measured five legacy row mutations for first hydration and nine for repetition in its constructed fixture.
Those historical counts exclude observation writes and later source changes. Recheck the fixture against the completed implementation.
Report legacy mutations, observation/snapshot mutations, and committed transactions separately.
Do not describe row mutations as disk writes, bytes written, or a performance improvement.

### Exact Request Identity

Keep the existing Index profile: `{"version":1,"path":path,"query":query}`.
Its unresolved parameter descriptor remains `{"path":path,"query":query}`.
Keep the RSS profile: `{"version":1,"operation":"rss_track_enrichment"}`.
Its parameters retain both `track_guid` and `enclosure_url`, including null and empty values.

Use the existing provider, resource, requested-subject, and profile identity rules.
Do not add a comparison or hydration discriminator to a shared request profile.
Equal detail and comparison descriptors share request-slot ordering through the existing writer.
Different include profiles remain different request identities.

## Typed Result And Failure Contract

### Successful Results

Keep `LibraryTrackCompare` as the comparison result. Attach receipts to its existing `track_context.observation_receipts`.
Add `observation_receipts: Vec<ObservationReceipt>` to `AlbumIdentityHydration` outside its projected fact values.
Do not serialize observations into API DTOs, local identity values, or tag comparison rows.

After recording, load comparison's typed provider state through packet 013's exact request binding and local reader.
The comparison callback replaces `frame.source_context`. It must not replace previously loaded provider state with an uninitialized default.
Keep this read inside the receipt-preserving result assembly. A read failure retains the committed receipts and its typed failure.
This additional state must not change DTO comparison inputs, local fallback fields, or tag comparison rows.
Use `feed_service::local_provider_request` when the comparison result needs the exact local RSS resource binding.
Preserve packet 013's `ProviderReadError::Storage`, including any preceding write capsule and committed receipts.

### Ordinary Query Failures

Define the small application-owned `ObservedQueryFailure` type in `application/errors/command.rs`.
It contains the existing query message and `Arc<[ObservationReceipt]>` for committed receipts.
The message is the exact `format!("{error:#}")` value that the ordinary `Query` mapping would receive.
Keep clone and equality support. Its debug output must omit the message and private receipt contents.
Provide only accessors reached by command display or Library retention.

Add `CommandError::ObservedQueryFailure(Arc<ObservedQueryFailure>)`.
Its display must equal `CommandError::Query` display for the same message:

```text
query refresh failed: {message}
```

Update `command_error_detail` exhaustively. For this variant, preserve the same unprefixed detail that `Query` supplies there.
Do not add `CommandError` or arbitrary query strings to `provider_observation`.
Do not pass an ordinary query cause through `ObservationCommandFailure`'s safe storage display.

### Application Result Assembly

Use one application-owned assembly helper for these two roots in `application/queries/library.rs`.
Run all fallible command work within the result that this helper receives.
Drain the recorder exactly once after that work, on every outcome.
No `?` after a completed request may bypass that assembly and discard receipts.

The helper follows this order:

1. On success, attach all committed receipts to the existing successful result.
2. On storage failure, preserve the write capsule or allocation error in the existing `ObservationCommandFailure`.
3. Attach all earlier committed receipts to that typed storage failure.
4. On an ordinary failure, retain its exact query message and receipts in `ObservedQueryFailure`.
5. Return the failure through `CommandError`, even when some observations committed successfully.

Classify storage failures before ordinary query wrapping. Preserve an existing typed capsule without reconstructing its body, token, or times.
Retained transport failure still permits existing comparison fallback. Storage failure does not.

Receipts describe committed observations. They never convert a failed tag read, legacy write, or local read into command success.
Do not replace either error variant with `CommandOutcome::without_events`.

Keep packet 014's verified-rollback retry and cloned-token replay behavior.
An identical committed replay returns its stored receipt without new mutations. Changed input and uncertain token states remain blocked.
This packet adds no retry action, automatic retry, filesystem spool, or pending-request abandonment.

## Callback And View-Model Ownership

Separate evidence retention from presentation in `LibraryViewModel`.
Use the existing generation-keyed receipt and capsule collections. Add narrow methods only when production callbacks consume them.
Retaining receipts alone must not set status, clear loading, cancel removal confirmation, or change busy state.

Both success callbacks retain receipts before checking the selected track or album.
The comparison callback then keeps its existing context assignment and loaded comparison panel behavior.
The hydration callback then keeps its existing tree updates and selected-album updates.

Both error callbacks retain available receipts and capsules before any selection check.
Ordinary comparison errors continue through `deferred_panel_error_message` only for the matching selected track.
Ordinary hydration errors remain silent after evidence retention.
Storage errors also use the existing narrow Library status owner, including after selection changes.
The matching comparison panel may keep its existing error presentation for that storage failure.

Do not call `set_error_status` for background retention. That helper cancels removal confirmation and clears loading.
Preserve earlier evidence through navigation, later successes, and later failures until Library session teardown.
Keep session teardown's existing callback rejection. It does not claim persistence for an uncommitted response.

## Mechanical Acceptance Criteria

Use behavioral tests with the `adr_0075_library_observation_` prefix beside the owning code.
Use disposable databases, scripted localhost responses, and temporary local audio files only.

| Case | Required proof |
| --- | --- |
| L38-01 | Both real command roots allocate durable generations before the server receives requests. Success results carry every committed receipt. |
| L38-02 | Comparison preserves exact paths, includes, fallback order, and the request bounds above. A missing local path causes no network or generation allocation. |
| L38-03 | Comparison results retain existing local fallback fields, tag rows, contributors, and payment-route values. No tag or audio file changes. |
| L38-04 | Hydration makes one Index request and no RSS request. Its success and repetition preserve legacy values and write behavior. |
| L38-05 | Original JSON/RSS bodies, unknown members, declared owners, and failed attempts survive reopening after both roots consume their results. |
| L38-06 | Allocation failure prevents that request. Response-write failure bypasses comparison fallback and prevents hydration's subsequent legacy writes. |
| L38-07 | Earlier committed receipts survive later allocation/write failures, tag-read failures, legacy-write failures, and local-read failures. |
| L38-08 | Ordinary failures use `ObservedQueryFailure` with exact existing query display. Storage failures retain their existing typed variant and safe display. |
| L38-09 | Successful receipts and all failure evidence reach the live callback/VM owner before selection checks, including after navigation. |
| L38-10 | Ordinary comparison errors remain panel-local. Ordinary hydration errors remain silent. Storage failures update only the established narrow status path. |
| L38-11 | Retention preserves selection, removal confirmation, unrelated loading/busy state, earlier receipts, and earlier capsules through later results. |
| L38-12 | Injected response failure permits storage-only retry after verified rollback. Committed cloned-token replay adds no requests or mutations. Uncertain or changed-input replay remains blocked. |
| L38-13 | Interleaved detail/comparison calls with equal descriptors respect shared request generations. Different profiles do not share request failure state. |
| L38-14 | The completed packet 013 RSS contract remains effective without new contracts. Index hydration cannot authorize provider snapshot replacement. |
| L38-15 | Repeated unchanged responses reuse retained evidence as the shared writer specifies. Report legacy mutations, observation/snapshot mutations, and transaction counts separately. |
| L38-16 | Hydration suppression and comparison panel caching/reload behavior stay unchanged at their existing owners. |
| L38-17 | Packet 014/013 guards remain effective. Other caller families stay outside this recorder conversion. |
| L38-18 | Successful comparison returns current typed provider state after recording. Its frame assignment preserves complete-empty state and separate refresh evidence. |

For L38-07, include a real tag-read failure after successful metadata retention.
Also inject hydration failure after the observation commits but before legacy persistence completes, and failure during its final local reads.
Use transaction or authorizer fixtures at existing database boundaries. Do not add a production failure setting.
Do not assert legacy rollback that the current hydration sequence does not provide.

For L38-08, compare displays and unprefixed details directly against `CommandError::Query` for the same ordinary cause.
Prove that a nonempty receipt list still produces a failed command result.
For L38-09, a model constructed only in a database test is insufficient. Exercise the production callback retention boundary.

## Test Commands

Root coordinates integrated Cargo checks after the final diff stabilizes.
Run the normal desktop build after all tests.

```bash
cargo test --locked --offline adr_0075_library_observation_
cargo test --locked --offline adr_0075_observation_
cargo test --locked --offline adr_0075_snapshot_
cargo test --locked --offline adr_0075_rss_
cargo test --locked --offline --test architecture_tests
cargo check --locked --offline
cargo clippy --locked --offline -- -D warnings
cargo fmt -- --check
cargo test --locked --offline
cargo build --locked --offline --bin v4vmm
python3 -B docs/runbooks/check-markdown-links.py docs/tasks/adr-0075-task-038-library-reader-observation-retention.md
python3 -B /home/citizen/.agents/skills/asd-ste100/scripts/ste_lint.py --check --no-heuristics docs/tasks/adr-0075-task-038-library-reader-observation-retention.md
git diff --check
```

The historical baseline is context, not proof of this packet.
Do not rerun unchanged startup-fixture tests unless a changed owner or confirmed failure requires them.

## Excluded Scope

- No schema, migration, database repair, restore, legacy backfill, or production data change.
- No shared writer, decoder, registry, timeout, source ownership, completeness, or request-profile redesign.
- No legacy source-grouping correction, comparison field change, source-priority change, or hydration suppression change.
- No renderer composition, layout, active identity projection, visible retry, or new global error surface.
- No tag write, media download, audio conversion, playback, broadcast, payment, or configuration change.
- No conversion of callers assigned to packets 039 through 044.
- No upstream request, upstream edit, dependency change, app launch, commit, or shared status-document edit.

## Rollback And Escalation

A code revert leaves schema 12 and retained observations intact. Do not delete evidence or rewind generations.
Preserve the reviewed dirty work from other packets.

Return to root if completed packet 013 types need broader changes.
Also return if a listed root cannot preserve its ordinary failure behavior.
Also return for a new field policy, new completeness assumption, extra request, or required owner outside the permitted files.
Do not invent a successful command state to avoid an error boundary problem.

## Expected Report

Report changed files, exact converted roots, request profiles, and success/failure receipt consumers.
Map L38-01 through L38-18 to concrete behavioral checks.
Record exact commands, exit results, log paths, request counts, row mutations, and committed transactions.
Separate inherited shared-writer behavior from new caller behavior.
Report any deviation and every remaining presentation or caller-coverage limit.
Report document links and the shared STE result without claiming complete standard compliance.

## Preparation Checks

This packet is the only documentation file created for this assignment.
No document moved, no folder was added, and no root document was created or changed.
Root retains ownership of the phase plan, ADR, delivery order, and pending-check index.

Local links and diff whitespace are Green. The prospective disposable server passes a Python syntax check.
The shared STE check reports lexical findings and no structural findings.
Technical terms retain their source meaning. The raw STE result is not Green and does not prove full standard compliance.
No Cargo command, app launch, upstream request, or fixture setup ran during preparation.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:

- This packet, its authority documents, and all Files To Inspect.
- The shared STE and Rust development skills before prose or code changes.

Goal:

- Retain Library comparison and hydration observations through the shared writer.
- Preserve ordinary errors, committed receipts, and storage capsules through the application and Library state.

Constraints:

- Wait for root review, packet 013 completion, and explicit dispatch.
- Preserve exact request profiles, comparison fallback, hydration behavior, and storage-failure classification.
- Consume evidence before callback selection checks. Keep ordinary hydration errors silent.

Do not touch:

- Any file outside Files To Change, or any behavior in Excluded Scope.
- Shared status documents, upstream code, operator data, or active display selection.

Acceptance criteria:

- Prove L38-01 through L38-18 at their production command, callback, and view-model boundaries.
- Keep the ordinary and storage failure paths distinct. A failed underlying operation remains a failed command.

Test commands:

- Use the commands in Test Commands. Coordinate the combined suite with root.
- Rebuild the normal desktop binary after tests. Run link and shared STE checks for this packet.

At the end, report:

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Implementation Evidence - 2026-09-21

Only `CompareLibraryTrack` and `HydrateAlbumIdentity` gained observation recording.
Their include strings, request order, comparison fields, tag reads, and legacy hydration writes remain unchanged.
The comparison result loads current provider state after recording. Its exact local RSS binding stays outside legacy DTO fields.

One application helper drains the recorder after all fallible work.
Successful comparison receipts remain in `TrackContext`. Hydration receipts remain outside its projected facts.
Ordinary errors retain their exact query messages and committed receipts in `ObservedQueryFailure`.
Storage errors retain their typed cause, earlier receipts, and original response capsule when available.
Provider read failures retain `ProviderReadError::Storage`, including after a preceding response-write failure.

The production callbacks retain evidence before selection checks.
Ordinary comparison errors remain in the matching comparison panel. Ordinary hydration errors remain silent.
Storage failures use the existing narrow status owner. Retention preserves selection, removal confirmation, loading, and busy state.
Later successes and failures preserve earlier receipts and capsules.

### Changed Files

| File | Result |
| --- | --- |
| `src/application/queries/library.rs` | Converted both roots, added receipt assembly, and added live command tests. |
| `src/application/errors/command.rs` | Added the ordinary observed query failure with private debug output. |
| `src/library/app_impl.rs` | Added receipt consumers and tested the production callback helpers. |
| `src/view_models/library.rs` | Separated evidence retention from status changes and tested operation preservation. |
| `tests/architecture_tests.rs` | Replaced comparison exclusion with caller, consumer, and compatibility guards. |
| This packet | Recorded implementation and verification evidence. |

No document moved, no folder was added, and no repository-root document changed during this assignment.
Root owns the shared status records. No additional production owner was required.

### Behavioral Evidence Map

All new unit test names start with `adr_0075_library_observation_`.
The table gives their suffixes. Shared writer tests retain their existing prefixes.

| Criteria | Test evidence |
| --- | --- |
| L38-01, L38-02 | `comparison_requests_fallback_and_tags_stay_equal`. The fixture verifies committed request generations before HTTP receipt. |
| L38-03 | `comparison_requests_fallback_and_tags_stay_equal` and `comparison_repetition_preserves_routes_and_counts_mutations`. Embedded bytes, comparison rows, contributor values, and payment routes remain unchanged. |
| L38-04 | `hydration_repetition_counts_and_reopened_evidence`. One Index request produces the same legacy values and mutation counts. |
| L38-05 | `hydration_repetition_counts_and_reopened_evidence` and `comparison_returns_current_empty_and_failed_refresh`. Reopened copies preserve original JSON, RSS, unknown members, owners, and failed attempts. |
| L38-06, L38-12 | `storage_failures_stop_roots_and_keep_retry_input`. Allocation prevents HTTP. Failed response storage prevents later work. Verified rollback permits storage-only retry. |
| L38-07 | `ordinary_failures_keep_receipts_and_exact_causes` and `comparison_later_failures_preserve_receipts_and_read_errors`. These inject tag, legacy-write, local-read, allocation, and response-write failures. |
| L38-08 | `ordinary_failures_keep_receipts_and_exact_causes` and `live_callbacks_keep_success_and_failure_after_navigation`. Displays, details, clone equality, and private debug output are checked. |
| L38-09, L38-10 | `live_callbacks_keep_success_and_failure_after_navigation`. Production consumers receive success, ordinary failure, typed read failure, and response capsules after navigation. |
| L38-11 | `receipt_retention_preserves_confirmation_loading_and_busy_state` and the callback test above. Packet 014's capsule-preservation test remains active. |
| L38-13 | `interleaved_detail_comparison_and_hydration_share_exact_slots`. A held detail response completes after comparison fails. The newer failure survives. Hydration uses a separate profile. |
| L38-14, L38-18 | `comparison_returns_current_empty_and_failed_refresh` and `comparison_callback_keeps_empty_state_and_album_updates`. Current empty snapshots and separate failed refresh evidence reach the mounted frame. |
| L38-15 | Both repetition tests measure table mutations and committed transactions. Repeated evidence rows remain stable. |
| L38-16, L38-17 | `adr_0075_library_observation_callers_and_consumers_are_guarded` preserves suppression, panel caching, reload routes, and excluded callers. Packet 014/013 guards remain active. |

The storage-only replay test verifies zero additional requests, row mutations, and transactions for committed cloned-token replay.
Changed-input replay fails without mutations.
The inherited `adr_0075_observation_token_replay_is_idempotent_and_uncertain_commit_blocks_retry` test covers uncertain replay rejection.
This packet adds no retry action or completeness contract.

### Measured Requests And Mutations

These measurements use disposable databases, localhost responses, and temporary ID3 files.
Each response test verifies its request generation before sending the response.
Counts exclude fixture setup, redirects, and transport-internal behavior.

| Case | Index | RSS | Legacy rows | Evidence rows | Snapshot rows | Head rows | Committed transactions |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| First comparison with contributors and payment routes | 2 | 1 | 0 | 139 | 0 | 36 | 6 |
| Repeated comparison with unchanged responses | 2 | 1 | 0 | 12 | 0 | 36 | 6 |
| First hydration with the baseline feed fields | 1 | 0 | 5 | 57 | 0 | 12 | 8 |
| Repeated hydration with unchanged responses | 1 | 0 | 9 | 4 | 0 | 12 | 8 |

Evidence rows include request allocation, occurrence updates, bodies, coverage, facts, resources, and subjects.
Head mutations above record incomplete collection attempts. They do not indicate snapshot replacement.
The separate complete-RSS tests verify accepted populated and empty snapshots through the existing packet 013 contract.
Row mutations are not disk writes or byte counts. Hydration keeps its existing sequence of independently committed legacy writes.

Comparison with scoped fallback uses three Index requests and one RSS request.
Comparison with all Index responses returning HTTP 503 uses three Index requests and no RSS request.
Comparison without a local feed GUID uses two Index requests and one RSS request in the measured fixture.
A missing local path uses no request, generation allocation, or committed transaction.

### Mechanical Check Record

| Command | Exit | Result |
| --- | ---: | --- |
| `cargo test --locked --offline adr_0075_library_observation_ -- --nocapture` | 0 | Green. Eleven unit tests and one guard. |
| `cargo test --locked --offline adr_0075_observation_` | 0 | Green. Twenty unit tests and one guard. |
| `cargo test --locked --offline adr_0075_snapshot_` | 0 | Green. Eighteen unit tests and one guard. |
| `cargo test --locked --offline adr_0075_rss_` | 0 | Green. Thirty-three unit tests and one guard. |
| `cargo test --locked --offline --test architecture_tests` | 0 | Green. All 268 guards. |
| `cargo test --locked --offline adr_0075_ -- --test-threads=4` | 0 | Green after the Clippy correction. All 141 unit tests and five guards. |
| `cargo check --locked --offline` | 0 | Green after the Clippy correction. |
| `cargo clippy --locked --offline -- -D warnings` | 0 | Green after the parameter changed to a reference. |
| `cargo fmt -- --check` | 0 | Green after the Clippy correction. |
| `cargo test --locked --offline -- --test-threads=4` | 0 | Green. All 1,642 unit tests and 268 guards. Ten documentation tests remain ignored. |
| `cargo build --locked --offline --bin v4vmm` | 0 | Green. Normal binary rebuilt after all tests. |
| `git diff --check` | 0 | Green. |

Commands use direct Cargo invocation. Recorded output remains in the session transcript. No redirected log file was created.

Two test-fixture defects were corrected during development.
The first used a nonexistent description helper. The second used the storage field name in an API payload.
The corrected payload uses `release_kind` and reproduces the historical five and nine legacy row mutations.
Neither correction changed production field rules.

Strict Clippy also required the status-retention method to receive a reference.
The corrected method and its callers preserve status and evidence behavior.
No command required additional approval.

Local document links are Green. All 27 links resolve.
The shared STE check reports lexical findings and one inherited paragraph finding in Successful Results.
New paragraph findings were corrected. Technical names retain their existing meaning.

The raw STE result is not Green. It does not prove full standard compliance.

### Limits And Remaining Gates

The storage presentation gate remains open and paused. No app or headless display ran.
No production data, configuration, audio, upstream code, or shared writer changed.
Other caller families remain assigned to packets 039 through 044.
MusicIndex completeness and visible retry remain deferred. Existing field policies remain unchanged.

## Operator Visual Check

The storage-failure presentation gate remains open and paused. Run no step until the operator resumes visual checks.
The procedure extends [packet 014's isolated desktop fixture](adr-0075-task-014-provider-observation-retention.md#isolated-desktop-procedure).
Use a Linux desktop, Python 3, and the rebuilt normal binary. The fixture uses Null playback and isolated service stubs.
No failed production service or special hardware is required.

1. Complete packet 014's fixture setup step 1.
2. Save its `observation-server.py` from step 2, but do not start the server yet.
3. Extend only that disposable server with comparison/hydration selection and a local HTTP-failure marker.

   ```bash
   python3 - "$O14_ROOT" <<'PY'
   import sys
   from pathlib import Path
   root = Path(sys.argv[1])
   path = root / "observation-server.py"
   text = path.read_text()
   text = text.replace(
       "from urllib.parse import unquote, urlsplit",
       "from urllib.parse import parse_qs, unquote, urlsplit",
   )
   needle = '        arm = root / "observation-arm"\n'
   assert text.count(needle) == 1
   extra = '''        hydration = (
               path.startswith("/v1/feeds/") and "/tracks/" not in path
               and parse_qs(urlsplit(self.path).query).get("include") == [
                   "source_links,source_ids,source_release_claims,source_contributors"
               ]
           )
           if hydration and (root / "observation-http-failure").exists():
               self.send_error(503, "Isolated hydration failure")
               return
   '''
   text = text.replace(needle, extra + needle)
   needle = '        if arm.exists() and "/tracks/" in path:\n'
   assert text.count(needle) == 1
   replacement = '''        if arm.exists() and (
               (arm.read_text().strip() == "comparison" and "/tracks/" in path)
               or (arm.read_text().strip() == "hydration" and hydration)
           ):
   '''
   text = text.replace(needle, replacement)
   compile(text, str(path), "exec")
   path.write_text(text)
   PY
   python3 -B "$O14_ROOT/observation-server.py" "$O14_ROOT" > "$O14_ROOT/observation-server.log" 2>&1 &
   O14_SERVER_PID=$!
   printf '%s\n' "$O14_SERVER_PID" > "$O14_ROOT/observation-server.pid"
   ```

4. Wait for the endpoint file, then start the fixture through its existing launcher.

   ```bash
   while [ ! -s "$O14_ROOT/observation-endpoint" ]; do sleep 0.1; done
   python3 -B docs/runbooks/startup-recovery-fixture.py run "$O14_ROOT"
   ```

5. Set `O14_ROOT` to the printed fixture path in a second terminal at the repository root.
6. Define cleanup for each injected failure.

   ```bash
   o38_clear() {
       python3 - "$O14_ROOT" <<'PY'
   import sqlite3
   import sys
   from pathlib import Path
   root = Path(sys.argv[1])
   with sqlite3.connect(root / "data/library.sqlite") as conn:
       conn.execute("DROP TRIGGER IF EXISTS o14_reject")
   for name in ("observation-arm", "observation-ready", "observation-release", "observation-http-failure"):
       (root / name).unlink(missing_ok=True)
   PY
   }
   ```

7. Select `a.wav`, wait for source loading, and open **Compare ID3**. Verify that comparison rows load.
8. Temporarily move only the fixture's audio file, then click **Re-read**.

   ```bash
   mv "$O14_ROOT/music/a.wav" "$O14_ROOT/music/a.wav.o38-held"
   ```

9. Verify that the tag-read error stays in the comparison panel. It must not become a metadata storage-failure status.
10. Restore the fixture file before continuing.

    ```bash
    mv "$O14_ROOT/music/a.wav.o38-held" "$O14_ROOT/music/a.wav"
    ```

11. Select `b.wav` and wait for source loading. Arm storage failure, then open **Compare ID3**.

    ```bash
    printf '%s\n' comparison > "$O14_ROOT/observation-arm"
    ```

12. During the twelve-second server delay, select `c.wav`.
13. Verify that storage-failure status appears while `c.wav` stays selected. Its comparison panel must not receive `b.wav`'s error.
14. Clear the fixture trigger, then enable ordinary hydration HTTP failure.

    ```bash
    o38_clear
    touch "$O14_ROOT/observation-http-failure"
    ```

15. Open the fixture's Library album. The ordinary hydration failure must not create a new global error or replace existing album values.
16. Clear that marker, leave the album, then arm hydration storage failure.

    ```bash
    o38_clear
    printf '%s\n' hydration > "$O14_ROOT/observation-arm"
    ```

17. Reopen the album, then select `a.wav` during the twelve-second delay.
18. Verify that storage-failure status appears without changing the selected track or unrelated busy/loading state.
19. Treat successful persistence claims, lost selection, or canceled removal confirmation as incorrect.
20. Close the fixture app. Restore any held fixture audio file and remove the injected state.

    ```bash
    if [ -f "$O14_ROOT/music/a.wav.o38-held" ]; then
        mv "$O14_ROOT/music/a.wav.o38-held" "$O14_ROOT/music/a.wav"
    fi
    o38_clear
    kill "$(cat "$O14_ROOT/observation-server.pid")"
    python3 -B docs/runbooks/startup-recovery-fixture.py cleanup "$O14_ROOT"
    unset -f o38_clear
    unset O14_ROOT O14_SERVER_PID
    ```

This procedure is prospective. Mechanical checks cannot close its presentation gate or any inherited visual gate.
